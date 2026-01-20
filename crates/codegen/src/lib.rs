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

    // --- EXTERNAL FUNCTIONS (LibC) ---

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

    fn get_scanf(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function("scanf") {
            return fn_val;
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        // scanf returns i32 and accepts variable pointers
        let fn_type = i32_type.fn_type(&[ptr_type.into()], true);
        self.module
            .add_function("scanf", fn_type, Some(Linkage::External))
    }

    // --- MAIN COMPILATION ---

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
                    let (alloca, var_type) = if val.is_array_value() {
                        let array_val = val.into_array_value();
                        let array_type = array_val.get_type();
                        let ptr = self.builder.build_alloca(array_type, name).unwrap();
                        self.builder.build_store(ptr, array_val).unwrap();
                        (ptr, array_type.into())
                    } else if val.is_float_value() {
                        let f64_type = self.context.f64_type();
                        let ptr = self.builder.build_alloca(f64_type, name).unwrap();
                        self.builder
                            .build_store(ptr, val.into_float_value())
                            .unwrap();
                        (ptr, f64_type.into())
                    } else if val.is_pointer_value() {
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let ptr = self.builder.build_alloca(ptr_type, name).unwrap();
                        self.builder
                            .build_store(ptr, val.into_pointer_value())
                            .unwrap();
                        (ptr, ptr_type.into())
                    } else {
                        let i64_type = self.context.i64_type();
                        let ptr = self.builder.build_alloca(i64_type, name).unwrap();
                        self.builder.build_store(ptr, val.into_int_value()).unwrap();
                        (ptr, i64_type.into())
                    };

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
                Literal::Bool(b) => {
                    let i64_type = self.context.i64_type();
                    let val = if *b { 1 } else { 0 };
                    Some(i64_type.const_int(val, false).as_basic_value_enum())
                }
                Literal::String(s) => {
                    let s_val = self.context.const_string(s.as_bytes(), true);
                    let global = self.module.add_global(s_val.get_type(), None, "str_lit");
                    global.set_initializer(&s_val);
                    global.set_linkage(Linkage::Internal);
                    let ptr = global.as_pointer_value();
                    let zero = self.context.i64_type().const_int(0, false);
                    let i8_ptr = unsafe {
                        self.builder
                            .build_gep(s_val.get_type(), ptr, &[zero, zero], "str_ptr")
                            .ok()?
                    };
                    Some(i8_ptr.as_basic_value_enum())
                }
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

            Expr::Call { func, args } => {
                if let Expr::Identifier(fn_name) = func.as_ref() {
                    if fn_name == "input" {
                        if args.len() == 0 {
                            eprintln!("Error: input() requires a type ('int', 'float', 'string').");
                            return None;
                        }
                        if let Expr::Literal(Literal::String(type_str)) = &args[0] {
                            match type_str.as_str() {
                                "int" => return self.compile_input_int(),
                                "float" => return self.compile_input_float(),
                                "string" => return self.compile_input_string(),
                                _ => {
                                    eprintln!("Error: Unknown input type '{}'.", type_str);
                                    return None;
                                }
                            }
                        }
                    }

                    if fn_name == "matrix" {
                        return self.compile_matrix_constructor(&args);
                    }

                    if fn_name == "read_csv" {
                        if args.len() != 1 {
                            eprintln!("Error: read_csv requires 1 argument (file name).");
                            return None;
                        }

                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);

                        let read_csv_fn =
                            self.module.get_function("read_csv").unwrap_or_else(|| {
                                self.module.add_function(
                                    "read_csv",
                                    fn_type,
                                    Some(Linkage::External),
                                )
                            });

                        let filename_arg = self.compile_expr(&args[0])?;
                        let call = self
                            .builder
                            .build_call(read_csv_fn, &[filename_arg.into()], "call_read_csv")
                            .unwrap();

                        return Some(call.try_as_basic_value().left().unwrap());
                    }

                    let mut compiled_args = Vec::new();
                    for arg in args {
                        let val = self.compile_expr(arg)?;
                        compiled_args.push(val.into());
                    }

                    let function = if let Some(f) = self.module.get_function(fn_name) {
                        f
                    } else {
                        let f64_type = self.context.f64_type();
                        let arg_types: Vec<inkwell::types::BasicMetadataTypeEnum> =
                            args.iter().map(|_| f64_type.into()).collect();
                        let fn_type = f64_type.fn_type(&arg_types, false);
                        self.module
                            .add_function(fn_name, fn_type, Some(Linkage::External))
                    };

                    let call_val = self
                        .builder
                        .build_call(function, &compiled_args, "tmp_call")
                        .unwrap();
                    return Some(call_val.try_as_basic_value().left().unwrap());
                }
                None
            }

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

            Expr::FieldAccess { target, field } => {
                let target_ptr = self.compile_expr(target)?.into_pointer_value();
                let matrix_type = self.get_matrix_type();

                let index = match field.as_str() {
                    "rows" => 0,
                    "cols" => 1,
                    "data" => 2,
                    _ => {
                        eprintln!("Erro: Campo desconhecido '{}'.", field);
                        return None;
                    }
                };

                let field_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, target_ptr, index, "field_ptr")
                    .unwrap();

                let loaded_val = match index {
                    0 | 1 => self
                        .builder
                        .build_load(self.context.i64_type(), field_ptr, "load_field")
                        .unwrap(),

                    _ => self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            field_ptr,
                            "load_ptr",
                        )
                        .unwrap(),
                };

                Some(loaded_val)
            }

            Expr::Match { .. } => {
                eprintln!("Warning: 'Match' expression not implemented in backend yet.");
                None
            }
        }
    }

    fn compile_input_int(&self) -> Option<BasicValueEnum<'ctx>> {
        let scanf_fn = self.get_scanf();
        let i64_type = self.context.i64_type();

        let alloca = self
            .builder
            .build_alloca(i64_type, "input_int_tmp")
            .unwrap();

        let format_str = self.context.const_string(b"%lld\0", true);
        let global_fmt = self
            .module
            .add_global(format_str.get_type(), None, "fmt_scan_int");
        global_fmt.set_initializer(&format_str);
        global_fmt.set_linkage(Linkage::Internal);

        let zero = i64_type.const_int(0, false);
        let fmt_ptr = unsafe {
            self.builder
                .build_gep(
                    format_str.get_type(),
                    global_fmt.as_pointer_value(),
                    &[zero, zero],
                    "fmt_ptr",
                )
                .ok()?
        };

        self.builder
            .build_call(scanf_fn, &[fmt_ptr.into(), alloca.into()], "call_scanf")
            .unwrap();
        let val = self
            .builder
            .build_load(i64_type, alloca, "read_int")
            .unwrap();
        Some(val)
    }

    fn compile_input_float(&self) -> Option<BasicValueEnum<'ctx>> {
        let scanf_fn = self.get_scanf();
        let f64_type = self.context.f64_type();
        let i64_type = self.context.i64_type();

        let alloca = self
            .builder
            .build_alloca(f64_type, "input_float_tmp")
            .unwrap();

        let format_str = self.context.const_string(b"%lf\0", true);
        let global_fmt = self
            .module
            .add_global(format_str.get_type(), None, "fmt_scan_float");
        global_fmt.set_initializer(&format_str);
        global_fmt.set_linkage(Linkage::Internal);

        let zero = i64_type.const_int(0, false);
        let fmt_ptr = unsafe {
            self.builder
                .build_gep(
                    format_str.get_type(),
                    global_fmt.as_pointer_value(),
                    &[zero, zero],
                    "fmt_ptr",
                )
                .ok()?
        };

        self.builder
            .build_call(scanf_fn, &[fmt_ptr.into(), alloca.into()], "call_scanf")
            .unwrap();
        let val = self
            .builder
            .build_load(f64_type, alloca, "read_float")
            .unwrap();
        Some(val)
    }

    fn compile_input_string(&self) -> Option<BasicValueEnum<'ctx>> {
        let scanf_fn = self.get_scanf();
        let i64_type = self.context.i64_type();

        // 1. Cria buffer de 256 bytes na pilha [256 x i8]
        let array_type = self.context.i8_type().array_type(256);
        let alloca = self
            .builder
            .build_alloca(array_type, "input_str_buffer")
            .unwrap();

        // 2. Formato "%s" (lê string até espaço ou enter)
        let format_str = self.context.const_string(b"%s\0", true);
        let global_fmt = self
            .module
            .add_global(format_str.get_type(), None, "fmt_scan_str");
        global_fmt.set_initializer(&format_str);
        global_fmt.set_linkage(Linkage::Internal);

        let zero = i64_type.const_int(0, false);
        let fmt_ptr = unsafe {
            self.builder
                .build_gep(
                    format_str.get_type(),
                    global_fmt.as_pointer_value(),
                    &[zero, zero],
                    "fmt_ptr",
                )
                .ok()?
        };
        let buffer_ptr = unsafe {
            self.builder
                .build_gep(array_type, alloca, &[zero, zero], "buff_ptr")
                .ok()?
        };

        self.builder
            .build_call(scanf_fn, &[fmt_ptr.into(), buffer_ptr.into()], "call_scanf")
            .unwrap();

        Some(buffer_ptr.as_basic_value_enum())
    }

    // --- MATH OPERATORS ---

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
            BinaryOp::BitAnd | BinaryOp::LogicalAnd => Some(
                self.builder
                    .build_and(lhs, rhs, "tmp_and")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::BitOr | BinaryOp::LogicalOr => Some(
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
        let int_val = self
            .builder
            .build_int_z_extend(bool_val, i64_type, "bool_to_int")
            .ok()?;
        Some(int_val.as_basic_value_enum())
    }

    fn get_matrix_type(&self) -> inkwell::types::StructType<'ctx> {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        self.context
            .struct_type(&[i64_type.into(), i64_type.into(), ptr_type.into()], false)
    }

    fn compile_matrix_constructor(&self, args: &[Expr]) -> Option<BasicValueEnum<'ctx>> {
        if args.len() != 2 {
            eprintln!("Erro: matrix() requer 2 argumentos: matrix(rows, cols)");
            return None;
        }

        let rows_val = self.compile_expr(&args[0])?.into_int_value();
        let cols_val = self.compile_expr(&args[1])?.into_int_value();

        let matrix_struct_type = self.get_matrix_type();
        let matrix_ptr = self
            .builder
            .build_alloca(matrix_struct_type, "matrix_obj")
            .unwrap();

        let rows_ptr = self
            .builder
            .build_struct_gep(matrix_struct_type, matrix_ptr, 0, "rows_ptr")
            .unwrap();
        self.builder.build_store(rows_ptr, rows_val).unwrap();

        let cols_ptr = self
            .builder
            .build_struct_gep(matrix_struct_type, matrix_ptr, 1, "cols_ptr")
            .unwrap();
        self.builder.build_store(cols_ptr, cols_val).unwrap();

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let null_ptr = ptr_type.const_null();

        let data_field_ptr = self
            .builder
            .build_struct_gep(matrix_struct_type, matrix_ptr, 2, "data_ptr")
            .unwrap();
        self.builder.build_store(data_field_ptr, null_ptr).unwrap();

        Some(matrix_ptr.as_basic_value_enum())
    }
}
