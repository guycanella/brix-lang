use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum, FloatValue, IntValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt};
use std::collections::HashMap;

// --- BRIX TYPE SYSTEM ---
#[derive(Debug, Clone, PartialEq)]
pub enum BrixType {
    Int,
    Float,
    String,
    Matrix,
    FloatPtr,
    Void,
}

pub struct Compiler<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a Module<'ctx>,
    pub variables: HashMap<String, (PointerValue<'ctx>, BrixType)>,
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

    // --- AUXILIARY LLVM FUNCTIONS ---

    fn create_entry_block_alloca(&self, ty: BasicTypeEnum<'ctx>, name: &str) -> PointerValue<'ctx> {
        let builder = self.context.create_builder();

        let entry = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap()
            .get_first_basic_block()
            .unwrap();

        match entry.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry),
        }

        builder.build_alloca(ty, name).unwrap()
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
            Stmt::VariableDecl {
                name,
                value,
                is_const: _,
            } => {
                if let Some((init_val, brix_type)) = self.compile_expr(value) {
                    // Choose LLVM type based on BrixType
                    let llvm_type: BasicTypeEnum = match brix_type {
                        BrixType::Int => self.context.i64_type().into(),
                        BrixType::Float => self.context.f64_type().into(),
                        // Strings, Matrices and Pointers are stored as ptr (pointer)
                        BrixType::String | BrixType::Matrix | BrixType::FloatPtr => {
                            self.context.ptr_type(AddressSpace::default()).into()
                        }
                        _ => self.context.i64_type().into(),
                    };

                    let alloca = self.create_entry_block_alloca(llvm_type, name);
                    self.builder.build_store(alloca, init_val).unwrap();

                    self.variables.insert(name.clone(), (alloca, brix_type));
                }
            }

            Stmt::Assignment { target, value } => {
                if let Some((ptr, _)) = self.variables.get(target) {
                    if let Some((val, _)) = self.compile_expr(value) {
                        self.builder.build_store(*ptr, val).unwrap();
                    }
                } else {
                    eprintln!("Erro: Variável '{}' não declarada.", target);
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
                    if let Some((val, _)) = self.compile_expr(arg) {
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
                let (cond_val, _) = self.compile_expr(condition).unwrap();
                let cond_int = cond_val.into_int_value(); // Assume int (booleano)

                let i64_type = self.context.i64_type();
                let zero = i64_type.const_int(0, false);
                let cond_bool = self
                    .builder
                    .build_int_compare(IntPredicate::NE, cond_int, zero, "ifcond")
                    .unwrap();

                let then_bb = self.context.append_basic_block(function, "then_block");
                let else_bb = self.context.append_basic_block(function, "else_block");
                let merge_bb = self.context.append_basic_block(function, "merge_block");

                let _ = self
                    .builder
                    .build_conditional_branch(cond_bool, then_bb, else_bb);

                // THEN
                self.builder.position_at_end(then_bb);
                self.compile_stmt(then_block, function);
                let _ = self.builder.build_unconditional_branch(merge_bb);

                // ELSE
                self.builder.position_at_end(else_bb);
                if let Some(else_stmt) = else_block {
                    self.compile_stmt(else_stmt, function);
                }
                let _ = self.builder.build_unconditional_branch(merge_bb);

                // MERGE
                self.builder.position_at_end(merge_bb);
            }

            Stmt::While { condition, body } => {
                let header_bb = self.context.append_basic_block(function, "while_header");
                let body_bb = self.context.append_basic_block(function, "while_body");
                let after_bb = self.context.append_basic_block(function, "while_after");

                let _ = self.builder.build_unconditional_branch(header_bb);
                self.builder.position_at_end(header_bb);

                let (cond_val, _) = self.compile_expr(condition).unwrap();
                let cond_int = cond_val.into_int_value();

                let i64_type = self.context.i64_type();
                let zero = i64_type.const_int(0, false);
                let cond_bool = self
                    .builder
                    .build_int_compare(IntPredicate::NE, cond_int, zero, "loop_cond")
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

    fn compile_expr(&self, expr: &Expr) -> Option<(BasicValueEnum<'ctx>, BrixType)> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(n) => {
                    let val = self.context.i64_type().const_int(*n as u64, false);
                    Some((val.into(), BrixType::Int))
                }
                Literal::Float(n) => {
                    let val = self.context.f64_type().const_float(*n);
                    Some((val.into(), BrixType::Float))
                }
                Literal::String(s) => {
                    let str_val = self.builder.build_global_string_ptr(s, "str_lit").unwrap();
                    Some((str_val.as_pointer_value().into(), BrixType::String))
                }
                Literal::Bool(b) => {
                    let val = self.context.bool_type().const_int(*b as u64, false);
                    Some((val.into(), BrixType::Int))
                }
            },

            Expr::Identifier(name) => match self.variables.get(name) {
                Some((ptr, brix_type)) => match brix_type {
                    BrixType::Matrix | BrixType::String | BrixType::FloatPtr => {
                        let val = self
                            .builder
                            .build_load(self.context.ptr_type(AddressSpace::default()), *ptr, name)
                            .unwrap();
                        Some((val, brix_type.clone()))
                    }

                    BrixType::Int => {
                        let val = self
                            .builder
                            .build_load(self.context.i64_type(), *ptr, name)
                            .unwrap();
                        Some((val, BrixType::Int))
                    }
                    BrixType::Float => {
                        let val = self
                            .builder
                            .build_load(self.context.f64_type(), *ptr, name)
                            .unwrap();
                        Some((val, BrixType::Float))
                    }
                    _ => {
                        eprintln!("Erro: Tipo não suportado em identificador.");
                        None
                    }
                },
                None => {
                    eprintln!("Error: Variable '{}' not found.", name);
                    None
                }
            },

            Expr::Binary { op, lhs, rhs } => {
                let (lhs_val, lhs_type) = self.compile_expr(lhs)?;
                let (rhs_val, rhs_type) = self.compile_expr(rhs)?;

                let is_float_op =
                    matches!(lhs_type, BrixType::Float) || matches!(rhs_type, BrixType::Float);

                if is_float_op {
                    // Operações com Float
                    // Nota: Aqui assumimos conversão implícita ou que ambos já são float
                    let val = self.compile_float_op(
                        op,
                        lhs_val.into_float_value(),
                        rhs_val.into_float_value(),
                    )?;

                    // Comparações retornam Int (0 ou 1), Aritmética retorna Float
                    let res_type = match op {
                        BinaryOp::Gt
                        | BinaryOp::Lt
                        | BinaryOp::GtEq
                        | BinaryOp::LtEq
                        | BinaryOp::Eq
                        | BinaryOp::NotEq => BrixType::Int,
                        _ => BrixType::Float,
                    };
                    Some((val, res_type))
                } else {
                    // Operações com Int
                    let val = self.compile_int_op(
                        op,
                        lhs_val.into_int_value(),
                        rhs_val.into_int_value(),
                    )?;
                    Some((val, BrixType::Int))
                }
            }

            Expr::Call { func, args } => {
                if let Expr::Identifier(fn_name) = func.as_ref() {
                    if fn_name == "input" {
                        return self.compile_input_call(args);
                    }
                    if fn_name == "matrix" {
                        let val = self.compile_matrix_constructor(args)?;
                        return Some((val, BrixType::Matrix));
                    }
                    if fn_name == "read_csv" {
                        let ptr = self.compile_read_csv(args)?;
                        return Some((ptr, BrixType::Matrix));
                    }
                }
                eprintln!("Função desconhecida: {:?}", func);
                None
            }

            Expr::FieldAccess { target, field } => {
                let (target_val, target_type) = self.compile_expr(target)?;

                if target_type == BrixType::Matrix {
                    let target_ptr = target_val.into_pointer_value();
                    let matrix_type = self.get_matrix_type();

                    let index = match field.as_str() {
                        "rows" => 0,
                        "cols" => 1,
                        "data" => 2,
                        _ => return None,
                    };

                    let field_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, target_ptr, index, "field_ptr")
                        .unwrap();

                    let val = match index {
                        0 | 1 => {
                            let v = self
                                .builder
                                .build_load(self.context.i64_type(), field_ptr, "load_field")
                                .unwrap();
                            (v, BrixType::Int)
                        }
                        _ => {
                            let v = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    field_ptr,
                                    "load_ptr",
                                )
                                .unwrap();
                            (v, BrixType::FloatPtr)
                        }
                    };
                    return Some(val);
                }
                eprintln!("Type error: Access field on non-matrix.");
                None
            }

            Expr::Index { array, index } => {
                let (target_val, target_type) = self.compile_expr(array)?;
                let (index_val, _) = self.compile_expr(index)?;
                let index_int = index_val.into_int_value();

                match target_type {
                    BrixType::Matrix => {
                        // m[i] -> Retorna ponteiro para o inicio da linha
                        let ptr = target_val.into_pointer_value();
                        let matrix_type = self.get_matrix_type();

                        // Cols
                        let cols_ptr = self
                            .builder
                            .build_struct_gep(matrix_type, ptr, 1, "cols")
                            .unwrap();
                        let cols = self
                            .builder
                            .build_load(self.context.i64_type(), cols_ptr, "cols")
                            .unwrap()
                            .into_int_value();

                        // Data
                        let data_ptr_ptr = self
                            .builder
                            .build_struct_gep(matrix_type, ptr, 2, "data")
                            .unwrap();
                        let data = self
                            .builder
                            .build_load(
                                self.context.ptr_type(AddressSpace::default()),
                                data_ptr_ptr,
                                "data",
                            )
                            .unwrap()
                            .into_pointer_value();

                        // Offset = i * cols
                        let offset = self
                            .builder
                            .build_int_mul(index_int, cols, "offset")
                            .unwrap();

                        unsafe {
                            let f64 = self.context.f64_type();
                            let row_start = self
                                .builder
                                .build_gep(f64, data, &[offset], "row_ptr")
                                .unwrap();
                            Some((row_start.as_basic_value_enum(), BrixType::FloatPtr))
                        }
                    }

                    BrixType::FloatPtr => {
                        // (m[i])[j] -> Retorna o float
                        let ptr = target_val.into_pointer_value();
                        unsafe {
                            let f64 = self.context.f64_type();
                            let item_ptr = self
                                .builder
                                .build_gep(f64, ptr, &[index_int], "item_ptr")
                                .unwrap();
                            let val = self.builder.build_load(f64, item_ptr, "val").unwrap();
                            Some((val, BrixType::Float))
                        }
                    }

                    _ => {
                        eprintln!("Type error: Trying to index {:?}", target_type);
                        None
                    }
                }
            }

            _ => {
                eprintln!("Expressão não implementada na v0.3");
                None
            }
        }
    }

    // --- HELPER FUNCTIONS ---

    fn compile_input_call(&self, args: &[Expr]) -> Option<(BasicValueEnum<'ctx>, BrixType)> {
        let arg_str = if args.len() > 0 {
            if let Expr::Literal(Literal::String(s)) = &args[0] {
                s.as_str()
            } else {
                "string"
            }
        } else {
            "string"
        };

        match arg_str {
            "int" => {
                let val = self.compile_input_int()?;
                Some((val, BrixType::Int))
            }
            "float" => {
                let val = self.compile_input_float()?;
                Some((val, BrixType::Float))
            }
            _ => {
                let val = self.compile_input_string()?;
                Some((val, BrixType::String))
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

        let zero = self.context.i64_type().const_int(0, false);
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
        let array_type = self.context.i8_type().array_type(256);
        let alloca = self
            .builder
            .build_alloca(array_type, "input_str_buffer")
            .unwrap();

        let format_str = self.context.const_string(b"%s\0", true);
        let global_fmt = self
            .module
            .add_global(format_str.get_type(), None, "fmt_scan_str");
        global_fmt.set_initializer(&format_str);
        global_fmt.set_linkage(Linkage::Internal);

        let zero = self.context.i64_type().const_int(0, false);
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

    fn compile_read_csv(&self, args: &[Expr]) -> Option<BasicValueEnum<'ctx>> {
        if args.len() != 1 {
            eprintln!("Erro: read_csv requer 1 argumento.");
            return None;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);

        let read_csv_fn = self.module.get_function("read_csv").unwrap_or_else(|| {
            self.module
                .add_function("read_csv", fn_type, Some(Linkage::External))
        });

        let (filename_arg, _) = self.compile_expr(&args[0])?;
        let call = self
            .builder
            .build_call(read_csv_fn, &[filename_arg.into()], "call_read_csv")
            .unwrap();

        Some(call.try_as_basic_value().left().unwrap())
    }

    fn get_matrix_type(&self) -> inkwell::types::StructType<'ctx> {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        self.context
            .struct_type(&[i64_type.into(), i64_type.into(), ptr_type.into()], false)
    }

    fn compile_matrix_constructor(&self, args: &[Expr]) -> Option<BasicValueEnum<'ctx>> {
        if args.len() != 2 {
            return None;
        }
        let (rows_val, _) = self.compile_expr(&args[0])?;
        let (cols_val, _) = self.compile_expr(&args[1])?;

        let matrix_struct_type = self.get_matrix_type();
        let matrix_ptr = self
            .builder
            .build_alloca(matrix_struct_type, "matrix_obj")
            .unwrap();

        let rows_ptr = self
            .builder
            .build_struct_gep(matrix_struct_type, matrix_ptr, 0, "rows_ptr")
            .unwrap();
        self.builder
            .build_store(rows_ptr, rows_val.into_int_value())
            .unwrap();

        let cols_ptr = self
            .builder
            .build_struct_gep(matrix_struct_type, matrix_ptr, 1, "cols_ptr")
            .unwrap();
        self.builder
            .build_store(cols_ptr, cols_val.into_int_value())
            .unwrap();

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let null_ptr = ptr_type.const_null();
        let data_field_ptr = self
            .builder
            .build_struct_gep(matrix_struct_type, matrix_ptr, 2, "data_ptr")
            .unwrap();
        self.builder.build_store(data_field_ptr, null_ptr).unwrap();

        Some(matrix_ptr.as_basic_value_enum())
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
            BinaryOp::Gt => self.compile_cmp(IntPredicate::SGT, lhs, rhs),
            BinaryOp::Lt => self.compile_cmp(IntPredicate::SLT, lhs, rhs),
            BinaryOp::GtEq => self.compile_cmp(IntPredicate::SGE, lhs, rhs),
            BinaryOp::LtEq => self.compile_cmp(IntPredicate::SLE, lhs, rhs),
            BinaryOp::Eq => self.compile_cmp(IntPredicate::EQ, lhs, rhs),
            BinaryOp::NotEq => self.compile_cmp(IntPredicate::NE, lhs, rhs),
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
        pred: IntPredicate,
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
}
