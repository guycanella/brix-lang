// Statement compilation
//
// This module contains logic for compiling Brix statements (if, while, for, etc.).
//
// REFACTORING NOTE (v1.2):
// - Extracted from lib.rs (originally ~630 lines)
// - Uses trait pattern (StatementCompiler) for organization
// - Handles 10 out of 12 statement types
//
// Refactored statements:
// - Print/Println/Printf - Output statements
// - If/While - Control flow
// - Block/Expr - Basic statements
// - Import/Return - Module and function control
// - VariableDecl/DestructuringDecl/Assignment - Variable management
//
// Still in lib.rs:
// - For loops (complex, ~500 lines with range/iterator/zip support)
// - FunctionDef (has dedicated compile_function_def method)

use crate::{Compiler, CodegenError, CodegenResult};
use crate::helpers::HelperFunctions;
use inkwell::AddressSpace;
use parser::ast::Expr;

/// Trait for statement compilation helper methods
pub trait StatementCompiler<'ctx> {
    /// Compile print statement (without newline)
    fn compile_print_stmt(&mut self, expr: &Expr) -> CodegenResult<()>;

    /// Compile println statement (with newline)
    fn compile_println_stmt(&mut self, expr: &Expr) -> CodegenResult<()>;

    /// Compile printf statement (formatted output)
    fn compile_printf_stmt(&mut self, format: &str, args: &[Expr]) -> CodegenResult<()>;

    /// Compile expression as statement (discard result)
    fn compile_expr_stmt(&mut self, expr: &Expr) -> CodegenResult<()>;

    /// Compile block statement (list of statements)
    fn compile_block_stmt(&mut self, statements: &[parser::ast::Stmt], function: inkwell::values::FunctionValue<'ctx>) -> CodegenResult<()>;

    /// Compile if statement (with optional else block)
    fn compile_if_stmt(
        &mut self,
        condition: &Expr,
        then_block: &parser::ast::Stmt,
        else_block: &Option<Box<parser::ast::Stmt>>,
        function: inkwell::values::FunctionValue<'ctx>,
    ) -> CodegenResult<()>;

    /// Compile while loop
    fn compile_while_stmt(&mut self, condition: &Expr, body: &parser::ast::Stmt, function: inkwell::values::FunctionValue<'ctx>) -> CodegenResult<()>;

    /// Compile import statement
    fn compile_import_stmt(&mut self, module: &str, alias: &Option<String>) -> CodegenResult<()>;

    /// Compile return statement (single, multiple, or void)
    fn compile_return_stmt(&mut self, values: &[Expr]) -> CodegenResult<()>;

    /// Compile variable declaration with type inference and casting
    fn compile_variable_decl_stmt(&mut self, name: &str, type_hint: &Option<String>, value: &Expr) -> CodegenResult<()>;

    /// Compile destructuring declaration (tuple unpacking)
    fn compile_destructuring_decl_stmt(&mut self, names: &[String], value: &Expr) -> CodegenResult<()>;

    /// Compile assignment statement
    fn compile_assignment_stmt(&mut self, target: &Expr, value: &Expr) -> CodegenResult<()>;
}

impl<'a, 'ctx> StatementCompiler<'ctx> for Compiler<'a, 'ctx> {
    fn compile_print_stmt(&mut self, expr: &Expr) -> CodegenResult<()> {
        let (val, brix_type) = self.compile_expr(expr)?;

        // Convert value to string
        let str_val = self.value_to_string(val, &brix_type, None)?;

        let printf_fn = self.get_printf();
        let fmt_str = self
            .builder
            .build_global_string_ptr("%s", "print_fmt")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_global_string_ptr".to_string(),
                details: "Failed to create format string for print".to_string(),
            })?;

        // Extract char* from BrixString
        let struct_ptr = str_val.into_pointer_value();
        let str_type = self.get_string_type();
        let data_ptr_ptr = self
            .builder
            .build_struct_gep(str_type, struct_ptr, 1, "str_data_ptr")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get string data pointer".to_string(),
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
                details: "Failed to load string data".to_string(),
            })?;

        self.builder
            .build_call(
                printf_fn,
                &[fmt_str.as_pointer_value().into(), data_ptr.into()],
                "call_print",
            )
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call printf".to_string(),
            })?;

        Ok(())
    }

    fn compile_println_stmt(&mut self, expr: &Expr) -> CodegenResult<()> {
        let (val, brix_type) = self.compile_expr(expr)?;

        // Convert value to string
        let str_val = self.value_to_string(val, &brix_type, None)?;

        let printf_fn = self.get_printf();
        let fmt_str = self
            .builder
            .build_global_string_ptr("%s\n", "println_fmt")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_global_string_ptr".to_string(),
                details: "Failed to create format string for println".to_string(),
            })?;

        // Extract char* from BrixString
        let struct_ptr = str_val.into_pointer_value();
        let str_type = self.get_string_type();
        let data_ptr_ptr = self
            .builder
            .build_struct_gep(str_type, struct_ptr, 1, "str_data_ptr")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get string data pointer".to_string(),
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
                details: "Failed to load string data".to_string(),
            })?;

        self.builder
            .build_call(
                printf_fn,
                &[fmt_str.as_pointer_value().into(), data_ptr.into()],
                "call_println",
            )
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call printf".to_string(),
            })?;

        Ok(())
    }

    fn compile_printf_stmt(&mut self, format: &str, args: &[Expr]) -> CodegenResult<()> {
        use crate::BrixType;

        let printf_fn = self.get_printf();
        let global_str = self
            .builder
            .build_global_string_ptr(format, "fmt_str")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_global_string_ptr".to_string(),
                details: "Failed to create format string for printf".to_string(),
            })?;

        use inkwell::values::BasicMetadataValueEnum;
        let mut compiled_args: Vec<BasicMetadataValueEnum> = Vec::new();
        compiled_args.push(global_str.as_pointer_value().into());

        for arg in args {
            let (val, brix_type) = self.compile_expr(arg)?;
            match brix_type {
                BrixType::String => {
                    let struct_ptr = val.into_pointer_value();
                    let str_type = self.get_string_type();
                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(str_type, struct_ptr, 1, "str_data_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_struct_gep".to_string(),
                            details: "Failed to get string data pointer".to_string(),
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
                            details: "Failed to load string data".to_string(),
                        })?;
                    compiled_args.push(data_ptr.into());
                }
                BrixType::Matrix => compiled_args.push(val.into()),
                _ => compiled_args.push(val.into()),
            }
        }
        self.builder
            .build_call(printf_fn, &compiled_args, "call_printf")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call printf".to_string(),
            })?;

        Ok(())
    }

    fn compile_expr_stmt(&mut self, expr: &Expr) -> CodegenResult<()> {
        self.compile_expr(expr)?;
        Ok(())
    }

    fn compile_block_stmt(&mut self, statements: &[parser::ast::Stmt], function: inkwell::values::FunctionValue<'ctx>) -> CodegenResult<()> {
        for s in statements {
            self.compile_stmt(s, function)?;
        }
        Ok(())
    }

    fn compile_if_stmt(
        &mut self,
        condition: &Expr,
        then_block: &parser::ast::Stmt,
        else_block: &Option<Box<parser::ast::Stmt>>,
        function: inkwell::values::FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        use inkwell::IntPredicate;

        let (cond_val, _) = self.compile_expr(condition)?;
        let cond_int = cond_val.into_int_value(); // Assume int (booleano)

        let i64_type = self.context.i64_type();
        let zero = i64_type.const_int(0, false);
        let cond_bool = self
            .builder
            .build_int_compare(IntPredicate::NE, cond_int, zero, "ifcond")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_int_compare".to_string(),
                details: "Failed to compare if condition".to_string(),
            })?;

        let then_bb = self.context.append_basic_block(function, "then_block");
        let else_bb = self.context.append_basic_block(function, "else_block");
        let merge_bb = self.context.append_basic_block(function, "merge_block");

        self.builder
            .build_conditional_branch(cond_bool, then_bb, else_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_conditional_branch".to_string(),
                details: "Failed to build if conditional branch".to_string(),
            })?;

        // THEN
        self.builder.position_at_end(then_bb);
        self.compile_stmt(then_block, function)?;
        // Only add branch if block doesn't already have a terminator (e.g., return)
        if self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "get_insert_block".to_string(),
                details: "Failed to get current block".to_string(),
            })?
            .get_terminator()
            .is_none()
        {
            self.builder.build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_unconditional_branch".to_string(),
                    details: "Failed to build branch to merge block".to_string(),
                })?;
        }

        // ELSE
        self.builder.position_at_end(else_bb);
        if let Some(else_stmt) = else_block {
            self.compile_stmt(else_stmt, function)?;
        }
        // Only add branch if block doesn't already have a terminator
        if self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "get_insert_block".to_string(),
                details: "Failed to get current block".to_string(),
            })?
            .get_terminator()
            .is_none()
        {
            self.builder.build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_unconditional_branch".to_string(),
                    details: "Failed to build branch to merge block".to_string(),
                })?;
        }

        // MERGE
        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    fn compile_while_stmt(&mut self, condition: &Expr, body: &parser::ast::Stmt, function: inkwell::values::FunctionValue<'ctx>) -> CodegenResult<()> {
        use inkwell::IntPredicate;

        let header_bb = self.context.append_basic_block(function, "while_header");
        let body_bb = self.context.append_basic_block(function, "while_body");
        let after_bb = self.context.append_basic_block(function, "while_after");

        self.builder.build_unconditional_branch(header_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_unconditional_branch".to_string(),
                details: "Failed to build branch to while header".to_string(),
            })?;
        self.builder.position_at_end(header_bb);

        let (cond_val, _) = self.compile_expr(condition)?;
        let cond_int = cond_val.into_int_value();

        let i64_type = self.context.i64_type();
        let zero = i64_type.const_int(0, false);
        let cond_bool = self
            .builder
            .build_int_compare(IntPredicate::NE, cond_int, zero, "loop_cond")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_int_compare".to_string(),
                details: "Failed to compare while condition".to_string(),
            })?;

        self.builder
            .build_conditional_branch(cond_bool, body_bb, after_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_conditional_branch".to_string(),
                details: "Failed to build while conditional branch".to_string(),
            })?;

        self.builder.position_at_end(body_bb);
        self.compile_stmt(body, function)?;
        self.builder.build_unconditional_branch(header_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_unconditional_branch".to_string(),
                details: "Failed to build branch back to while header".to_string(),
            })?;

        self.builder.position_at_end(after_bb);
        Ok(())
    }

    fn compile_import_stmt(&mut self, module: &str, alias: &Option<String>) -> CodegenResult<()> {
        use crate::builtins::math::MathFunctions;

        // Register math functions when importing math module
        if module == "math" {
            let default_prefix = module.to_string();
            let prefix = alias.as_ref().unwrap_or(&default_prefix);
            self.register_math_functions(prefix);
        }
        Ok(())
    }

    fn compile_return_stmt(&mut self, values: &[Expr]) -> CodegenResult<()> {
        use crate::BrixType;

        if values.is_empty() {
            // Void return
            self.builder.build_return(None)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "Failed to build void return".to_string(),
                })?;
        } else if values.len() == 1 {
            // Single return
            let (val, _) = self.compile_expr(&values[0])?;
            self.builder.build_return(Some(&val))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "Failed to build single value return".to_string(),
                })?;
        } else {
            // Multiple returns - create struct
            let mut compiled_values = Vec::new();
            let mut value_types = Vec::new();

            for val_expr in values {
                let (val, val_type) = self.compile_expr(val_expr)?;
                compiled_values.push(val);
                value_types.push(val_type);
            }

            // Create struct type
            let tuple_type = BrixType::Tuple(value_types);
            let struct_llvm_type = self.brix_type_to_llvm(&tuple_type);

            // Create an undef struct value
            let struct_type = struct_llvm_type.into_struct_type();
            let mut struct_val = struct_type.get_undef();

            // Insert each value into the struct
            for (i, val) in compiled_values.iter().enumerate() {
                struct_val = self
                    .builder
                    .build_insert_value(struct_val, *val, i as u32, "insert")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_insert_value".to_string(),
                        details: format!("Failed to insert value {} into return tuple", i),
                    })?
                    .into_struct_value();
            }

            self.builder.build_return(Some(&struct_val))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "Failed to build multiple value return".to_string(),
                })?;
        }
        Ok(())
    }

    fn compile_variable_decl_stmt(&mut self, name: &str, type_hint: &Option<String>, value: &Expr) -> CodegenResult<()> {
        use crate::BrixType;
        use inkwell::types::BasicTypeEnum;

        let (init_val, mut val_type) = self.compile_expr(value)?;
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
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_float_to_signed_int".to_string(),
                                details: "Failed to cast float to int".to_string(),
                            })?
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
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_signed_int_to_float".to_string(),
                                details: "Failed to cast int to float".to_string(),
                            })?
                            .into();
                        val_type = BrixType::Float;
                    }
                }
                "bool" => {
                    val_type = BrixType::Int;
                }
                "string" => {
                    if val_type != BrixType::String {
                        return Err(CodegenError::TypeError {
                            expected: "String".to_string(),
                            found: format!("{:?}", val_type),
                            context: format!("Variable declaration '{}'", name),
                        });
                    }
                }
                "error" => {
                    if val_type != BrixType::Error && val_type != BrixType::Nil {
                        return Err(CodegenError::TypeError {
                            expected: "Error or Nil".to_string(),
                            found: format!("{:?}", val_type),
                            context: format!("Variable declaration '{}'", name),
                        });
                    }
                    // Accept both Error and Nil for error type
                    // val_type remains as-is (Error or Nil)
                }
                _ => {
                    if hint != "matrix" && hint != "intmatrix" && hint != "complex" {
                        return Err(CodegenError::TypeError {
                            expected: "Known type".to_string(),
                            found: hint.clone(),
                            context: format!("Variable declaration '{}'", name),
                        });
                    }
                }
            }
        }

        // --- ALLOCATION ---
        let llvm_type: BasicTypeEnum = match &val_type {
            BrixType::Int | BrixType::Atom => self.context.i64_type().into(), // Atom = i64 (atom ID)
            BrixType::Float => self.context.f64_type().into(),
            BrixType::String
            | BrixType::Matrix
            | BrixType::IntMatrix
            | BrixType::ComplexMatrix
            | BrixType::FloatPtr
            | BrixType::Nil
            | BrixType::Error => self.context.ptr_type(AddressSpace::default()).into(),
            BrixType::Complex => {
                // Allocate space for complex struct { f64, f64 }
                self.brix_type_to_llvm(&BrixType::Complex)
            }
            BrixType::Tuple(types) => {
                // Allocate space for tuple struct
                self.brix_type_to_llvm(&BrixType::Tuple(types.clone()))
            }
            _ => {
                return Err(CodegenError::TypeError {
                    expected: "Known type".to_string(),
                    found: format!("{:?}", val_type),
                    context: format!("Variable declaration '{}'", name),
                });
            }
        };

        let alloca = self.create_entry_block_alloca(llvm_type, name)?;
        self.builder.build_store(alloca, final_val)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: format!("Failed to store value in variable '{}'", name),
            })?;

        self.variables.insert(name.to_string(), (alloca, val_type));
        Ok(())
    }

    fn compile_destructuring_decl_stmt(&mut self, names: &[String], value: &Expr) -> CodegenResult<()> {
        use crate::BrixType;

        // Compile the expression that returns a tuple
        let (tuple_val, tuple_type) = self.compile_expr(value)?;

        // Ensure it's a tuple type
        let field_types = match tuple_type {
            BrixType::Tuple(field_types) => field_types,
            _ => {
                return Err(CodegenError::TypeError {
                    expected: "Tuple".to_string(),
                    found: format!("{:?}", tuple_type),
                    context: "Destructuring declaration".to_string(),
                });
            }
        };

        // Check that the number of names matches the tuple size
        if names.len() != field_types.len() {
            return Err(CodegenError::InvalidOperation {
                operation: "Destructuring".to_string(),
                reason: format!(
                    "Mismatch in number of values - expected {} values, got {}",
                    names.len(),
                    field_types.len()
                ),
            });
        }

        // Extract each field and assign to a variable
        for (i, (name, field_type)) in names.iter().zip(field_types.iter()).enumerate() {
            // Skip if name is "_" (ignore value)
            if name == "_" {
                continue;
            }

            // Extract the field from the struct
            let extracted = self
                .builder
                .build_extract_value(
                    tuple_val.into_struct_value(),
                    i as u32,
                    &format!("extract_{}", name),
                )
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_extract_value".to_string(),
                    details: format!("Failed to extract field {} from tuple", i),
                })?;

            // Allocate and store the variable
            let llvm_type = self.brix_type_to_llvm(field_type);
            let alloca = self.builder.build_alloca(llvm_type, name)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_alloca".to_string(),
                    details: format!("Failed to allocate variable '{}'", name),
                })?;
            self.builder.build_store(alloca, extracted)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: format!("Failed to store value in variable '{}'", name),
                })?;

            // Register in symbol table
            self.variables
                .insert(name.clone(), (alloca, field_type.clone()));
        }

        Ok(())
    }

    fn compile_assignment_stmt(&mut self, target: &Expr, value: &Expr) -> CodegenResult<()> {
        use crate::BrixType;

        let (target_ptr, target_type) = self.compile_lvalue_addr(target)?;
        let (val, val_type) = self.compile_expr(value)?;

        // Only cast Int→Float if the target expects Float
        let final_val = if target_type == BrixType::Float && val_type == BrixType::Int {
            self.builder
                .build_signed_int_to_float(
                    val.into_int_value(),
                    self.context.f64_type(),
                    "cast",
                )
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_signed_int_to_float".to_string(),
                    details: "Failed to cast int to float in assignment".to_string(),
                })?
                .into()
        } else {
            val
        };

        self.builder.build_store(target_ptr, final_val)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: "Failed to store value in assignment target".to_string(),
            })?;

        Ok(())
    }
}
