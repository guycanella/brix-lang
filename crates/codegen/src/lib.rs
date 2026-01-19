use inkwell::AddressSpace;
use inkwell::FloatPredicate;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum, FloatValue, IntValue, PointerValue};
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt};
use std::collections::HashMap;

pub struct Compiler<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a Module<'ctx>,
    pub variables: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>)>,
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

    fn get_printf(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function("printf") {
            return fn_val;
        }

        let i32_type = self.context.i32_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        let fn_type = i32_type.fn_type(&[ptr_type.into()], true);

        self.module
            .add_function("printf", fn_type, Some(Linkage::External))
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
                    let alloca: PointerValue;
                    let var_type: BasicTypeEnum;

                    if val.is_array_value() {
                        let array_val = val.into_array_value();
                        let array_type = array_val.get_type();
                        alloca = self.builder.build_alloca(array_type, name).unwrap();
                        self.builder.build_store(alloca, array_val).unwrap();

                        var_type = array_type.into();
                    } else if val.is_float_value() {
                        let f64_type = self.context.f64_type();
                        alloca = self.builder.build_alloca(f64_type, name).unwrap();
                        self.builder
                            .build_store(alloca, val.into_float_value())
                            .unwrap();

                        var_type = f64_type.into();
                    } else {
                        let i64_type = self.context.i64_type();
                        alloca = self.builder.build_alloca(i64_type, name).unwrap();
                        self.builder
                            .build_store(alloca, val.into_int_value())
                            .unwrap();

                        var_type = i64_type.into();
                    }

                    self.variables.insert(name.clone(), (alloca, var_type));
                }
            }

            Stmt::Assignment { target, value } => {
                if let Some((ptr, _)) = self.variables.get(target) {
                    if let Some(val) = self.compile_expr(value) {
                        self.builder.build_store(*ptr, val).unwrap();
                    }
                }
            }

            Stmt::Printf { format, args } => {
                let printf_fn = self.get_printf();
                let global_str = self
                    .builder
                    .build_global_string_ptr(format, "fmt_str")
                    .unwrap();

                use inkwell::values::BasicMetadataValueEnum;
                let mut compiled_args: Vec<BasicMetadataValueEnum> = Vec::new();

                compiled_args.push(global_str.as_pointer_value().into());

                for arg in args {
                    if let Some(val) = self.compile_expr(arg) {
                        compiled_args.push(val.into());
                    }
                }

                self.builder
                    .build_call(printf_fn, &compiled_args, "call_printf")
                    .unwrap();
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

                let _ = self
                    .builder
                    .build_conditional_branch(cond_bool, body_bb, after_bb);

                self.builder.position_at_end(body_bb);
                self.compile_stmt(body, function);
                let _ = self.builder.build_unconditional_branch(header_bb);

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
                Literal::Float(f) => {
                    let f64_type = self.context.f64_type();
                    Some(f64_type.const_float(*f).as_basic_value_enum())
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
                    if let Some((ptr, _)) = self.variables.get(name) {
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
                Some((ptr, var_type)) => {
                    let loaded = self.builder.build_load(*var_type, *ptr, name).unwrap();
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

                match (left_val, right_val) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        self.compile_int_op(op, l, r)
                    }
                    (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                        self.compile_float_op(op, l, r)
                    }
                    _ => {
                        eprintln!("Error: Mismatched types in binary operation.");
                        None
                    }
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

    fn compile_float_op(
        &self,
        op: &BinaryOp,
        lhs: FloatValue<'ctx>,
        rhs: FloatValue<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        match op {
            BinaryOp::Add => Some(
                self.builder
                    .build_float_add(lhs, rhs, "tmp_fadd")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::Sub => Some(
                self.builder
                    .build_float_sub(lhs, rhs, "tmp_fsub")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::Mul => Some(
                self.builder
                    .build_float_mul(lhs, rhs, "tmp_fmul")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::Div => Some(
                self.builder
                    .build_float_div(lhs, rhs, "tmp_fdiv")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::Mod => Some(
                self.builder
                    .build_float_rem(lhs, rhs, "tmp_frem")
                    .ok()?
                    .as_basic_value_enum(),
            ),

            // Bitwise does not exist for float, returns error or None
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => None,

            BinaryOp::Gt => self.compile_float_cmp(FloatPredicate::OGT, lhs, rhs),
            BinaryOp::Lt => self.compile_float_cmp(FloatPredicate::OLT, lhs, rhs),
            BinaryOp::GtEq => self.compile_float_cmp(FloatPredicate::OGE, lhs, rhs),
            BinaryOp::LtEq => self.compile_float_cmp(FloatPredicate::OLE, lhs, rhs),
            BinaryOp::Eq => self.compile_float_cmp(FloatPredicate::OEQ, lhs, rhs),
            BinaryOp::NotEq => self.compile_float_cmp(FloatPredicate::ONE, lhs, rhs),
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

    fn compile_float_cmp(
        &self,
        pred: FloatPredicate,
        lhs: FloatValue<'ctx>,
        rhs: FloatValue<'ctx>,
    ) -> Option<BasicValueEnum<'ctx>> {
        let bool_val = self
            .builder
            .build_float_compare(pred, lhs, rhs, "tmp_fcmp")
            .ok()?;
        let i64_type = self.context.i64_type();
        // Convert bool (1 bit) to int (64 bits) to keep consistency
        let int_val = self
            .builder
            .build_int_z_extend(bool_val, i64_type, "bool_to_int")
            .ok()?;
        Some(int_val.as_basic_value_enum())
    }
}
