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
                            span: None,
            })?;

        // Extract char* from BrixString
        let struct_ptr = str_val.into_pointer_value();
        let str_type = self.get_string_type();
        let data_ptr_ptr = self
            .builder
            .build_struct_gep(str_type, struct_ptr, 2, "str_data_ptr")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get string data pointer".to_string(),
                            span: None,
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
                            span: None,
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
                            span: None,
            })?;

        // ARC: Release temporary BrixString created by value_to_string or str_new.
        // For non-String types, value_to_string always allocates a new BrixString.
        // For String types, only release if the expression is a temporary (not a
        // variable reference or field access which are "borrowed").
        if Self::is_print_temp(&brix_type, &expr.kind) {
            self.insert_release(struct_ptr, &crate::BrixType::String)?;
        }

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
                            span: None,
            })?;

        // Extract char* from BrixString
        let struct_ptr = str_val.into_pointer_value();
        let str_type = self.get_string_type();
        let data_ptr_ptr = self
            .builder
            .build_struct_gep(str_type, struct_ptr, 2, "str_data_ptr")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get string data pointer".to_string(),
                            span: None,
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
                            span: None,
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
                            span: None,
            })?;

        // ARC: Release temporary BrixString (same logic as compile_print_stmt)
        if Self::is_print_temp(&brix_type, &expr.kind) {
            self.insert_release(struct_ptr, &crate::BrixType::String)?;
        }

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
                            span: None,
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
                        .build_struct_gep(str_type, struct_ptr, 2, "str_data_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_struct_gep".to_string(),
                            details: "Failed to get string data pointer".to_string(),
                                                    span: None,
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
                                                    span: None,
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
                            span: None,
            })?;

        Ok(())
    }

    fn compile_expr_stmt(&mut self, expr: &Expr) -> CodegenResult<()> {
        let (val, brix_type) = self.compile_expr(expr)?;

        // ARC: Release discarded ref-counted temporaries.
        // If the expression produced a ref-counted value that's not stored anywhere,
        // it would leak. Release it unless it's a variable reference (which is owned
        // by the variable and will be released at scope exit).
        if Compiler::is_ref_counted(&brix_type) {
            let is_borrowed = matches!(
                &expr.kind,
                parser::ast::ExprKind::Identifier(_) | parser::ast::ExprKind::FieldAccess { .. }
            );
            if !is_borrowed {
                self.insert_release(val.into_pointer_value(), &brix_type)?;
            }
        }

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
                            span: None,
            })?;

        let then_bb = self.context.append_basic_block(function, "then_block");
        let else_bb = self.context.append_basic_block(function, "else_block");
        let merge_bb = self.context.append_basic_block(function, "merge_block");

        self.builder
            .build_conditional_branch(cond_bool, then_bb, else_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_conditional_branch".to_string(),
                details: "Failed to build if conditional branch".to_string(),
                            span: None,
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
                            span: None,
            })?
            .get_terminator()
            .is_none()
        {
            self.builder.build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_unconditional_branch".to_string(),
                    details: "Failed to build branch to merge block".to_string(),
                                    span: None,
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
                            span: None,
            })?
            .get_terminator()
            .is_none()
        {
            self.builder.build_unconditional_branch(merge_bb)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_unconditional_branch".to_string(),
                    details: "Failed to build branch to merge block".to_string(),
                                    span: None,
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
                            span: None,
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
                            span: None,
            })?;

        self.builder
            .build_conditional_branch(cond_bool, body_bb, after_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_conditional_branch".to_string(),
                details: "Failed to build while conditional branch".to_string(),
                            span: None,
            })?;

        self.builder.position_at_end(body_bb);
        self.compile_stmt(body, function)?;
        self.builder.build_unconditional_branch(header_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_unconditional_branch".to_string(),
                details: "Failed to build branch back to while header".to_string(),
                            span: None,
            })?;

        self.builder.position_at_end(after_bb);
        Ok(())
    }

    fn compile_import_stmt(&mut self, module: &str, alias: &Option<String>) -> CodegenResult<()> {
        use crate::builtins::math::MathFunctions;
        use crate::builtins::test::TestFunctions;

        let default_prefix = module.to_string();
        let prefix = alias.as_ref().unwrap_or(&default_prefix).clone();

        // Register math functions when importing math module
        if module == "math" {
            self.register_math_functions(&prefix);
        }

        // Register test library functions when importing test module
        if module == "test" {
            self.register_test_functions(&prefix);
        }

        self.imported_modules.push((module.to_string(), prefix));

        Ok(())
    }

    fn compile_return_stmt(&mut self, values: &[Expr]) -> CodegenResult<()> {
        use crate::BrixType;

        if values.is_empty() {
            // ARC: Release all ref-counted variables before explicit void return
            self.release_function_scope_vars()?;

            self.builder.build_return(None)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "Failed to build void return".to_string(),
                                    span: None,
                })?;
        } else if values.len() == 1 {
            // Single return
            let (val, _) = self.compile_expr(&values[0])?;
            self.builder.build_return(Some(&val))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "Failed to build single value return".to_string(),
                                    span: None,
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
                                            span: None,
                    })?
                    .into_struct_value();
            }

            self.builder.build_return(Some(&struct_val))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "Failed to build multiple value return".to_string(),
                                    span: None,
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
            // Resolve type aliases before processing
            let resolved_hint = if let Some(definition) = self.type_aliases.get(hint) {
                definition.clone()
            } else {
                hint.clone()
            };
            let hint = &resolved_hint;

            // Check for Union type (contains " | ")
            // Check for Intersection type (contains " & ")
            // Check for Optional type (ends with "?")
            // Otherwise, process as normal type with casting
            if hint.contains(" | ") || hint.ends_with('?') {
                let union_type = self.string_to_brix_type(hint);
                if let BrixType::Union(types) = &union_type {
                    // Find which variant of the union matches the value type
                    let mut tag = None;

                    // Try exact match first
                    for (i, t) in types.iter().enumerate() {
                        if t == &val_type {
                            tag = Some(i);
                            break;
                        }
                    }

                    // If no exact match, try with casting (int -> float)
                    if tag.is_none() {
                        for (i, t) in types.iter().enumerate() {
                            if *t == BrixType::Float && val_type == BrixType::Int {
                                // Cast int to float
                                final_val = self.builder
                                    .build_signed_int_to_float(
                                        init_val.into_int_value(),
                                        self.context.f64_type(),
                                        "cast_i2f_union",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_signed_int_to_float".to_string(),
                                        details: "Failed to cast int to float for Union".to_string(),
                                        span: None,
                                    })?
                                    .into();
                                val_type = BrixType::Float;
                                tag = Some(i);
                                break;
                            }
                        }
                    }

                    if tag.is_none() {
                        return Err(CodegenError::TypeError {
                            expected: hint.clone(),
                            found: format!("{:?}", val_type),
                            context: format!("Variable declaration '{}'", name),
                            span: None,
                        });
                    }

                    // Create tagged union: { i64 tag, value }
                    let tag_val = self.context.i64_type().const_int(tag.unwrap() as u64, false);
                    let union_llvm_type = self.brix_type_to_llvm(&union_type);
                    let struct_type = union_llvm_type.into_struct_type();
                    let mut union_val = struct_type.get_undef();

                    // Insert tag
                    union_val = self.builder.build_insert_value(union_val, tag_val, 0, "insert_tag")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_insert_value".to_string(),
                            details: "Failed to insert tag in union".to_string(),
                            span: None,
                        })?.into_struct_value();

                    // Insert value
                    union_val = self.builder.build_insert_value(union_val, final_val, 1, "insert_value")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_insert_value".to_string(),
                            details: "Failed to insert value in union".to_string(),
                            span: None,
                        })?.into_struct_value();

                    final_val = union_val.into();
                    val_type = union_type.clone();
                }
            } else {
                // Non-Union types - existing logic
                // Non-optional types - existing logic
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
                                                            span: None,
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
                                                            span: None,
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
                                                    span: None,
                        });
                    }
                }
                "error" => {
                    if val_type != BrixType::Error && val_type != BrixType::Nil {
                        return Err(CodegenError::TypeError {
                            expected: "Error or Nil".to_string(),
                            found: format!("{:?}", val_type),
                            context: format!("Variable declaration '{}'", name),
                                                    span: None,
                        });
                    }
                    // Accept both Error and Nil for error type
                    // val_type remains as-is (Error or Nil)
                }
                    _ => {
                        // Allow matrix, intmatrix, complex, and struct types
                        if hint != "matrix" && hint != "intmatrix" && hint != "complex" {
                            // Check if it's a struct, intersection, or type alias
                            if !matches!(val_type, BrixType::Struct(_) | BrixType::Intersection(_)) {
                                return Err(CodegenError::TypeError {
                                    expected: "Known type".to_string(),
                                    found: hint.clone(),
                                    context: format!("Variable declaration '{}'", name),
                                    span: None,
                                });
                            }
                        }
                    }
                }
            } // End of else block for non-optional types
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
                // Closures are represented as Tuple(Int,Int,Int) but are heap-allocated;
                // store only the pointer, not the full struct by value.
                if Compiler::is_closure_type(&BrixType::Tuple(types.clone())) {
                    self.context.ptr_type(AddressSpace::default()).into()
                } else {
                    self.brix_type_to_llvm(&BrixType::Tuple(types.clone()))
                }
            }
            BrixType::Struct(_) => {
                // Allocate space for user-defined struct
                self.brix_type_to_llvm(&val_type)
            }
            BrixType::Optional(_) => {
                // Allocate space for Optional (struct or pointer depending on inner type)
                self.brix_type_to_llvm(&val_type)
            }
            BrixType::Union(_) => {
                // Allocate space for Union (tagged union: { i64 tag, largest_type value })
                self.brix_type_to_llvm(&val_type)
            }
            BrixType::Intersection(_) => {
                // Allocate space for Intersection (merged struct)
                self.brix_type_to_llvm(&val_type)
            }
            _ => {
                return Err(CodegenError::TypeError {
                    expected: "Known type".to_string(),
                    found: format!("{:?}", val_type),
                    context: format!("Variable declaration '{}'", name),
                    span: None,
                });
            }
        };

        // ARC: Retain if value is ref-counted (except for literals which come with ref_count=1)
        // We need to retain when copying from another variable
        let should_retain = Compiler::is_ref_counted(&val_type) && !matches!(
            &value.kind,
            parser::ast::ExprKind::Literal(_) |
            parser::ast::ExprKind::Array(_) |
            parser::ast::ExprKind::Binary { .. }  // String concatenation returns new string
        );

        if should_retain {
            final_val = self.insert_retain(final_val, &val_type)?;
        }

        let alloca = if Compiler::is_ref_counted(&val_type) {
            // Ref-counted types use null-initialized alloca so that:
            // 1. Conditional declarations don't leave garbage (prevents SIGSEGV on release)
            // 2. Loop re-declarations can safely release the old value on every iteration
            let alloca = self.create_null_init_entry_block_alloca(llvm_type, name)?;

            // Release old value before overwriting. On first execution the alloca
            // holds null, so the runtime release function returns immediately (no-op).
            // On subsequent loop iterations this frees the previous allocation.
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            let old_val = self.builder
                .build_load(ptr_type, alloca, &format!("{}_old_release", name))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_load".to_string(),
                    details: format!("Failed to load old value for variable '{}' release", name),
                    span: None,
                })?
                .into_pointer_value();
            self.insert_release(old_val, &val_type)?;

            alloca
        } else {
            self.create_entry_block_alloca(llvm_type, name)?
        };

        self.builder.build_store(alloca, final_val)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: format!("Failed to store value in variable '{}'", name),
                            span: None,
            })?;

        self.variables.insert(name.to_string(), (alloca, val_type.clone()));

        // ARC: Track ref-counted variables for cleanup at function exit
        // Avoid duplicates (e.g., same variable compiled inside a loop body)
        if Compiler::is_ref_counted(&val_type) {
            if !self.function_scope_vars.iter().any(|(n, _)| n == name) {
                self.function_scope_vars.push((name.to_string(), val_type));
            }
        }

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
                                    span: None,
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
                            span: None,
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
                                    span: None,
                })?;

            // Allocate and store the variable
            let llvm_type = self.brix_type_to_llvm(field_type);
            let alloca = self.builder.build_alloca(llvm_type, name)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_alloca".to_string(),
                    details: format!("Failed to allocate variable '{}'", name),
                                    span: None,
                })?;
            self.builder.build_store(alloca, extracted)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: format!("Failed to store value in variable '{}'", name),
                                    span: None,
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

        // ARC: Release old value if it's ref-counted or a closure
        // Skip release for Union types (managed internally)
        let is_closure = if let BrixType::Tuple(ref fields) = target_type {
            fields.len() == 3 && fields[0] == BrixType::Int && fields[1] == BrixType::Int && fields[2] == BrixType::Int
        } else {
            false
        };

        if !matches!(target_type, BrixType::Union(_)) && (is_closure || Compiler::is_ref_counted(&target_type)) {
            let ptr_type = self.context.ptr_type(AddressSpace::default());
            let old_value = self.builder
                .build_load(ptr_type, target_ptr, "old_value")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_load".to_string(),
                    details: "Failed to load old value for release".to_string(),
                    span: None,
                })?
                .into_pointer_value();

            // Release the old value (closure or ref-counted type)
            if is_closure {
                self.closure_release(old_value)?;
            } else {
                self.insert_release(old_value, &target_type)?;
            }
        }

        let (val, val_type) = self.compile_expr(value)?;

        // Check if target is Union - if so, wrap value in Union
        let mut final_val = val;
        let mut final_type = val_type.clone();

        if let BrixType::Union(types) = &target_type {
            // Find which variant of the union matches the value type
            let mut tag = None;

            // Try exact match first
            for (i, t) in types.iter().enumerate() {
                if t == &val_type {
                    tag = Some(i);
                    break;
                }
            }

            // If no exact match, try with casting (int -> float)
            if tag.is_none() {
                for (i, t) in types.iter().enumerate() {
                    if *t == BrixType::Float && val_type == BrixType::Int {
                        // Cast int to float
                        final_val = self.builder
                            .build_signed_int_to_float(
                                val.into_int_value(),
                                self.context.f64_type(),
                                "cast_i2f_union_assign",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_signed_int_to_float".to_string(),
                                details: "Failed to cast int to float for Union assignment".to_string(),
                                span: None,
                            })?
                            .into();
                        final_type = BrixType::Float;
                        tag = Some(i);
                        break;
                    }
                }
            }

            if tag.is_none() {
                return Err(CodegenError::TypeError {
                    expected: format!("{:?}", target_type),
                    found: format!("{:?}", val_type),
                    context: "Union assignment".to_string(),
                    span: None,
                });
            }

            // Create tagged union: { i64 tag, value }
            let tag_val = self.context.i64_type().const_int(tag.unwrap() as u64, false);
            let union_llvm_type = self.brix_type_to_llvm(&target_type);
            let struct_type = union_llvm_type.into_struct_type();
            let mut union_val = struct_type.get_undef();

            // Insert tag
            union_val = self.builder.build_insert_value(union_val, tag_val, 0, "insert_tag")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_insert_value".to_string(),
                    details: "Failed to insert tag in union assignment".to_string(),
                    span: None,
                })?.into_struct_value();

            // Insert value
            union_val = self.builder.build_insert_value(union_val, final_val, 1, "insert_value")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_insert_value".to_string(),
                    details: "Failed to insert value in union assignment".to_string(),
                    span: None,
                })?.into_struct_value();

            final_val = union_val.into();
        } else if target_type == BrixType::Float && val_type == BrixType::Int {
            // Only cast Int→Float if the target expects Float
            final_val = self.builder
                .build_signed_int_to_float(
                    val.into_int_value(),
                    self.context.f64_type(),
                    "cast",
                )
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_signed_int_to_float".to_string(),
                    details: "Failed to cast int to float in assignment".to_string(),
                                    span: None,
                })?
                .into();
        }

        // ARC: Retain new value if ref-counted
        // Skip retain for Union types (already wrapped)
        if !matches!(target_type, BrixType::Union(_)) {
            final_val = self.insert_retain(final_val, &final_type)?;
        }

        self.builder.build_store(target_ptr, final_val)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: "Failed to store value in assignment target".to_string(),
                            span: None,
            })?;

        Ok(())
    }
}
