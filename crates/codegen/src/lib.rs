use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt};
use std::collections::HashMap;

pub struct Compiler<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a Module<'ctx>,
    pub variables: HashMap<String, PointerValue<'ctx>>,
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub fn new(
        context: &'ctx Context,
        builder: &'a Builder<'ctx>,
        module: &'a Module<'ctx>,
    ) -> Self {
        Self {
            context,
            builder,
            module,
            variables: HashMap::new(),
        }
    }

    pub fn compile_program(&mut self, program: &Program) {
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[], false);
        let function = self.module.add_function("main", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");

        self.builder.position_at_end(basic_block);

        for stmt in &program.statements {
            self.compile_stmt(stmt, function);
        }

        let _ = self
            .builder
            .build_return(Some(&i64_type.const_int(0, false)));
    }

    fn compile_stmt(&mut self, stmt: &Stmt, function: inkwell::values::FunctionValue<'ctx>) {
        match stmt {
            Stmt::VariableDecl { name, value, .. } => {
                if let Some(val) = self.compile_expr(value) {
                    if val.is_array_value() {
                        let array_val = val.into_array_value();
                        let array_type = array_val.get_type();
                        let alloca = self.builder.build_alloca(array_type, name).unwrap();

                        self.builder.build_store(alloca, array_val).unwrap();
                        self.variables.insert(name.clone(), alloca);
                    } else {
                        let i64_type = self.context.i64_type();
                        let alloca = self.builder.build_alloca(i64_type, name).unwrap();

                        self.builder
                            .build_store(alloca, val.into_int_value())
                            .unwrap();
                        self.variables.insert(name.clone(), alloca);
                    }
                }
            }

            Stmt::Assignment { target, value } => {
                if let Some(ptr) = self.variables.get(target) {
                    if let Some(val) = self.compile_expr(value) {
                        self.builder
                            .build_store(*ptr, val.into_int_value())
                            .unwrap();
                    }
                } else {
                    eprintln!("Error: Variable '{}' not declared.", target);
                }
            }

            Stmt::Expr(expr) => {
                self.compile_expr(expr);
            }

            Stmt::Block(statements) => {
                for s in statements {
                    self.compile_stmt(s, function);
                }
            }

            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond_val = self.compile_expr(condition).unwrap().into_int_value();

                let i64_type = self.context.i64_type();
                let zero = i64_type.const_int(0, false);
                let cond_bool = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::NE, cond_val, zero, "ifcond")
                    .unwrap();

                let then_bb = self.context.append_basic_block(function, "then_block");
                let else_bb = self.context.append_basic_block(function, "else_block");
                let merge_bb = self.context.append_basic_block(function, "merge_block");

                let _ = self
                    .builder
                    .build_conditional_branch(cond_bool, then_bb, else_bb);

                self.builder.position_at_end(then_bb);
                self.compile_stmt(then_block, function);
                let _ = self.builder.build_unconditional_branch(merge_bb);

                self.builder.position_at_end(else_bb);
                if let Some(else_stmt) = else_block {
                    self.compile_stmt(else_stmt, function);
                }
                let _ = self.builder.build_unconditional_branch(merge_bb);

                self.builder.position_at_end(merge_bb);
            }

            Stmt::While { condition, body } => {
                let header_bb = self.context.append_basic_block(function, "while_header");
                let body_bb = self.context.append_basic_block(function, "while_body");
                let after_bb = self.context.append_basic_block(function, "while_after");

                let _ = self.builder.build_unconditional_branch(header_bb);

                self.builder.position_at_end(header_bb);

                let cond_val = self.compile_expr(condition).unwrap().into_int_value();
                let i64_type = self.context.i64_type();
                let zero = i64_type.const_int(0, false);

                let cond_bool = self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::NE, cond_val, zero, "loop_cond")
                    .unwrap();

                // If true -> Go to Body. If false -> Go to After.
                let _ = self
                    .builder
                    .build_conditional_branch(cond_bool, body_bb, after_bb);

                // --- BODY BLOCK ---
                self.builder.position_at_end(body_bb);

                self.compile_stmt(body, function);

                // CRITICAL: Jump back to Header to check condition again!
                let _ = self.builder.build_unconditional_branch(header_bb);

                // Continue the rest of the program
                self.builder.position_at_end(after_bb);
            }
        }
    }

    pub fn compile_expr(&self, expr: &Expr) -> Option<BasicValueEnum<'ctx>> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(i) => {
                    let i64_type = self.context.i64_type();
                    Some(i64_type.const_int(*i as u64, false).as_basic_value_enum())
                }
                _ => None,
            },

            Expr::Array(elements) => {
                let i64_type = self.context.i64_type();

                let mut compiled_elements = Vec::new();
                for el in elements {
                    let val = self.compile_expr(el)?.into_int_value();
                    compiled_elements.push(val);
                }

                let const_array = i64_type.const_array(&compiled_elements);
                Some(const_array.as_basic_value_enum())
            }

            Expr::Index { array, index } => {
                let index_val = self.compile_expr(index)?.into_int_value();

                if let Expr::Identifier(name) = array.as_ref() {
                    if let Some(ptr) = self.variables.get(name) {
                        let i64_type = self.context.i64_type();
                        let zero = i64_type.const_int(0, false);

                        unsafe {
                            let item_ptr = self
                                .builder
                                .build_gep(i64_type, *ptr, &[zero, index_val], "array_item_ptr")
                                .ok()?;

                            let loaded = self
                                .builder
                                .build_load(i64_type, item_ptr, "array_item")
                                .ok()?;
                            return Some(loaded);
                        }
                    }
                }
                None
            }

            Expr::Identifier(name) => match self.variables.get(name) {
                Some(ptr) => {
                    let i64_type = self.context.i64_type();
                    let loaded = self.builder.build_load(i64_type, *ptr, name).unwrap();
                    Some(loaded)
                }
                None => {
                    eprintln!("Error: Variable '{}' not found.", name);
                    None
                }
            },

            Expr::Binary { op, lhs, rhs } => {
                let left_val = self.compile_expr(lhs)?;
                let right_val = self.compile_expr(rhs)?;

                if left_val.is_int_value() && right_val.is_int_value() {
                    let l_int = left_val.into_int_value();
                    let r_int = right_val.into_int_value();
                    self.compile_int_op(op, l_int, r_int)
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    fn compile_int_op(
        &self,
        op: &BinaryOp,
        lhs: IntValue<'ctx>,
        rhs: IntValue<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        match op {
            BinaryOp::Add => Some(
                self.builder
                    .build_int_add(lhs, rhs, "tmp_add")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::Sub => Some(
                self.builder
                    .build_int_sub(lhs, rhs, "tmp_sub")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::Mul => Some(
                self.builder
                    .build_int_mul(lhs, rhs, "tmp_mul")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::Div => Some(
                self.builder
                    .build_int_signed_div(lhs, rhs, "tmp_div")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::Mod => Some(
                self.builder
                    .build_int_signed_rem(lhs, rhs, "tmp_mod")
                    .ok()?
                    .as_basic_value_enum(),
            ),

            BinaryOp::BitAnd => Some(
                self.builder
                    .build_and(lhs, rhs, "tmp_and")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::BitOr => Some(
                self.builder
                    .build_or(lhs, rhs, "tmp_or")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::BitXor => Some(
                self.builder
                    .build_xor(lhs, rhs, "tmp_xor")
                    .ok()?
                    .as_basic_value_enum(),
            ),

            BinaryOp::Gt => self.compile_cmp(inkwell::IntPredicate::SGT, lhs, rhs),
            BinaryOp::Lt => self.compile_cmp(inkwell::IntPredicate::SLT, lhs, rhs),
            BinaryOp::GtEq => self.compile_cmp(inkwell::IntPredicate::SGE, lhs, rhs),
            BinaryOp::LtEq => self.compile_cmp(inkwell::IntPredicate::SLE, lhs, rhs),
            BinaryOp::Eq => self.compile_cmp(inkwell::IntPredicate::EQ, lhs, rhs),
            BinaryOp::NotEq => self.compile_cmp(inkwell::IntPredicate::NE, lhs, rhs),

            _ => None,
        }
    }

    fn compile_cmp(
        &self,
        pred: inkwell::IntPredicate,
        lhs: IntValue<'ctx>,
        rhs: IntValue<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        let bool_val = self
            .builder
            .build_int_compare(pred, lhs, rhs, "tmp_cmp")
            .ok()?;

        let i64_type = self.context.i64_type();

        let int_val = self
            .builder
            .build_int_z_extend(bool_val, i64_type, "bool_to_int")
            .ok()?;
        Some(int_val.as_basic_value_enum())
    }
}
