// Statement compilation
//
// This module contains logic for compiling Brix statements (if, while, for, etc.).

use crate::Compiler;
use crate::helpers::HelperFunctions;
use inkwell::AddressSpace;
use parser::ast::Expr;

/// Trait for statement compilation helper methods
pub trait StatementCompiler<'ctx> {
    /// Compile print statement (without newline)
    fn compile_print_stmt(&mut self, expr: &Expr);

    /// Compile println statement (with newline)
    fn compile_println_stmt(&mut self, expr: &Expr);

    /// Compile printf statement (formatted output)
    fn compile_printf_stmt(&mut self, format: &str, args: &[Expr]);

    /// Compile expression as statement (discard result)
    fn compile_expr_stmt(&mut self, expr: &Expr);

    /// Compile block statement (list of statements)
    fn compile_block_stmt(&mut self, statements: &[parser::ast::Stmt], function: inkwell::values::FunctionValue<'ctx>);

    /// Compile if statement (with optional else block)
    fn compile_if_stmt(
        &mut self,
        condition: &Expr,
        then_block: &parser::ast::Stmt,
        else_block: &Option<Box<parser::ast::Stmt>>,
        function: inkwell::values::FunctionValue<'ctx>,
    );

    /// Compile while loop
    fn compile_while_stmt(&mut self, condition: &Expr, body: &parser::ast::Stmt, function: inkwell::values::FunctionValue<'ctx>);

    /// Compile import statement
    fn compile_import_stmt(&mut self, module: &str, alias: &Option<String>);

    /// Compile return statement (single, multiple, or void)
    fn compile_return_stmt(&mut self, values: &[Expr]);

    /// Compile variable declaration with type inference and casting
    fn compile_variable_decl_stmt(&mut self, name: &str, type_hint: &Option<String>, value: &Expr);

    /// Compile destructuring declaration (tuple unpacking)
    fn compile_destructuring_decl_stmt(&mut self, names: &[String], value: &Expr);

    /// Compile assignment statement
    fn compile_assignment_stmt(&mut self, target: &Expr, value: &Expr);
}

impl<'a, 'ctx> StatementCompiler<'ctx> for Compiler<'a, 'ctx> {
    fn compile_print_stmt(&mut self, expr: &Expr) {
        if let Some((val, brix_type)) = self.compile_expr(expr) {
            // Convert value to string
            let string_val = self.value_to_string(val, &brix_type, None);

            if let Some(str_val) = string_val {
                let printf_fn = self.get_printf();
                let fmt_str = self
                    .builder
                    .build_global_string_ptr("%s", "print_fmt")
                    .unwrap();

                // Extract char* from BrixString
                let struct_ptr = str_val.into_pointer_value();
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

                self.builder
                    .build_call(
                        printf_fn,
                        &[fmt_str.as_pointer_value().into(), data_ptr.into()],
                        "call_print",
                    )
                    .unwrap();
            }
        }
    }

    fn compile_println_stmt(&mut self, expr: &Expr) {
        if let Some((val, brix_type)) = self.compile_expr(expr) {
            // Convert value to string
            let string_val = self.value_to_string(val, &brix_type, None);

            if let Some(str_val) = string_val {
                let printf_fn = self.get_printf();
                let fmt_str = self
                    .builder
                    .build_global_string_ptr("%s\n", "println_fmt")
                    .unwrap();

                // Extract char* from BrixString
                let struct_ptr = str_val.into_pointer_value();
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

                self.builder
                    .build_call(
                        printf_fn,
                        &[fmt_str.as_pointer_value().into(), data_ptr.into()],
                        "call_println",
                    )
                    .unwrap();
            }
        }
    }

    fn compile_printf_stmt(&mut self, format: &str, args: &[Expr]) {
        use crate::BrixType;

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

    fn compile_expr_stmt(&mut self, expr: &Expr) {
        self.compile_expr(expr);
    }

    fn compile_block_stmt(&mut self, statements: &[parser::ast::Stmt], function: inkwell::values::FunctionValue<'ctx>) {
        for s in statements {
            self.compile_stmt(s, function);
        }
    }

    fn compile_if_stmt(
        &mut self,
        condition: &Expr,
        then_block: &parser::ast::Stmt,
        else_block: &Option<Box<parser::ast::Stmt>>,
        function: inkwell::values::FunctionValue<'ctx>,
    ) {
        use inkwell::IntPredicate;

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
        // Only add branch if block doesn't already have a terminator (e.g., return)
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            let _ = self.builder.build_unconditional_branch(merge_bb);
        }

        // ELSE
        self.builder.position_at_end(else_bb);
        if let Some(else_stmt) = else_block {
            self.compile_stmt(else_stmt, function);
        }
        // Only add branch if block doesn't already have a terminator
        if self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_none()
        {
            let _ = self.builder.build_unconditional_branch(merge_bb);
        }

        // MERGE
        self.builder.position_at_end(merge_bb);
    }

    fn compile_while_stmt(&mut self, condition: &Expr, body: &parser::ast::Stmt, function: inkwell::values::FunctionValue<'ctx>) {
        use inkwell::IntPredicate;

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

    fn compile_import_stmt(&mut self, module: &str, alias: &Option<String>) {
        use crate::builtins::math::MathFunctions;

        // Register math functions when importing math module
        if module == "math" {
            let default_prefix = module.to_string();
            let prefix = alias.as_ref().unwrap_or(&default_prefix);
            self.register_math_functions(prefix);
        }
    }

    fn compile_return_stmt(&mut self, values: &[Expr]) {
        use crate::BrixType;

        if values.is_empty() {
            // Void return
            self.builder.build_return(None).unwrap();
        } else if values.len() == 1 {
            // Single return
            if let Some((val, _)) = self.compile_expr(&values[0]) {
                self.builder.build_return(Some(&val)).unwrap();
            }
        } else {
            // Multiple returns - create struct
            let mut compiled_values = Vec::new();
            let mut value_types = Vec::new();

            for val_expr in values {
                if let Some((val, val_type)) = self.compile_expr(val_expr) {
                    compiled_values.push(val);
                    value_types.push(val_type);
                }
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
                    .unwrap()
                    .into_struct_value();
            }

            self.builder.build_return(Some(&struct_val)).unwrap();
        }
    }

    fn compile_variable_decl_stmt(&mut self, name: &str, type_hint: &Option<String>, value: &Expr) {
        use crate::BrixType;
        use inkwell::types::BasicTypeEnum;

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
                    "error" => {
                        if val_type != BrixType::Error && val_type != BrixType::Nil {
                            eprintln!(
                                "Warning: Trying to assign incompatible type to error."
                            );
                        }
                        // Accept both Error and Nil for error type
                        // val_type remains as-is (Error or Nil)
                    }
                    _ => {
                        if hint != "matrix" && hint != "intmatrix" && hint != "complex" {
                            eprintln!(
                                "Warning: Unknown type '{}', defaulting to Int",
                                hint
                            );
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
                    eprintln!("Warning: Unknown type for allocation, using i64");
                    self.context.i64_type().into()
                }
            };

            let alloca = self.create_entry_block_alloca(llvm_type, name);
            self.builder.build_store(alloca, final_val).unwrap();

            self.variables.insert(name.to_string(), (alloca, val_type));
        }
    }

    fn compile_destructuring_decl_stmt(&mut self, names: &[String], value: &Expr) {
        use crate::BrixType;

        // Compile the expression that returns a tuple
        if let Some((tuple_val, tuple_type)) = self.compile_expr(value) {
            // Ensure it's a tuple type
            if let BrixType::Tuple(field_types) = tuple_type {
                // Check that the number of names matches the tuple size
                if names.len() != field_types.len() {
                    eprintln!(
                        "Error: Destructuring mismatch - expected {} values, got {}",
                        names.len(),
                        field_types.len()
                    );
                    return;
                }

                // Extract each field and assign to a variable
                for (i, (name, field_type)) in
                    names.iter().zip(field_types.iter()).enumerate()
                {
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
                        .unwrap();

                    // Allocate and store the variable
                    let llvm_type = self.brix_type_to_llvm(field_type);
                    let alloca = self.builder.build_alloca(llvm_type, name).unwrap();
                    self.builder.build_store(alloca, extracted).unwrap();

                    // Register in symbol table
                    self.variables
                        .insert(name.clone(), (alloca, field_type.clone()));
                }
            } else {
                eprintln!(
                    "Error: Destructuring requires a tuple, got {:?}",
                    tuple_type
                );
            }
        }
    }

    fn compile_assignment_stmt(&mut self, target: &Expr, value: &Expr) {
        use crate::BrixType;

        if let Some((target_ptr, target_type)) = self.compile_lvalue_addr(target) {
            if let Some((val, val_type)) = self.compile_expr(value) {
                // Only cast Int→Float if the target expects Float
                let final_val =
                    if target_type == BrixType::Float && val_type == BrixType::Int {
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
}
