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

    fn compile_lvalue_addr(&self, expr: &Expr) -> Option<PointerValue<'ctx>> {
        match expr {
            Expr::Identifier(name) => {
                if let Some((ptr, _)) = self.variables.get(name) {
                    Some(*ptr)
                } else {
                    eprintln!("Error: Variable '{}' not found for assignment.", name);
                    None
                }
            }

            Expr::Index { array, indices } => {
                let (target_val, target_type) = self.compile_expr(array)?;

                if target_type != BrixType::Matrix {
                    return None;
                }

                let matrix_ptr = target_val.into_pointer_value();
                let matrix_type = self.get_matrix_type();
                let i64_type = self.context.i64_type();

                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "cols")
                    .unwrap();
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .unwrap()
                    .into_int_value();

                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "data")
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

                let final_offset = if indices.len() == 1 {
                    let (idx0_val, _) = self.compile_expr(&indices[0])?;
                    idx0_val.into_int_value()
                } else if indices.len() == 2 {
                    let (row_val, _) = self.compile_expr(&indices[0])?;
                    let (col_val, _) = self.compile_expr(&indices[1])?;
                    let row_offset = self
                        .builder
                        .build_int_mul(row_val.into_int_value(), cols, "row_off")
                        .unwrap();
                    self.builder
                        .build_int_add(row_offset, col_val.into_int_value(), "final_off")
                        .unwrap()
                } else {
                    return None;
                };

                unsafe {
                    let f64 = self.context.f64_type();
                    let item_ptr = self
                        .builder
                        .build_gep(f64, data, &[final_offset], "addr_ptr")
                        .unwrap();
                    Some(item_ptr)
                }
            }

            _ => {
                eprintln!("Error: Invalid expression for the left side of an assignment.");
                None
            }
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt, function: inkwell::values::FunctionValue<'ctx>) {
        match stmt {
            Stmt::VariableDecl {
                name,
                type_hint,
                value,
                is_const: _,
            } => {
                if let Some((init_val, mut val_type)) = self.compile_expr(value) {
                    let mut final_val = init_val;

                    // --- AUTOMATIC CASTING ---
                    if let Some(hint) = type_hint {
                        match hint.as_str() {
                            "int" => {
                                if val_type == BrixType::Float {
                                    final_val = self
                                        .builder
                                        .build_float_to_signed_int(
                                            init_val.into_float_value(),
                                            self.context.i64_type(),
                                            "cast_f2i",
                                        )
                                        .unwrap()
                                        .into();
                                    val_type = BrixType::Int;
                                }
                            }
                            "float" => {
                                if val_type == BrixType::Int {
                                    final_val = self
                                        .builder
                                        .build_signed_int_to_float(
                                            init_val.into_int_value(),
                                            self.context.f64_type(),
                                            "cast_i2f",
                                        )
                                        .unwrap()
                                        .into();
                                    val_type = BrixType::Float;
                                }
                            }
                            "bool" => {
                                val_type = BrixType::Int;
                            }
                            "string" => {
                                if val_type != BrixType::String {
                                    eprintln!(
                                        "Aviso: Tentando atribuir tipo incompatível para string."
                                    );
                                }
                            }
                            _ => {}
                        }
                    }

                    // --- ALLOCATION ---
                    let llvm_type: BasicTypeEnum = match val_type {
                        BrixType::Int => self.context.i64_type().into(),
                        BrixType::Float => self.context.f64_type().into(),
                        BrixType::String | BrixType::Matrix | BrixType::FloatPtr => {
                            self.context.ptr_type(AddressSpace::default()).into()
                        }
                        _ => self.context.i64_type().into(),
                    };

                    let alloca = self.create_entry_block_alloca(llvm_type, name);
                    self.builder.build_store(alloca, final_val).unwrap();

                    self.variables.insert(name.clone(), (alloca, val_type));
                }
            }

            Stmt::Assignment { target, value } => {
                if let Some(target_ptr) = self.compile_lvalue_addr(target) {
                    if let Some((val, val_type)) = self.compile_expr(value) {
                        let final_val = if val_type == BrixType::Int {
                            self.builder
                                .build_signed_int_to_float(
                                    val.into_int_value(),
                                    self.context.f64_type(),
                                    "cast",
                                )
                                .unwrap()
                                .into()
                        } else {
                            val
                        };

                        self.builder.build_store(target_ptr, final_val).unwrap();
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
                    if let Some((val, brix_type)) = self.compile_expr(arg) {
                        match brix_type {
                            BrixType::String => {
                                let struct_ptr = val.into_pointer_value();
                                let str_type = self.get_string_type();
                                let data_ptr_ptr = self
                                    .builder
                                    .build_struct_gep(str_type, struct_ptr, 1, "str_data_ptr")
                                    .unwrap();
                                let data_ptr = self
                                    .builder
                                    .build_load(
                                        self.context.ptr_type(AddressSpace::default()),
                                        data_ptr_ptr,
                                        "str_data",
                                    )
                                    .unwrap();
                                compiled_args.push(data_ptr.into());
                            }
                            BrixType::Matrix => compiled_args.push(val.into()),
                            _ => compiled_args.push(val.into()),
                        }
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

            Stmt::For {
                var_name,
                iterable,
                body,
            } => {
                if let Expr::Range { start, end, step } = iterable {
                    let (start_val, _) = self.compile_expr(start).unwrap();
                    let (end_val, _) = self.compile_expr(end).unwrap();

                    let step_val = if let Some(step_expr) = step {
                        self.compile_expr(step_expr).unwrap().0.into_int_value()
                    } else {
                        self.context.i64_type().const_int(1, false)
                    };

                    // Converte tudo para Int (Range float é possível, mas vamos focar em Int agora)
                    let start_int = start_val.into_int_value();
                    let end_int = end_val.into_int_value();

                    // --- LOOP ---

                    let i_alloca =
                        self.create_entry_block_alloca(self.context.i64_type().into(), var_name);
                    self.builder.build_store(i_alloca, start_int).unwrap();

                    let old_var = self.variables.remove(var_name);
                    self.variables
                        .insert(var_name.clone(), (i_alloca, BrixType::Int));

                    // 2. Basic blocks
                    let cond_bb = self.context.append_basic_block(function, "for_cond");
                    let body_bb = self.context.append_basic_block(function, "for_body");
                    let inc_bb = self.context.append_basic_block(function, "for_inc");
                    let after_bb = self.context.append_basic_block(function, "for_after");

                    self.builder.build_unconditional_branch(cond_bb).unwrap();

                    // --- BLOCK: COND ---
                    self.builder.position_at_end(cond_bb);
                    let cur_i = self
                        .builder
                        .build_load(self.context.i64_type(), i_alloca, "i_val")
                        .unwrap()
                        .into_int_value();

                    let loop_cond = self
                        .builder
                        .build_int_compare(IntPredicate::SLE, cur_i, end_int, "loop_cond")
                        .unwrap();
                    self.builder
                        .build_conditional_branch(loop_cond, body_bb, after_bb)
                        .unwrap();

                    // --- BLOCK: BODY ---
                    self.builder.position_at_end(body_bb);
                    self.compile_stmt(body, function);
                    self.builder.build_unconditional_branch(inc_bb).unwrap();

                    // --- BLOCK: INC ---
                    self.builder.position_at_end(inc_bb);
                    let tmp_i = self
                        .builder
                        .build_load(self.context.i64_type(), i_alloca, "i_load")
                        .unwrap()
                        .into_int_value();
                    let next_i = self
                        .builder
                        .build_int_add(tmp_i, step_val, "i_next")
                        .unwrap();
                    self.builder.build_store(i_alloca, next_i).unwrap();
                    self.builder.build_unconditional_branch(cond_bb).unwrap();

                    // --- BLOCK: AFTER ---
                    self.builder.position_at_end(after_bb);

                    if let Some(old) = old_var {
                        self.variables.insert(var_name.clone(), old);
                    } else {
                        self.variables.remove(var_name);
                    }
                }

                let (iterable_val, iterable_type) = self
                    .compile_expr(iterable)
                    .expect("Error to compile iterable of the loop");

                match iterable_type {
                    BrixType::Matrix => {
                        let matrix_ptr = iterable_val.into_pointer_value();
                        let matrix_type = self.get_matrix_type();
                        let i64_type = self.context.i64_type();

                        let rows_ptr = self
                            .builder
                            .build_struct_gep(matrix_type, matrix_ptr, 0, "rows")
                            .unwrap();
                        let cols_ptr = self
                            .builder
                            .build_struct_gep(matrix_type, matrix_ptr, 1, "cols")
                            .unwrap();

                        let rows = self
                            .builder
                            .build_load(i64_type, rows_ptr, "rows")
                            .unwrap()
                            .into_int_value();
                        let cols = self
                            .builder
                            .build_load(i64_type, cols_ptr, "cols")
                            .unwrap()
                            .into_int_value();

                        let total_len =
                            self.builder.build_int_mul(rows, cols, "total_len").unwrap();

                        let idx_alloca =
                            self.create_entry_block_alloca(i64_type.into(), "_hidden_idx");
                        self.builder
                            .build_store(idx_alloca, i64_type.const_int(0, false))
                            .unwrap();

                        let user_var_alloca = self
                            .create_entry_block_alloca(self.context.f64_type().into(), var_name);
                        let old_var = self.variables.remove(var_name);
                        self.variables
                            .insert(var_name.clone(), (user_var_alloca, BrixType::Float));

                        let cond_bb = self.context.append_basic_block(function, "arr_cond");
                        let body_bb = self.context.append_basic_block(function, "arr_body");
                        let inc_bb = self.context.append_basic_block(function, "arr_inc");
                        let after_bb = self.context.append_basic_block(function, "arr_after");

                        self.builder.build_unconditional_branch(cond_bb).unwrap();

                        // --- COND ---
                        self.builder.position_at_end(cond_bb);
                        let cur_idx = self
                            .builder
                            .build_load(i64_type, idx_alloca, "cur_idx")
                            .unwrap()
                            .into_int_value();
                        let loop_cond = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, cur_idx, total_len, "check_idx")
                            .unwrap();
                        self.builder
                            .build_conditional_branch(loop_cond, body_bb, after_bb)
                            .unwrap();

                        // --- BODY ---
                        self.builder.position_at_end(body_bb);

                        let data_ptr_ptr = self
                            .builder
                            .build_struct_gep(matrix_type, matrix_ptr, 2, "data_ptr")
                            .unwrap();
                        let data_base = self
                            .builder
                            .build_load(
                                self.context.ptr_type(AddressSpace::default()),
                                data_ptr_ptr,
                                "data_base",
                            )
                            .unwrap()
                            .into_pointer_value();

                        unsafe {
                            let elem_ptr = self
                                .builder
                                .build_gep(
                                    self.context.f64_type(),
                                    data_base,
                                    &[cur_idx],
                                    "elem_ptr",
                                )
                                .unwrap();
                            let elem_val = self
                                .builder
                                .build_load(self.context.f64_type(), elem_ptr, "elem_val")
                                .unwrap();
                            self.builder.build_store(user_var_alloca, elem_val).unwrap();
                        }

                        self.compile_stmt(body, function);
                        self.builder.build_unconditional_branch(inc_bb).unwrap();

                        // --- INC ---
                        self.builder.position_at_end(inc_bb);
                        let tmp_idx = self
                            .builder
                            .build_load(i64_type, idx_alloca, "idx_load")
                            .unwrap()
                            .into_int_value();
                        let next_idx = self
                            .builder
                            .build_int_add(tmp_idx, i64_type.const_int(1, false), "idx_next")
                            .unwrap();
                        self.builder.build_store(idx_alloca, next_idx).unwrap();
                        self.builder.build_unconditional_branch(cond_bb).unwrap();

                        // --- AFTER ---
                        self.builder.position_at_end(after_bb);

                        if let Some(old) = old_var {
                            self.variables.insert(var_name.clone(), old);
                        } else {
                            self.variables.remove(var_name);
                        }
                    }
                    _ => eprintln!("Erro: Tipo {:?} não é iterável.", iterable_type),
                }
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
                    let raw_str = self.builder.build_global_string_ptr(s, "raw_str").unwrap();

                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                    let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                        self.module
                            .add_function("str_new", fn_type, Some(Linkage::External))
                    });

                    let call = self
                        .builder
                        .build_call(str_new_fn, &[raw_str.as_pointer_value().into()], "new_str")
                        .unwrap();

                    Some((call.try_as_basic_value().left().unwrap(), BrixType::String))
                }
                Literal::Bool(b) => {
                    let bool_val = self.context.bool_type().const_int(*b as u64, false);
                    let int_val = self
                        .builder
                        .build_int_z_extend(bool_val, self.context.i64_type(), "bool_ext")
                        .unwrap();
                    Some((int_val.into(), BrixType::Int))
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
                if matches!(op, BinaryOp::LogicalAnd) || matches!(op, BinaryOp::LogicalOr) {
                    let (lhs_val, _) = self.compile_expr(lhs)?;
                    let lhs_int = lhs_val.into_int_value();

                    let parent_fn = self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap();
                    let rhs_bb = self.context.append_basic_block(parent_fn, "logic_rhs");
                    let merge_bb = self.context.append_basic_block(parent_fn, "logic_merge");

                    let entry_bb = self.builder.get_insert_block().unwrap();

                    match op {
                        BinaryOp::LogicalAnd => {
                            let zero = self.context.i64_type().const_int(0, false);
                            let lhs_bool = self
                                .builder
                                .build_int_compare(IntPredicate::NE, lhs_int, zero, "tobool")
                                .unwrap();

                            self.builder
                                .build_conditional_branch(lhs_bool, rhs_bb, merge_bb)
                                .unwrap();
                        }
                        BinaryOp::LogicalOr => {
                            let zero = self.context.i64_type().const_int(0, false);
                            let lhs_bool = self
                                .builder
                                .build_int_compare(IntPredicate::NE, lhs_int, zero, "tobool")
                                .unwrap();

                            self.builder
                                .build_conditional_branch(lhs_bool, merge_bb, rhs_bb)
                                .unwrap();
                        }
                        _ => unreachable!(),
                    }

                    self.builder.position_at_end(rhs_bb);
                    let (rhs_val, _) = self.compile_expr(rhs)?;
                    let rhs_int = rhs_val.into_int_value();

                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                    let rhs_end_bb = self.builder.get_insert_block().unwrap();

                    self.builder.position_at_end(merge_bb);
                    let phi = self
                        .builder
                        .build_phi(self.context.i64_type(), "logic_result")
                        .unwrap();

                    match op {
                        BinaryOp::LogicalAnd => {
                            let zero = self.context.i64_type().const_int(0, false);
                            phi.add_incoming(&[(&zero, entry_bb), (&rhs_int, rhs_end_bb)]);
                        }
                        BinaryOp::LogicalOr => {
                            let one = self.context.i64_type().const_int(1, false);
                            phi.add_incoming(&[(&one, entry_bb), (&rhs_int, rhs_end_bb)]);
                        }
                        _ => unreachable!(),
                    }

                    return Some((phi.as_basic_value().into(), BrixType::Int));
                }

                let (lhs_val, lhs_type) = self.compile_expr(lhs)?;
                let (rhs_val, rhs_type) = self.compile_expr(rhs)?;

                // --- Strings ---
                if lhs_type == BrixType::String && rhs_type == BrixType::String {
                    match op {
                        BinaryOp::Add => {
                            let ptr_type = self.context.ptr_type(AddressSpace::default());
                            let fn_type =
                                ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);

                            let concat_fn =
                                self.module.get_function("str_concat").unwrap_or_else(|| {
                                    self.module.add_function(
                                        "str_concat",
                                        fn_type,
                                        Some(Linkage::External),
                                    )
                                });

                            let res = self
                                .builder
                                .build_call(concat_fn, &[lhs_val.into(), rhs_val.into()], "str_add")
                                .unwrap();
                            return Some((
                                res.try_as_basic_value().left().unwrap(),
                                BrixType::String,
                            ));
                        }
                        BinaryOp::Eq => {
                            let ptr_type = self.context.ptr_type(AddressSpace::default());
                            let i64_type = self.context.i64_type();
                            let fn_type =
                                i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);

                            let eq_fn = self.module.get_function("str_eq").unwrap_or_else(|| {
                                self.module
                                    .add_function("str_eq", fn_type, Some(Linkage::External))
                            });

                            let res = self
                                .builder
                                .build_call(eq_fn, &[lhs_val.into(), rhs_val.into()], "str_eq_call")
                                .unwrap();
                            return Some((res.try_as_basic_value().left().unwrap(), BrixType::Int));
                        }
                        _ => {
                            eprintln!("Erro: Operação não suportada para strings (apenas + e ==).");
                            return None;
                        }
                    }
                }

                // --- Numbers (Int and Float) ---
                let is_float_op =
                    matches!(lhs_type, BrixType::Float) || matches!(rhs_type, BrixType::Float);

                if is_float_op {
                    let l_float = if lhs_type == BrixType::Int {
                        self.builder
                            .build_signed_int_to_float(
                                lhs_val.into_int_value(),
                                self.context.f64_type(),
                                "cast_l",
                            )
                            .unwrap()
                    } else {
                        lhs_val.into_float_value()
                    };

                    let r_float = if rhs_type == BrixType::Int {
                        self.builder
                            .build_signed_int_to_float(
                                rhs_val.into_int_value(),
                                self.context.f64_type(),
                                "cast_r",
                            )
                            .unwrap()
                    } else {
                        rhs_val.into_float_value()
                    };

                    let val = self.compile_float_op(op, l_float, r_float)?;

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
                    if fn_name == "typeof" {
                        if args.len() != 1 {
                            eprintln!("Error: typeof expects exactly 1 argument.");
                            return None;
                        }
                        let (_, arg_type) = self.compile_expr(&args[0])?;

                        let type_str = match arg_type {
                            BrixType::Int => "int",
                            BrixType::Float => "float",
                            BrixType::String => "string",
                            BrixType::Matrix => "matrix",
                            BrixType::FloatPtr => "float_ptr",
                            BrixType::Void => "void",
                        };

                        return self
                            .compile_expr(&Expr::Literal(Literal::String(type_str.to_string())));
                    }
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
                eprintln!("Error: Unknown function: {:?}", func);
                None
            }

            Expr::FieldAccess { target, field } => {
                let (target_val, target_type) = self.compile_expr(target)?;

                if target_type == BrixType::String {
                    if field == "len" {
                        let ptr = target_val.into_pointer_value();
                        let str_type = self.get_string_type();
                        let len_ptr = self
                            .builder
                            .build_struct_gep(str_type, ptr, 0, "len_ptr")
                            .unwrap();
                        let len_val = self
                            .builder
                            .build_load(self.context.i64_type(), len_ptr, "len_val")
                            .unwrap();
                        return Some((len_val, BrixType::Int));
                    }
                }

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

            Expr::Index { array, indices } => {
                let (target_val, target_type) = self.compile_expr(array)?;

                if target_type != BrixType::Matrix {
                    eprintln!("Erro: Tentando indexar algo que não é matriz.");
                    return None;
                }

                let matrix_ptr = target_val.into_pointer_value();
                let matrix_type = self.get_matrix_type();
                let i64_type = self.context.i64_type();

                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "cols")
                    .unwrap();
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .unwrap()
                    .into_int_value();

                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "data")
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

                let final_offset = if indices.len() == 1 {
                    let (idx0_val, _) = self.compile_expr(&indices[0])?;
                    idx0_val.into_int_value()
                } else if indices.len() == 2 {
                    let (row_val, _) = self.compile_expr(&indices[0])?;
                    let (col_val, _) = self.compile_expr(&indices[1])?;

                    let row_offset = self
                        .builder
                        .build_int_mul(row_val.into_int_value(), cols, "row_off")
                        .unwrap();
                    self.builder
                        .build_int_add(row_offset, col_val.into_int_value(), "final_off")
                        .unwrap()
                } else {
                    eprintln!("Erro: Suporte apenas para 1 ou 2 índices.");
                    return None;
                };

                unsafe {
                    let f64 = self.context.f64_type();
                    let item_ptr = self
                        .builder
                        .build_gep(f64, data, &[final_offset], "item_ptr")
                        .unwrap();
                    let val = self.builder.build_load(f64, item_ptr, "val").unwrap();

                    Some((val, BrixType::Float))
                }
            }

            Expr::Array(elements) => {
                let n = elements.len() as u64;

                let i64_type = self.context.i64_type();
                let rows_val = i64_type.const_int(1, false);
                let cols_val = i64_type.const_int(n, false);

                let matrix_type = self.get_matrix_type();
                let matrix_ptr = self.builder.build_alloca(matrix_type, "array_lit").unwrap();

                let rows_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 0, "rows")
                    .unwrap();
                self.builder.build_store(rows_ptr, rows_val).unwrap();
                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "cols")
                    .unwrap();
                self.builder.build_store(cols_ptr, cols_val).unwrap();

                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let matrix_new_fn = self.module.get_function("matrix_new").unwrap_or_else(|| {
                    self.module
                        .add_function("matrix_new", fn_type, Some(Linkage::External))
                });

                let call = self
                    .builder
                    .build_call(
                        matrix_new_fn,
                        &[rows_val.into(), cols_val.into()],
                        "alloc_arr",
                    )
                    .unwrap();
                let new_matrix_ptr = call
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, new_matrix_ptr, 2, "data_ptr")
                    .unwrap();
                let data_ptr = self
                    .builder
                    .build_load(ptr_type, data_ptr_ptr, "data_base")
                    .unwrap()
                    .into_pointer_value();

                for (i, expr) in elements.iter().enumerate() {
                    let (val, val_type) = self.compile_expr(expr)?;

                    let float_val = if val_type == BrixType::Int {
                        self.builder
                            .build_signed_int_to_float(
                                val.into_int_value(),
                                self.context.f64_type(),
                                "cast",
                            )
                            .unwrap()
                    } else {
                        val.into_float_value()
                    };

                    let index = i64_type.const_int(i as u64, false);
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(self.context.f64_type(), data_ptr, &[index], "elem_ptr")
                            .unwrap();
                        self.builder.build_store(elem_ptr, float_val).unwrap();
                    }
                }

                Some((new_matrix_ptr.as_basic_value_enum(), BrixType::Matrix))
            }

            Expr::Range { .. } => {
                eprintln!(
                    "Error: Ranges cannot be assigned to variables, use only inside 'for' loops."
                );
                None
            }

            _ => {
                eprintln!("Expression not implemented in v0.3");
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

    fn get_string_type(&self) -> inkwell::types::StructType<'ctx> {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        // Struct { len: i64, data: char* }
        self.context
            .struct_type(&[i64_type.into(), ptr_type.into()], false)
    }

    fn compile_matrix_constructor(&self, args: &[Expr]) -> Option<BasicValueEnum<'ctx>> {
        if args.len() != 2 {
            return None;
        }
        let (rows_val, _) = self.compile_expr(&args[0])?;
        let (cols_val, _) = self.compile_expr(&args[1])?;

        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);

        let matrix_new_fn = self.module.get_function("matrix_new").unwrap_or_else(|| {
            self.module
                .add_function("matrix_new", fn_type, Some(Linkage::External))
        });

        let call = self
            .builder
            .build_call(
                matrix_new_fn,
                &[rows_val.into(), cols_val.into()],
                "alloc_matrix",
            )
            .unwrap();

        Some(call.try_as_basic_value().left().unwrap())
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
