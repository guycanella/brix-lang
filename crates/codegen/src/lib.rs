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

    variables: HashMap<String, PointerValue<'ctx>>,
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

        let mut last_val = i64_type.const_int(0, false).as_basic_value_enum();

        for stmt in &program.statements {
            match stmt {
                Stmt::VariableDecl { name, value, .. } => {
                    if let Some(val) = self.compile_expr(value) {
                        let alloca = self.builder.build_alloca(i64_type, name).unwrap();

                        self.builder
                            .build_store(alloca, val.into_int_value())
                            .unwrap();

                        self.variables.insert(name.clone(), alloca);

                        last_val = val;
                    }
                }

                Stmt::Assignment { target, value } => {
                    if let Some(ptr) = self.variables.get(target) {
                        if let Some(val) = self.compile_expr(value) {
                            self.builder
                                .build_store(*ptr, val.into_int_value())
                                .unwrap();
                            last_val = val;
                        }
                    } else {
                        eprintln!("Error: Variable '{}' not declared.", target);
                    }
                }

                Stmt::Expr(expr) => {
                    if let Some(val) = self.compile_expr(expr) {
                        last_val = val;
                    }
                }
            }
        }

        if last_val.is_int_value() {
            let _ = self.builder.build_return(Some(&last_val.into_int_value()));
        } else {
            let _ = self
                .builder
                .build_return(Some(&i64_type.const_int(0, false)));
        }
    }

    pub fn compile_expr(&self, expr: &Expr) -> Option<BasicValueEnum<'ctx>> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(i) => {
                    let i64_type = self.context.i64_type();
                    Some(i64_type.const_int(*i as u64, false).as_basic_value_enum())
                }
                _ => None, // Floats/Strings not implemented yet
            },

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

            // Bitwise operations
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

            _ => None, // Power (**) requires a math library function, skipped for now
        }
    }
}
