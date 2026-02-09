use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValue, BasicValueEnum, FloatValue, IntValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt, UnaryOp};
use std::collections::HashMap;

// --- MODULE DECLARATIONS ---
// These modules will be gradually populated during refactoring
mod types;
mod helpers;
mod builtins;
mod operators;
mod stmt;
mod expr;
mod error;

// Re-export BrixType for public API
pub use types::BrixType;

// Re-export error types for public API
pub use error::{CodegenError, CodegenResult};

// Import helper trait to make functions available on Compiler
use helpers::HelperFunctions;

// Import builtin function traits
// Note: These traits are imported in respective modules (stmt.rs, expr.rs)
// and made available on Compiler via trait implementations
use builtins::string::StringFunctions;

// Import statement compiler trait
use stmt::StatementCompiler;

// Import expression compiler trait
use expr::ExpressionCompiler;

#[cfg(test)]
mod tests;

pub struct Compiler<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a Module<'ctx>,
    pub variables: HashMap<String, (PointerValue<'ctx>, BrixType)>,
    pub functions: HashMap<String, (inkwell::values::FunctionValue<'ctx>, Option<Vec<BrixType>>)>, // (function, return_types)
    pub function_params: HashMap<String, Vec<(String, BrixType, Option<Expr>)>>, // (param_name, type, default_value)
    pub current_function: Option<inkwell::values::FunctionValue<'ctx>>, // Track current function being compiled
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
            functions: HashMap::new(),
            function_params: HashMap::new(),
            current_function: None,
        }
    }

    // --- HELPER & BUILTIN FUNCTIONS ---
    // Moved to respective modules (available via traits):
    // - helpers.rs: HelperFunctions trait (create_entry_block_alloca, get_printf, etc.)
    // - builtins/math.rs: MathFunctions trait (math library + constants)
    // - builtins/stats.rs: StatsFunctions trait (statistics functions)
    // - builtins/linalg.rs: LinalgFunctions trait (linear algebra functions)
    // - builtins/string.rs: StringFunctions trait (string manipulation)

    // --- TYPE SYSTEM HELPERS ---
    // Note: These are kept in lib.rs because they need access to self.context
    // The BrixType enum itself is defined in types.rs

    fn string_to_brix_type(&self, type_str: &str) -> BrixType {
        match type_str {
            "int" => BrixType::Int,
            "float" => BrixType::Float,
            "string" => BrixType::String,
            "matrix" => BrixType::Matrix,
            "intmatrix" => BrixType::IntMatrix,
            "complex" => BrixType::Complex,
            "nil" => BrixType::Nil,
            "error" => BrixType::Error,
            "atom" => BrixType::Atom,
            "void" => BrixType::Void,
            _ => {
                eprintln!("Warning: Unknown type '{}', defaulting to Int", type_str);
                BrixType::Int
            }
        }
    }

    fn brix_type_to_llvm(&self, brix_type: &BrixType) -> BasicTypeEnum<'ctx> {
        match brix_type {
            BrixType::Int | BrixType::Atom => self.context.i64_type().into(), // Atom = i64 (atom ID)
            BrixType::Float => self.context.f64_type().into(),
            BrixType::String
            | BrixType::Matrix
            | BrixType::IntMatrix
            | BrixType::FloatPtr
            | BrixType::Nil
            | BrixType::Error => self.context.ptr_type(AddressSpace::default()).into(),
            BrixType::Complex => {
                // Complex number: struct { f64 real, f64 imag }
                let f64_type = self.context.f64_type();
                self.context
                    .struct_type(&[f64_type.into(), f64_type.into()], false)
                    .into()
            }
            BrixType::ComplexArray | BrixType::ComplexMatrix => {
                // Pointer to runtime struct
                self.context.ptr_type(AddressSpace::default()).into()
            }
            BrixType::Void => self.context.i64_type().into(), // Placeholder (shouldn't be used)
            BrixType::Tuple(types) => {
                // Create struct type for tuple
                let field_types: Vec<BasicTypeEnum> =
                    types.iter().map(|t| self.brix_type_to_llvm(t)).collect();
                self.context.struct_type(&field_types, false).into()
            }
        }
    }

    // --- FUNCTION DEFINITION ---
    fn compile_function_def(
        &mut self,
        name: &str,
        params: &[(String, String, Option<Expr>)],
        return_type: &Option<Vec<String>>,
        body: &Stmt,
        _parent_function: inkwell::values::FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        // 1. Parse return type
        let ret_types: Vec<BrixType> = match return_type {
            None => vec![], // void
            Some(types) => types.iter().map(|t| self.string_to_brix_type(t)).collect(),
        };

        // 2. Create LLVM function type
        let param_types: Vec<BasicTypeEnum> = params
            .iter()
            .map(|(_, t, _)| self.brix_type_to_llvm(&self.string_to_brix_type(t)))
            .collect();

        let fn_type = if ret_types.is_empty() {
            // Void function
            self.context.void_type().fn_type(
                &param_types.iter().map(|t| (*t).into()).collect::<Vec<_>>(),
                false,
            )
        } else if ret_types.len() == 1 {
            // Single return
            let ret_llvm = self.brix_type_to_llvm(&ret_types[0]);
            ret_llvm.fn_type(
                &param_types.iter().map(|t| (*t).into()).collect::<Vec<_>>(),
                false,
            )
        } else {
            // Multiple returns - create struct type
            let tuple_type = BrixType::Tuple(ret_types.clone());
            let ret_llvm = self.brix_type_to_llvm(&tuple_type);
            ret_llvm.fn_type(
                &param_types.iter().map(|t| (*t).into()).collect::<Vec<_>>(),
                false,
            )
        };

        // 3. Create the function
        let llvm_function = self.module.add_function(name, fn_type, None);

        // 4. Store function in registry
        self.functions
            .insert(name.to_string(), (llvm_function, Some(ret_types.clone())));

        // 4.5. Store parameter metadata (including default values)
        let param_metadata: Vec<(String, BrixType, Option<Expr>)> = params
            .iter()
            .map(|(name, ty, default)| {
                (name.clone(), self.string_to_brix_type(ty), default.clone())
            })
            .collect();
        self.function_params
            .insert(name.to_string(), param_metadata);

        // 5. Create entry block
        let entry_block = self.context.append_basic_block(llvm_function, "entry");
        self.builder.position_at_end(entry_block);

        // 6. Save current state and set current function
        let saved_vars = self.variables.clone();
        self.current_function = Some(llvm_function);

        // 7. Create allocas for parameters and store them
        for (i, (param_name, param_type_str, _default)) in params.iter().enumerate() {
            let param_value = llvm_function.get_nth_param(i as u32)
                .ok_or_else(|| CodegenError::LLVMError {
                    operation: "get_nth_param".to_string(),
                    details: format!("Failed to get parameter {} in function definition", i),
                })?;
            let param_type = self.string_to_brix_type(param_type_str);
            let llvm_type = self.brix_type_to_llvm(&param_type);

            let alloca = self.create_entry_block_alloca(llvm_type, param_name)?;
            self.builder.build_store(alloca, param_value)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: format!("Failed to store parameter '{}'", param_name),
                })?;
            self.variables
                .insert(param_name.clone(), (alloca, param_type));
        }

        // 8. Compile function body
        self.compile_stmt(body, llvm_function)?;

        // 9. Add implicit return for void functions if missing
        if ret_types.is_empty() {
            // Check if last instruction is already a return
            if let Some(block) = self.builder.get_insert_block() {
                if block.get_terminator().is_none() {
                    self.builder.build_return(None)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "Failed to build implicit void return".to_string(),
                        })?;
                }
            }
        }

        // 10. Restore state
        self.variables = saved_vars;
        self.current_function = Some(_parent_function);

        // 11. Position builder back at the end of parent function
        if let Some(block) = _parent_function.get_last_basic_block() {
            self.builder.position_at_end(block);
        }

        Ok(())
    }

    // --- MAIN COMPILATION ---

    pub fn compile_program(&mut self, program: &Program) {
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[], false);
        let function = self.module.add_function("main", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");

        self.builder.position_at_end(basic_block);
        self.current_function = Some(function);

        for stmt in &program.statements {
            let _ = self.compile_stmt(stmt, function);
        }

        let _ = self
            .builder
            .build_return(Some(&i64_type.const_int(0, false)));
    }

    fn compile_lvalue_addr(&mut self, expr: &Expr) -> CodegenResult<(PointerValue<'ctx>, BrixType)> {
        match expr {
            Expr::Identifier(name) => {
                if let Some((ptr, var_type)) = self.variables.get(name) {
                    Ok((*ptr, var_type.clone()))
                } else {
                    Err(CodegenError::UndefinedSymbol {
                        name: name.clone(),
                        context: "Assignment target".to_string(),
                    })
                }
            }

            Expr::Index { array, indices } => {
                let (target_val, target_type) = self.compile_expr(array)?;

                // Support both Matrix and IntMatrix for lvalue assignment
                if target_type != BrixType::Matrix && target_type != BrixType::IntMatrix {
                    return Err(CodegenError::TypeError {
                        expected: "Matrix or IntMatrix".to_string(),
                        found: format!("{:?}", target_type),
                        context: "Index assignment target".to_string(),
                    });
                }

                let is_int_matrix = target_type == BrixType::IntMatrix;
                let matrix_ptr = target_val.into_pointer_value();
                let matrix_type = if is_int_matrix {
                    self.get_intmatrix_type()
                } else {
                    self.get_matrix_type()
                };
                let i64_type = self.context.i64_type();

                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "cols")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed to get matrix columns pointer".to_string(),
                    })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load matrix columns".to_string(),
                    })?
                    .into_int_value();

                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "data")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed to get matrix data pointer".to_string(),
                    })?;
                let data = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load matrix data".to_string(),
                    })?
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
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_int_mul".to_string(),
                            details: "Failed to calculate row offset".to_string(),
                        })?;
                    self.builder
                        .build_int_add(row_offset, col_val.into_int_value(), "final_off")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_int_add".to_string(),
                            details: "Failed to calculate final offset".to_string(),
                        })?
                } else {
                    return Err(CodegenError::InvalidOperation {
                        operation: "Matrix indexing".to_string(),
                        reason: format!("Invalid number of indices: {}", indices.len()),
                    });
                };

                unsafe {
                    if is_int_matrix {
                        // IntMatrix: GEP with i64 type, returns Int element
                        let item_ptr = self
                            .builder
                            .build_gep(i64_type, data, &[final_offset], "addr_ptr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_gep".to_string(),
                                details: "Failed to get IntMatrix element pointer".to_string(),
                            })?;
                        Ok((item_ptr, BrixType::Int))
                    } else {
                        // Matrix: GEP with f64 type, returns Float element
                        let f64 = self.context.f64_type();
                        let item_ptr = self
                            .builder
                            .build_gep(f64, data, &[final_offset], "addr_ptr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_gep".to_string(),
                                details: "Failed to get Matrix element pointer".to_string(),
                            })?;
                        Ok((item_ptr, BrixType::Float))
                    }
                }
            }

            _ => {
                Err(CodegenError::InvalidOperation {
                    operation: "Assignment".to_string(),
                    reason: "Invalid expression for the left side of an assignment".to_string(),
                })
            }
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt, function: inkwell::values::FunctionValue<'ctx>) -> CodegenResult<()> {
        match stmt {
            Stmt::VariableDecl {
                name,
                type_hint,
                value,
                is_const: _,
            } => {
                self.compile_variable_decl_stmt(name, type_hint, value)?;
                Ok(())
            }

            Stmt::DestructuringDecl {
                names,
                value,
                is_const: _,
            } => {
                self.compile_destructuring_decl_stmt(names, value)?;
                Ok(())
            }

            Stmt::Assignment { target, value } => {
                self.compile_assignment_stmt(target, value)?;
                Ok(())
            }

            Stmt::Printf { format, args } => {
                self.compile_printf_stmt(format, args)?;
                Ok(())
            }

            Stmt::Print { expr } => {
                self.compile_print_stmt(expr)?;
                Ok(())
            }

            Stmt::Println { expr } => {
                self.compile_println_stmt(expr)?;
                Ok(())
            }

            Stmt::Expr(expr) => {
                self.compile_expr_stmt(expr)?;
                Ok(())
            }

            Stmt::Block(statements) => {
                self.compile_block_stmt(statements, function)?;
                Ok(())
            }

            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.compile_if_stmt(condition, then_block, else_block, function)?;
                Ok(())
            }

            Stmt::While { condition, body } => {
                self.compile_while_stmt(condition, body, function)?;
                Ok(())
            }

            Stmt::For {
                var_names,
                iterable,
                body,
            } => {
                // For ranges, we only support single variable
                if let Expr::Range { start, end, step } = iterable {
                    if var_names.len() != 1 {
                        return Err(CodegenError::InvalidOperation {
                            operation: "Range iteration".to_string(),
                            reason: "Range iteration supports only single variable".to_string(),
                        });
                    }
                    let var_name = &var_names[0];
                    let (start_val, _) = self.compile_expr(start)?;
                    let (end_val, _) = self.compile_expr(end)?;

                    let step_val = if let Some(step_expr) = step {
                        self.compile_expr(step_expr)?.0.into_int_value()
                    } else {
                        self.context.i64_type().const_int(1, false)
                    };

                    // Converte tudo para Int (Range float é possível, mas vamos focar em Int agora)
                    let start_int = start_val.into_int_value();
                    let end_int = end_val.into_int_value();

                    // --- LOOP ---

                    let i_alloca =
                        self.create_entry_block_alloca(self.context.i64_type().into(), var_name)?;
                    self.builder.build_store(i_alloca, start_int)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "Failed to store initial value in for loop".to_string(),
                        })?;

                    let old_var = self.variables.remove(var_name);
                    self.variables
                        .insert(var_name.clone(), (i_alloca, BrixType::Int));

                    // 2. Basic blocks
                    let cond_bb = self.context.append_basic_block(function, "for_cond");
                    let body_bb = self.context.append_basic_block(function, "for_body");
                    let inc_bb = self.context.append_basic_block(function, "for_inc");
                    let after_bb = self.context.append_basic_block(function, "for_after");

                    self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                    // --- BLOCK: COND ---
                    self.builder.position_at_end(cond_bb);
                    let cur_i = self
                        .builder
                        .build_load(self.context.i64_type(), i_alloca, "i_val")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                        .into_int_value();

                    let loop_cond = self
                        .builder
                        .build_int_compare(IntPredicate::SLE, cur_i, end_int, "loop_cond")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                    self.builder
                        .build_conditional_branch(loop_cond, body_bb, after_bb)
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                    // --- BLOCK: BODY ---
                    self.builder.position_at_end(body_bb);
                    self.compile_stmt(body, function)?;
                    self.builder.build_unconditional_branch(inc_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                    // --- BLOCK: INC ---
                    self.builder.position_at_end(inc_bb);
                    let tmp_i = self
                        .builder
                        .build_load(self.context.i64_type(), i_alloca, "i_load")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                        .into_int_value();
                    let next_i = self
                        .builder
                        .build_int_add(tmp_i, step_val, "i_next")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                    self.builder.build_store(i_alloca, next_i).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                    self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                    // --- BLOCK: AFTER ---
                    self.builder.position_at_end(after_bb);

                    if let Some(old) = old_var {
                        self.variables.insert(var_name.clone(), old);
                        Ok(())
                    } else {
                        self.variables.remove(var_name);
                        Ok(())
                    }
                } else {
                    // For iterating over arrays/matrices
                    let (iterable_val, iterable_type) = self
                        .compile_expr(iterable)
                        ?;

                    match iterable_type {
                        BrixType::Matrix => {
                            let matrix_ptr = iterable_val.into_pointer_value();
                            let matrix_type = self.get_matrix_type();
                            let i64_type = self.context.i64_type();

                            let rows_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 0, "rows")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            let cols_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 1, "cols")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            let rows = self
                                .builder
                                .build_load(i64_type, rows_ptr, "rows")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_int_value();
                            let cols = self
                                .builder
                                .build_load(i64_type, cols_ptr, "cols")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_int_value();

                            // Destructuring: iterate rows, each row is a tuple
                            // Normal: iterate all elements linearly
                            let (total_len, is_destructuring) = if var_names.len() > 1 {
                                // Destructuring: number of iterations = rows
                                // Each iteration extracts cols elements
                                // TODO: Add runtime check for cols == var_names.len()
                                (rows, true)
                            } else {
                                // Normal: iterate all elements
                                (
                                    self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?,
                                    false,
                                )
                            };

                            let idx_alloca =
                                self.create_entry_block_alloca(i64_type.into(), "_hidden_idx")?;
                            self.builder
                                .build_store(idx_alloca, i64_type.const_int(0, false))
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            // Allocate variables
                            let mut old_vars = Vec::new();
                            let mut var_allocas = Vec::new();

                            if is_destructuring {
                                // Create allocas for each variable in destructuring
                                for var_name in var_names.iter() {
                                    let user_var_alloca = self.create_entry_block_alloca(
                                        self.context.f64_type().into(),
                                        var_name,
                                    )?;
                                    let old_var = self.variables.remove(var_name);
                                    self.variables.insert(
                                        var_name.clone(),
                                        (user_var_alloca, BrixType::Float),
                                    );
                                    old_vars.push((var_name.clone(), old_var));
                                    var_allocas.push(user_var_alloca);
                                }
                            } else {
                                // Single variable
                                let var_name = &var_names[0];
                                let user_var_alloca = self.create_entry_block_alloca(
                                    self.context.f64_type().into(),
                                    var_name,
                                )?;
                                let old_var = self.variables.remove(var_name);
                                self.variables
                                    .insert(var_name.clone(), (user_var_alloca, BrixType::Float));
                                old_vars.push((var_name.clone(), old_var));
                                var_allocas.push(user_var_alloca);
                            }

                            let cond_bb = self.context.append_basic_block(function, "arr_cond");
                            let body_bb = self.context.append_basic_block(function, "arr_body");
                            let inc_bb = self.context.append_basic_block(function, "arr_inc");
                            let after_bb = self.context.append_basic_block(function, "arr_after");

                            self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            // --- COND ---
                            self.builder.position_at_end(cond_bb);
                            let cur_idx = self
                                .builder
                                .build_load(i64_type, idx_alloca, "cur_idx")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_int_value();
                            let loop_cond = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::SLT,
                                    cur_idx,
                                    total_len,
                                    "check_idx",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            self.builder
                                .build_conditional_branch(loop_cond, body_bb, after_bb)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            // --- BODY ---
                            self.builder.position_at_end(body_bb);

                            let data_ptr_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 2, "data_ptr")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            let data_base = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_ptr_ptr,
                                    "data_base",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_pointer_value();

                            if is_destructuring {
                                // Load multiple elements (one row)
                                // cur_idx is the row number
                                // Load data[cur_idx * cols + j] for j in 0..cols
                                for (j, var_alloca) in var_allocas.iter().enumerate() {
                                    unsafe {
                                        let offset = self
                                            .builder
                                            .build_int_mul(cur_idx, cols, "row_offset")
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                        let col_offset = self
                                            .builder
                                            .build_int_add(
                                                offset,
                                                i64_type.const_int(j as u64, false),
                                                "elem_offset",
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                                        let elem_ptr = self
                                            .builder
                                            .build_gep(
                                                self.context.f64_type(),
                                                data_base,
                                                &[col_offset],
                                                &format!("elem_{}_ptr", j),
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                        let elem_val = self
                                            .builder
                                            .build_load(
                                                self.context.f64_type(),
                                                elem_ptr,
                                                &format!("elem_{}", j),
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                        self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                    }
                                }
                            } else {
                                // Load single element
                                unsafe {
                                    let elem_ptr = self
                                        .builder
                                        .build_gep(
                                            self.context.f64_type(),
                                            data_base,
                                            &[cur_idx],
                                            "elem_ptr",
                                        )
                                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                    let elem_val = self
                                        .builder
                                        .build_load(self.context.f64_type(), elem_ptr, "elem_val")
                                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                    self.builder.build_store(var_allocas[0], elem_val).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                }
                            }

                            self.compile_stmt(body, function)?;
                            self.builder.build_unconditional_branch(inc_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            // --- INC ---
                            self.builder.position_at_end(inc_bb);
                            let tmp_idx = self
                                .builder
                                .build_load(i64_type, idx_alloca, "idx_load")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_int_value();
                            let next_idx = self
                                .builder
                                .build_int_add(tmp_idx, i64_type.const_int(1, false), "idx_next")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            // --- AFTER ---
                            self.builder.position_at_end(after_bb);

                            // Restore old variables
                            for (var_name, old_var_opt) in old_vars {
                                if let Some(old) = old_var_opt {
                                    self.variables.insert(var_name, old);
                                } else {
                                    self.variables.remove(&var_name);
                                }
                            }
                            Ok(())
                        }
                        BrixType::IntMatrix => {
                            // Similar to Matrix but for integers
                            let matrix_ptr = iterable_val.into_pointer_value();
                            let matrix_type = self.get_intmatrix_type();
                            let i64_type = self.context.i64_type();

                            let rows_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 0, "rows")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            let cols_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 1, "cols")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            let rows = self
                                .builder
                                .build_load(i64_type, rows_ptr, "rows")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_int_value();
                            let cols = self
                                .builder
                                .build_load(i64_type, cols_ptr, "cols")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_int_value();

                            let (total_len, is_destructuring) = if var_names.len() > 1 {
                                // Destructuring: iterate rows, assuming cols matches var_names.len()
                                // TODO: Add runtime check for cols == var_names.len()
                                (rows, true)
                            } else {
                                (
                                    self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?,
                                    false,
                                )
                            };

                            let idx_alloca =
                                self.create_entry_block_alloca(i64_type.into(), "_hidden_idx")?;
                            self.builder
                                .build_store(idx_alloca, i64_type.const_int(0, false))
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            let mut old_vars = Vec::new();
                            let mut var_allocas = Vec::new();

                            if is_destructuring {
                                for var_name in var_names.iter() {
                                    let user_var_alloca =
                                        self.create_entry_block_alloca(i64_type.into(), var_name)?;
                                    let old_var = self.variables.remove(var_name);
                                    self.variables
                                        .insert(var_name.clone(), (user_var_alloca, BrixType::Int));
                                    old_vars.push((var_name.clone(), old_var));
                                    var_allocas.push(user_var_alloca);
                                }
                            } else {
                                let var_name = &var_names[0];
                                let user_var_alloca =
                                    self.create_entry_block_alloca(i64_type.into(), var_name)?;
                                let old_var = self.variables.remove(var_name);
                                self.variables
                                    .insert(var_name.clone(), (user_var_alloca, BrixType::Int));
                                old_vars.push((var_name.clone(), old_var));
                                var_allocas.push(user_var_alloca);
                            }

                            let cond_bb = self.context.append_basic_block(function, "arr_cond");
                            let body_bb = self.context.append_basic_block(function, "arr_body");
                            let inc_bb = self.context.append_basic_block(function, "arr_inc");
                            let after_bb = self.context.append_basic_block(function, "arr_after");

                            self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            // --- COND ---
                            self.builder.position_at_end(cond_bb);
                            let cur_idx = self
                                .builder
                                .build_load(i64_type, idx_alloca, "cur_idx")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_int_value();
                            let loop_cond = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::SLT,
                                    cur_idx,
                                    total_len,
                                    "check_idx",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            self.builder
                                .build_conditional_branch(loop_cond, body_bb, after_bb)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            // --- BODY ---
                            self.builder.position_at_end(body_bb);

                            let data_ptr_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 2, "data_ptr")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            let data_base = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_ptr_ptr,
                                    "data_base",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_pointer_value();

                            if is_destructuring {
                                for (j, var_alloca) in var_allocas.iter().enumerate() {
                                    unsafe {
                                        let offset = self
                                            .builder
                                            .build_int_mul(cur_idx, cols, "row_offset")
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                        let col_offset = self
                                            .builder
                                            .build_int_add(
                                                offset,
                                                i64_type.const_int(j as u64, false),
                                                "elem_offset",
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                                        let elem_ptr = self
                                            .builder
                                            .build_gep(
                                                i64_type,
                                                data_base,
                                                &[col_offset],
                                                &format!("elem_{}_ptr", j),
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                        let elem_val = self
                                            .builder
                                            .build_load(i64_type, elem_ptr, &format!("elem_{}", j))
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                        self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                    }
                                }
                            } else {
                                unsafe {
                                    let elem_ptr = self
                                        .builder
                                        .build_gep(i64_type, data_base, &[cur_idx], "elem_ptr")
                                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                    let elem_val = self
                                        .builder
                                        .build_load(i64_type, elem_ptr, "elem_val")
                                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                    self.builder.build_store(var_allocas[0], elem_val).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                                }
                            }

                            self.compile_stmt(body, function)?;
                            self.builder.build_unconditional_branch(inc_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            // --- INC ---
                            self.builder.position_at_end(inc_bb);
                            let tmp_idx = self
                                .builder
                                .build_load(i64_type, idx_alloca, "idx_load")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?
                                .into_int_value();
                            let next_idx = self
                                .builder
                                .build_int_add(tmp_idx, i64_type.const_int(1, false), "idx_next")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;
                            self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string() })?;

                            // --- AFTER ---
                            self.builder.position_at_end(after_bb);

                            for (var_name, old_var_opt) in old_vars {
                                if let Some(old) = old_var_opt {
                                    self.variables.insert(var_name, old);
                                } else {
                                    self.variables.remove(&var_name);
                                }
                            }
                            Ok(())
                        }
                        _ => {
                            Err(CodegenError::TypeError {
                                expected: "Matrix or IntMatrix".to_string(),
                                found: format!("{:?}", iterable_type),
                                context: "For loop iterable".to_string(),
                            })
                        }
                    }
                }
            }

            Stmt::Import { module, alias } => {
                self.compile_import_stmt(module, alias)?;
                Ok(())
            }

            Stmt::FunctionDef {
                name,
                params,
                return_type,
                body,
            } => {
                self.compile_function_def(name, params, return_type, body, function)?;
                Ok(())
            }

            Stmt::Return { values } => {
                self.compile_return_stmt(values)?;
                Ok(())
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        match expr {
            Expr::Literal(lit) => self.compile_literal_expr(lit),

            Expr::Identifier(name) => {
                // First check if it's a user-defined variable
                match self.variables.get(name) {
                    Some((ptr, brix_type)) => match brix_type {
                        BrixType::String | BrixType::FloatPtr => {
                            let val = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    *ptr,
                                    name,
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, brix_type.clone()))
                        }

                        BrixType::Int => {
                            let val = self
                                .builder
                                .build_load(self.context.i64_type(), *ptr, name)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::Int))
                        }
                        BrixType::Atom => {
                            let val = self
                                .builder
                                .build_load(self.context.i64_type(), *ptr, name)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::Atom))
                        }
                        BrixType::Float => {
                            let val = self
                                .builder
                                .build_load(self.context.f64_type(), *ptr, name)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::Float))
                        }
                        BrixType::Matrix => {
                            // Load the pointer to the matrix struct
                            let val = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    *ptr,
                                    name,
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::Matrix))
                        }
                        BrixType::IntMatrix => {
                            // Load the pointer to the intmatrix struct
                            let val = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    *ptr,
                                    name,
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::IntMatrix))
                        }
                        BrixType::ComplexMatrix => {
                            // Load the pointer to the complexmatrix struct
                            let val = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    *ptr,
                                    name,
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::ComplexMatrix))
                        }
                        BrixType::Tuple(types) => {
                            // Load the tuple struct
                            let struct_type =
                                self.brix_type_to_llvm(&BrixType::Tuple(types.clone()));
                            let val = self.builder.build_load(struct_type, *ptr, name).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::Tuple(types.clone())))
                        }
                        BrixType::Complex => {
                            // Load the complex struct { f64 real, f64 imag }
                            let complex_type = self.brix_type_to_llvm(&BrixType::Complex);
                            let val = self.builder.build_load(complex_type, *ptr, name).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::Complex))
                        }
                        BrixType::Nil => {
                            // Load nil (null pointer)
                            let ptr_type = self.context.ptr_type(AddressSpace::default());
                            let val = self.builder.build_load(ptr_type, *ptr, name).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::Nil))
                        }
                        BrixType::Error => {
                            // Load error (pointer to BrixError struct)
                            let ptr_type = self.context.ptr_type(AddressSpace::default());
                            let val = self.builder.build_load(ptr_type, *ptr, name).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string() })?;
                            Ok((val, BrixType::Error))
                        }
                        _ => {
                            eprintln!("Error: Type not supported in identifier.");
                            Err(CodegenError::MissingValue { what: "expression value".to_string(), context: "compile_expr".to_string() })
                        }
                    },
                    None => {
                        // Special case: 'im' is the imaginary unit (0+1i), like Julia
                        // Only use this if no variable named 'im' exists
                        if name == "im" {
                            let complex_type = self.context.struct_type(
                                &[
                                    self.context.f64_type().into(),
                                    self.context.f64_type().into(),
                                ],
                                false,
                            );
                            let zero = self.context.f64_type().const_float(0.0);
                            let one = self.context.f64_type().const_float(1.0);
                            let im_val =
                                complex_type.const_named_struct(&[zero.into(), one.into()]);
                            return Ok((im_val.into(), BrixType::Complex));
                        }

                        eprintln!("Error: Variable '{}' not found.", name);
                        Err(CodegenError::MissingValue { what: "expression value".to_string(), context: "compile_expr".to_string() })
                    }
                }
            }

            Expr::Unary { op, expr } => {
                let (val, val_type) = self.compile_expr(expr)?;

                match op {
                    UnaryOp::Not => {
                        // Logical NOT: convert to bool, then invert
                        let int_val = if val_type == BrixType::Float {
                            // Convert float to int first for comparison
                            self.builder
                                .build_float_to_signed_int(
                                    val.into_float_value(),
                                    self.context.i64_type(),
                                    "f2i",
                                )
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_float_to_signed_int".to_string(),
                                    details: "Failed to convert float to int for NOT".to_string(),
                                })?
                        } else {
                            val.into_int_value()
                        };

                        let zero = self.context.i64_type().const_int(0, false);
                        let is_zero = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, int_val, zero, "is_zero")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to compare for NOT".to_string(),
                            })?;

                        // Extend i1 to i64
                        let result = self
                            .builder
                            .build_int_z_extend(is_zero, self.context.i64_type(), "not_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_z_extend".to_string(),
                                details: "Failed to extend NOT result to i64".to_string(),
                            })?;

                        Ok((result.into(), BrixType::Int))
                    }
                    UnaryOp::Negate => {
                        // Arithmetic negation
                        if val_type == BrixType::Int {
                            let neg = self
                                .builder
                                .build_int_neg(val.into_int_value(), "neg_int")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_int_neg".to_string(),
                                    details: "Failed to negate integer".to_string(),
                                })?;
                            Ok((neg.into(), BrixType::Int))
                        } else if val_type == BrixType::Float {
                            let neg = self
                                .builder
                                .build_float_neg(val.into_float_value(), "neg_float")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_float_neg".to_string(),
                                    details: "Failed to negate float".to_string(),
                                })?;
                            Ok((neg.into(), BrixType::Float))
                        } else {
                            Err(CodegenError::TypeError {
                                expected: "Int or Float".to_string(),
                                found: format!("{:?}", val_type),
                                context: "Unary negation".to_string(),
                            })
                        }
                    }
                }
            }

            Expr::Binary { op, lhs, rhs } => {
                if matches!(op, BinaryOp::LogicalAnd) || matches!(op, BinaryOp::LogicalOr) {
                    let (lhs_val, _) = self.compile_expr(lhs)?;
                    let lhs_int = lhs_val.into_int_value();

                    let parent_fn = self
                        .builder
                        .get_insert_block()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_insert_block".to_string(),
                            details: "No current block for logical operator".to_string(),
                        })?
                        .get_parent()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_parent".to_string(),
                            details: "Block has no parent for logical operator".to_string(),
                        })?;
                    let rhs_bb = self.context.append_basic_block(parent_fn, "logic_rhs");
                    let merge_bb = self.context.append_basic_block(parent_fn, "logic_merge");

                    let entry_bb = self.builder.get_insert_block()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_insert_block".to_string(),
                            details: "No current block for logical entry".to_string(),
                        })?;

                    match op {
                        BinaryOp::LogicalAnd => {
                            let zero = self.context.i64_type().const_int(0, false);
                            let lhs_bool = self
                                .builder
                                .build_int_compare(IntPredicate::NE, lhs_int, zero, "tobool")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_int_compare".to_string(),
                                    details: "Failed to compare for LogicalAnd".to_string(),
                                })?;

                            self.builder
                                .build_conditional_branch(lhs_bool, rhs_bb, merge_bb)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_conditional_branch".to_string(),
                                    details: "Failed to branch for LogicalAnd".to_string(),
                                })?;
                        }
                        BinaryOp::LogicalOr => {
                            let zero = self.context.i64_type().const_int(0, false);
                            let lhs_bool = self
                                .builder
                                .build_int_compare(IntPredicate::NE, lhs_int, zero, "tobool")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_int_compare".to_string(),
                                    details: "Failed to compare for LogicalOr".to_string(),
                                })?;

                            self.builder
                                .build_conditional_branch(lhs_bool, merge_bb, rhs_bb)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_conditional_branch".to_string(),
                                    details: "Failed to branch for LogicalOr".to_string(),
                                })?;
                        }
                        _ => unreachable!(),
                    }

                    self.builder.position_at_end(rhs_bb);
                    let (rhs_val, _) = self.compile_expr(rhs)?;
                    let rhs_int = rhs_val.into_int_value();

                    self.builder.build_unconditional_branch(merge_bb)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_unconditional_branch".to_string(),
                            details: "Failed to branch to merge in logical op".to_string(),
                        })?;
                    let rhs_end_bb = self.builder.get_insert_block()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_insert_block".to_string(),
                            details: "No block after logical rhs".to_string(),
                        })?;

                    self.builder.position_at_end(merge_bb);
                    let phi = self
                        .builder
                        .build_phi(self.context.i64_type(), "logic_result")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_phi".to_string(),
                            details: "Failed to build PHI for logical result".to_string(),
                        })?;

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

                    return Ok((phi.as_basic_value().into(), BrixType::Int));
                }

                let (mut lhs_val, mut lhs_type) = self.compile_expr(lhs)?;
                let (mut rhs_val, mut rhs_type) = self.compile_expr(rhs)?;

                // --- INTMATRIX → MATRIX PROMOTION (v1.1) ---
                // Automatically promote IntMatrix to Matrix when operating with Float or Matrix
                // Only for arithmetic operators: +, -, *, /, %, **
                let is_arithmetic_op = matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow
                );

                if is_arithmetic_op {
                    // Case 1: IntMatrix op Float → promote IntMatrix to Matrix
                    // Case 2: Float op IntMatrix → promote IntMatrix to Matrix
                    // Case 3: IntMatrix op Matrix → promote IntMatrix to Matrix
                    // Case 4: Matrix op IntMatrix → promote IntMatrix to Matrix
                    let needs_promotion = (lhs_type == BrixType::IntMatrix && rhs_type == BrixType::Float)
                        || (lhs_type == BrixType::Float && rhs_type == BrixType::IntMatrix)
                        || (lhs_type == BrixType::IntMatrix && rhs_type == BrixType::Matrix)
                        || (lhs_type == BrixType::Matrix && rhs_type == BrixType::IntMatrix);

                    if needs_promotion {
                        // Declare intmatrix_to_matrix runtime function
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                        let func = self.module.get_function("intmatrix_to_matrix").unwrap_or_else(|| {
                            self.module.add_function("intmatrix_to_matrix", fn_type, Some(Linkage::External))
                        });

                        // Promote left side if it's IntMatrix
                        if lhs_type == BrixType::IntMatrix {
                            let call = self
                                .builder
                                .build_call(func, &[lhs_val.into()], "promote_lhs")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_call".to_string(),
                                    details: "Failed to call intmatrix_to_matrix for lhs".to_string(),
                                })?;
                            let promoted = call.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "promoted matrix value".to_string(),
                                    context: "IntMatrix promotion lhs".to_string(),
                                })?;
                            lhs_val = promoted;
                            lhs_type = BrixType::Matrix;
                        }

                        // Promote right side if it's IntMatrix
                        if rhs_type == BrixType::IntMatrix {
                            let call = self
                                .builder
                                .build_call(func, &[rhs_val.into()], "promote_rhs")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_call".to_string(),
                                    details: "Failed to call intmatrix_to_matrix for rhs".to_string(),
                                })?;
                            let promoted = call.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "promoted matrix value".to_string(),
                                    context: "IntMatrix promotion rhs".to_string(),
                                })?;
                            rhs_val = promoted;
                            rhs_type = BrixType::Matrix;
                        }
                    }
                }

                // --- MATRIX ARITHMETIC OPERATIONS (v1.1) ---
                // Handle Matrix/IntMatrix operations with scalars and other matrices
                if is_arithmetic_op {
                    let ptr_type = self.context.ptr_type(AddressSpace::default());

                    // Matrix op scalar (Float or Int)
                    if lhs_type == BrixType::Matrix && (rhs_type == BrixType::Float || rhs_type == BrixType::Int) {
                        let fn_name = match op {
                            BinaryOp::Add => "matrix_add_scalar",
                            BinaryOp::Sub => "matrix_sub_scalar",
                            BinaryOp::Mul => "matrix_mul_scalar",
                            BinaryOp::Div => "matrix_div_scalar",
                            BinaryOp::Mod => "matrix_mod_scalar",
                            BinaryOp::Pow => "matrix_pow_scalar",
                            _ => unreachable!(),
                        };

                        // Convert Int to Float if necessary
                        let scalar_val = if rhs_type == BrixType::Int {
                            self.builder
                                .build_signed_int_to_float(rhs_val.into_int_value(), self.context.f64_type(), "int_to_float")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_signed_int_to_float".to_string(),
                                    details: "Failed to convert int to float for matrix scalar op".to_string(),
                                })?
                        } else {
                            rhs_val.into_float_value()
                        };

                        let fn_type = ptr_type.fn_type(&[ptr_type.into(), self.context.f64_type().into()], false);
                        let func = self.module.get_function(fn_name).unwrap_or_else(|| {
                            self.module.add_function(fn_name, fn_type, Some(Linkage::External))
                        });

                        let call = self
                            .builder
                            .build_call(func, &[lhs_val.into(), scalar_val.into()], "matrix_op")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call {}", fn_name),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "matrix op result".to_string(),
                                context: "Matrix op scalar".to_string(),
                            })?;

                        return Ok((result, BrixType::Matrix));
                    }

                    // scalar (Float or Int) op Matrix
                    if (lhs_type == BrixType::Float || lhs_type == BrixType::Int) && rhs_type == BrixType::Matrix {
                        let fn_name = match op {
                            BinaryOp::Add => "matrix_add_scalar",  // Commutative
                            BinaryOp::Sub => "scalar_sub_matrix",  // Non-commutative
                            BinaryOp::Mul => "matrix_mul_scalar",  // Commutative
                            BinaryOp::Div => "scalar_div_matrix",  // Non-commutative
                            _ => {
                                // For Mod and Pow, scalar op Matrix doesn't make sense
                                // Fall through to error
                                return Err(CodegenError::InvalidOperation {
                                    operation: format!("scalar {:?} Matrix", op),
                                    reason: "Mod and Pow operations not supported for scalar-matrix".to_string(),
                                });
                            }
                        };

                        let scalar_val = if lhs_type == BrixType::Int {
                            self.builder
                                .build_signed_int_to_float(lhs_val.into_int_value(), self.context.f64_type(), "int_to_float")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_signed_int_to_float".to_string(),
                                    details: "Failed to convert int to float for scalar-matrix op".to_string(),
                                })?
                        } else {
                            lhs_val.into_float_value()
                        };

                        // Commutative operations: swap arguments
                        let (arg1, arg2) = if matches!(op, BinaryOp::Add | BinaryOp::Mul) {
                            (rhs_val, scalar_val.as_basic_value_enum())
                        } else {
                            // Non-commutative: scalar is first arg
                            (scalar_val.as_basic_value_enum(), rhs_val)
                        };

                        let fn_type = if matches!(op, BinaryOp::Add | BinaryOp::Mul) {
                            ptr_type.fn_type(&[ptr_type.into(), self.context.f64_type().into()], false)
                        } else {
                            ptr_type.fn_type(&[self.context.f64_type().into(), ptr_type.into()], false)
                        };

                        let func = self.module.get_function(fn_name).unwrap_or_else(|| {
                            self.module.add_function(fn_name, fn_type, Some(Linkage::External))
                        });

                        let call = self
                            .builder
                            .build_call(func, &[arg1.into(), arg2.into()], "scalar_matrix_op")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call {}", fn_name),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "scalar-matrix op result".to_string(),
                                context: "scalar op Matrix".to_string(),
                            })?;

                        return Ok((result, BrixType::Matrix));
                    }

                    // Matrix op Matrix
                    if lhs_type == BrixType::Matrix && rhs_type == BrixType::Matrix {
                        let fn_name = match op {
                            BinaryOp::Add => "matrix_add_matrix",
                            BinaryOp::Sub => "matrix_sub_matrix",
                            BinaryOp::Mul => "matrix_mul_matrix",
                            BinaryOp::Div => "matrix_div_matrix",
                            BinaryOp::Mod => "matrix_mod_matrix",
                            BinaryOp::Pow => "matrix_pow_matrix",
                            _ => unreachable!(),
                        };

                        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
                        let func = self.module.get_function(fn_name).unwrap_or_else(|| {
                            self.module.add_function(fn_name, fn_type, Some(Linkage::External))
                        });

                        let call = self
                            .builder
                            .build_call(func, &[lhs_val.into(), rhs_val.into()], "matrix_matrix_op")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call {}", fn_name),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "matrix-matrix op result".to_string(),
                                context: "Matrix op Matrix".to_string(),
                            })?;

                        return Ok((result, BrixType::Matrix));
                    }

                    // IntMatrix op Int scalar
                    if lhs_type == BrixType::IntMatrix && rhs_type == BrixType::Int {
                        let fn_name = match op {
                            BinaryOp::Add => "intmatrix_add_scalar",
                            BinaryOp::Sub => "intmatrix_sub_scalar",
                            BinaryOp::Mul => "intmatrix_mul_scalar",
                            BinaryOp::Div => "intmatrix_div_scalar",
                            BinaryOp::Mod => "intmatrix_mod_scalar",
                            BinaryOp::Pow => "intmatrix_pow_scalar",
                            _ => unreachable!(),
                        };

                        let fn_type = ptr_type.fn_type(&[ptr_type.into(), self.context.i64_type().into()], false);
                        let func = self.module.get_function(fn_name).unwrap_or_else(|| {
                            self.module.add_function(fn_name, fn_type, Some(Linkage::External))
                        });

                        let call = self
                            .builder
                            .build_call(func, &[lhs_val.into(), rhs_val.into()], "intmatrix_op")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call {}", fn_name),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "intmatrix op result".to_string(),
                                context: "IntMatrix op Int".to_string(),
                            })?;

                        return Ok((result, BrixType::IntMatrix));
                    }

                    // Int scalar op IntMatrix
                    if lhs_type == BrixType::Int && rhs_type == BrixType::IntMatrix {
                        let fn_name = match op {
                            BinaryOp::Add => "intmatrix_add_scalar",  // Commutative
                            BinaryOp::Sub => "scalar_sub_intmatrix",  // Non-commutative
                            BinaryOp::Mul => "intmatrix_mul_scalar",  // Commutative
                            _ => {
                                // For Div, Mod, Pow: scalar op IntMatrix doesn't make sense
                                return Err(CodegenError::InvalidOperation {
                                    operation: format!("scalar {:?} IntMatrix", op),
                                    reason: "Div, Mod, and Pow operations not supported for scalar-intmatrix".to_string(),
                                });
                            }
                        };

                        let (arg1, arg2) = if matches!(op, BinaryOp::Add | BinaryOp::Mul) {
                            (rhs_val, lhs_val)
                        } else {
                            // scalar_sub_intmatrix(scalar, intmatrix)
                            (lhs_val, rhs_val)
                        };

                        let fn_type = if matches!(op, BinaryOp::Add | BinaryOp::Mul) {
                            ptr_type.fn_type(&[ptr_type.into(), self.context.i64_type().into()], false)
                        } else {
                            ptr_type.fn_type(&[self.context.i64_type().into(), ptr_type.into()], false)
                        };

                        let func = self.module.get_function(fn_name).unwrap_or_else(|| {
                            self.module.add_function(fn_name, fn_type, Some(Linkage::External))
                        });

                        let call = self
                            .builder
                            .build_call(func, &[arg1.into(), arg2.into()], "scalar_intmatrix_op")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call {}", fn_name),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "scalar-intmatrix op result".to_string(),
                                context: "Int op IntMatrix".to_string(),
                            })?;

                        return Ok((result, BrixType::IntMatrix));
                    }

                    // IntMatrix op IntMatrix
                    if lhs_type == BrixType::IntMatrix && rhs_type == BrixType::IntMatrix {
                        let fn_name = match op {
                            BinaryOp::Add => "intmatrix_add_intmatrix",
                            BinaryOp::Sub => "intmatrix_sub_intmatrix",
                            BinaryOp::Mul => "intmatrix_mul_intmatrix",
                            BinaryOp::Div => "intmatrix_div_intmatrix",
                            BinaryOp::Mod => "intmatrix_mod_intmatrix",
                            BinaryOp::Pow => "intmatrix_pow_intmatrix",
                            _ => unreachable!(),
                        };

                        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
                        let func = self.module.get_function(fn_name).unwrap_or_else(|| {
                            self.module.add_function(fn_name, fn_type, Some(Linkage::External))
                        });

                        let call = self
                            .builder
                            .build_call(func, &[lhs_val.into(), rhs_val.into()], "intmatrix_intmatrix_op")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call {}", fn_name),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "intmatrix-intmatrix op result".to_string(),
                                context: "IntMatrix op IntMatrix".to_string(),
                            })?;

                        return Ok((result, BrixType::IntMatrix));
                    }
                }

                // --- NIL COMPARISON: x == nil, x != nil, err == nil, err != nil ---
                // Handle comparisons with nil (null pointer comparison)
                // Allow comparison of Error, String, Matrix, etc. with Nil
                let is_pointer_type = |t: &BrixType| {
                    matches!(
                        t,
                        BrixType::Nil
                            | BrixType::Error
                            | BrixType::String
                            | BrixType::Matrix
                            | BrixType::IntMatrix
                            | BrixType::Complex
                            | BrixType::ComplexMatrix
                            | BrixType::FloatPtr
                    )
                };

                if (is_pointer_type(&lhs_type) || is_pointer_type(&rhs_type))
                    && (matches!(op, BinaryOp::Eq) || matches!(op, BinaryOp::NotEq))
                    && (lhs_type == BrixType::Nil || rhs_type == BrixType::Nil)
                {
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let null_ptr = ptr_type.const_null();

                    // Get the non-nil value (or use null if both are nil)
                    let value_to_compare = if lhs_type == BrixType::Nil {
                        rhs_val.into_pointer_value()
                    } else {
                        lhs_val.into_pointer_value()
                    };

                    // Compare pointer with null
                    let predicate = if matches!(op, BinaryOp::Eq) {
                        IntPredicate::EQ
                    } else {
                        IntPredicate::NE
                    };

                    let cmp = self
                        .builder
                        .build_int_compare(predicate, value_to_compare, null_ptr, "nil_cmp")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_int_compare".to_string(),
                            details: "Failed to compare with nil".to_string(),
                        })?;

                    // Extend i1 to i64 for consistency
                    let result = self
                        .builder
                        .build_int_z_extend(cmp, self.context.i64_type(), "nil_cmp_ext")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_int_z_extend".to_string(),
                            details: "Failed to extend nil comparison result".to_string(),
                        })?;

                    return Ok((result.into(), BrixType::Int));
                }

                // --- COMPLEX PATTERN DETECTION: 3.0 + 4.0i ---
                // Detect pattern: Float/Int +/- Complex(0, imag) → Complex(real, imag)
                if (lhs_type == BrixType::Float || lhs_type == BrixType::Int)
                    && rhs_type == BrixType::Complex
                    && (matches!(op, BinaryOp::Add) || matches!(op, BinaryOp::Sub))
                {
                    // Extract imaginary part from rhs
                    let rhs_struct = rhs_val.into_struct_value();
                    let rhs_real = self
                        .builder
                        .build_extract_value(rhs_struct, 0, "rhs_real")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_extract_value".to_string(),
                            details: "Failed to extract real part from complex rhs".to_string(),
                        })?
                        .into_float_value();
                    let rhs_imag = self
                        .builder
                        .build_extract_value(rhs_struct, 1, "rhs_imag")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_extract_value".to_string(),
                            details: "Failed to extract imag part from complex rhs".to_string(),
                        })?
                        .into_float_value();

                    // Check if rhs is pure imaginary (real part ≈ 0)
                    let zero = self.context.f64_type().const_float(0.0);
                    let _is_pure_imag = self
                        .builder
                        .build_float_compare(FloatPredicate::OEQ, rhs_real, zero, "is_pure_imag")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_float_compare".to_string(),
                            details: "Failed to check if complex is pure imaginary".to_string(),
                        })?;

                    // If pure imaginary, create complex from lhs + rhs_imag
                    // For now, assume it's always pure imaginary (parser creates Complex(0, imag) for "4.0i")

                    // Convert lhs to f64 if needed
                    let lhs_float = if lhs_type == BrixType::Int {
                        self.builder
                            .build_signed_int_to_float(
                                lhs_val.into_int_value(),
                                self.context.f64_type(),
                                "lhs_to_float",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_signed_int_to_float".to_string(),
                                details: "Failed to convert int to float for complex literal".to_string(),
                            })?
                    } else {
                        lhs_val.into_float_value()
                    };

                    // Create complex: (lhs_float, ±rhs_imag)
                    let final_imag = if matches!(op, BinaryOp::Sub) {
                        self.builder.build_float_neg(rhs_imag, "neg_imag")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_float_neg".to_string(),
                                details: "Failed to negate imaginary part".to_string(),
                            })?
                    } else {
                        rhs_imag
                    };

                    let complex_type = self.context.struct_type(
                        &[
                            self.context.f64_type().into(),
                            self.context.f64_type().into(),
                        ],
                        false,
                    );

                    let complex_val = self
                        .builder
                        .build_insert_value(complex_type.get_undef(), lhs_float, 0, "complex_real")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_insert_value".to_string(),
                            details: "Failed to insert real part into complex".to_string(),
                        })?;

                    let complex_val = self
                        .builder
                        .build_insert_value(complex_val, final_imag, 1, "complex_full")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_insert_value".to_string(),
                            details: "Failed to insert imag part into complex".to_string(),
                        })?;

                    return Ok((complex_val.into_struct_value().into(), BrixType::Complex));
                }

                // --- COMPLEX ARITHMETIC ---
                // If either operand is complex, promote and use complex arithmetic
                if lhs_type == BrixType::Complex || rhs_type == BrixType::Complex {
                    // Special handling for power operator - use optimized variants
                    if *op == BinaryOp::Pow && lhs_type == BrixType::Complex {
                        let base_complex = lhs_val.into_struct_value();
                        let complex_type = self.brix_type_to_llvm(&BrixType::Complex);

                        let result = if rhs_type == BrixType::Int {
                            // Use complex_powi for integer exponent
                            let fn_type = complex_type.fn_type(
                                &[complex_type.into(), self.context.i64_type().into()],
                                false,
                            );
                            let func =
                                self.module.get_function("complex_powi").unwrap_or_else(|| {
                                    self.module.add_function(
                                        "complex_powi",
                                        fn_type,
                                        Some(Linkage::External),
                                    )
                                });
                            {
                                let call = self.builder
                                    .build_call(
                                        func,
                                        &[base_complex.into(), rhs_val.into()],
                                        "complex_powi",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_call".to_string(),
                                        details: "Failed to call complex_powi".to_string(),
                                    })?;
                                call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "complex_powi result".to_string(),
                                        context: "Complex power (int exp)".to_string(),
                                    })?
                            }
                        } else if rhs_type == BrixType::Float {
                            // Use complex_powf for float exponent
                            let fn_type = complex_type.fn_type(
                                &[complex_type.into(), self.context.f64_type().into()],
                                false,
                            );
                            let func =
                                self.module.get_function("complex_powf").unwrap_or_else(|| {
                                    self.module.add_function(
                                        "complex_powf",
                                        fn_type,
                                        Some(Linkage::External),
                                    )
                                });
                            {
                                let call = self.builder
                                    .build_call(
                                        func,
                                        &[base_complex.into(), rhs_val.into()],
                                        "complex_powf",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_call".to_string(),
                                        details: "Failed to call complex_powf".to_string(),
                                    })?;
                                call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "complex_powf result".to_string(),
                                        context: "Complex power (float exp)".to_string(),
                                    })?
                            }
                        } else {
                            // Use complex_pow for complex exponent
                            let exp_complex = rhs_val.into_struct_value();
                            let fn_type = complex_type
                                .fn_type(&[complex_type.into(), complex_type.into()], false);
                            let func =
                                self.module.get_function("complex_pow").unwrap_or_else(|| {
                                    self.module.add_function(
                                        "complex_pow",
                                        fn_type,
                                        Some(Linkage::External),
                                    )
                                });
                            {
                                let call = self.builder
                                    .build_call(
                                        func,
                                        &[base_complex.into(), exp_complex.into()],
                                        "complex_pow",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_call".to_string(),
                                        details: "Failed to call complex_pow".to_string(),
                                    })?;
                                call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "complex_pow result".to_string(),
                                        context: "Complex power (complex exp)".to_string(),
                                    })?
                            }
                        };

                        return Ok((result, BrixType::Complex));
                    }

                    // For other operators, promote non-complex to complex
                    let lhs_complex = if lhs_type == BrixType::Complex {
                        lhs_val.into_struct_value()
                    } else {
                        let real_val = if lhs_type == BrixType::Int {
                            self.builder
                                .build_signed_int_to_float(
                                    lhs_val.into_int_value(),
                                    self.context.f64_type(),
                                    "int_to_float",
                                )
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_signed_int_to_float".to_string(),
                                    details: "Failed to convert lhs int to float for complex promotion".to_string(),
                                })?
                        } else {
                            lhs_val.into_float_value()
                        };
                        let zero = self.context.f64_type().const_float(0.0);
                        let complex_type = self.context.struct_type(
                            &[
                                self.context.f64_type().into(),
                                self.context.f64_type().into(),
                            ],
                            false,
                        );
                        let inner = self.builder
                            .build_insert_value(
                                complex_type.get_undef(),
                                real_val,
                                0,
                                "real",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_insert_value".to_string(),
                                details: "Failed to insert real part for lhs complex promotion".to_string(),
                            })?;
                        self.builder
                            .build_insert_value(inner, zero, 1, "imag")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_insert_value".to_string(),
                                details: "Failed to insert imag part for lhs complex promotion".to_string(),
                            })?
                            .into_struct_value()
                    };

                    let rhs_complex = if rhs_type == BrixType::Complex {
                        rhs_val.into_struct_value()
                    } else {
                        let real_val = if rhs_type == BrixType::Int {
                            self.builder
                                .build_signed_int_to_float(
                                    rhs_val.into_int_value(),
                                    self.context.f64_type(),
                                    "int_to_float",
                                )
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_signed_int_to_float".to_string(),
                                    details: "Failed to convert rhs int to float for complex promotion".to_string(),
                                })?
                        } else {
                            rhs_val.into_float_value()
                        };
                        let zero = self.context.f64_type().const_float(0.0);
                        let complex_type = self.context.struct_type(
                            &[
                                self.context.f64_type().into(),
                                self.context.f64_type().into(),
                            ],
                            false,
                        );
                        let inner = self.builder
                            .build_insert_value(
                                complex_type.get_undef(),
                                real_val,
                                0,
                                "real",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_insert_value".to_string(),
                                details: "Failed to insert real part for rhs complex promotion".to_string(),
                            })?;
                        self.builder
                            .build_insert_value(inner, zero, 1, "imag")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_insert_value".to_string(),
                                details: "Failed to insert imag part for rhs complex promotion".to_string(),
                            })?
                            .into_struct_value()
                    };

                    // Call appropriate complex function
                    let fn_name = match op {
                        BinaryOp::Add => "complex_add",
                        BinaryOp::Sub => "complex_sub",
                        BinaryOp::Mul => "complex_mul",
                        BinaryOp::Div => "complex_div",
                        BinaryOp::Pow => "complex_pow", // Fallback (shouldn't reach here for pow)
                        _ => {
                            eprintln!("Error: Operator {:?} not supported for complex numbers", op);
                            return Err(CodegenError::InvalidOperation {
                                operation: format!("{:?}", op),
                                reason: "Operator not supported for complex numbers".to_string(),
                            });
                        }
                    };

                    let complex_type = self.brix_type_to_llvm(&BrixType::Complex);
                    let fn_type =
                        complex_type.fn_type(&[complex_type.into(), complex_type.into()], false);
                    let func = self.module.get_function(fn_name).unwrap_or_else(|| {
                        self.module
                            .add_function(fn_name, fn_type, Some(Linkage::External))
                    });

                    let call = self
                        .builder
                        .build_call(
                            func,
                            &[lhs_complex.into(), rhs_complex.into()],
                            "complex_op",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_call".to_string(),
                            details: format!("Failed to call {}", fn_name),
                        })?;
                    let result = call.try_as_basic_value().left()
                        .ok_or_else(|| CodegenError::MissingValue {
                            what: "complex op result".to_string(),
                            context: "Complex arithmetic".to_string(),
                        })?;

                    return Ok((result, BrixType::Complex));
                }

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
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_call".to_string(),
                                    details: "Failed to call str_concat".to_string(),
                                })?;
                            let result = res.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "str_concat result".to_string(),
                                    context: "String concatenation".to_string(),
                                })?;
                            return Ok((result, BrixType::String));
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
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_call".to_string(),
                                    details: "Failed to call str_eq".to_string(),
                                })?;
                            let result = res.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "str_eq result".to_string(),
                                    context: "String equality".to_string(),
                                })?;
                            return Ok((result, BrixType::Int));
                        }
                        _ => {
                            eprintln!("Erro: Operação não suportada para strings (apenas + e ==).");
                            return Err(CodegenError::InvalidOperation {
                                operation: format!("{:?}", op),
                                reason: "Only + and == are supported for strings".to_string(),
                            });
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
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_signed_int_to_float".to_string(),
                                details: "Failed to cast lhs int to float".to_string(),
                            })?
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
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_signed_int_to_float".to_string(),
                                details: "Failed to cast rhs int to float".to_string(),
                            })?
                    } else {
                        rhs_val.into_float_value()
                    };

                    let val = self.compile_float_op(op, l_float, r_float).ok_or_else(|| {
                        CodegenError::InvalidOperation {
                            operation: format!("{:?}", op),
                            reason: "unsupported float operation".to_string(),
                        }
                    })?;

                    let res_type = match op {
                        BinaryOp::Gt
                        | BinaryOp::Lt
                        | BinaryOp::GtEq
                        | BinaryOp::LtEq
                        | BinaryOp::Eq
                        | BinaryOp::NotEq => BrixType::Int,
                        _ => BrixType::Float,
                    };
                    Ok((val, res_type))
                } else {
                    let val = self.compile_int_op(
                        op,
                        lhs_val.into_int_value(),
                        rhs_val.into_int_value(),
                    ).ok_or_else(|| {
                        CodegenError::InvalidOperation {
                            operation: format!("{:?}", op),
                            reason: "unsupported integer operation".to_string(),
                        }
                    })?;
                    Ok((val, BrixType::Int))
                }
            }

            Expr::Call { func, args } => {
                // Handle math.function() calls (e.g., math.sin, math.cos, math.sum, etc.)
                if let Expr::FieldAccess { target, field } = func.as_ref() {
                    if let Expr::Identifier(_module_name) = target.as_ref() {
                        // Check if this is a math module function
                        let fn_name = field.as_str();

                        // Check for brix_ prefixed functions (stats/linalg)
                        let brix_fn_name = format!("brix_{}", fn_name);
                        let lookup_name = if self.module.get_function(&brix_fn_name).is_some() {
                            &brix_fn_name
                        } else {
                            fn_name
                        };

                        if let Some(llvm_fn) = self.module.get_function(lookup_name) {
                            // Compile arguments
                            let mut llvm_args = Vec::new();
                            for arg in args {
                                let (arg_val, arg_type) = self.compile_expr(arg)?;

                                // For stats/linalg functions, pass Matrix* directly
                                if arg_type == BrixType::Matrix || arg_type == BrixType::IntMatrix {
                                    llvm_args.push(arg_val.into());
                                } else if fn_name == "eye" {
                                    // eye(n) expects i64, don't convert to float
                                    llvm_args.push(arg_val.into());
                                } else {
                                    // Auto-convert Int to Float for regular math functions
                                    let final_val = if arg_type == BrixType::Int {
                                        self.builder
                                            .build_signed_int_to_float(
                                                arg_val.into_int_value(),
                                                self.context.f64_type(),
                                                "int_to_float_arg",
                                            )
                                            .map_err(|_| CodegenError::LLVMError {
                                                operation: "build_signed_int_to_float".to_string(),
                                                details: "Failed to convert int arg to float for math call".to_string(),
                                            })?
                                            .into()
                                    } else {
                                        arg_val
                                    };

                                    llvm_args.push(final_val.into());
                                }
                            }

                            // Call the function
                            let call = self
                                .builder
                                .build_call(llvm_fn, &llvm_args, "math_call")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_call".to_string(),
                                    details: format!("Failed to call math function {}", lookup_name),
                                })?;
                            let result = call.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "math function result".to_string(),
                                    context: format!("math.{}", fn_name),
                                })?;

                            // Determine return type based on function name
                            let return_type =
                                if fn_name == "tr" || fn_name == "inv" || fn_name == "eye" {
                                    BrixType::Matrix
                                } else if fn_name == "eigvals" || fn_name == "eigvecs" {
                                    BrixType::ComplexMatrix
                                } else {
                                    BrixType::Float
                                };

                            return Ok((result, return_type));
                        }
                    }
                }

                if let Expr::Identifier(fn_name) = func.as_ref() {
                    if fn_name == "typeof" {
                        if args.len() != 1 {
                            eprintln!("Error: typeof expects exactly 1 argument.");
                            return Err(CodegenError::InvalidOperation {
                                operation: "typeof".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                            });
                        }
                        let (_, arg_type) = self.compile_expr(&args[0])?;

                        let type_str = match arg_type {
                            BrixType::Int => "int",
                            BrixType::Float => "float",
                            BrixType::String => "string",
                            BrixType::Matrix => "matrix",
                            BrixType::IntMatrix => "intmatrix",
                            BrixType::Complex => "complex",
                            BrixType::ComplexArray => "complexarray",
                            BrixType::ComplexMatrix => "complexmatrix",
                            BrixType::FloatPtr => "float_ptr",
                            BrixType::Void => "void",
                            BrixType::Tuple(_) => "tuple",
                            BrixType::Nil => "nil",
                            BrixType::Error => "error",
                            BrixType::Atom => "atom",
                        };

                        return self
                            .compile_expr(&Expr::Literal(Literal::String(type_str.to_string())));
                    }

                    // Conversion functions
                    if fn_name == "int" {
                        if args.len() != 1 {
                            eprintln!("Error: int() expects exactly 1 argument.");
                            return Err(CodegenError::InvalidOperation {
                                operation: "int()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                            });
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        let result = match val_type {
                            BrixType::Int => val, // Already int
                            BrixType::Float => {
                                // Float to int (truncate)
                                self.builder
                                    .build_float_to_signed_int(
                                        val.into_float_value(),
                                        self.context.i64_type(),
                                        "float_to_int",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_float_to_signed_int".to_string(),
                                        details: "Failed to convert float to int".to_string(),
                                    })?
                                    .into()
                            }
                            BrixType::String => {
                                // String to int using atoi()
                                let atoi_fn = self.get_atoi();

                                // Extract char* from BrixString
                                let struct_ptr = val.into_pointer_value();
                                let str_type = self.get_string_type();
                                let data_ptr_ptr = self
                                    .builder
                                    .build_struct_gep(str_type, struct_ptr, 1, "str_data_ptr")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_struct_gep".to_string(),
                                        details: "Failed to get string data ptr for int()".to_string(),
                                    })?;
                                let data_ptr = self
                                    .builder
                                    .build_load(
                                        self.context.ptr_type(AddressSpace::default()),
                                        data_ptr_ptr,
                                        "str_data",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_load".to_string(),
                                        details: "Failed to load string data for int()".to_string(),
                                    })?;

                                let call = self
                                    .builder
                                    .build_call(atoi_fn, &[data_ptr.into()], "atoi_result")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_call".to_string(),
                                        details: "Failed to call atoi".to_string(),
                                    })?;
                                let i32_result = call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "atoi result".to_string(),
                                        context: "int() conversion".to_string(),
                                    })?;

                                // Extend i32 to i64
                                self.builder
                                    .build_int_s_extend(
                                        i32_result.into_int_value(),
                                        self.context.i64_type(),
                                        "int_extend",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_s_extend".to_string(),
                                        details: "Failed to extend i32 to i64".to_string(),
                                    })?
                                    .into()
                            }
                            _ => {
                                eprintln!("Error: Cannot convert {:?} to int", val_type);
                                return Err(CodegenError::TypeError {
                                    expected: "int-convertible type".to_string(),
                                    found: format!("{:?}", val_type),
                                    context: "int() conversion".to_string(),
                                });
                            }
                        };

                        return Ok((result, BrixType::Int));
                    }

                    if fn_name == "float" {
                        if args.len() != 1 {
                            eprintln!("Error: float() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        let result = match val_type {
                            BrixType::Float => val, // Already float
                            BrixType::Int => {
                                // Int to float
                                self.builder
                                    .build_signed_int_to_float(
                                        val.into_int_value(),
                                        self.context.f64_type(),
                                        "int_to_float",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_signed_int_to_float".to_string(),
                                        details: "Failed to convert int to float in float()".to_string(),
                                    })?
                                    .into()
                            }
                            BrixType::String => {
                                // String to float using atof()
                                let atof_fn = self.get_atof();

                                // Extract char* from BrixString
                                let struct_ptr = val.into_pointer_value();
                                let str_type = self.get_string_type();
                                let data_ptr_ptr = self
                                    .builder
                                    .build_struct_gep(str_type, struct_ptr, 1, "str_data_ptr")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_struct_gep".to_string(),
                                        details: "Failed to get string data ptr for float()".to_string(),
                                    })?;
                                let data_ptr = self
                                    .builder
                                    .build_load(
                                        self.context.ptr_type(AddressSpace::default()),
                                        data_ptr_ptr,
                                        "str_data",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_load".to_string(),
                                        details: "Failed to load string data for float()".to_string(),
                                    })?;

                                let call = self.builder
                                    .build_call(atof_fn, &[data_ptr.into()], "atof_result")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_call".to_string(),
                                        details: "Failed to call atof".to_string(),
                                    })?;
                                call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "atof result".to_string(),
                                        context: "float() conversion".to_string(),
                                    })?
                            }
                            _ => {
                                eprintln!("Error: Cannot convert {:?} to float", val_type);
                                return Err(CodegenError::General("compilation error".to_string()));
                            }
                        };

                        return Ok((result, BrixType::Float));
                    }

                    if fn_name == "string" {
                        if args.len() != 1 {
                            eprintln!("Error: string() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        // Reuse value_to_string() which already handles all types
                        let result = self.value_to_string(val, &val_type, None)?;
                        return Ok((result, BrixType::String));
                    }

                    if fn_name == "bool" {
                        if args.len() != 1 {
                            eprintln!("Error: bool() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        let result = match val_type {
                            BrixType::Int => {
                                // Int to bool: x != 0
                                let zero = self.context.i64_type().const_int(0, false);
                                let cmp = self
                                    .builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::NE,
                                        val.into_int_value(),
                                        zero,
                                        "int_to_bool",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_compare".to_string(),
                                        details: "Failed to compare int for bool()".to_string(),
                                    })?;

                                // Extend i1 to i64
                                self.builder
                                    .build_int_z_extend(cmp, self.context.i64_type(), "bool_extend")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_z_extend".to_string(),
                                        details: "Failed to extend bool result".to_string(),
                                    })?
                                    .into()
                            }
                            BrixType::Float => {
                                // Float to bool: x != 0.0
                                let zero = self.context.f64_type().const_float(0.0);
                                let cmp = self
                                    .builder
                                    .build_float_compare(
                                        inkwell::FloatPredicate::ONE,
                                        val.into_float_value(),
                                        zero,
                                        "float_to_bool",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_float_compare".to_string(),
                                        details: "Failed to compare float for bool()".to_string(),
                                    })?;

                                // Extend i1 to i64
                                self.builder
                                    .build_int_z_extend(cmp, self.context.i64_type(), "bool_extend")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_z_extend".to_string(),
                                        details: "Failed to extend float bool result".to_string(),
                                    })?
                                    .into()
                            }
                            BrixType::String => {
                                // String to bool: len > 0
                                let struct_ptr = val.into_pointer_value();
                                let str_type = self.get_string_type();
                                let len_ptr = self
                                    .builder
                                    .build_struct_gep(str_type, struct_ptr, 0, "str_len_ptr")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_struct_gep".to_string(),
                                        details: "Failed to get string len ptr for bool()".to_string(),
                                    })?;
                                let len_val = self
                                    .builder
                                    .build_load(self.context.i64_type(), len_ptr, "str_len")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_load".to_string(),
                                        details: "Failed to load string length for bool()".to_string(),
                                    })?
                                    .into_int_value();

                                let zero = self.context.i64_type().const_int(0, false);
                                let cmp = self
                                    .builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::SGT,
                                        len_val,
                                        zero,
                                        "str_to_bool",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_compare".to_string(),
                                        details: "Failed to compare string length for bool()".to_string(),
                                    })?;

                                // Extend i1 to i64
                                self.builder
                                    .build_int_z_extend(cmp, self.context.i64_type(), "bool_extend")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_z_extend".to_string(),
                                        details: "Failed to extend string bool result".to_string(),
                                    })?
                                    .into()
                            }
                            _ => {
                                return Err(CodegenError::TypeError {
                                    expected: "Int, Float, or String".to_string(),
                                    found: format!("{:?}", val_type),
                                    context: "bool() conversion".to_string(),
                                });
                            }
                        };

                        return Ok((result, BrixType::Int)); // bool is represented as int
                    }

                    // ===== TYPE CHECKING FUNCTIONS =====
                    // All return 1 (true) or 0 (false) as i64

                    // is_nil(x) - Check if value is nil (null pointer for pointer types)
                    if fn_name == "is_nil" {
                        if args.len() != 1 {
                            eprintln!("Error: is_nil() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        let result = match val_type {
                            BrixType::Nil => {
                                // nil is always nil
                                self.context.i64_type().const_int(1, false).into()
                            }
                            BrixType::Error | BrixType::String => {
                                // Check if pointer is null
                                let ptr = val.into_pointer_value();
                                let null_ptr =
                                    self.context.ptr_type(AddressSpace::default()).const_null();
                                let cmp = self
                                    .builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        ptr,
                                        null_ptr,
                                        "is_nil_cmp",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_compare".to_string(),
                                        details: "Failed to compare pointer for is_nil()".to_string(),
                                    })?;

                                self.builder
                                    .build_int_z_extend(
                                        cmp,
                                        self.context.i64_type(),
                                        "is_nil_result",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_z_extend".to_string(),
                                        details: "Failed to extend is_nil result".to_string(),
                                    })?
                                    .into()
                            }
                            _ => {
                                // Non-pointer types are never nil
                                self.context.i64_type().const_int(0, false).into()
                            }
                        };

                        return Ok((result, BrixType::Int));
                    }

                    // is_atom(x) - Check if value is atom
                    if fn_name == "is_atom" {
                        if args.len() != 1 {
                            eprintln!("Error: is_atom() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (_, val_type) = self.compile_expr(&args[0])?;

                        let result = if val_type == BrixType::Atom {
                            self.context.i64_type().const_int(1, false)
                        } else {
                            self.context.i64_type().const_int(0, false)
                        };

                        return Ok((result.into(), BrixType::Int));
                    }

                    // is_boolean(x) - Check if int value is 0 or 1
                    if fn_name == "is_boolean" {
                        if args.len() != 1 {
                            eprintln!("Error: is_boolean() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        let result = match val_type {
                            BrixType::Int => {
                                // Check if x == 0 || x == 1
                                let int_val = val.into_int_value();
                                let zero = self.context.i64_type().const_int(0, false);
                                let one = self.context.i64_type().const_int(1, false);

                                let is_zero = self
                                    .builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        int_val,
                                        zero,
                                        "is_zero",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_compare".to_string(),
                                        details: "Failed to compare for is_boolean() (is_zero)".to_string(),
                                    })?;

                                let is_one = self
                                    .builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        int_val,
                                        one,
                                        "is_one",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_compare".to_string(),
                                        details: "Failed to compare for is_boolean() (is_one)".to_string(),
                                    })?;

                                let is_bool = self
                                    .builder
                                    .build_or(is_zero, is_one, "is_bool_or")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_or".to_string(),
                                        details: "Failed to OR for is_boolean()".to_string(),
                                    })?;

                                self.builder
                                    .build_int_z_extend(
                                        is_bool,
                                        self.context.i64_type(),
                                        "is_bool_result",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_z_extend".to_string(),
                                        details: "Failed to extend is_boolean result".to_string(),
                                    })?
                                    .into()
                            }
                            _ => {
                                // Non-int types are not boolean
                                self.context.i64_type().const_int(0, false).into()
                            }
                        };

                        return Ok((result, BrixType::Int));
                    }

                    // is_integer(x) - Check if value is int
                    if fn_name == "is_integer" {
                        if args.len() != 1 {
                            eprintln!("Error: is_integer() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (_, val_type) = self.compile_expr(&args[0])?;

                        let result = if val_type == BrixType::Int {
                            self.context.i64_type().const_int(1, false)
                        } else {
                            self.context.i64_type().const_int(0, false)
                        };

                        return Ok((result.into(), BrixType::Int));
                    }

                    // is_float(x) - Check if value is float
                    if fn_name == "is_float" {
                        if args.len() != 1 {
                            eprintln!("Error: is_float() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (_, val_type) = self.compile_expr(&args[0])?;

                        let result = if val_type == BrixType::Float {
                            self.context.i64_type().const_int(1, false)
                        } else {
                            self.context.i64_type().const_int(0, false)
                        };

                        return Ok((result.into(), BrixType::Int));
                    }

                    // is_number(x) - Check if value is int or float
                    if fn_name == "is_number" {
                        if args.len() != 1 {
                            eprintln!("Error: is_number() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (_, val_type) = self.compile_expr(&args[0])?;

                        let result = if val_type == BrixType::Int || val_type == BrixType::Float {
                            self.context.i64_type().const_int(1, false)
                        } else {
                            self.context.i64_type().const_int(0, false)
                        };

                        return Ok((result.into(), BrixType::Int));
                    }

                    // is_string(x) - Check if value is string
                    if fn_name == "is_string" {
                        if args.len() != 1 {
                            eprintln!("Error: is_string() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (_, val_type) = self.compile_expr(&args[0])?;

                        let result = if val_type == BrixType::String {
                            self.context.i64_type().const_int(1, false)
                        } else {
                            self.context.i64_type().const_int(0, false)
                        };

                        return Ok((result.into(), BrixType::Int));
                    }

                    // is_list(x) - Check if value is matrix or intmatrix
                    if fn_name == "is_list" {
                        if args.len() != 1 {
                            eprintln!("Error: is_list() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (_, val_type) = self.compile_expr(&args[0])?;

                        let result =
                            if val_type == BrixType::Matrix || val_type == BrixType::IntMatrix {
                                self.context.i64_type().const_int(1, false)
                            } else {
                                self.context.i64_type().const_int(0, false)
                            };

                        return Ok((result.into(), BrixType::Int));
                    }

                    // is_tuple(x) - Check if value is tuple
                    if fn_name == "is_tuple" {
                        if args.len() != 1 {
                            eprintln!("Error: is_tuple() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (_, val_type) = self.compile_expr(&args[0])?;

                        let result = if matches!(val_type, BrixType::Tuple(_)) {
                            self.context.i64_type().const_int(1, false)
                        } else {
                            self.context.i64_type().const_int(0, false)
                        };

                        return Ok((result.into(), BrixType::Int));
                    }

                    // is_function(x) - Check if value is function (not implemented yet, always returns 0)
                    if fn_name == "is_function" {
                        if args.len() != 1 {
                            eprintln!("Error: is_function() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let _ = self.compile_expr(&args[0])?;

                        // Functions are not first-class yet, so always return false
                        let result = self.context.i64_type().const_int(0, false);
                        return Ok((result.into(), BrixType::Int));
                    }

                    // ===== STRING FUNCTIONS (v1.1) =====

                    // uppercase(str) - Convert string to uppercase
                    if fn_name == "uppercase" {
                        if args.len() != 1 {
                            eprintln!("Error: uppercase() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            eprintln!("Error: uppercase() expects a string argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let uppercase_fn = self.get_uppercase();
                        let call = self
                            .builder
                            .build_call(uppercase_fn, &[val.into()], "uppercase_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call uppercase()".to_string(),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "uppercase()".to_string(),
                            }
                        })?;

                        return Ok((result, BrixType::String));
                    }

                    // lowercase(str) - Convert string to lowercase
                    if fn_name == "lowercase" {
                        if args.len() != 1 {
                            eprintln!("Error: lowercase() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            eprintln!("Error: lowercase() expects a string argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let lowercase_fn = self.get_lowercase();
                        let call = self
                            .builder
                            .build_call(lowercase_fn, &[val.into()], "lowercase_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call lowercase()".to_string(),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "lowercase()".to_string(),
                            }
                        })?;

                        return Ok((result, BrixType::String));
                    }

                    // capitalize(str) - Capitalize first character
                    if fn_name == "capitalize" {
                        if args.len() != 1 {
                            eprintln!("Error: capitalize() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            eprintln!("Error: capitalize() expects a string argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let capitalize_fn = self.get_capitalize();
                        let call = self
                            .builder
                            .build_call(capitalize_fn, &[val.into()], "capitalize_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call capitalize()".to_string(),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "capitalize()".to_string(),
                            }
                        })?;

                        return Ok((result, BrixType::String));
                    }

                    // byte_size(str) - Get byte size of string
                    if fn_name == "byte_size" {
                        if args.len() != 1 {
                            eprintln!("Error: byte_size() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            eprintln!("Error: byte_size() expects a string argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let byte_size_fn = self.get_byte_size();
                        let call = self
                            .builder
                            .build_call(byte_size_fn, &[val.into()], "byte_size_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call byte_size()".to_string(),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "byte_size()".to_string(),
                            }
                        })?;

                        return Ok((result, BrixType::Int));
                    }

                    // length(str) - Get number of characters (UTF-8 aware)
                    if fn_name == "length" {
                        if args.len() != 1 {
                            eprintln!("Error: length() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            eprintln!("Error: length() expects a string argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let length_fn = self.get_length();
                        let call = self
                            .builder
                            .build_call(length_fn, &[val.into()], "length_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call length()".to_string(),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "length()".to_string(),
                            }
                        })?;

                        return Ok((result, BrixType::Int));
                    }

                    // replace(str, old, new) - Replace first occurrence
                    if fn_name == "replace" {
                        if args.len() != 3 {
                            eprintln!(
                                "Error: replace() expects exactly 3 arguments (str, old, new)."
                            );
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (str_val, str_type) = self.compile_expr(&args[0])?;
                        let (old_val, old_type) = self.compile_expr(&args[1])?;
                        let (new_val, new_type) = self.compile_expr(&args[2])?;

                        if str_type != BrixType::String
                            || old_type != BrixType::String
                            || new_type != BrixType::String
                        {
                            eprintln!("Error: replace() expects all arguments to be strings.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let replace_fn = self.get_replace();
                        let call = self
                            .builder
                            .build_call(
                                replace_fn,
                                &[str_val.into(), old_val.into(), new_val.into()],
                                "replace_result",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call replace()".to_string(),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "replace()".to_string(),
                            }
                        })?;

                        return Ok((result, BrixType::String));
                    }

                    // replace_all(str, old, new) - Replace all occurrences
                    if fn_name == "replace_all" {
                        if args.len() != 3 {
                            eprintln!(
                                "Error: replace_all() expects exactly 3 arguments (str, old, new)."
                            );
                            return Err(CodegenError::General("compilation error".to_string()));
                        }
                        let (str_val, str_type) = self.compile_expr(&args[0])?;
                        let (old_val, old_type) = self.compile_expr(&args[1])?;
                        let (new_val, new_type) = self.compile_expr(&args[2])?;

                        if str_type != BrixType::String
                            || old_type != BrixType::String
                            || new_type != BrixType::String
                        {
                            eprintln!("Error: replace_all() expects all arguments to be strings.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let replace_all_fn = self.get_replace_all();
                        let call = self
                            .builder
                            .build_call(
                                replace_all_fn,
                                &[str_val.into(), old_val.into(), new_val.into()],
                                "replace_all_result",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call replace_all()".to_string(),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "replace_all()".to_string(),
                            }
                        })?;

                        return Ok((result, BrixType::String));
                    }

                    // error(msg: string) -> error - create error
                    if fn_name == "error" {
                        if args.len() != 1 {
                            eprintln!(
                                "Error: error() expects exactly 1 argument (message string)."
                            );
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let (msg_val, msg_type) = self.compile_expr(&args[0])?;

                        if msg_type != BrixType::String {
                            eprintln!("Error: error() expects a string argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        // Declare brix_error_new(char* msg) -> BrixError*
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                        let error_new_fn = self
                            .module
                            .get_function("brix_error_new")
                            .unwrap_or_else(|| {
                                self.module.add_function(
                                    "brix_error_new",
                                    fn_type,
                                    Some(Linkage::External),
                                )
                            });

                        // Extract char* from BrixString struct
                        let str_struct_ptr = msg_val.into_pointer_value();
                        let str_type = self.get_string_type();
                        let data_ptr_ptr = self
                            .builder
                            .build_struct_gep(str_type, str_struct_ptr, 1, "str_data_ptr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_struct_gep".to_string(),
                                details: "Failed to get string data pointer for error()".to_string(),
                            })?;
                        let char_ptr = self
                            .builder
                            .build_load(ptr_type, data_ptr_ptr, "str_data")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load string data for error()".to_string(),
                            })?
                            .into_pointer_value();

                        // Call brix_error_new(char_ptr)
                        let call = self
                            .builder
                            .build_call(error_new_fn, &[char_ptr.into()], "error_new")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call brix_error_new()".to_string(),
                            })?;
                        let error_ptr = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "brix_error_new()".to_string(),
                            }
                        })?;

                        return Ok((error_ptr, BrixType::Error));
                    }

                    // === COMPLEX NUMBER FUNCTIONS ===

                    // complex(re, im) - constructor
                    if fn_name == "complex" {
                        if args.len() != 2 {
                            eprintln!("Error: complex() expects exactly 2 arguments (real, imag).");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let (re_val, re_type) = self.compile_expr(&args[0])?;
                        let (im_val, im_type) = self.compile_expr(&args[1])?;

                        // Convert to float if needed
                        let re_float = if re_type == BrixType::Int {
                            self.builder
                                .build_signed_int_to_float(
                                    re_val.into_int_value(),
                                    self.context.f64_type(),
                                    "re_to_float",
                                )
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_signed_int_to_float".to_string(),
                                    details: "Failed to convert real part to float for complex()".to_string(),
                                })?
                        } else {
                            re_val.into_float_value()
                        };

                        let im_float = if im_type == BrixType::Int {
                            self.builder
                                .build_signed_int_to_float(
                                    im_val.into_int_value(),
                                    self.context.f64_type(),
                                    "im_to_float",
                                )
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_signed_int_to_float".to_string(),
                                    details: "Failed to convert imag part to float for complex()".to_string(),
                                })?
                        } else {
                            im_val.into_float_value()
                        };

                        // Create complex struct
                        let complex_type = self.context.struct_type(
                            &[
                                self.context.f64_type().into(),
                                self.context.f64_type().into(),
                            ],
                            false,
                        );

                        let inner = self
                            .builder
                            .build_insert_value(
                                complex_type.get_undef(),
                                re_float,
                                0,
                                "real",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_insert_value".to_string(),
                                details: "Failed to insert real part for complex()".to_string(),
                            })?;
                        let complex_val = self
                            .builder
                            .build_insert_value(
                                inner,
                                im_float,
                                1,
                                "imag",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_insert_value".to_string(),
                                details: "Failed to insert imag part for complex()".to_string(),
                            })?;

                        return Ok((complex_val.into_struct_value().into(), BrixType::Complex));
                    }

                    // real(z) - extract real part
                    if fn_name == "real" {
                        if args.len() != 1 {
                            eprintln!("Error: real() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let (val, val_type) = self.compile_expr(&args[0])?;
                        if val_type != BrixType::Complex {
                            eprintln!("Error: real() expects a complex number.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let real_part = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 0, "real_part")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_extract_value".to_string(),
                                details: "Failed to extract real part".to_string(),
                            })?
                            .into_float_value();

                        return Ok((real_part.into(), BrixType::Float));
                    }

                    // imag(z) - extract imaginary part
                    if fn_name == "imag" {
                        if args.len() != 1 {
                            eprintln!("Error: imag() expects exactly 1 argument.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let (val, val_type) = self.compile_expr(&args[0])?;
                        if val_type != BrixType::Complex {
                            eprintln!("Error: imag() expects a complex number.");
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let imag_part = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 1, "imag_part")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_extract_value".to_string(),
                                details: "Failed to extract imaginary part".to_string(),
                            })?
                            .into_float_value();

                        return Ok((imag_part.into(), BrixType::Float));
                    }

                    // Single-argument complex functions that return complex
                    let complex_to_complex_fns = [
                        "conj", "exp", "log", "sqrt", "csin", "ccos", "ctan", "csinh", "ccosh",
                        "ctanh",
                    ];
                    if complex_to_complex_fns.contains(&fn_name.as_str()) {
                        if args.len() != 1 {
                            eprintln!("Error: {}() expects exactly 1 argument.", fn_name);
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let (val, val_type) = self.compile_expr(&args[0])?;
                        if val_type != BrixType::Complex {
                            eprintln!("Error: {}() expects a complex number.", fn_name);
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let runtime_fn_name = format!("complex_{}", fn_name);
                        let complex_type = self.brix_type_to_llvm(&BrixType::Complex);
                        let fn_type = complex_type.fn_type(&[complex_type.into()], false);
                        let func =
                            self.module
                                .get_function(&runtime_fn_name)
                                .unwrap_or_else(|| {
                                    self.module.add_function(
                                        &runtime_fn_name,
                                        fn_type,
                                        Some(Linkage::External),
                                    )
                                });

                        let call_site = self
                            .builder
                            .build_call(func, &[val.into()], &format!("{}_result", fn_name))
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call complex_{}", fn_name),
                            })?;
                        let result = call_site
                            .try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: format!("complex_{} return value", fn_name),
                                context: "complex function call".to_string(),
                            })?;

                        return Ok((result, BrixType::Complex));
                    }

                    // Single-argument complex functions that return float
                    let complex_to_float_fns = ["abs", "abs2", "angle"];
                    if complex_to_float_fns.contains(&fn_name.as_str()) {
                        if args.len() != 1 {
                            eprintln!("Error: {}() expects exactly 1 argument.", fn_name);
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let (val, val_type) = self.compile_expr(&args[0])?;
                        if val_type != BrixType::Complex {
                            eprintln!("Error: {}() expects a complex number.", fn_name);
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        let runtime_fn_name = format!("complex_{}", fn_name);
                        let complex_type = self.brix_type_to_llvm(&BrixType::Complex);
                        let f64_type = self.context.f64_type();
                        let fn_type = f64_type.fn_type(&[complex_type.into()], false);
                        let func =
                            self.module
                                .get_function(&runtime_fn_name)
                                .unwrap_or_else(|| {
                                    self.module.add_function(
                                        &runtime_fn_name,
                                        fn_type,
                                        Some(Linkage::External),
                                    )
                                });

                        let call_site = self
                            .builder
                            .build_call(func, &[val.into()], &format!("{}_result", fn_name))
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call complex_{}", fn_name),
                            })?;
                        let result = call_site
                            .try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: format!("complex_{} return value", fn_name),
                                context: "complex to float function call".to_string(),
                            })?;

                        return Ok((result, BrixType::Float));
                    }

                    if fn_name == "input" {
                        return self.compile_input_call(args).ok_or_else(|| {
                            CodegenError::General("input() call failed".to_string())
                        });
                    }
                    if fn_name == "matrix" {
                        let val = self.compile_matrix_constructor(args).ok_or_else(|| {
                            CodegenError::General("matrix() constructor failed".to_string())
                        })?;
                        return Ok((val, BrixType::Matrix));
                    }
                    if fn_name == "read_csv" {
                        let ptr = self.compile_read_csv(args).ok_or_else(|| {
                            CodegenError::General("read_csv() call failed".to_string())
                        })?;
                        return Ok((ptr, BrixType::Matrix));
                    }
                    if fn_name == "zeros" {
                        let val = self.compile_zeros(args)?;
                        return Ok((val, BrixType::Matrix));
                    }
                    if fn_name == "izeros" {
                        let val = self.compile_izeros(args)?;
                        return Ok((val, BrixType::IntMatrix));
                    }
                    if fn_name == "zip" {
                        let (val, tuple_type) = self.compile_zip(args).ok_or_else(|| {
                            CodegenError::General("zip() call failed".to_string())
                        })?;
                        return Ok((val, tuple_type));
                    }
                }

                // Check if it's a user-defined function
                if let Expr::Identifier(fn_name) = func.as_ref() {
                    // Clone the data we need to avoid borrow conflicts
                    let fn_data = self.functions.get(fn_name).map(|(f, r)| (*f, r.clone()));

                    if let Some((user_fn, ret_types_opt)) = fn_data {
                        // Get parameter metadata to check for defaults
                        let param_metadata = self.function_params.get(fn_name).cloned();

                        // Compile provided arguments
                        let mut llvm_args = Vec::new();
                        for arg in args {
                            if let Ok((arg_val, _)) = self.compile_expr(arg) {
                                llvm_args.push(arg_val.into());
                            }
                        }

                        // Check if we need to add default arguments
                        if let Some(params) = param_metadata {
                            let num_provided = args.len();
                            let num_required = params.len();

                            if num_provided < num_required {
                                // Fill in default values for missing parameters
                                for i in num_provided..num_required {
                                    let (_param_name, _param_type, default_opt) = &params[i];

                                    if let Some(default_expr) = default_opt {
                                        // Compile the default value expression
                                        if let Ok((default_val, _)) =
                                            self.compile_expr(default_expr)
                                        {
                                            llvm_args.push(default_val.into());
                                        } else {
                                            return Err(CodegenError::MissingValue {
                                                what: format!("default value for parameter {}", i),
                                                context: "function call".to_string(),
                                            });
                                        }
                                    } else {
                                        eprintln!(
                                            "Error: Missing required parameter {} for function {}",
                                            i, fn_name
                                        );
                                        return Err(CodegenError::General("compilation error".to_string()));
                                    }
                                }
                            } else if num_provided > num_required {
                                eprintln!(
                                    "Error: Too many arguments for function {} (expected {}, got {})",
                                    fn_name, num_required, num_provided
                                );
                                return Err(CodegenError::General("compilation error".to_string()));
                            }
                        }

                        // Call the user function
                        let call_result = self
                            .builder
                            .build_call(user_fn, &llvm_args, "call")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call user function '{}'", fn_name),
                            })?;

                        // Determine return type
                        if let Some(ret_types) = ret_types_opt {
                            if ret_types.is_empty() {
                                // Void function
                                return Err(CodegenError::General("compilation error".to_string()));
                            } else if ret_types.len() == 1 {
                                // Single return
                                let result = call_result.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "function return value".to_string(),
                                        context: format!("call to '{}'", fn_name),
                                    })?;
                                return Ok((result, ret_types[0].clone()));
                            } else {
                                // Multiple returns - return struct as Tuple type
                                let result = call_result.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "function return value".to_string(),
                                        context: format!("call to '{}'", fn_name),
                                    })?;
                                let tuple_type = BrixType::Tuple(ret_types.clone());
                                return Ok((result, tuple_type));
                            }
                        }
                    }
                }

                eprintln!("Error: Unknown function: {:?}", func);
                Err(CodegenError::MissingValue { what: "expression value".to_string(), context: "compile_expr".to_string() })
            }

            Expr::FieldAccess { target, field } => {
                // Check if this is a module constant access (e.g., math.pi)
                if let Expr::Identifier(module_name) = target.as_ref() {
                    let const_name = format!("{}.{}", module_name, field);
                    if let Some((ptr, brix_type)) = self.variables.get(&const_name) {
                        // Load the constant value
                        let loaded_val = self
                            .builder
                            .build_load(self.context.f64_type(), *ptr, &const_name)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: format!("Failed to load constant '{}'", const_name),
                            })?;
                        return Ok((loaded_val, brix_type.clone()));
                    }
                }

                let (target_val, target_type) = self.compile_expr(target)?;

                if target_type == BrixType::String {
                    if field == "len" {
                        let ptr = target_val.into_pointer_value();
                        let str_type = self.get_string_type();
                        let len_ptr = self
                            .builder
                            .build_struct_gep(str_type, ptr, 0, "len_ptr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_struct_gep".to_string(),
                                details: "Failed to get string len pointer".to_string(),
                            })?;
                        let len_val = self
                            .builder
                            .build_load(self.context.i64_type(), len_ptr, "len_val")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load string length".to_string(),
                            })?;
                        return Ok((len_val, BrixType::Int));
                    }
                }

                if target_type == BrixType::Matrix || target_type == BrixType::IntMatrix {
                    let target_ptr = target_val.into_pointer_value();
                    let matrix_type = if target_type == BrixType::Matrix {
                        self.get_matrix_type()
                    } else {
                        self.get_intmatrix_type()
                    };

                    let index = match field.as_str() {
                        "rows" => 0,
                        "cols" => 1,
                        "data" => 2,
                        _ => return Err(CodegenError::General(format!("unknown field '{}'", field))),
                    };

                    let field_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, target_ptr, index, "field_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_struct_gep".to_string(),
                            details: format!("Failed to get matrix field '{}' pointer", field),
                        })?;

                    let val = match index {
                        0 | 1 => {
                            let v = self
                                .builder
                                .build_load(self.context.i64_type(), field_ptr, "load_field")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!("Failed to load matrix field '{}'", field),
                                })?;
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
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!("Failed to load matrix field '{}' pointer", field),
                                })?;
                            (v, BrixType::FloatPtr)
                        }
                    };
                    return Ok(val);
                }
                eprintln!("Type error: Access field on non-matrix.");
                Err(CodegenError::MissingValue { what: "expression value".to_string(), context: "compile_expr".to_string() })
            }

            Expr::Index { array, indices } => {
                let (target_val, target_type) = self.compile_expr(array)?;

                // Check if indexing a tuple
                if let BrixType::Tuple(types) = &target_type {
                    // Tuple indexing: result[0], result[1], etc.
                    if indices.len() != 1 {
                        eprintln!("Error: Tuple indexing requires exactly one index");
                        return Err(CodegenError::General("compilation error".to_string()));
                    }

                    // Extract index (must be a constant integer)
                    if let Expr::Literal(Literal::Int(idx)) = &indices[0] {
                        let idx_u32 = *idx as u32;
                        if idx_u32 >= types.len() as u32 {
                            eprintln!(
                                "Error: Tuple index {} out of bounds (max: {})",
                                idx,
                                types.len() - 1
                            );
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        // Extract value from struct
                        let extracted = self
                            .builder
                            .build_extract_value(target_val.into_struct_value(), idx_u32, "extract")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_extract_value".to_string(),
                                details: format!("Failed to extract tuple element at index {}", idx_u32),
                            })?;

                        return Ok((extracted, types[idx_u32 as usize].clone()));
                    } else {
                        eprintln!("Error: Tuple index must be a constant integer");
                        return Err(CodegenError::General("compilation error".to_string()));
                    }
                }

                // Support both Matrix (f64*) and IntMatrix (i64*)
                if target_type != BrixType::Matrix && target_type != BrixType::IntMatrix {
                    eprintln!("Error: Trying to index something that is not a matrix or tuple.");
                    return Err(CodegenError::General("compilation error".to_string()));
                }

                let is_int_matrix = target_type == BrixType::IntMatrix;
                let matrix_ptr = target_val.into_pointer_value();
                let matrix_type = if is_int_matrix {
                    self.get_intmatrix_type()
                } else {
                    self.get_matrix_type()
                };
                let i64_type = self.context.i64_type();

                // Get cols (same for both Matrix and IntMatrix)
                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "cols")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed to get matrix cols pointer".to_string(),
                    })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load matrix cols".to_string(),
                    })?
                    .into_int_value();

                // Get data pointer (field 2 for both)
                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "data")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed to get matrix data pointer".to_string(),
                    })?;
                let data = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load matrix data pointer".to_string(),
                    })?
                    .into_pointer_value();

                // Calculate offset (same logic for both)
                let final_offset = if indices.len() == 1 {
                    let (idx0_val, _) = self.compile_expr(&indices[0])?;
                    idx0_val.into_int_value()
                } else if indices.len() == 2 {
                    let (row_val, _) = self.compile_expr(&indices[0])?;
                    let (col_val, _) = self.compile_expr(&indices[1])?;

                    let row_offset = self
                        .builder
                        .build_int_mul(row_val.into_int_value(), cols, "row_off")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_int_mul".to_string(),
                            details: "Failed to compute row offset for matrix indexing".to_string(),
                        })?;
                    self.builder
                        .build_int_add(row_offset, col_val.into_int_value(), "final_off")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_int_add".to_string(),
                            details: "Failed to compute final offset for matrix indexing".to_string(),
                        })?
                } else {
                    eprintln!("Erro: Suporte apenas para 1 ou 2 índices.");
                    return Err(CodegenError::General("compilation error".to_string()));
                };

                // Load value with appropriate type
                unsafe {
                    if is_int_matrix {
                        // IntMatrix: load i64
                        let item_ptr = self
                            .builder
                            .build_gep(i64_type, data, &[final_offset], "item_ptr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_gep".to_string(),
                                details: "Failed to compute IntMatrix element pointer".to_string(),
                            })?;
                        let val = self.builder.build_load(i64_type, item_ptr, "val")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load IntMatrix element".to_string(),
                            })?;
                        Ok((val, BrixType::Int))
                    } else {
                        // Matrix: load f64
                        let f64 = self.context.f64_type();
                        let item_ptr = self
                            .builder
                            .build_gep(f64, data, &[final_offset], "item_ptr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_gep".to_string(),
                                details: "Failed to compute Matrix element pointer".to_string(),
                            })?;
                        let val = self.builder.build_load(f64, item_ptr, "val")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load Matrix element".to_string(),
                            })?;
                        Ok((val, BrixType::Float))
                    }
                }
            }

            Expr::Array(elements) => {
                let n = elements.len() as u64;
                let i64_type = self.context.i64_type();

                // Step 1: Infer type by checking all elements
                let mut all_int = true;
                let mut compiled_elements = Vec::new();

                for expr in elements {
                    let (val, val_type) = self.compile_expr(expr)?;
                    compiled_elements.push((val, val_type.clone()));

                    if val_type != BrixType::Int {
                        all_int = false;
                    }
                }

                let rows_val = i64_type.const_int(1, false);
                let cols_val = i64_type.const_int(n, false);

                // Step 2: Create IntMatrix or Matrix based on inference
                if all_int {
                    // Create IntMatrix (i64*)
                    let intmatrix_type = self.get_intmatrix_type();
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);

                    let intmatrix_new_fn = self
                        .module
                        .get_function("intmatrix_new")
                        .unwrap_or_else(|| {
                            self.module.add_function(
                                "intmatrix_new",
                                fn_type,
                                Some(Linkage::External),
                            )
                        });

                    let call = self
                        .builder
                        .build_call(
                            intmatrix_new_fn,
                            &[rows_val.into(), cols_val.into()],
                            "alloc_intarr",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_call".to_string(),
                            details: "Failed to call intmatrix_new".to_string(),
                        })?;
                    let new_intmatrix_ptr = call
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| CodegenError::MissingValue {
                            what: "intmatrix_new return value".to_string(),
                            context: "array literal".to_string(),
                        })?
                        .into_pointer_value();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(intmatrix_type, new_intmatrix_ptr, 2, "data_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_struct_gep".to_string(),
                            details: "Failed to get IntMatrix data pointer".to_string(),
                        })?;
                    let data_ptr = self
                        .builder
                        .build_load(ptr_type, data_ptr_ptr, "data_base")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed to load IntMatrix data pointer".to_string(),
                        })?
                        .into_pointer_value();

                    // Store integer values
                    for (i, (val, _)) in compiled_elements.iter().enumerate() {
                        let index = i64_type.const_int(i as u64, false);
                        unsafe {
                            let elem_ptr = self
                                .builder
                                .build_gep(i64_type, data_ptr, &[index], "elem_ptr")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_gep".to_string(),
                                    details: format!("Failed to compute IntMatrix element {} pointer", i),
                                })?;
                            self.builder
                                .build_store(elem_ptr, val.into_int_value())
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_store".to_string(),
                                    details: format!("Failed to store IntMatrix element {}", i),
                                })?;
                        }
                    }

                    Ok((new_intmatrix_ptr.as_basic_value_enum(), BrixType::IntMatrix))
                } else {
                    // Create Matrix (f64*) with int→float promotion
                    let matrix_type = self.get_matrix_type();
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);

                    let matrix_new_fn =
                        self.module.get_function("matrix_new").unwrap_or_else(|| {
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
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_call".to_string(),
                            details: "Failed to call matrix_new".to_string(),
                        })?;
                    let new_matrix_ptr = call
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| CodegenError::MissingValue {
                            what: "matrix_new return value".to_string(),
                            context: "array literal".to_string(),
                        })?
                        .into_pointer_value();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, new_matrix_ptr, 2, "data_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_struct_gep".to_string(),
                            details: "Failed to get Matrix data pointer".to_string(),
                        })?;
                    let data_ptr = self
                        .builder
                        .build_load(ptr_type, data_ptr_ptr, "data_base")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed to load Matrix data pointer".to_string(),
                        })?
                        .into_pointer_value();

                    // Store with int→float conversion
                    for (i, (val, val_type)) in compiled_elements.iter().enumerate() {
                        let float_val = if *val_type == BrixType::Int {
                            self.builder
                                .build_signed_int_to_float(
                                    val.into_int_value(),
                                    self.context.f64_type(),
                                    "cast",
                                )
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_signed_int_to_float".to_string(),
                                    details: format!("Failed to cast int to float for array element {}", i),
                                })?
                        } else {
                            val.into_float_value()
                        };

                        let index = i64_type.const_int(i as u64, false);
                        unsafe {
                            let elem_ptr = self
                                .builder
                                .build_gep(self.context.f64_type(), data_ptr, &[index], "elem_ptr")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_gep".to_string(),
                                    details: format!("Failed to compute Matrix element {} pointer", i),
                                })?;
                            self.builder.build_store(elem_ptr, float_val)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_store".to_string(),
                                    details: format!("Failed to store Matrix element {}", i),
                                })?;
                        }
                    }

                    Ok((new_matrix_ptr.as_basic_value_enum(), BrixType::Matrix))
                }
            }

            Expr::Range { .. } => self.compile_range_expr(),

            Expr::ListComprehension { expr, generators } => {
                self.compile_list_comprehension(expr, generators)
            }

            Expr::StaticInit {
                element_type,
                dimensions,
            } => self.compile_static_init_expr(element_type, dimensions),

            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => self.compile_ternary_expr(condition, then_expr, else_expr),

            Expr::Match { value, arms } => {
                use parser::ast::Pattern;

                // Compile the match value once
                let (match_val, match_type) = self.compile_expr(value)?;

                // Check for exhaustiveness (warning only)
                let has_wildcard = arms
                    .iter()
                    .any(|arm| matches!(arm.pattern, Pattern::Wildcard));
                if !has_wildcard {
                    eprintln!("⚠️  Warning: Non-exhaustive match expression");
                    eprintln!("    Consider adding: _ -> ...");
                }

                // Get parent function
                let parent_fn = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_insert_block".to_string(),
                        details: "No current block in match expression".to_string(),
                    })?
                    .get_parent()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_parent".to_string(),
                        details: "Current block has no parent function in match expression".to_string(),
                    })?;

                // Create ALL basic blocks first
                let merge_bb = self.context.append_basic_block(parent_fn, "match_merge");
                let mut arm_test_bbs = Vec::new();
                let mut arm_body_bbs = Vec::new();

                for i in 0..arms.len() {
                    arm_test_bbs.push(
                        self.context
                            .append_basic_block(parent_fn, &format!("match_arm_{}_test", i)),
                    );
                    arm_body_bbs.push(
                        self.context
                            .append_basic_block(parent_fn, &format!("match_arm_{}_body", i)),
                    );
                }

                // Jump to first arm's test
                self.builder
                    .build_unconditional_branch(arm_test_bbs[0])
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed to branch to first match arm".to_string(),
                    })?;

                // Store results from each arm for PHI node
                let mut phi_incoming: Vec<(BasicValueEnum, inkwell::basic_block::BasicBlock)> =
                    Vec::new();
                let mut result_type: Option<BrixType> = None;

                // Process each arm
                for (i, arm) in arms.iter().enumerate() {
                    // Position at test block
                    self.builder.position_at_end(arm_test_bbs[i]);

                    // If pattern is binding, create the variable BEFORE evaluating guard
                    let _binding_name = if let Pattern::Binding(name) = &arm.pattern {
                        let llvm_type = self.brix_type_to_llvm(&match_type);
                        let ptr = self.builder.build_alloca(llvm_type, name)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_alloca".to_string(),
                                details: format!("Failed to allocate variable '{}'", name),
                            })?;
                        self.builder.build_store(ptr, match_val)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "Failed to store match value".to_string(),
                            })?;
                        self.variables
                            .insert(name.clone(), (ptr, match_type.clone()));
                        Some(name.clone())
                    } else {
                        None
                    };

                    // Check if pattern matches
                    let pattern_matches =
                        self.compile_pattern_match(&arm.pattern, match_val, &match_type)?;

                    // If guard exists, evaluate it (binding is already available)
                    let final_condition = if let Some(guard_expr) = &arm.guard {
                        let (guard_val, _) = self.compile_expr(guard_expr)?;
                        let guard_int = guard_val.into_int_value();
                        let zero = self.context.i64_type().const_int(0, false);
                        let guard_bool = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::NE,
                                guard_int,
                                zero,
                                "guard_bool",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to compare guard value".to_string(),
                            })?;

                        // pattern_matches AND guard
                        self.builder
                            .build_and(pattern_matches, guard_bool, "pattern_and_guard")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_and".to_string(),
                                details: "Failed to AND pattern match with guard".to_string(),
                            })?
                    } else {
                        pattern_matches
                    };

                    // Determine next block if this arm doesn't match
                    let next_bb = if i < arms.len() - 1 {
                        arm_test_bbs[i + 1]
                    } else {
                        merge_bb // Last arm: if doesn't match, go to merge (undefined behavior, but warning was issued)
                    };

                    // Branch: if pattern matches (and guard passes), execute body; otherwise try next arm
                    self.builder
                        .build_conditional_branch(final_condition, arm_body_bbs[i], next_bb)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_conditional_branch".to_string(),
                            details: "Failed to branch on match arm condition".to_string(),
                        })?;

                    // Compile arm body
                    self.builder.position_at_end(arm_body_bbs[i]);

                    // Binding was already created above if needed

                    let (body_val, body_type) = self.compile_expr(&arm.body)?;

                    // Type checking: ensure all arms return compatible types
                    if let Some(ref expected_type) = result_type {
                        // Check compatibility
                        if !self.are_types_compatible(expected_type, &body_type) {
                            eprintln!("Error: Match arms return incompatible types");
                            eprintln!("  Expected: {:?}, Got: {:?}", expected_type, body_type);
                            return Err(CodegenError::General("compilation error".to_string()));
                        }

                        // Update result type to promoted type if needed
                        if *expected_type == BrixType::Int && body_type == BrixType::Float {
                            result_type = Some(BrixType::Float);
                        }
                    } else {
                        result_type = Some(body_type.clone());
                    }

                    // Type coercion for PHI node
                    let coerced_val = if result_type.as_ref().ok_or_else(|| CodegenError::MissingValue {
                        what: "result type".to_string(),
                        context: "match expression type coercion".to_string(),
                    })? == &BrixType::Float
                        && body_type == BrixType::Int
                    {
                        self.builder
                            .build_signed_int_to_float(
                                body_val.into_int_value(),
                                self.context.f64_type(),
                                &format!("arm_{}_cast", i),
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_signed_int_to_float".to_string(),
                                details: "Failed to cast match arm value from int to float".to_string(),
                            })?
                            .into()
                    } else {
                        body_val
                    };

                    let current_bb = self.builder.get_insert_block().ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_insert_block".to_string(),
                        details: "No current block after match arm body".to_string(),
                    })?;
                    phi_incoming.push((coerced_val, current_bb));

                    // Jump to merge block
                    self.builder.build_unconditional_branch(merge_bb).map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed to branch to merge block from match arm".to_string(),
                    })?;
                }

                // Position at merge block and create PHI node
                self.builder.position_at_end(merge_bb);

                let final_type = result_type.ok_or_else(|| CodegenError::MissingValue {
                    what: "result type".to_string(),
                    context: "match expression has no arms or no result type".to_string(),
                })?;
                let phi_type = match final_type {
                    BrixType::Int => self.context.i64_type().as_basic_type_enum(),
                    BrixType::Float => self.context.f64_type().as_basic_type_enum(),
                    BrixType::String
                    | BrixType::Matrix
                    | BrixType::IntMatrix
                    | BrixType::FloatPtr => self
                        .context
                        .ptr_type(AddressSpace::default())
                        .as_basic_type_enum(),
                    _ => self.context.i64_type().as_basic_type_enum(),
                };

                let phi = self.builder.build_phi(phi_type, "match_result").map_err(|_| CodegenError::LLVMError {
                    operation: "build_phi".to_string(),
                    details: "Failed to build PHI node for match expression result".to_string(),
                })?;

                for (val, bb) in phi_incoming {
                    phi.add_incoming(&[(&val, bb)]);
                }

                Ok((phi.as_basic_value(), final_type))
            }

            Expr::Increment { expr, is_prefix } => {
                // Get the address of the l-value
                let (var_ptr, _) = self.compile_lvalue_addr(expr)?;

                // Load current value
                let current_val = self
                    .builder
                    .build_load(self.context.i64_type(), var_ptr, "load_for_inc")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load value for increment".to_string(),
                    })?
                    .into_int_value();

                // Increment
                let one = self.context.i64_type().const_int(1, false);
                let new_val = self
                    .builder
                    .build_int_add(current_val, one, "incremented")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_add".to_string(),
                        details: "Failed to build increment operation".to_string(),
                    })?;

                // Store new value
                self.builder.build_store(var_ptr, new_val).map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "Failed to store incremented value".to_string(),
                })?;

                // Return value depends on prefix/postfix
                if *is_prefix {
                    // Prefix: return new value (++x)
                    Ok((new_val.into(), BrixType::Int))
                } else {
                    // Postfix: return old value (x++)
                    Ok((current_val.into(), BrixType::Int))
                }
            }

            Expr::Decrement { expr, is_prefix } => {
                // Get the address of the l-value
                let (var_ptr, _) = self.compile_lvalue_addr(expr)?;

                // Load current value
                let current_val = self
                    .builder
                    .build_load(self.context.i64_type(), var_ptr, "load_for_dec")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load value for decrement".to_string(),
                    })?
                    .into_int_value();

                // Decrement
                let one = self.context.i64_type().const_int(1, false);
                let new_val = self
                    .builder
                    .build_int_sub(current_val, one, "decremented")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_sub".to_string(),
                        details: "Failed to build decrement operation".to_string(),
                    })?;

                // Store new value
                self.builder.build_store(var_ptr, new_val).map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "Failed to store decremented value".to_string(),
                })?;

                // Return value depends on prefix/postfix
                if *is_prefix {
                    // Prefix: return new value (--x)
                    Ok((new_val.into(), BrixType::Int))
                } else {
                    // Postfix: return old value (x--)
                    Ok((current_val.into(), BrixType::Int))
                }
            }

            Expr::FString { parts } => {
                use parser::ast::FStringPart;

                if parts.is_empty() {
                    // Empty f-string -> empty string
                    let raw_str = self
                        .builder
                        .build_global_string_ptr("", "empty_fstr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_global_string_ptr".to_string(),
                            details: "Failed to create empty f-string literal".to_string(),
                        })?;
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                    let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                        self.module
                            .add_function("str_new", fn_type, Some(Linkage::External))
                    });
                    let call = self
                        .builder
                        .build_call(
                            str_new_fn,
                            &[raw_str.as_pointer_value().into()],
                            "empty_str",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_call".to_string(),
                            details: "Failed to call str_new for empty f-string".to_string(),
                        })?;
                    let val = call.try_as_basic_value().left().ok_or_else(|| CodegenError::MissingValue {
                        what: "return value from str_new".to_string(),
                        context: "empty f-string".to_string(),
                    })?;
                    return Ok((val, BrixType::String));
                }

                // Compile each part and convert to string
                let mut string_parts = Vec::new();

                for part in parts {
                    let str_val = match part {
                        FStringPart::Text(text) => {
                            // Create string from text literal
                            let raw_str = self
                                .builder
                                .build_global_string_ptr(text, "fstr_text")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_global_string_ptr".to_string(),
                                    details: "Failed to create f-string text literal".to_string(),
                                })?;
                            let ptr_type = self.context.ptr_type(AddressSpace::default());
                            let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                            let str_new_fn =
                                self.module.get_function("str_new").unwrap_or_else(|| {
                                    self.module.add_function(
                                        "str_new",
                                        fn_type,
                                        Some(Linkage::External),
                                    )
                                });
                            let call = self
                                .builder
                                .build_call(
                                    str_new_fn,
                                    &[raw_str.as_pointer_value().into()],
                                    "text_str",
                                )
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_call".to_string(),
                                    details: "Failed to call str_new for f-string text part".to_string(),
                                })?;
                            call.try_as_basic_value().left().ok_or_else(|| CodegenError::MissingValue {
                                what: "return value from str_new".to_string(),
                                context: "f-string text part".to_string(),
                            })?
                        }
                        FStringPart::Expr { expr, format } => {
                            // Compile expression and convert to string with optional format
                            let (val, typ) = self.compile_expr(expr)?;
                            self.value_to_string(val, &typ, format.as_deref())?
                        }
                    };
                    string_parts.push(str_val);
                }

                // Concatenate all parts
                let mut result = string_parts[0];
                for part in &string_parts[1..] {
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
                    let str_concat_fn =
                        self.module.get_function("str_concat").unwrap_or_else(|| {
                            self.module
                                .add_function("str_concat", fn_type, Some(Linkage::External))
                        });
                    let call = self
                        .builder
                        .build_call(
                            str_concat_fn,
                            &[result.into(), (*part).into()],
                            "concat_fstr",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_call".to_string(),
                            details: "Failed to call str_concat for f-string concatenation".to_string(),
                        })?;
                    result = call.try_as_basic_value().left().ok_or_else(|| CodegenError::MissingValue {
                        what: "return value from str_concat".to_string(),
                        context: "f-string concatenation".to_string(),
                    })?;
                }

                Ok((result, BrixType::String))
            }

            #[allow(unreachable_patterns)]
            _ => {
                eprintln!("Expression not implemented");
                Err(CodegenError::MissingValue { what: "expression value".to_string(), context: "compile_expr".to_string() })
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

    fn value_to_string(
        &self,
        val: BasicValueEnum<'ctx>,
        typ: &BrixType,
        format: Option<&str>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        match typ {
            BrixType::String => Ok(val), // Already a string

            BrixType::Int => {
                // Use sprintf to convert int to string
                let sprintf_fn = self.get_sprintf();

                // Allocate buffer for string (enough for i64: 32 chars + null)
                let i8_type = self.context.i8_type();
                let buffer_size = i8_type.const_int(64, false);
                let buffer = self
                    .builder
                    .build_array_alloca(i8_type, buffer_size, "int_str_buf")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Map format specifier to sprintf format
                let fmt_string = if let Some(fmt) = format {
                    match fmt {
                        "x" => "%x".to_string(),   // hex lowercase
                        "X" => "%X".to_string(),   // hex uppercase
                        "o" => "%o".to_string(),   // octal
                        "d" => "%lld".to_string(), // decimal (default)
                        _ => "%lld".to_string(),   // default for unknown
                    }
                } else {
                    "%lld".to_string() // default: decimal
                };

                let fmt_str = self
                    .builder
                    .build_global_string_ptr(&fmt_string, "fmt_int")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Call sprintf
                self.builder
                    .build_call(
                        sprintf_fn,
                        &[buffer.into(), fmt_str.as_pointer_value().into(), val.into()],
                        "sprintf_int",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Create BrixString from buffer
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                });

                let call = self
                    .builder
                    .build_call(str_new_fn, &[buffer.into()], "int_to_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "Failed to call str_new in value_to_string".to_string() })?;
                let value = call.try_as_basic_value().left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "str_new did not return a value".to_string(),
                    })?;
                Ok(value)
            }

            BrixType::Float => {
                // Use sprintf to convert float to string
                let sprintf_fn = self.get_sprintf();

                // Allocate buffer for string (enough for f64: 32 chars + null)
                let i8_type = self.context.i8_type();
                let buffer_size = i8_type.const_int(64, false);
                let buffer = self
                    .builder
                    .build_array_alloca(i8_type, buffer_size, "float_str_buf")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Map format specifier to sprintf format
                let fmt_string = if let Some(fmt) = format {
                    // Check for .Nf format (e.g., .2f, .6f)
                    if fmt.starts_with('.') && fmt.ends_with('f') {
                        format!("%{}", fmt) // .2f → %.2f
                    } else if fmt.starts_with('.') && fmt.ends_with('e') {
                        format!("%{}", fmt) // .2e → %.2e
                    } else if fmt.starts_with('.') && fmt.ends_with('E') {
                        format!("%{}", fmt) // .2E → %.2E
                    } else {
                        match fmt {
                            "e" => "%e".to_string(), // scientific notation lowercase
                            "E" => "%E".to_string(), // scientific notation uppercase
                            "f" => "%f".to_string(), // fixed-point
                            "g" => "%g".to_string(), // compact (default)
                            "G" => "%G".to_string(), // compact uppercase
                            _ => "%g".to_string(),   // default for unknown
                        }
                    }
                } else {
                    "%g".to_string() // default: compact
                };

                let fmt_str = self
                    .builder
                    .build_global_string_ptr(&fmt_string, "fmt_float")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Call sprintf
                self.builder
                    .build_call(
                        sprintf_fn,
                        &[buffer.into(), fmt_str.as_pointer_value().into(), val.into()],
                        "sprintf_float",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Create BrixString from buffer
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                });

                let call = self
                    .builder
                    .build_call(str_new_fn, &[buffer.into()], "float_to_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let value = call.try_as_basic_value().left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "str_new did not return a value".to_string(),
                    })?;
                Ok(value)
            }

            BrixType::Matrix | BrixType::IntMatrix => {
                // Convert array to string format: [1, 2, 3] or [1.5, 2.3, 3.7]
                let is_int = matches!(typ, BrixType::IntMatrix);
                let matrix_ptr = val.into_pointer_value();
                let matrix_type = if is_int {
                    self.get_intmatrix_type()
                } else {
                    self.get_matrix_type()
                };
                let i64_type = self.context.i64_type();

                // Load dimensions
                let (rows, cols) = {
                    let rows_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, matrix_ptr, 0, "rows_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                    let rows = self
                        .builder
                        .build_load(i64_type, rows_ptr, "rows")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                        .into_int_value();

                    let cols_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, matrix_ptr, 1, "cols_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                    let cols = self
                        .builder
                        .build_load(i64_type, cols_ptr, "cols")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                        .into_int_value();

                    (rows, cols)
                };

                // Load data pointer
                let data_ptr = {
                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, matrix_ptr, 2, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                    self.builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "data_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                        .into_pointer_value()
                };

                // Create initial string "["
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                });

                let open_bracket = self
                    .builder
                    .build_global_string_ptr("[", "open_bracket")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let result_alloca = self.create_entry_block_alloca(ptr_type.into(), "array_str")?;
                let initial_str = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[open_bracket.as_pointer_value().into()],
                        "init_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                self.builder
                    .build_store(result_alloca, initial_str)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Calculate total length
                let total_len = self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;

                // Loop through elements
                let block = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_insert_block".to_string(),
                        details: "No current basic block".to_string(),
                    })?;
                let parent_fn = block.get_parent()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_parent".to_string(),
                        details: "Block has no parent function".to_string(),
                    })?;
                let loop_cond = self.context.append_basic_block(parent_fn, "array_str_cond");
                let loop_body = self.context.append_basic_block(parent_fn, "array_str_body");
                let loop_after = self
                    .context
                    .append_basic_block(parent_fn, "array_str_after");

                let idx_alloca = self.create_entry_block_alloca(i64_type.into(), "array_idx")?;
                self.builder
                    .build_store(idx_alloca, i64_type.const_int(0, false))
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                self.builder.build_unconditional_branch(loop_cond).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;

                // Condition: idx < total_len
                self.builder.position_at_end(loop_cond);
                let idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx, total_len, "cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                self.builder
                    .build_conditional_branch(cond, loop_body, loop_after)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Body: append element
                self.builder.position_at_end(loop_body);

                // Load current result string
                let current_str = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Load element
                let elem_val = if is_int {
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(i64_type, data_ptr, &[idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                        self.builder.build_load(i64_type, elem_ptr, "elem").map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?
                    }
                } else {
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(self.context.f64_type(), data_ptr, &[idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                        self.builder
                            .build_load(self.context.f64_type(), elem_ptr, "elem")
                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    }
                };

                // Convert element to string
                let elem_type = if is_int {
                    BrixType::Int
                } else {
                    BrixType::Float
                };
                let elem_str = self.value_to_string(elem_val, &elem_type, None)?;

                // Concatenate
                let str_concat_fn = self.module.get_function("str_concat").unwrap_or_else(|| {
                    let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
                    self.module
                        .add_function("str_concat", fn_type, Some(Linkage::External))
                });

                let concatenated = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_str.into(), elem_str.into()],
                        "concat",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;

                // Store concatenated result
                self.builder
                    .build_store(result_alloca, concatenated)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Add comma if not last element
                let next_idx = self
                    .builder
                    .build_int_add(idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let is_not_last = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, next_idx, total_len, "is_not_last")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                let add_comma_bb = self.context.append_basic_block(parent_fn, "add_comma");
                let continue_bb = self.context.append_basic_block(parent_fn, "continue_loop");

                self.builder
                    .build_conditional_branch(is_not_last, add_comma_bb, continue_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Add comma
                self.builder.position_at_end(add_comma_bb);
                let current_with_elem = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_with_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let comma_str = self.builder.build_global_string_ptr(", ", "comma").map_err(|_| CodegenError::LLVMError { operation: "build_global_string_ptr".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;
                let comma_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[comma_str.as_pointer_value().into()],
                        "comma_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                let with_comma = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_with_elem.into(), comma_brix.into()],
                        "with_comma",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                self.builder.build_store(result_alloca, with_comma).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;
                self.builder
                    .build_unconditional_branch(continue_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Continue: increment and loop
                self.builder.position_at_end(continue_bb);
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;
                self.builder.build_unconditional_branch(loop_cond).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;

                // After loop: append "]"
                self.builder.position_at_end(loop_after);
                let final_result = self
                    .builder
                    .build_load(ptr_type, result_alloca, "final_result")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let close_bracket = self
                    .builder
                    .build_global_string_ptr("]", "close_bracket")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let close_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[close_bracket.as_pointer_value().into()],
                        "close_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                let final_str = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[final_result.into(), close_brix.into()],
                        "final_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;

                Ok(final_str)
            }

            BrixType::Complex => {
                // Call runtime complex_to_string function
                let ptr_type = self.context.ptr_type(AddressSpace::default());

                // Declare complex_to_string if not already declared
                let complex_to_string_fn = if let Some(func) =
                    self.module.get_function("complex_to_string")
                {
                    func
                } else {
                    let f64_type = self.context.f64_type();
                    let complex_type = self
                        .context
                        .struct_type(&[f64_type.into(), f64_type.into()], false);
                    // char* complex_to_string(Complex z)
                    let fn_type = ptr_type.fn_type(&[complex_type.into()], false);
                    self.module
                        .add_function("complex_to_string", fn_type, Some(Linkage::External))
                };

                // Call complex_to_string
                let call = self
                    .builder
                    .build_call(complex_to_string_fn, &[val.into()], "complex_c_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let c_str = call.try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "Function did not return a value".to_string(),
                    })?
                    .into_pointer_value();

                // Convert C string to BrixString
                let str_new_fn = if let Some(func) = self.module.get_function("str_new") {
                    func
                } else {
                    let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                };

                let brix_string = self
                    .builder
                    .build_call(str_new_fn, &[c_str.into()], "complex_brix_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;

                Ok(brix_string)
            }

            BrixType::ComplexMatrix => {
                // Convert ComplexMatrix to string format: [3+4i, 1-2i, 5+0i]
                let matrix_ptr = val.into_pointer_value();
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let i64_type = self.context.i64_type();

                // Get ComplexMatrix struct type (rows, cols, Complex* data)
                let complexmatrix_type = self
                    .context
                    .struct_type(&[i64_type.into(), i64_type.into(), ptr_type.into()], false);

                // Load dimensions
                let (rows, cols) = {
                    let rows_ptr = self
                        .builder
                        .build_struct_gep(complexmatrix_type, matrix_ptr, 0, "rows_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                    let rows = self
                        .builder
                        .build_load(i64_type, rows_ptr, "rows")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                        .into_int_value();

                    let cols_ptr = self
                        .builder
                        .build_struct_gep(complexmatrix_type, matrix_ptr, 1, "cols_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                    let cols = self
                        .builder
                        .build_load(i64_type, cols_ptr, "cols")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                        .into_int_value();

                    (rows, cols)
                };

                // Load data pointer (Complex*)
                let data_ptr = {
                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(complexmatrix_type, matrix_ptr, 2, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                    self.builder
                        .build_load(ptr_type, data_ptr_ptr, "data_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                        .into_pointer_value()
                };

                // Create initial string "["
                let str_new_fn = if let Some(func) = self.module.get_function("str_new") {
                    func
                } else {
                    let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                };

                let open_bracket = self
                    .builder
                    .build_global_string_ptr("[", "open_bracket")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let result_alloca = self.create_entry_block_alloca(ptr_type.into(), "cmatrix_str")?;
                let initial_str = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[open_bracket.as_pointer_value().into()],
                        "init_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                self.builder
                    .build_store(result_alloca, initial_str)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Calculate total length
                let total_len = self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;

                // Get complex_to_string function
                let f64_type = self.context.f64_type();
                let complex_type = self
                    .context
                    .struct_type(&[f64_type.into(), f64_type.into()], false);
                let complex_to_string_fn = if let Some(func) =
                    self.module.get_function("complex_to_string")
                {
                    func
                } else {
                    let fn_type = ptr_type.fn_type(&[complex_type.into()], false);
                    self.module
                        .add_function("complex_to_string", fn_type, Some(Linkage::External))
                };

                // Get str_concat function
                let str_concat_fn = if let Some(func) = self.module.get_function("str_concat") {
                    func
                } else {
                    let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
                    self.module
                        .add_function("str_concat", fn_type, Some(Linkage::External))
                };

                // Loop through elements
                let block = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_insert_block".to_string(),
                        details: "No current basic block".to_string(),
                    })?;
                let parent_fn = block.get_parent()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_parent".to_string(),
                        details: "Block has no parent function".to_string(),
                    })?;
                let loop_cond = self
                    .context
                    .append_basic_block(parent_fn, "cmatrix_str_cond");
                let loop_body = self
                    .context
                    .append_basic_block(parent_fn, "cmatrix_str_body");
                let loop_after = self
                    .context
                    .append_basic_block(parent_fn, "cmatrix_str_after");

                let idx_alloca = self.create_entry_block_alloca(i64_type.into(), "cmatrix_idx")?;
                self.builder
                    .build_store(idx_alloca, i64_type.const_int(0, false))
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                self.builder.build_unconditional_branch(loop_cond).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;

                // Condition: idx < total_len
                self.builder.position_at_end(loop_cond);
                let idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx, total_len, "cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                self.builder
                    .build_conditional_branch(cond, loop_body, loop_after)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Body: append element
                self.builder.position_at_end(loop_body);

                // Check if we're at the start of a new row (idx % cols == 0)
                let col_pos = self
                    .builder
                    .build_int_unsigned_rem(idx, cols, "col_pos")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let is_row_start = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        col_pos,
                        i64_type.const_int(0, false),
                        "is_row_start",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // If start of row, add "["
                let after_row_start_bb = self
                    .context
                    .append_basic_block(parent_fn, "after_row_start");
                let add_row_start_bb = self.context.append_basic_block(parent_fn, "add_row_start");
                self.builder
                    .build_conditional_branch(is_row_start, add_row_start_bb, after_row_start_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                self.builder.position_at_end(add_row_start_bb);
                let current_with_row_bracket = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_str_2")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let row_open = self
                    .builder
                    .build_global_string_ptr("[", "row_open")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let row_open_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[row_open.as_pointer_value().into()],
                        "row_open_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                let with_row_open = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_with_row_bracket.into(), row_open_brix.into()],
                        "with_row_open",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                self.builder
                    .build_store(result_alloca, with_row_open)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                self.builder
                    .build_unconditional_branch(after_row_start_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                self.builder.position_at_end(after_row_start_bb);
                let current_str = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_str_3")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Load Complex element (struct with 2 f64s)
                let complex_elem = unsafe {
                    let elem_ptr = self
                        .builder
                        .build_gep(complex_type, data_ptr, &[idx], "elem_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                    self.builder
                        .build_load(complex_type, elem_ptr, "complex_elem")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                };

                // Convert Complex to C string
                let call = self
                    .builder
                    .build_call(
                        complex_to_string_fn,
                        &[complex_elem.into()],
                        "complex_c_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let c_str = call.try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "Function did not return a value".to_string(),
                    })?
                    .into_pointer_value();

                // Convert C string to BrixString
                let elem_str = self
                    .builder
                    .build_call(str_new_fn, &[c_str.into()], "elem_brix_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;

                // Concatenate element
                let concatenated = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_str.into(), elem_str.into()],
                        "concat",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                self.builder
                    .build_store(result_alloca, concatenated)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Determine what to add after element: ", " or "]" or "], "
                let next_idx = self
                    .builder
                    .build_int_add(idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let is_last_elem = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, next_idx, total_len, "is_last_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Check if we're at end of row (next_idx % cols == 0)
                let next_col_pos = self
                    .builder
                    .build_int_unsigned_rem(next_idx, cols, "next_col_pos")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let is_row_end = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        next_col_pos,
                        i64_type.const_int(0, false),
                        "is_row_end",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                let add_separator_bb = self.context.append_basic_block(parent_fn, "add_separator");
                let continue_bb = self.context.append_basic_block(parent_fn, "continue_loop");

                // Skip separator if it's the very last element
                self.builder
                    .build_conditional_branch(is_last_elem, continue_bb, add_separator_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Add separator ("]" or "], " or ", ")
                self.builder.position_at_end(add_separator_bb);

                let row_end_bb = self.context.append_basic_block(parent_fn, "row_end");
                let elem_comma_bb = self.context.append_basic_block(parent_fn, "elem_comma");
                self.builder
                    .build_conditional_branch(is_row_end, row_end_bb, elem_comma_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // End of row: add "]" and maybe ", "
                self.builder.position_at_end(row_end_bb);
                let current_for_row_end = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_for_row_end")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let row_close = self
                    .builder
                    .build_global_string_ptr("]", "row_close")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let row_close_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[row_close.as_pointer_value().into()],
                        "row_close_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                let with_row_close = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_for_row_end.into(), row_close_brix.into()],
                        "with_row_close",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                self.builder
                    .build_store(result_alloca, with_row_close)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Add ", " between rows if not last row
                let current_after_bracket = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_after_bracket")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let comma_str = self.builder.build_global_string_ptr(", ", "comma").map_err(|_| CodegenError::LLVMError { operation: "build_global_string_ptr".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;
                let comma_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[comma_str.as_pointer_value().into()],
                        "comma_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                let with_comma = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_after_bracket.into(), comma_brix.into()],
                        "with_comma",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                self.builder.build_store(result_alloca, with_comma).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;
                self.builder
                    .build_unconditional_branch(continue_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Not end of row: just add ", "
                self.builder.position_at_end(elem_comma_bb);
                let current_for_comma = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_for_comma")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let elem_comma = self
                    .builder
                    .build_global_string_ptr(", ", "elem_comma")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let elem_comma_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[elem_comma.as_pointer_value().into()],
                        "elem_comma_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                let with_elem_comma = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_for_comma.into(), elem_comma_brix.into()],
                        "with_elem_comma",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                self.builder
                    .build_store(result_alloca, with_elem_comma)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                self.builder
                    .build_unconditional_branch(continue_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;

                // Continue: increment and loop
                self.builder.position_at_end(continue_bb);
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;
                self.builder.build_unconditional_branch(loop_cond).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "LLVM operation failed in value_to_string".to_string() })?;

                // After loop: append "]"
                self.builder.position_at_end(loop_after);
                let final_result = self
                    .builder
                    .build_load(ptr_type, result_alloca, "final_result")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let close_bracket = self
                    .builder
                    .build_global_string_ptr("]", "close_bracket")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let close_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[close_bracket.as_pointer_value().into()],
                        "close_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;
                let final_str = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[final_result.into(), close_brix.into()],
                        "final_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;

                Ok(final_str)
            }

            BrixType::Nil => {
                // Convert nil to string "nil"
                let nil_str = self
                    .builder
                    .build_global_string_ptr("nil", "nil_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                });

                let brix_string = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[nil_str.as_pointer_value().into()],
                        "nil_brix_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;

                Ok(brix_string)
            }

            BrixType::Error => {
                // Call brix_error_message(error_ptr) to get the message string
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                let error_msg_fn = self
                    .module
                    .get_function("brix_error_message")
                    .unwrap_or_else(|| {
                        self.module.add_function(
                            "brix_error_message",
                            fn_type,
                            Some(Linkage::External),
                        )
                    });

                let error_ptr = val.into_pointer_value();
                let call = self
                    .builder
                    .build_call(error_msg_fn, &[error_ptr.into()], "error_msg")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let msg_char_ptr = call.try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "Function did not return a value".to_string(),
                    })?
                    .into_pointer_value();

                // Convert char* to BrixString using str_new
                let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                    let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                });

                let brix_string = self
                    .builder
                    .build_call(str_new_fn, &[msg_char_ptr.into()], "error_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;

                Ok(brix_string)
            }

            BrixType::Atom => {
                // Call atom_name(atom_id) to get the name string
                let i64_type = self.context.i64_type();
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
                let atom_name_fn = self.module.get_function("atom_name").unwrap_or_else(|| {
                    self.module
                        .add_function("atom_name", fn_type, Some(Linkage::External))
                });

                let atom_id = val.into_int_value();
                let call = self
                    .builder
                    .build_call(atom_name_fn, &[atom_id.into()], "atom_name")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?;
                let name_char_ptr = call.try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "Function did not return a value".to_string(),
                    })?
                    .into_pointer_value();

                // Convert char* to BrixString using str_new
                let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                    let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                });

                let brix_string = self
                    .builder
                    .build_call(str_new_fn, &[name_char_ptr.into()], "atom_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string() })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                        })?;

                Ok(brix_string)
            }

            _ => {
                eprintln!("value_to_string not implemented for type: {:?}", typ);
                Err(CodegenError::MissingValue { what: "value_to_string conversion".to_string(), context: format!("{:?}", typ) })
            }
        }
    }

    // get_sprintf, get_atoi, get_atof moved to helpers.rs
    // String functions moved to builtins/string.rs (available via StringFunctions trait)

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

    fn compile_read_csv(&mut self, args: &[Expr]) -> Option<BasicValueEnum<'ctx>> {
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

        let (filename_arg, _) = self.compile_expr(&args[0]).ok()?;
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

    fn get_intmatrix_type(&self) -> inkwell::types::StructType<'ctx> {
        // Same structure as Matrix: { rows: i64, cols: i64, data: i64* }
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

    fn compile_matrix_constructor(&mut self, args: &[Expr]) -> Option<BasicValueEnum<'ctx>> {
        if args.len() != 2 {
            return None;
        }
        let (rows_val, _) = self.compile_expr(&args[0]).ok()?;
        let (cols_val, _) = self.compile_expr(&args[1]).ok()?;

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

    pub(crate) fn compile_zeros(&mut self, args: &[Expr]) -> CodegenResult<BasicValueEnum<'ctx>> {
        // zeros(n) → 1D array of n floats
        // zeros(r, c) → 2D matrix of r×c floats
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);

        let matrix_new_fn = self.module.get_function("matrix_new").unwrap_or_else(|| {
            self.module
                .add_function("matrix_new", fn_type, Some(Linkage::External))
        });

        let (rows_val, cols_val) = if args.len() == 1 {
            // 1D: zeros(n) → matrix(1, n)
            let (n_val, _) = self.compile_expr(&args[0])?;
            (i64_type.const_int(1, false), n_val.into_int_value())
        } else if args.len() == 2 {
            // 2D: zeros(r, c) → matrix(r, c)
            let (r_val, _) = self.compile_expr(&args[0])?;
            let (c_val, _) = self.compile_expr(&args[1])?;
            (r_val.into_int_value(), c_val.into_int_value())
        } else {
            return Err(CodegenError::InvalidOperation {
                operation: "zeros()".to_string(),
                reason: format!("Expected 1 or 2 arguments, got {}", args.len()),
            });
        };

        let call = self
            .builder
            .build_call(
                matrix_new_fn,
                &[rows_val.into(), cols_val.into()],
                "zeros_matrix",
            )
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call matrix_new for zeros()".to_string(),
            })?;

        call.try_as_basic_value().left()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "try_as_basic_value".to_string(),
                details: "matrix_new did not return a value".to_string(),
            })
    }

    pub(crate) fn compile_izeros(&mut self, args: &[Expr]) -> CodegenResult<BasicValueEnum<'ctx>> {
        // izeros(n) → 1D array of n integers
        // izeros(r, c) → 2D matrix of r×c integers
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);

        let intmatrix_new_fn = self
            .module
            .get_function("intmatrix_new")
            .unwrap_or_else(|| {
                self.module
                    .add_function("intmatrix_new", fn_type, Some(Linkage::External))
            });

        let (rows_val, cols_val) = if args.len() == 1 {
            // 1D: izeros(n) → intmatrix(1, n)
            let (n_val, _) = self.compile_expr(&args[0])?;
            (i64_type.const_int(1, false), n_val.into_int_value())
        } else if args.len() == 2 {
            // 2D: izeros(r, c) → intmatrix(r, c)
            let (r_val, _) = self.compile_expr(&args[0])?;
            let (c_val, _) = self.compile_expr(&args[1])?;
            (r_val.into_int_value(), c_val.into_int_value())
        } else {
            return Err(CodegenError::InvalidOperation {
                operation: "izeros()".to_string(),
                reason: format!("Expected 1 or 2 arguments, got {}", args.len()),
            });
        };

        let call = self
            .builder
            .build_call(
                intmatrix_new_fn,
                &[rows_val.into(), cols_val.into()],
                "izeros_intmatrix",
            )
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call intmatrix_new for izeros()".to_string(),
            })?;

        call.try_as_basic_value().left()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "try_as_basic_value".to_string(),
                details: "intmatrix_new did not return a value".to_string(),
            })
    }

    fn compile_zip(&mut self, args: &[Expr]) -> Option<(BasicValueEnum<'ctx>, BrixType)> {
        // SIMPLIFIED VERSION: zip() for exactly 2 arrays
        // zip([1,2,3], [4,5,6]) → Matrix 3x2 where each row is a pair
        // This works with our existing Matrix system!

        if args.len() != 2 {
            eprintln!("Error: zip() currently supports exactly 2 arrays");
            return None;
        }

        let (arr1_val, arr1_type) = self.compile_expr(&args[0]).ok()?;
        let (arr2_val, arr2_type) = self.compile_expr(&args[1]).ok()?;

        // Both must be matrices
        let elem_type1 = match &arr1_type {
            BrixType::IntMatrix => BrixType::Int,
            BrixType::Matrix => BrixType::Float,
            _ => {
                eprintln!("Error: zip() argument 1 must be a matrix/array");
                return None;
            }
        };

        let elem_type2 = match &arr2_type {
            BrixType::IntMatrix => BrixType::Int,
            BrixType::Matrix => BrixType::Float,
            _ => {
                eprintln!("Error: zip() argument 2 must be a matrix/array");
                return None;
            }
        };

        // Determine output type: if both Int → IntMatrix, otherwise Matrix (float)
        let (_result_is_int, result_type) =
            if elem_type1 == BrixType::Int && elem_type2 == BrixType::Int {
                (true, BrixType::IntMatrix)
            } else {
                (false, BrixType::Matrix)
            };

        // Call runtime function: zip_ii, zip_if, zip_fi, zip_ff
        let fn_name = match (&arr1_type, &arr2_type) {
            (BrixType::IntMatrix, BrixType::IntMatrix) => "brix_zip_ii",
            (BrixType::IntMatrix, BrixType::Matrix) => "brix_zip_if",
            (BrixType::Matrix, BrixType::IntMatrix) => "brix_zip_fi",
            (BrixType::Matrix, BrixType::Matrix) => "brix_zip_ff",
            _ => unreachable!(),
        };

        // Declare function if not exists
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);

        let zip_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
            self.module
                .add_function(fn_name, fn_type, Some(Linkage::External))
        });

        // Call zip function
        let result = self
            .builder
            .build_call(zip_fn, &[arr1_val.into(), arr2_val.into()], "zip_result")
            .unwrap()
            .try_as_basic_value()
            .left()
            .unwrap();

        Some((result, result_type))
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
            BinaryOp::Pow => {
                // Convert integers to float, call pow, convert back to int
                let f64_type = self.context.f64_type();
                let lhs_float = self
                    .builder
                    .build_signed_int_to_float(lhs, f64_type, "lhs_f")
                    .ok()?;
                let rhs_float = self
                    .builder
                    .build_signed_int_to_float(rhs, f64_type, "rhs_f")
                    .ok()?;

                // Get or declare llvm.pow.f64 intrinsic
                let pow_fn = self.module.get_function("llvm.pow.f64").unwrap_or_else(|| {
                    let fn_type = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
                    self.module.add_function("llvm.pow.f64", fn_type, None)
                });

                let result = self
                    .builder
                    .build_call(pow_fn, &[lhs_float.into(), rhs_float.into()], "pow_result")
                    .ok()?
                    .try_as_basic_value()
                    .left()?
                    .into_float_value();

                // Convert back to int
                let i64_type = self.context.i64_type();
                let int_result = self
                    .builder
                    .build_float_to_signed_int(result, i64_type, "pow_int")
                    .ok()?;

                Some(int_result.as_basic_value_enum())
            }
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
                    .build_float_rem(lhs, rhs, "tmp_fmod")
                    .ok()?
                    .as_basic_value_enum(),
            ),
            BinaryOp::Gt => self.compile_float_cmp(FloatPredicate::OGT, lhs, rhs),
            BinaryOp::Lt => self.compile_float_cmp(FloatPredicate::OLT, lhs, rhs),
            BinaryOp::GtEq => self.compile_float_cmp(FloatPredicate::OGE, lhs, rhs),
            BinaryOp::LtEq => self.compile_float_cmp(FloatPredicate::OLE, lhs, rhs),
            BinaryOp::Eq => self.compile_float_cmp(FloatPredicate::OEQ, lhs, rhs),
            BinaryOp::NotEq => self.compile_float_cmp(FloatPredicate::ONE, lhs, rhs),
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                eprintln!(
                    "Error: Bitwise operations (&, |, ^) are only supported on integers, not floats."
                );
                None
            }
            BinaryOp::Pow => {
                let f64_type = self.context.f64_type();

                // Get or declare llvm.pow.f64 intrinsic
                let pow_fn = self.module.get_function("llvm.pow.f64").unwrap_or_else(|| {
                    let fn_type = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
                    self.module.add_function("llvm.pow.f64", fn_type, None)
                });

                let result = self
                    .builder
                    .build_call(pow_fn, &[lhs.into(), rhs.into()], "pow_result")
                    .ok()?
                    .try_as_basic_value()
                    .left()?;

                Some(result)
            }
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

    fn compile_list_comprehension(
        &mut self,
        expr: &Expr,
        generators: &[parser::ast::ComprehensionGen],
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        // For now, we'll compile this as a for loop with pre-allocation
        // [expr for x in arr if cond] becomes:
        // temp = zeros(max_size)
        // count = 0
        // for x in arr:
        //     if cond:
        //         temp[count] = expr
        //         count++
        // result = type[count]
        // copy temp to result

        if generators.is_empty() {
            eprintln!("Error: List comprehension must have at least one generator");
            return Err(CodegenError::InvalidOperation {
                operation: "list comprehension".to_string(),
                reason: "must have at least one generator".to_string(),
            });
        }

        let i64_type = self.context.i64_type();
        let f64_type = self.context.f64_type();

        // Step 1: Determine result type
        // For now, we'll use Float (Matrix) for all list comprehensions
        // TODO: Add type inference to support IntMatrix when appropriate
        let result_elem_type = BrixType::Float;

        // Step 2: Calculate max size (product of all iterable lengths)
        let mut total_size = i64_type.const_int(1, false);

        for generator in generators.iter() {
            let (iterable_val, iterable_type) = self.compile_expr(&generator.iterable)?;

            let len = match iterable_type {
                BrixType::Matrix | BrixType::IntMatrix => {
                    // Get rows * cols for total element count
                    let matrix_ptr = iterable_val.into_pointer_value();

                    // Load rows (field 0)
                    let rows_ptr = self
                        .builder
                        .build_struct_gep(
                            if iterable_type == BrixType::Matrix {
                                self.get_matrix_type()
                            } else {
                                self.get_intmatrix_type()
                            },
                            matrix_ptr,
                            0,
                            "rows_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get rows_ptr in list comprehension".to_string() })?;
                    let rows = self
                        .builder
                        .build_load(i64_type, rows_ptr, "rows")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load rows in list comprehension".to_string() })?
                        .into_int_value();

                    // Load cols (field 1)
                    let cols_ptr = self
                        .builder
                        .build_struct_gep(
                            if iterable_type == BrixType::Matrix {
                                self.get_matrix_type()
                            } else {
                                self.get_intmatrix_type()
                            },
                            matrix_ptr,
                            1,
                            "cols_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get cols_ptr in list comprehension".to_string() })?;
                    let cols = self
                        .builder
                        .build_load(i64_type, cols_ptr, "cols")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cols in list comprehension".to_string() })?
                        .into_int_value();

                    self.builder.build_int_mul(rows, cols, "len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute len in list comprehension".to_string() })?
                }
                _ => {
                    eprintln!(
                        "Error: List comprehension only supports Matrix/IntMatrix iterables for now"
                    );
                    return Err(CodegenError::InvalidOperation {
                        operation: "list comprehension".to_string(),
                        reason: "only supports Matrix/IntMatrix iterables for now".to_string(),
                    });
                }
            };

            total_size = self
                .builder
                .build_int_mul(total_size, len, "total_size")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_int_mul".to_string(),
                    details: "Failed to compute total size for list comprehension".to_string(),
                })?;
        }

        // Step 3: Allocate temporary array with max size
        let (temp_array, temp_type) = match result_elem_type {
            BrixType::Int => {
                // Allocate IntMatrix
                let fn_name = "intmatrix_new";
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let new_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
                    self.module
                        .add_function(fn_name, fn_type, Some(Linkage::External))
                });

                let one = i64_type.const_int(1, false);
                let result = self
                    .builder
                    .build_call(new_fn, &[one.into(), total_size.into()], "temp_array")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call intmatrix_new for temp array".to_string() })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "intmatrix_new return value".to_string(), context: "list comprehension temp array".to_string() })?;
                (result, BrixType::IntMatrix)
            }
            BrixType::Float => {
                // Allocate Matrix
                let fn_name = "matrix_new";
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let new_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
                    self.module
                        .add_function(fn_name, fn_type, Some(Linkage::External))
                });

                let one = i64_type.const_int(1, false);
                let result = self
                    .builder
                    .build_call(new_fn, &[one.into(), total_size.into()], "temp_array")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call matrix_new for temp array".to_string() })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "matrix_new return value".to_string(), context: "list comprehension temp array".to_string() })?;
                (result, BrixType::Matrix)
            }
            _ => {
                eprintln!("Error: List comprehension result type must be Int or Float for now");
                return Err(CodegenError::InvalidOperation {
                    operation: "list comprehension".to_string(),
                    reason: "result type must be Int or Float for now".to_string(),
                });
            }
        };

        // Step 4: Create counter variable
        let count_alloca = self.create_entry_block_alloca(i64_type.into(), "comp_count")?;
        self.builder
            .build_store(count_alloca, i64_type.const_int(0, false))
            .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to initialize comp_count".to_string() })?;

        // Step 5: Generate nested loops recursively
        self.generate_comp_loop(
            expr,
            generators,
            0,
            &temp_array,
            temp_type.clone(),
            count_alloca,
        )?;

        // Step 6: Load final count
        let final_count = self
            .builder
            .build_load(i64_type, count_alloca, "final_count")
            .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load final_count".to_string() })?
            .into_int_value();

        // Step 7: Create result array with actual size
        let (result_array, result_type) = match temp_type {
            BrixType::IntMatrix => {
                let fn_name = "intmatrix_new";
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let new_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
                    self.module
                        .add_function(fn_name, fn_type, Some(Linkage::External))
                });

                let one = i64_type.const_int(1, false);
                let result = self
                    .builder
                    .build_call(new_fn, &[one.into(), final_count.into()], "result_array")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call intmatrix_new for result array".to_string() })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "intmatrix_new return value".to_string(), context: "list comprehension result array".to_string() })?;
                (result, BrixType::IntMatrix)
            }
            BrixType::Matrix => {
                let fn_name = "matrix_new";
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let new_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
                    self.module
                        .add_function(fn_name, fn_type, Some(Linkage::External))
                });

                let one = i64_type.const_int(1, false);
                let result = self
                    .builder
                    .build_call(new_fn, &[one.into(), final_count.into()], "result_array")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call matrix_new for result array".to_string() })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "matrix_new return value".to_string(), context: "list comprehension result array".to_string() })?;
                (result, BrixType::Matrix)
            }
            _ => unreachable!(),
        };

        // Step 8: Copy elements from temp to result
        let parent_fn = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::LLVMError { operation: "get_insert_block".to_string(), details: "no current block in list comprehension copy".to_string() })?
            .get_parent()
            .ok_or_else(|| CodegenError::LLVMError { operation: "get_parent".to_string(), details: "block has no parent function in list comprehension copy".to_string() })?;
        let copy_cond_bb = self.context.append_basic_block(parent_fn, "copy_cond");
        let copy_body_bb = self.context.append_basic_block(parent_fn, "copy_body");
        let copy_after_bb = self.context.append_basic_block(parent_fn, "copy_after");

        // Initialize copy index
        let copy_idx_alloca = self.create_entry_block_alloca(i64_type.into(), "copy_idx")?;
        self.builder
            .build_store(copy_idx_alloca, i64_type.const_int(0, false))
            .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to initialize copy_idx".to_string() })?;
        self.builder
            .build_unconditional_branch(copy_cond_bb)
            .map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to copy_cond".to_string() })?;

        // Copy condition: idx < final_count
        self.builder.position_at_end(copy_cond_bb);
        let copy_idx = self
            .builder
            .build_load(i64_type, copy_idx_alloca, "copy_idx")
            .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load copy_idx".to_string() })?
            .into_int_value();
        let copy_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, copy_idx, final_count, "copy_cond")
            .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare copy_idx < final_count".to_string() })?;
        self.builder
            .build_conditional_branch(copy_cond, copy_body_bb, copy_after_bb)
            .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch in copy loop".to_string() })?;

        // Copy body: result[idx] = temp[idx]
        self.builder.position_at_end(copy_body_bb);

        unsafe {
            let temp_matrix_ptr = temp_array.into_pointer_value();
            let result_matrix_ptr = result_array.into_pointer_value();

            if temp_type == BrixType::Matrix {
                let matrix_type = self.get_matrix_type();

                // Get temp data pointer
                let temp_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, temp_matrix_ptr, 2, "temp_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get temp data_ptr_ptr in copy loop".to_string() })?;
                let temp_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        temp_data_ptr_ptr,
                        "temp_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load temp data_ptr in copy loop".to_string() })?
                    .into_pointer_value();

                // Get result data pointer
                let result_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, result_matrix_ptr, 2, "result_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get result data_ptr_ptr in copy loop".to_string() })?;
                let result_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        result_data_ptr_ptr,
                        "result_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load result data_ptr in copy loop".to_string() })?
                    .into_pointer_value();

                // Load temp[idx]
                let temp_elem_ptr = self
                    .builder
                    .build_gep(f64_type, temp_data_ptr, &[copy_idx], "temp_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get temp elem_ptr in copy loop".to_string() })?;
                let temp_elem = self
                    .builder
                    .build_load(f64_type, temp_elem_ptr, "temp_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load temp elem in copy loop".to_string() })?;

                // Store to result[idx]
                let result_elem_ptr = self
                    .builder
                    .build_gep(f64_type, result_data_ptr, &[copy_idx], "result_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get result elem_ptr in copy loop".to_string() })?;
                self.builder
                    .build_store(result_elem_ptr, temp_elem)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store temp elem to result in copy loop".to_string() })?;
            } else {
                let matrix_type = self.get_intmatrix_type();

                // Get temp data pointer
                let temp_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, temp_matrix_ptr, 2, "temp_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get int temp data_ptr_ptr in copy loop".to_string() })?;
                let temp_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        temp_data_ptr_ptr,
                        "temp_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int temp data_ptr in copy loop".to_string() })?
                    .into_pointer_value();

                // Get result data pointer
                let result_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, result_matrix_ptr, 2, "result_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get int result data_ptr_ptr in copy loop".to_string() })?;
                let result_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        result_data_ptr_ptr,
                        "result_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int result data_ptr in copy loop".to_string() })?
                    .into_pointer_value();

                // Load temp[idx]
                let temp_elem_ptr = self
                    .builder
                    .build_gep(i64_type, temp_data_ptr, &[copy_idx], "temp_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get int temp elem_ptr in copy loop".to_string() })?;
                let temp_elem = self
                    .builder
                    .build_load(i64_type, temp_elem_ptr, "temp_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int temp elem in copy loop".to_string() })?;

                // Store to result[idx]
                let result_elem_ptr = self
                    .builder
                    .build_gep(i64_type, result_data_ptr, &[copy_idx], "result_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get int result elem_ptr in copy loop".to_string() })?;
                self.builder
                    .build_store(result_elem_ptr, temp_elem)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store int temp elem to result in copy loop".to_string() })?;
            }
        }

        // Increment copy_idx
        let next_copy_idx = self
            .builder
            .build_int_add(copy_idx, i64_type.const_int(1, false), "next_copy_idx")
            .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment copy_idx".to_string() })?;
        self.builder
            .build_store(copy_idx_alloca, next_copy_idx)
            .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next copy_idx".to_string() })?;
        self.builder
            .build_unconditional_branch(copy_cond_bb)
            .map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch back to copy_cond".to_string() })?;

        // After copy loop
        self.builder.position_at_end(copy_after_bb);

        Ok((result_array, result_type))
    }

    fn generate_comp_loop(
        &mut self,
        expr: &Expr,
        generators: &[parser::ast::ComprehensionGen],
        gen_idx: usize,
        temp_array: &BasicValueEnum<'ctx>,
        temp_type: BrixType,
        count_alloca: PointerValue<'ctx>,
    ) -> CodegenResult<()> {
        if gen_idx >= generators.len() {
            // Base case: we're inside the innermost loop
            // Evaluate expr and add to temp_array[count++]

            let (expr_val, expr_type) = self.compile_expr(expr)?;

            let i64_type = self.context.i64_type();
            let f64_type = self.context.f64_type();

            // Load current count
            let count = self
                .builder
                .build_load(i64_type, count_alloca, "count")
                .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load count in comp loop".to_string() })?
                .into_int_value();

            // Get data pointer from temp_array
            let temp_matrix_ptr = temp_array.into_pointer_value();

            unsafe {
                if temp_type == BrixType::Matrix {
                    let matrix_type = self.get_matrix_type();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, temp_matrix_ptr, 2, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get data_ptr_ptr in comp loop base case".to_string() })?;
                    let data_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "data_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load data_ptr in comp loop base case".to_string() })?
                        .into_pointer_value();

                    // Convert expr_val to correct type if needed
                    let val_to_store = if expr_type == BrixType::Float {
                        expr_val
                    } else if expr_type == BrixType::Int {
                        // int -> float
                        let int_val = expr_val.into_int_value();
                        self.builder
                            .build_signed_int_to_float(int_val, f64_type, "int_to_float")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_signed_int_to_float".to_string(), details: "failed to convert int to float in comp loop".to_string() })?
                            .into()
                    } else {
                        eprintln!("Error: Type mismatch in list comprehension");
                        return Err(CodegenError::TypeError { expected: "Float or Int".to_string(), found: format!("{:?}", expr_type), context: "list comprehension expression".to_string() });
                    };

                    // Store at temp_array[count]
                    let elem_ptr = self
                        .builder
                        .build_gep(f64_type, data_ptr, &[count], "elem_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop base case".to_string() })?;
                    self.builder.build_store(elem_ptr, val_to_store).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store value in comp loop base case".to_string() })?;
                } else {
                    // IntMatrix
                    let matrix_type = self.get_intmatrix_type();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, temp_matrix_ptr, 2, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get int data_ptr_ptr in comp loop base case".to_string() })?;
                    let data_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "data_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int data_ptr in comp loop base case".to_string() })?
                        .into_pointer_value();

                    // Ensure type is Int
                    if expr_type != BrixType::Int {
                        eprintln!(
                            "Error: Type mismatch in list comprehension (expected Int for IntMatrix)"
                        );
                        return Err(CodegenError::TypeError { expected: "Int".to_string(), found: format!("{:?}", expr_type), context: "list comprehension IntMatrix expression".to_string() });
                    }

                    // Store at temp_array[count]
                    let elem_ptr = self
                        .builder
                        .build_gep(i64_type, data_ptr, &[count], "elem_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get int elem_ptr in comp loop base case".to_string() })?;
                    self.builder.build_store(elem_ptr, expr_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store int value in comp loop base case".to_string() })?;
                }
            }

            // Increment count
            let next_count = self
                .builder
                .build_int_add(count, i64_type.const_int(1, false), "next_count")
                .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment count in comp loop".to_string() })?;
            self.builder.build_store(count_alloca, next_count).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next_count in comp loop".to_string() })?;

            return Ok(());
        }

        // Recursive case: generate this loop level
        let generator = &generators[gen_idx];

        // Compile iterable
        let (iterable_val, iterable_type) = self.compile_expr(&generator.iterable)?;

        match iterable_type {
            BrixType::Matrix => {
                let i64_type = self.context.i64_type();
                let f64_type = self.context.f64_type();

                let matrix_ptr = iterable_val.into_pointer_value();
                let matrix_type = self.get_matrix_type();

                // Load dimensions
                let rows_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 0, "rows_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get rows_ptr in comp loop Matrix".to_string() })?;
                let rows = self
                    .builder
                    .build_load(i64_type, rows_ptr, "rows")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load rows in comp loop Matrix".to_string() })?
                    .into_int_value();

                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "cols_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get cols_ptr in comp loop Matrix".to_string() })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cols in comp loop Matrix".to_string() })?
                    .into_int_value();

                // Load data pointer
                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get data_ptr_ptr in comp loop Matrix".to_string() })?;
                let data_base = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data_base",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load data_base in comp loop Matrix".to_string() })?
                    .into_pointer_value();

                // Determine if destructuring
                let (total_len, is_destructuring) = if generator.var_names.len() > 1 {
                    (rows, true)
                } else {
                    (
                        self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute total_len in comp loop Matrix".to_string() })?,
                        false,
                    )
                };

                // Create loop blocks
                let parent_fn = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_insert_block".to_string(), details: "no current block in comp loop Matrix".to_string() })?
                    .get_parent()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_parent".to_string(), details: "block has no parent in comp loop Matrix".to_string() })?;
                let cond_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_cond_{}", gen_idx));
                let body_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_body_{}", gen_idx));
                let check_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_check_{}", gen_idx));
                let incr_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_incr_{}", gen_idx));
                let after_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_after_{}", gen_idx));

                // Allocate loop index
                let idx_alloca = self
                    .create_entry_block_alloca(i64_type.into(), &format!("comp_idx_{}", gen_idx))?;
                self.builder
                    .build_store(idx_alloca, i64_type.const_int(0, false))
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to init loop idx in comp loop Matrix".to_string() })?;

                // Allocate variables and save old ones
                let mut var_allocas = Vec::new();
                let mut old_vars = Vec::new();

                if is_destructuring {
                    for var_name in generator.var_names.iter() {
                        let var_alloca = self.create_entry_block_alloca(f64_type.into(), var_name)?;
                        let old_var = self.variables.remove(var_name);
                        self.variables
                            .insert(var_name.clone(), (var_alloca, BrixType::Float));
                        old_vars.push((var_name.clone(), old_var));
                        var_allocas.push(var_alloca);
                    }
                } else {
                    let var_name = &generator.var_names[0];
                    let var_alloca = self.create_entry_block_alloca(f64_type.into(), var_name)?;
                    let old_var = self.variables.remove(var_name);
                    self.variables
                        .insert(var_name.clone(), (var_alloca, BrixType::Float));
                    old_vars.push((var_name.clone(), old_var));
                }

                // Jump to condition
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to cond in comp loop Matrix".to_string() })?;

                // Condition: idx < total_len
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "cur_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cur_idx in comp loop Matrix".to_string() })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total_len, "loop_cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare idx < total_len in comp loop Matrix".to_string() })?;
                self.builder
                    .build_conditional_branch(cond, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch in comp loop Matrix".to_string() })?;

                // Body: load variables
                self.builder.position_at_end(body_bb);

                if is_destructuring {
                    // Load row elements
                    for (j, var_alloca) in var_allocas.iter().enumerate() {
                        unsafe {
                            let offset = self
                                .builder
                                .build_int_mul(cur_idx, cols, "row_offset")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute row_offset in comp loop Matrix".to_string() })?;
                            let col_offset = self
                                .builder
                                .build_int_add(
                                    offset,
                                    i64_type.const_int(j as u64, false),
                                    "elem_offset",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to compute elem_offset in comp loop Matrix".to_string() })?;

                            let elem_ptr = self
                                .builder
                                .build_gep(
                                    f64_type,
                                    data_base,
                                    &[col_offset],
                                    &format!("elem_{}_ptr", j),
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop Matrix destructuring".to_string() })?;
                            let elem_val = self
                                .builder
                                .build_load(f64_type, elem_ptr, &format!("elem_{}", j))
                                .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop Matrix destructuring".to_string() })?;
                            self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop Matrix destructuring".to_string() })?;
                        }
                    }
                } else {
                    // Load single element
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(f64_type, data_base, &[cur_idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop Matrix".to_string() })?;
                        let elem_val = self.builder.build_load(f64_type, elem_ptr, "elem").map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop Matrix".to_string() })?;
                        let current_var = self.variables.get(&generator.var_names[0]).ok_or_else(|| CodegenError::UndefinedSymbol { name: generator.var_names[0].clone(), context: "comp loop Matrix variable lookup".to_string() })?.0;
                        self.builder.build_store(current_var, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop Matrix".to_string() })?;
                    }
                }

                // Jump to check block (for conditions)
                self.builder.build_unconditional_branch(check_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to check in comp loop Matrix".to_string() })?;

                // Check block: evaluate all conditions
                self.builder.position_at_end(check_bb);

                if !generator.conditions.is_empty() {
                    let mut combined_cond = None;

                    for condition in &generator.conditions {
                        let (cond_val, _) = self.compile_expr(condition)?;
                        let cond_int = cond_val.into_int_value();
                        let cond_bool = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                cond_int,
                                i64_type.const_int(0, false),
                                "cond_bool",
                            )
                            .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare condition in comp loop Matrix".to_string() })?;

                        combined_cond = Some(if let Some(prev) = combined_cond {
                            self.builder
                                .build_and(prev, cond_bool, "combined_cond")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_and".to_string(), details: "failed to combine conditions in comp loop Matrix".to_string() })?
                        } else {
                            cond_bool
                        });
                    }

                    // If conditions pass, recurse to next generator or evaluate expr
                    let recurse_bb = self
                        .context
                        .append_basic_block(parent_fn, &format!("comp_recurse_{}", gen_idx));
                    let combined = combined_cond.ok_or_else(|| CodegenError::MissingValue { what: "combined_cond".to_string(), context: "comp loop Matrix conditions".to_string() })?;
                    self.builder
                        .build_conditional_branch(combined, recurse_bb, incr_bb)
                        .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch on condition in comp loop Matrix".to_string() })?;

                    self.builder.position_at_end(recurse_bb);
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr in comp loop Matrix".to_string() })?;
                } else {
                    // No conditions, just recurse
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr (no cond) in comp loop Matrix".to_string() })?;
                }

                // Increment block
                self.builder.position_at_end(incr_bb);
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment idx in comp loop Matrix".to_string() })?;
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next_idx in comp loop Matrix".to_string() })?;
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to loop back in comp loop Matrix".to_string() })?;

                // After block: restore variables
                self.builder.position_at_end(after_bb);

                for (var_name, old_var_opt) in old_vars {
                    if let Some(old) = old_var_opt {
                        self.variables.insert(var_name, old);
                    } else {
                        self.variables.remove(&var_name);
                    }
                }

                Ok(())
            }

            BrixType::IntMatrix => {
                let i64_type = self.context.i64_type();

                let matrix_ptr = iterable_val.into_pointer_value();
                let matrix_type = self.get_intmatrix_type();

                // Load dimensions
                let rows_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 0, "rows_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get rows_ptr in comp loop IntMatrix".to_string() })?;
                let rows = self
                    .builder
                    .build_load(i64_type, rows_ptr, "rows")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load rows in comp loop IntMatrix".to_string() })?
                    .into_int_value();

                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "cols_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get cols_ptr in comp loop IntMatrix".to_string() })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cols in comp loop IntMatrix".to_string() })?
                    .into_int_value();

                // Load data pointer
                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get data_ptr_ptr in comp loop IntMatrix".to_string() })?;
                let data_base = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data_base",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load data_base in comp loop IntMatrix".to_string() })?
                    .into_pointer_value();

                // Determine if destructuring
                let (total_len, is_destructuring) = if generator.var_names.len() > 1 {
                    (rows, true)
                } else {
                    (
                        self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute total_len in comp loop IntMatrix".to_string() })?,
                        false,
                    )
                };

                // Create loop blocks
                let parent_fn = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_insert_block".to_string(), details: "no current block in comp loop IntMatrix".to_string() })?
                    .get_parent()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_parent".to_string(), details: "block has no parent in comp loop IntMatrix".to_string() })?;
                let cond_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_cond_{}", gen_idx));
                let body_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_body_{}", gen_idx));
                let check_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_check_{}", gen_idx));
                let incr_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_incr_{}", gen_idx));
                let after_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_after_{}", gen_idx));

                // Allocate loop index
                let idx_alloca = self
                    .create_entry_block_alloca(i64_type.into(), &format!("comp_idx_{}", gen_idx))?;
                self.builder
                    .build_store(idx_alloca, i64_type.const_int(0, false))
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to init loop idx in comp loop IntMatrix".to_string() })?;

                // Allocate variables and save old ones
                let mut var_allocas = Vec::new();
                let mut old_vars = Vec::new();

                if is_destructuring {
                    for var_name in generator.var_names.iter() {
                        let var_alloca = self.create_entry_block_alloca(i64_type.into(), var_name)?;
                        let old_var = self.variables.remove(var_name);
                        self.variables
                            .insert(var_name.clone(), (var_alloca, BrixType::Int));
                        old_vars.push((var_name.clone(), old_var));
                        var_allocas.push(var_alloca);
                    }
                } else {
                    let var_name = &generator.var_names[0];
                    let var_alloca = self.create_entry_block_alloca(i64_type.into(), var_name)?;
                    let old_var = self.variables.remove(var_name);
                    self.variables
                        .insert(var_name.clone(), (var_alloca, BrixType::Int));
                    old_vars.push((var_name.clone(), old_var));
                }

                // Jump to condition
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to cond in comp loop IntMatrix".to_string() })?;

                // Condition: idx < total_len
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "cur_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cur_idx in comp loop IntMatrix".to_string() })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total_len, "loop_cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare idx < total_len in comp loop IntMatrix".to_string() })?;
                self.builder
                    .build_conditional_branch(cond, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch in comp loop IntMatrix".to_string() })?;

                // Body: load variables
                self.builder.position_at_end(body_bb);

                if is_destructuring {
                    // Load row elements
                    for (j, var_alloca) in var_allocas.iter().enumerate() {
                        unsafe {
                            let offset = self
                                .builder
                                .build_int_mul(cur_idx, cols, "row_offset")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute row_offset in comp loop IntMatrix".to_string() })?;
                            let col_offset = self
                                .builder
                                .build_int_add(
                                    offset,
                                    i64_type.const_int(j as u64, false),
                                    "elem_offset",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to compute elem_offset in comp loop IntMatrix".to_string() })?;

                            let elem_ptr = self
                                .builder
                                .build_gep(
                                    i64_type,
                                    data_base,
                                    &[col_offset],
                                    &format!("elem_{}_ptr", j),
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop IntMatrix destructuring".to_string() })?;
                            let elem_val = self
                                .builder
                                .build_load(i64_type, elem_ptr, &format!("elem_{}", j))
                                .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop IntMatrix destructuring".to_string() })?;
                            self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop IntMatrix destructuring".to_string() })?;
                        }
                    }
                } else {
                    // Load single element
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(i64_type, data_base, &[cur_idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop IntMatrix".to_string() })?;
                        let elem_val = self.builder.build_load(i64_type, elem_ptr, "elem").map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop IntMatrix".to_string() })?;
                        let current_var = self.variables.get(&generator.var_names[0]).ok_or_else(|| CodegenError::UndefinedSymbol { name: generator.var_names[0].clone(), context: "comp loop IntMatrix variable lookup".to_string() })?.0;
                        self.builder.build_store(current_var, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop IntMatrix".to_string() })?;
                    }
                }

                // Jump to check block (for conditions)
                self.builder.build_unconditional_branch(check_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to check in comp loop IntMatrix".to_string() })?;

                // Check block: evaluate all conditions
                self.builder.position_at_end(check_bb);

                if !generator.conditions.is_empty() {
                    let mut combined_cond = None;

                    for condition in &generator.conditions {
                        let (cond_val, _) = self.compile_expr(condition)?;
                        let cond_int = cond_val.into_int_value();
                        let cond_bool = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                cond_int,
                                i64_type.const_int(0, false),
                                "cond_bool",
                            )
                            .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare condition in comp loop IntMatrix".to_string() })?;

                        combined_cond = Some(if let Some(prev) = combined_cond {
                            self.builder
                                .build_and(prev, cond_bool, "combined_cond")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_and".to_string(), details: "failed to combine conditions in comp loop IntMatrix".to_string() })?
                        } else {
                            cond_bool
                        });
                    }

                    // If conditions pass, recurse to next generator or evaluate expr
                    let recurse_bb = self
                        .context
                        .append_basic_block(parent_fn, &format!("comp_recurse_{}", gen_idx));
                    let combined = combined_cond.ok_or_else(|| CodegenError::MissingValue { what: "combined_cond".to_string(), context: "comp loop IntMatrix conditions".to_string() })?;
                    self.builder
                        .build_conditional_branch(combined, recurse_bb, incr_bb)
                        .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch on condition in comp loop IntMatrix".to_string() })?;

                    self.builder.position_at_end(recurse_bb);
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr in comp loop IntMatrix".to_string() })?;
                } else {
                    // No conditions, just recurse
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr (no cond) in comp loop IntMatrix".to_string() })?;
                }

                // Increment block
                self.builder.position_at_end(incr_bb);
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment idx in comp loop IntMatrix".to_string() })?;
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next_idx in comp loop IntMatrix".to_string() })?;
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to loop back in comp loop IntMatrix".to_string() })?;

                // After block: restore variables
                self.builder.position_at_end(after_bb);

                for (var_name, old_var_opt) in old_vars {
                    if let Some(old) = old_var_opt {
                        self.variables.insert(var_name, old);
                    } else {
                        self.variables.remove(&var_name);
                    }
                }

                Ok(())
            }

            _ => {
                eprintln!(
                    "Error: Unsupported iterable type in list comprehension: {:?}",
                    iterable_type
                );
                Err(CodegenError::InvalidOperation {
                    operation: "list comprehension".to_string(),
                    reason: format!("unsupported iterable type: {:?}", iterable_type),
                })
            }
        }
    }

    /// Compile pattern matching: returns i1 (bool) indicating if pattern matches
    fn compile_pattern_match(
        &mut self,
        pattern: &parser::ast::Pattern,
        value: BasicValueEnum<'ctx>,
        value_type: &BrixType,
    ) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        use parser::ast::Pattern;

        match pattern {
            Pattern::Literal(lit) => {
                // Compare value with literal
                match (lit, value_type) {
                    (parser::ast::Literal::Int(n), BrixType::Int) => {
                        let literal_val = self.context.i64_type().const_int(*n as u64, false);
                        let cmp = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                value.into_int_value(),
                                literal_val,
                                "pat_int_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to compare int pattern".to_string(),
                            })?;
                        Ok(cmp)
                    }
                    (parser::ast::Literal::Float(f), BrixType::Float) => {
                        let literal_val = self.context.f64_type().const_float(*f);
                        let cmp = self
                            .builder
                            .build_float_compare(
                                inkwell::FloatPredicate::OEQ,
                                value.into_float_value(),
                                literal_val,
                                "pat_float_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_float_compare".to_string(),
                                details: "Failed to compare float pattern".to_string(),
                            })?;
                        Ok(cmp)
                    }
                    (parser::ast::Literal::Bool(b), BrixType::Int) => {
                        // bool is stored as i64
                        let literal_val = self.context.i64_type().const_int(*b as u64, false);
                        let cmp = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                value.into_int_value(),
                                literal_val,
                                "pat_bool_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to compare bool pattern".to_string(),
                            })?;
                        Ok(cmp)
                    }
                    (parser::ast::Literal::String(s), BrixType::String) => {
                        // String comparison via runtime str_eq
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let fn_type = self
                            .context
                            .bool_type()
                            .fn_type(&[ptr_type.into(), ptr_type.into()], false);

                        let str_eq_fn = self.module.get_function("str_eq").unwrap_or_else(|| {
                            self.module
                                .add_function("str_eq", fn_type, Some(Linkage::External))
                        });

                        // Create literal string
                        let raw_str = self.builder.build_global_string_ptr(s, "pat_str")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_global_string_ptr".to_string(),
                                details: "Failed to create pattern string".to_string(),
                            })?;
                        let str_new_fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                        let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                            self.module.add_function(
                                "str_new",
                                str_new_fn_type,
                                Some(Linkage::External),
                            )
                        });

                        let call = self
                            .builder
                            .build_call(
                                str_new_fn,
                                &[raw_str.as_pointer_value().into()],
                                "pat_lit_str",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call str_new for pattern".to_string(),
                            })?;
                        let literal_str = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "str_new result".to_string(),
                                context: "pattern string literal".to_string(),
                            })?;

                        // Compare strings
                        let call = self
                            .builder
                            .build_call(
                                str_eq_fn,
                                &[value.into(), literal_str.into()],
                                "pat_str_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call str_eq for pattern".to_string(),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "str_eq result".to_string(),
                                context: "pattern string comparison".to_string(),
                            })?
                            .into_int_value();

                        Ok(result)
                    }
                    (parser::ast::Literal::Atom(name), BrixType::Atom) => {
                        // Atom comparison: compare atom IDs (i64)
                        // First, intern the pattern atom to get its ID
                        let i64_type = self.context.i64_type();
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
                        let atom_intern_fn =
                            self.module.get_function("atom_intern").unwrap_or_else(|| {
                                self.module.add_function(
                                    "atom_intern",
                                    fn_type,
                                    Some(Linkage::External),
                                )
                            });

                        // Create string literal for atom name
                        let name_cstr = self
                            .builder
                            .build_global_string_ptr(name, "pat_atom_name")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_global_string_ptr".to_string(),
                                details: "Failed to create atom name string for pattern".to_string(),
                            })?;

                        // Call atom_intern(name) to get the pattern atom ID
                        let call = self
                            .builder
                            .build_call(
                                atom_intern_fn,
                                &[name_cstr.as_pointer_value().into()],
                                "pat_atom_id",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call atom_intern for pattern".to_string(),
                            })?;
                        let pattern_atom_id = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "atom_intern result".to_string(),
                                context: "pattern atom comparison".to_string(),
                            })?
                            .into_int_value();

                        // Compare atom IDs (O(1) comparison)
                        let cmp = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                value.into_int_value(),
                                pattern_atom_id,
                                "pat_atom_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to compare atom IDs in pattern".to_string(),
                            })?;

                        Ok(cmp)
                    }
                    _ => {
                        Err(CodegenError::TypeError {
                            expected: format!("{:?}", value_type),
                            found: format!("{:?}", lit),
                            context: "Pattern literal type mismatch".to_string(),
                        })
                    }
                }
            }

            Pattern::Wildcard => {
                // Wildcard always matches
                Ok(self.context.bool_type().const_int(1, false))
            }

            Pattern::Binding(_) => {
                // Binding always matches (variable name is bound in caller)
                Ok(self.context.bool_type().const_int(1, false))
            }

            Pattern::Or(patterns) => {
                // Or pattern: match any of the sub-patterns
                let mut result = self.context.bool_type().const_int(0, false);

                for pat in patterns {
                    let pat_match = self.compile_pattern_match(pat, value, value_type)?;
                    result = self.builder.build_or(result, pat_match, "or_pat")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_or".to_string(),
                            details: "Failed to OR pattern results".to_string(),
                        })?;
                }

                Ok(result)
            }
        }
    }

    /// Check if two types are compatible for match arms
    fn are_types_compatible(&self, type1: &BrixType, type2: &BrixType) -> bool {
        // Same type is always compatible
        if type1 == type2 {
            return true;
        }

        // Int and Float are compatible (can promote int to float)
        if (*type1 == BrixType::Int && *type2 == BrixType::Float)
            || (*type1 == BrixType::Float && *type2 == BrixType::Int)
        {
            return true;
        }

        // All other combinations are incompatible
        false
    }
}
