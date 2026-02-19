use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::BasicMetadataValueEnum;
use inkwell::values::{BasicValue, BasicValueEnum, FloatValue, IntValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};
use parser::ast::{BinaryOp, Closure, Expr, ExprKind, Literal, MethodDef, Program, Stmt, StmtKind, StructDef, UnaryOp};
use std::collections::{HashMap, HashSet};

// --- MODULE DECLARATIONS ---
// These modules will be gradually populated during refactoring
mod types;
mod helpers;
mod builtins;
mod operators;
mod stmt;
mod expr;
mod error;
mod error_report;

// Re-export BrixType for public API
pub use types::BrixType;

// Re-export error types for public API
pub use error::{CodegenError, CodegenResult, Span};
pub use error_report::{report_codegen_error, report_codegen_errors};

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
    pub struct_defs: HashMap<String, Vec<(String, BrixType, Option<Expr>)>>, // Struct definitions: name -> fields
    pub struct_types: HashMap<String, inkwell::types::StructType<'ctx>>, // LLVM struct types
    pub current_function: Option<inkwell::values::FunctionValue<'ctx>>, // Track current function being compiled
    pub filename: String,    // Source filename for error reporting
    pub source: String,      // Source code for error reporting

    // Generics support
    pub generic_functions: HashMap<String, Stmt>,                       // Generic function AST (name -> body)
    pub generic_structs: HashMap<String, StructDef>,                    // Generic struct definitions
    pub generic_methods: HashMap<String, Vec<MethodDef>>,               // struct_name -> methods
    pub monomorphized_cache: HashMap<(String, Vec<String>), String>,    // (name, type_args) -> specialized_name

    // Type Aliases (v1.4)
    pub type_aliases: HashMap<String, String>,                          // type_name -> definition (e.g., "MyInt" -> "int")

    // Closures support
    pub closure_counter: usize,  // Counter for unique closure names

    // ARC scope tracking (v1.4)
    pub function_scope_vars: Vec<(String, BrixType)>,  // Variables in current function scope (for ARC release)
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub fn new(
        context: &'ctx Context,
        builder: &'a Builder<'ctx>,
        module: &'a Module<'ctx>,
        filename: String,
        source: String,
    ) -> Self {
        Self {
            context,
            builder,
            module,
            variables: HashMap::new(),
            functions: HashMap::new(),
            function_params: HashMap::new(),
            struct_defs: HashMap::new(),
            struct_types: HashMap::new(),
            current_function: None,
            filename,
            source,
            generic_functions: HashMap::new(),
            generic_structs: HashMap::new(),
            generic_methods: HashMap::new(),
            monomorphized_cache: HashMap::new(),
            type_aliases: HashMap::new(),
            closure_counter: 0,
            function_scope_vars: Vec::new(),
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
        // Step 1: Resolve type aliases
        let resolved_type_str = if let Some(definition) = self.type_aliases.get(type_str) {
            definition.as_str()
        } else {
            type_str
        };

        // Step 2: Check for Union type (contains " | ")
        if resolved_type_str.contains(" | ") {
            let types: Vec<BrixType> = resolved_type_str
                .split(" | ")
                .map(|s| self.string_to_brix_type(s.trim()))
                .collect();
            return BrixType::Union(types);
        }

        // Step 3: Check for Intersection type (contains " & ")
        if resolved_type_str.contains(" & ") {
            let types: Vec<BrixType> = resolved_type_str
                .split(" & ")
                .map(|s| self.string_to_brix_type(s.trim()))
                .collect();
            return BrixType::Intersection(types);
        }

        // Step 4: Check for Optional type (suffix "?")
        // Optional is now syntactic sugar for Union(T, nil)
        if let Some(base_type_str) = resolved_type_str.strip_suffix('?') {
            let inner_type = self.string_to_brix_type(base_type_str);
            return BrixType::Union(vec![inner_type, BrixType::Nil]);
        }

        // Step 5: Base types
        match resolved_type_str {
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
                // Check if it's a struct type
                if self.struct_defs.contains_key(resolved_type_str) {
                    BrixType::Struct(resolved_type_str.to_string())
                } else {
                    eprintln!("Warning: Unknown type '{}', defaulting to Int", resolved_type_str);
                    BrixType::Int
                }
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
            BrixType::Struct(name) => {
                // Return the LLVM struct type stored in struct_types
                self.struct_types.get(name)
                    .expect(&format!("Struct type '{}' not found", name))
                    .as_basic_type_enum()
            }
            BrixType::Union(types) => {
                // Union type (tagged union): struct { i64 tag, largest_type value }
                // Tag indicates which variant is active (0, 1, 2, ...)
                // Value holds the largest type to accommodate all variants

                // Find largest type for union value field
                let mut max_size = 0;
                let mut max_type = self.context.i64_type().as_basic_type_enum();

                for t in types {
                    let llvm_type = self.brix_type_to_llvm(t);
                    let size = llvm_type.size_of().unwrap().get_zero_extended_constant().unwrap_or(8);
                    if size > max_size {
                        max_size = size;
                        max_type = llvm_type;
                    }
                }

                // Tagged union: { i64 tag, max_type value }
                let i64_type = self.context.i64_type();
                self.context.struct_type(&[i64_type.into(), max_type], false).into()
            }
            BrixType::Intersection(types) => {
                // Intersection type: merge all struct fields into one struct
                // For now, just create a struct with all fields from all types
                let mut all_fields: Vec<BasicTypeEnum> = Vec::new();

                for t in types {
                    match t {
                        BrixType::Struct(name) => {
                            // Get struct fields and add to merged fields
                            if let Some(fields) = self.struct_defs.get(name) {
                                for (_, field_type, _) in fields {
                                    all_fields.push(self.brix_type_to_llvm(field_type));
                                }
                            }
                        }
                        _ => {
                            // For non-struct types, just add the type itself
                            all_fields.push(self.brix_type_to_llvm(t));
                        }
                    }
                }

                self.context.struct_type(&all_fields, false).into()
            }
            BrixType::Optional(_) => {
                // Optional is now syntactic sugar for Union(T, nil)
                // This case should never be reached
                panic!("Optional type should have been converted to Union")
            }
        }
    }

    // --- FUNCTION DEFINITION ---
    fn compile_function_def(
        &mut self,
        name: &str,
        type_params: &[parser::ast::TypeParam],
        params: &[(String, String, Option<Expr>)],
        return_type: &Option<Vec<String>>,
        body: &Stmt,
        stmt: &Stmt,  // Full statement for storing generics
        _parent_function: inkwell::values::FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        // Handle generic functions - store for later monomorphization
        if !type_params.is_empty() {
            self.generic_functions.insert(name.to_string(), stmt.clone());
            return Ok(());  // Don't compile yet
        }

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
        let saved_scope_vars = self.function_scope_vars.clone();
        self.current_function = Some(llvm_function);

        // ARC: Clear function scope vars for new function
        self.function_scope_vars.clear();

        // 7. Create allocas for parameters and store them
        for (i, (param_name, param_type_str, _default)) in params.iter().enumerate() {
            let param_value = llvm_function.get_nth_param(i as u32)
                .ok_or_else(|| CodegenError::LLVMError {
                    operation: "get_nth_param".to_string(),
                    details: format!("Failed to get parameter {} in function definition", i),
                                    span: None,
                })?;
            let param_type = self.string_to_brix_type(param_type_str);
            let llvm_type = self.brix_type_to_llvm(&param_type);

            let alloca = self.create_entry_block_alloca(llvm_type, param_name)?;
            self.builder.build_store(alloca, param_value)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: format!("Failed to store parameter '{}'", param_name),
                                    span: None,
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
                    // ARC: Release all ref-counted variables before implicit void return
                    self.release_function_scope_vars()?;

                    self.builder.build_return(None)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "Failed to build implicit void return".to_string(),
                                                    span: None,
                        })?;
                }
            }
        }

        // 10. Restore state
        self.variables = saved_vars;
        self.function_scope_vars = saved_scope_vars;
        self.current_function = Some(_parent_function);

        // 11. Position builder back at the end of parent function
        if let Some(block) = _parent_function.get_last_basic_block() {
            self.builder.position_at_end(block);
        }

        Ok(())
    }

    // --- STRUCT DEFINITION ---
    fn compile_struct_def(
        &mut self,
        name: &str,
        type_params: &[parser::ast::TypeParam],
        fields: &[(String, String, Option<Expr>)],
        struct_def: &StructDef,  // Full struct definition for storing generics
    ) -> CodegenResult<()> {
        // Handle generic structs - store for later monomorphization
        if !type_params.is_empty() {
            self.generic_structs.insert(name.to_string(), struct_def.clone());
            return Ok(());  // Don't compile yet
        }

        // 1. Parse field types
        let field_metadata: Vec<(String, BrixType, Option<Expr>)> = fields
            .iter()
            .map(|(field_name, field_type, default)| {
                (field_name.clone(), self.string_to_brix_type(field_type), default.clone())
            })
            .collect();

        // 2. Create named LLVM struct type
        let struct_type = self.context.opaque_struct_type(name);

        // 3. Define struct body with field types
        let field_llvm_types: Vec<BasicTypeEnum> = field_metadata
            .iter()
            .map(|(_, brix_type, _)| self.brix_type_to_llvm(brix_type))
            .collect();

        struct_type.set_body(&field_llvm_types, false);

        // 4. Store in registries
        self.struct_defs.insert(name.to_string(), field_metadata);
        self.struct_types.insert(name.to_string(), struct_type);

        Ok(())
    }

    // --- METHOD DEFINITION ---
    fn compile_method_def(
        &mut self,
        receiver_name: &str,
        receiver_type: &str,
        method_name: &str,
        params: &[(String, String, Option<Expr>)],
        return_type: &Option<Vec<String>>,
        body: &Stmt,
        _parent_function: inkwell::values::FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        // Check if receiver type is a generic struct
        if self.generic_structs.contains_key(receiver_type) {
            // Store method for later monomorphization
            let method_def = MethodDef {
                receiver_name: receiver_name.to_string(),
                receiver_type: receiver_type.to_string(),
                method_name: method_name.to_string(),
                params: params.to_vec(),
                return_type: return_type.clone(),
                body: Box::new(body.clone()),
            };

            self.generic_methods
                .entry(receiver_type.to_string())
                .or_insert_with(Vec::new)
                .push(method_def);

            return Ok(());  // Don't compile yet
        }

        // Mangle method name: StructName_methodname
        let mangled_name = format!("{}_{}", receiver_type, method_name);

        // 1. Parse return type
        let ret_types: Vec<BrixType> = match return_type {
            None => vec![],
            Some(types) => types.iter().map(|t| self.string_to_brix_type(t)).collect(),
        };

        // 2. Create parameter types
        // First parameter is receiver (pointer to struct)
        let receiver_brix_type = self.string_to_brix_type(receiver_type);
        let _receiver_struct_type = match &receiver_brix_type {
            BrixType::Struct(name) => self.struct_types.get(name).ok_or_else(|| {
                CodegenError::UndefinedSymbol {
                    name: name.clone(),
                    context: "method receiver type".to_string(),
                    span: None,
                }
            })?,
            _ => {
                return Err(CodegenError::TypeError {
                    expected: "Struct type".to_string(),
                    found: format!("{:?}", receiver_brix_type),
                    context: "method receiver".to_string(),
                    span: None,
                });
            }
        };

        // Build parameter types: receiver pointer + other params
        let mut param_types: Vec<BasicMetadataTypeEnum> = vec![self.context.ptr_type(AddressSpace::default()).into()];
        for (_, param_type_str, _) in params {
            let param_type = self.string_to_brix_type(param_type_str);
            param_types.push(self.brix_type_to_llvm(&param_type).into());
        }

        // 3. Create function type
        let fn_type = if ret_types.is_empty() {
            self.context.void_type().fn_type(&param_types, false)
        } else if ret_types.len() == 1 {
            let ret_llvm = self.brix_type_to_llvm(&ret_types[0]);
            ret_llvm.fn_type(&param_types, false)
        } else {
            let tuple_type = BrixType::Tuple(ret_types.clone());
            let ret_llvm = self.brix_type_to_llvm(&tuple_type);
            ret_llvm.fn_type(&param_types, false)
        };

        // 4. Create the function
        let llvm_function = self.module.add_function(&mangled_name, fn_type, None);

        // 5. Store function in registry
        self.functions.insert(mangled_name.clone(), (llvm_function, Some(ret_types.clone())));

        // 6. Create entry block
        let entry_block = self.context.append_basic_block(llvm_function, "entry");
        self.builder.position_at_end(entry_block);

        // 7. Save current state
        let saved_vars = self.variables.clone();
        self.current_function = Some(llvm_function);

        // 8. Store receiver parameter (as pointer - no alloca needed)
        let receiver_param = llvm_function.get_nth_param(0).ok_or_else(|| {
            CodegenError::LLVMError {
                operation: "get_nth_param".to_string(),
                details: "Failed to get receiver parameter".to_string(),
                span: None,
            }
        })?;
        // Store receiver pointer directly in symbol table
        self.variables.insert(
            receiver_name.to_string(),
            (receiver_param.into_pointer_value(), receiver_brix_type.clone()),
        );

        // 9. Store other parameters
        for (i, (param_name, param_type_str, _)) in params.iter().enumerate() {
            let param_value = llvm_function.get_nth_param((i + 1) as u32).ok_or_else(|| {
                CodegenError::LLVMError {
                    operation: "get_nth_param".to_string(),
                    details: format!("Failed to get parameter {}", i + 1),
                    span: None,
                }
            })?;
            let param_type = self.string_to_brix_type(param_type_str);
            let llvm_type = self.brix_type_to_llvm(&param_type);

            let alloca = self.create_entry_block_alloca(llvm_type, param_name)?;
            self.builder.build_store(alloca, param_value).map_err(|_| {
                CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: format!("Failed to store parameter '{}'", param_name),
                    span: None,
                }
            })?;
            self.variables.insert(param_name.clone(), (alloca, param_type));
        }

        // 10. Compile body
        self.compile_stmt(body, llvm_function)?;

        // 11. Add implicit return for void functions if missing
        if ret_types.is_empty() {
            if let Some(block) = self.builder.get_insert_block() {
                if block.get_terminator().is_none() {
                    self.builder.build_return(None).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "Failed to build implicit void return".to_string(),
                            span: None,
                        }
                    })?;
                }
            }
        }

        // 12. Restore state
        self.variables = saved_vars;
        self.current_function = Some(_parent_function);

        // 13. Position builder back
        if let Some(block) = _parent_function.get_last_basic_block() {
            self.builder.position_at_end(block);
        }

        Ok(())
    }

    // --- STRUCT INITIALIZATION ---
    fn compile_struct_init(
        &mut self,
        struct_name: &str,
        type_args: &[String],
        field_inits: &[(String, Expr)],
        _expr: &Expr,
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        // Determine the actual struct name to use (mangled if generic)
        let actual_struct_name = if !type_args.is_empty() {
            // Generic struct - monomorphize it
            self.monomorphize_struct(struct_name, type_args)?
        } else {
            // Non-generic struct - use as-is
            struct_name.to_string()
        };

        // 1. Get struct definition
        let struct_def = self.struct_defs.get(&actual_struct_name)
            .ok_or_else(|| CodegenError::UndefinedSymbol {
                name: actual_struct_name.clone(),
                context: "struct initialization".to_string(),
                span: None,
            })?
            .clone();

        let struct_type = *self.struct_types.get(&actual_struct_name)
            .ok_or_else(|| CodegenError::General(format!("Struct type '{}' not found", actual_struct_name)))?;

        // 2. Create field value map from field_inits
        let mut field_values: HashMap<String, BasicValueEnum> = HashMap::new();
        for (field_name, field_expr) in field_inits {
            let (value, _) = self.compile_expr(field_expr)?;
            field_values.insert(field_name.clone(), value);
        }

        // 3. Build struct value with all fields (like tuples)
        let mut struct_val = struct_type.get_undef();
        for (i, (field_name, _field_type, default)) in struct_def.iter().enumerate() {
            let value = if let Some(val) = field_values.get(field_name) {
                // Use provided value
                *val
            } else if let Some(default_expr) = default {
                // Use default value
                let (val, _) = self.compile_expr(default_expr)?;
                val
            } else {
                return Err(CodegenError::InvalidOperation {
                    operation: format!("struct initialization for '{}'", actual_struct_name),
                    reason: format!("field '{}' has no value and no default", field_name),
                    span: None,
                });
            };

            // Insert the value into the struct
            struct_val = self.builder
                .build_insert_value(struct_val, value, i as u32, "field")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_insert_value".to_string(),
                    details: format!("Failed to insert field '{}'", field_name),
                    span: None,
                })?
                .into_struct_value();
        }

        // Return struct value
        Ok((struct_val.into(), BrixType::Struct(actual_struct_name)))
    }

    // --- CLOSURES ---

    /// Compile a closure expression
    ///
    /// Creates:
    /// 1. Environment struct with captured variables (as pointers)
    /// 2. Closure function that receives env_ptr as first parameter
    /// 3. Returns a struct { fn_ptr, env_ptr }
    fn compile_closure(
        &mut self,
        closure: &Closure,
        expr: &Expr,
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        // Generate unique closure name
        let closure_name = format!("__closure_{}", self.closure_counter);
        self.closure_counter += 1;

        // 1. Create environment struct type if there are captured variables
        let env_struct_type = if !closure.captured_vars.is_empty() {
            let mut field_types = Vec::new();
            for _var in &closure.captured_vars {
                // All captured variables are stored as pointers (i8*)
                field_types.push(self.context.ptr_type(AddressSpace::default()).into());
            }
            let env_type = self.context.struct_type(&field_types, false);
            Some(env_type)
        } else {
            None
        };

        // 2. Build closure function signature
        // Function takes: (env_ptr, param1, param2, ...) -> return_type
        let mut param_types: Vec<BasicMetadataTypeEnum> = Vec::new();

        // First parameter is always env_ptr (even if empty, for consistency)
        param_types.push(self.context.ptr_type(AddressSpace::default()).into());

        // Add user parameters
        let mut param_brix_types = Vec::new();
        for (_param_name, param_type_str) in &closure.params {
            let brix_type = self.string_to_brix_type(param_type_str);
            param_brix_types.push(brix_type.clone());
            param_types.push(self.brix_type_to_llvm(&brix_type).into());
        }

        // Return type
        let return_brix_type = if let Some(ret_type_str) = &closure.return_type {
            self.string_to_brix_type(ret_type_str)
        } else {
            BrixType::Void
        };

        let fn_type = if return_brix_type == BrixType::Void {
            self.context.void_type().fn_type(&param_types, false)
        } else {
            self.brix_type_to_llvm(&return_brix_type).fn_type(&param_types, false)
        };

        // 3. Create the closure function
        let closure_fn = self.module.add_function(&closure_name, fn_type, None);

        // Save current function and switch to closure function
        let prev_function = self.current_function;
        self.current_function = Some(closure_fn);

        // Save current variable scope
        let prev_variables = self.variables.clone();

        // Create entry block for closure function
        let entry = self.context.append_basic_block(closure_fn, "entry");
        self.builder.position_at_end(entry);

        // 4. Set up parameters in the closure function
        let env_ptr = closure_fn.get_nth_param(0).unwrap().into_pointer_value();

        // Load captured variables from environment
        if let Some(env_type) = env_struct_type {
            for (i, var_name) in closure.captured_vars.iter().enumerate() {
                // Get pointer to field in environment struct
                let field_ptr = self.builder
                    .build_struct_gep(env_type, env_ptr, i as u32, &format!("{}_ptr", var_name))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: format!("Failed to access captured variable {}", var_name),
                        span: Some(expr.span.clone()),
                    })?;

                // Load the actual variable pointer
                let var_ptr = self.builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        field_ptr,
                        var_name
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: format!("Failed to load captured variable {}", var_name),
                        span: Some(expr.span.clone()),
                    })?
                    .into_pointer_value();

                // Get the type of the captured variable from outer scope
                let var_type = prev_variables.get(var_name)
                    .map(|(_, t)| t.clone())
                    .ok_or_else(|| CodegenError::UndefinedSymbol {
                        name: var_name.clone(),
                        context: "closure captured variable".to_string(),
                        span: Some(expr.span.clone()),
                    })?;

                // Add to current scope
                self.variables.insert(var_name.clone(), (var_ptr, var_type));
            }
        }

        // Add closure parameters to scope
        for (i, (param_name, _param_type)) in closure.params.iter().enumerate() {
            let param_val = closure_fn.get_nth_param((i + 1) as u32).unwrap(); // +1 for env_ptr
            let param_type = &param_brix_types[i];

            // Allocate space for parameter and store it
            let param_llvm_type = self.brix_type_to_llvm(param_type);
            let param_ptr = self.create_entry_block_alloca(param_llvm_type, param_name)?;
            self.builder.build_store(param_ptr, param_val)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: format!("Failed to store parameter {}", param_name),
                    span: Some(expr.span.clone()),
                })?;

            self.variables.insert(param_name.clone(), (param_ptr, param_type.clone()));
        }

        // 5. Compile closure body
        self.compile_stmt(&closure.body, closure_fn)?;

        // If no return was emitted and return type is void, add ret void
        if return_brix_type == BrixType::Void {
            let current_block = self.builder.get_insert_block().unwrap();
            if current_block.get_terminator().is_none() {
                self.builder.build_return(None)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_return".to_string(),
                        details: "Failed to build return for void closure".to_string(),
                        span: Some(expr.span.clone()),
                    })?;
            }
        }

        // Restore previous function and variables
        self.current_function = prev_function;
        self.variables = prev_variables;

        // Position builder back in original function
        if let Some(prev_fn) = prev_function {
            if let Some(bb) = prev_fn.get_last_basic_block() {
                self.builder.position_at_end(bb);
            }
        }

        // 6. Allocate environment and store captured variable pointers
        let env_ptr_in_caller = if !closure.captured_vars.is_empty() {
            let env_type = env_struct_type.unwrap();

            // Allocate environment on HEAP (not stack) so closures can be returned from functions
            // Declare brix_malloc: void* brix_malloc(size_t size)
            let i64_type = self.context.i64_type();
            let ptr_type = self.context.ptr_type(AddressSpace::default());
            let malloc_fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            let malloc_fn = self.module.get_function("brix_malloc").unwrap_or_else(|| {
                self.module.add_function("brix_malloc", malloc_fn_type, Some(Linkage::External))
            });

            // Calculate size of environment struct
            let env_size = env_type.size_of().ok_or_else(|| CodegenError::LLVMError {
                operation: "size_of".to_string(),
                details: "Failed to get size of environment struct".to_string(),
                span: Some(expr.span.clone()),
            })?;

            // Call brix_malloc(size)
            let malloc_call = self.builder
                .build_call(malloc_fn, &[env_size.into()], "env_malloc")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "Failed to call brix_malloc".to_string(),
                    span: Some(expr.span.clone()),
                })?;

            let env_ptr_raw = malloc_call.try_as_basic_value().left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "brix_malloc result".to_string(),
                    context: "closure environment allocation".to_string(),
                    span: Some(expr.span.clone()),
                })?
                .into_pointer_value();

            // Cast i8* to env_type*
            let env_alloca = env_ptr_raw; // No cast needed - LLVM treats all pointers uniformly

            // Store pointers to captured variables
            for (i, var_name) in closure.captured_vars.iter().enumerate() {
                let (var_ptr, _) = self.variables.get(var_name)
                    .ok_or_else(|| CodegenError::UndefinedSymbol {
                        name: var_name.clone(),
                        context: "closure environment".to_string(),
                        span: Some(expr.span.clone()),
                    })?;

                let field_ptr = self.builder
                    .build_struct_gep(env_type, env_alloca, i as u32, &format!("{}_field", var_name))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: format!("Failed to get field pointer for {}", var_name),
                        span: Some(expr.span.clone()),
                    })?;

                self.builder.build_store(field_ptr, *var_ptr)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: format!("Failed to store pointer for {}", var_name),
                        span: Some(expr.span.clone()),
                    })?;
            }

            env_alloca
        } else {
            // No captured variables, use null pointer
            self.context.ptr_type(AddressSpace::default()).const_null()
        };

        // 7. Create closure struct { ref_count, fn_ptr, env_ptr } on HEAP
        // ARC: ref_count tracks how many references exist to this closure
        let closure_struct_type = self.context.struct_type(&[
            self.context.i64_type().into(),                         // ref_count
            self.context.ptr_type(AddressSpace::default()).into(), // fn_ptr
            self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
        ], false);

        // Allocate closure struct on heap (for ARC to work)
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let malloc_fn_type = ptr_type.fn_type(&[i64_type.into()], false);
        let malloc_fn = self.module.get_function("brix_malloc").unwrap_or_else(|| {
            self.module.add_function("brix_malloc", malloc_fn_type, Some(Linkage::External))
        });

        let closure_size = closure_struct_type.size_of().ok_or_else(|| CodegenError::LLVMError {
            operation: "size_of".to_string(),
            details: "Failed to get size of closure struct".to_string(),
            span: Some(expr.span.clone()),
        })?;

        let closure_malloc = self.builder
            .build_call(malloc_fn, &[closure_size.into()], "closure_malloc")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call brix_malloc for closure".to_string(),
                span: Some(expr.span.clone()),
            })?;

        let closure_ptr = closure_malloc.try_as_basic_value().left()
            .ok_or_else(|| CodegenError::MissingValue {
                what: "brix_malloc result for closure".to_string(),
                context: "closure struct allocation".to_string(),
                span: Some(expr.span.clone()),
            })?
            .into_pointer_value();

        // Store ref_count = 1 (initial reference)
        let ref_count_field = self.builder
            .build_struct_gep(closure_struct_type, closure_ptr, 0, "ref_count_field")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get ref_count field".to_string(),
                span: Some(expr.span.clone()),
            })?;

        let one = self.context.i64_type().const_int(1, false);
        self.builder.build_store(ref_count_field, one)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: "Failed to store initial ref_count".to_string(),
                span: Some(expr.span.clone()),
            })?;

        // Store function pointer (field 1)
        let fn_ptr_field = self.builder
            .build_struct_gep(closure_struct_type, closure_ptr, 1, "fn_ptr_field")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get fn_ptr field".to_string(),
                span: Some(expr.span.clone()),
            })?;

        let fn_ptr = closure_fn.as_global_value().as_pointer_value();
        self.builder.build_store(fn_ptr_field, fn_ptr)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: "Failed to store function pointer".to_string(),
                span: Some(expr.span.clone()),
            })?;

        // Store environment pointer (field 2)
        let env_ptr_field = self.builder
            .build_struct_gep(closure_struct_type, closure_ptr, 2, "env_ptr_field")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get env_ptr field".to_string(),
                span: Some(expr.span.clone()),
            })?;

        self.builder.build_store(env_ptr_field, env_ptr_in_caller)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: "Failed to store environment pointer".to_string(),
                span: Some(expr.span.clone()),
            })?;

        // Return the closure pointer directly (not loaded - we want the pointer)
        let closure_val = closure_ptr.into();

        // Return closure as a special type
        // For now, we'll use a Tuple type to represent closure with ARC
        Ok((closure_val, BrixType::Tuple(vec![
            BrixType::Int, // ref_count
            BrixType::Int, // fn_ptr
            BrixType::Int, // env_ptr
        ])))
    }

    // --- ARC: AUTOMATIC REFERENCE COUNTING FOR CLOSURES ---

    /// Call closure_retain() to increment ref_count
    /// Returns the same closure pointer
    fn closure_retain(
        &self,
        closure_ptr: PointerValue<'ctx>,
    ) -> CodegenResult<PointerValue<'ctx>> {
        // Declare closure_retain: void* closure_retain(void* closure_ptr)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let retain_fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        let retain_fn = self.module.get_function("closure_retain").unwrap_or_else(|| {
            self.module.add_function("closure_retain", retain_fn_type, Some(Linkage::External))
        });

        // Call closure_retain(closure_ptr)
        let result = self.builder
            .build_call(retain_fn, &[closure_ptr.into()], "retain_call")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call closure_retain".to_string(),
                span: None,
            })?;

        let retained_ptr = result.try_as_basic_value().left()
            .ok_or_else(|| CodegenError::MissingValue {
                what: "closure_retain result".to_string(),
                context: "ARC retain".to_string(),
                span: None,
            })?
            .into_pointer_value();

        Ok(retained_ptr)
    }

    /// Call closure_release() to decrement ref_count and free if zero
    fn closure_release(
        &self,
        closure_ptr: PointerValue<'ctx>,
    ) -> CodegenResult<()> {
        // Declare closure_release: void closure_release(void* closure_ptr)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let void_type = self.context.void_type();
        let release_fn_type = void_type.fn_type(&[ptr_type.into()], false);
        let release_fn = self.module.get_function("closure_release").unwrap_or_else(|| {
            self.module.add_function("closure_release", release_fn_type, Some(Linkage::External))
        });

        // Call closure_release(closure_ptr)
        self.builder
            .build_call(release_fn, &[closure_ptr.into()], "release_call")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call closure_release".to_string(),
                span: None,
            })?;

        Ok(())
    }

    // --- ARC FOR HEAP TYPES (v1.4) ---

    /// Check if a type is reference-counted (needs retain/release)
    fn is_ref_counted(brix_type: &BrixType) -> bool {
        matches!(
            brix_type,
            BrixType::String | BrixType::Matrix | BrixType::IntMatrix | BrixType::ComplexMatrix
        )
    }

    // Optional types are now implemented as Union(T, nil)
    // No separate Optional helpers needed

    /// Check if a print/println expression produces a temporary BrixString
    /// that should be released after printing.
    ///
    /// Returns true when value_to_string created a new allocation OR when
    /// compile_expr produced a temporary string (literal, f-string, concat, etc.).
    /// Returns false for variable references and field accesses (borrowed pointers).
    pub fn is_print_temp(brix_type: &BrixType, expr_kind: &parser::ast::ExprKind) -> bool {
        use parser::ast::ExprKind;
        match brix_type {
            // Non-String types: value_to_string always allocates a new BrixString
            BrixType::String => {
                // For String type, only Identifier and FieldAccess are "borrowed"
                !matches!(expr_kind, ExprKind::Identifier(_) | ExprKind::FieldAccess { .. })
            }
            // Non-string types: value_to_string creates a temp BrixString
            _ => true,
        }
    }

    /// Insert retain() call for ref-counted types
    /// Returns the same pointer (retains are transparent)
    fn insert_retain(
        &self,
        value: BasicValueEnum<'ctx>,
        brix_type: &BrixType,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        if !Self::is_ref_counted(brix_type) {
            return Ok(value); // No retain needed
        }

        let ptr_value = value.into_pointer_value();
        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // Determine which retain function to call
        let fn_name = match brix_type {
            BrixType::String => "string_retain",
            BrixType::Matrix => "matrix_retain",
            BrixType::IntMatrix => "intmatrix_retain",
            BrixType::ComplexMatrix => "complexmatrix_retain",
            _ => unreachable!("is_ref_counted should have filtered this"),
        };

        // Declare: void* xxx_retain(void* ptr)
        let retain_fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        let retain_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
            self.module.add_function(fn_name, retain_fn_type, Some(Linkage::External))
        });

        // Call xxx_retain(ptr) - returns same ptr
        let retained_ptr = self.builder
            .build_call(retain_fn, &[ptr_value.into()], "retain_call")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: format!("Failed to call {}", fn_name),
                span: None,
            })?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "try_as_basic_value".to_string(),
                details: format!("{} should return a pointer", fn_name),
                span: None,
            })?;

        Ok(retained_ptr)
    }

    /// Insert release() call for ref-counted types
    fn insert_release(
        &self,
        ptr_value: PointerValue<'ctx>,
        brix_type: &BrixType,
    ) -> CodegenResult<()> {
        if !Self::is_ref_counted(brix_type) {
            return Ok(()); // No release needed
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // Determine which release function to call
        let fn_name = match brix_type {
            BrixType::String => "string_release",
            BrixType::Matrix => "matrix_release",
            BrixType::IntMatrix => "intmatrix_release",
            BrixType::ComplexMatrix => "complexmatrix_release",
            _ => unreachable!("is_ref_counted should have filtered this"),
        };

        // Declare: void xxx_release(void* ptr)
        let void_type = self.context.void_type();
        let release_fn_type = void_type.fn_type(&[ptr_type.into()], false);
        let release_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
            self.module.add_function(fn_name, release_fn_type, Some(Linkage::External))
        });

        // Call xxx_release(ptr)
        self.builder
            .build_call(release_fn, &[ptr_value.into()], "release_call")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: format!("Failed to call {}", fn_name),
                span: None,
            })?;

        Ok(())
    }

    /// Release all ref-counted variables in current function scope.
    /// Called at function exit (for void functions) or before return.
    ///
    /// Clones function_scope_vars to avoid borrow conflicts with insert_release.
    /// Deduplicates by variable name to prevent double-release when the same
    /// variable is tracked multiple times (e.g. loop re-declarations).
    fn release_function_scope_vars(&mut self) -> CodegenResult<()> {
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let scope_vars = self.function_scope_vars.clone();
        let mut released = HashSet::new();

        for (var_name, var_type) in &scope_vars {
            if !released.insert(var_name.clone()) {
                continue;
            }
            if let Some((var_ptr, _)) = self.variables.get(var_name) {
                let value = self.builder
                    .build_load(ptr_type, *var_ptr, &format!("{}_release_load", var_name))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: format!("Failed to load variable '{}' for release", var_name),
                        span: None,
                    })?
                    .into_pointer_value();

                self.insert_release(value, var_type)?;
            }
        }

        Ok(())
    }

    // --- GENERICS: MONOMORPHIZATION ---

    /// Generate mangled name for a specialized generic function
    /// Example: swap<int, float> -> "swap_int_float"
    fn mangle_generic_name(&self, base_name: &str, type_args: &[String]) -> String {
        format!("{}_{}", base_name, type_args.join("_"))
    }

    /// Substitute type parameters in a type string
    /// Example: T -> int, U -> float in "T" returns "int"
    fn substitute_type(
        &self,
        type_str: &str,
        type_params: &[parser::ast::TypeParam],
        type_args: &[String],
    ) -> String {
        // Create substitution map
        let mut subst_map: HashMap<String, String> = HashMap::new();
        for (param, arg) in type_params.iter().zip(type_args.iter()) {
            subst_map.insert(param.name.clone(), arg.clone());
        }

        // Apply substitution
        subst_map.get(type_str).cloned().unwrap_or_else(|| type_str.to_string())
    }

    /// Substitute type parameters in function parameters
    fn substitute_params(
        &self,
        params: &[(String, String, Option<Expr>)],
        type_params: &[parser::ast::TypeParam],
        type_args: &[String],
    ) -> Vec<(String, String, Option<Expr>)> {
        params
            .iter()
            .map(|(name, type_str, default)| {
                let new_type = self.substitute_type(type_str, type_params, type_args);
                (name.clone(), new_type, default.clone())
            })
            .collect()
    }

    /// Substitute type parameters in return type
    fn substitute_return_type(
        &self,
        return_type: &Option<Vec<String>>,
        type_params: &[parser::ast::TypeParam],
        type_args: &[String],
    ) -> Option<Vec<String>> {
        return_type.as_ref().map(|types| {
            types
                .iter()
                .map(|t| self.substitute_type(t, type_params, type_args))
                .collect()
        })
    }

    /// Infer type arguments from function call arguments
    /// Example: add(1, 2.5) with add<T>(a: T, b: T) -> infers T = float (promotion)
    fn infer_generic_types(
        &mut self,
        func_name: &str,
        args: &[Expr],
    ) -> CodegenResult<Vec<String>> {
        // Get generic function definition
        let generic_stmt = self.generic_functions.get(func_name)
            .ok_or_else(|| CodegenError::UndefinedSymbol {
                name: func_name.to_string(),
                context: "generic type inference".to_string(),
                span: None,
            })?
            .clone();

        // Extract type parameters and params
        let (type_params, params) = match &generic_stmt.kind {
            StmtKind::FunctionDef {
                type_params,
                params,
                ..
            } => (type_params, params),
            _ => return Err(CodegenError::General(
                format!("Stored generic '{}' is not a function", func_name)
            )),
        };

        // Compile arguments to get their types
        let mut arg_types: Vec<BrixType> = Vec::new();
        for arg in args {
            let (_, arg_type) = self.compile_expr(arg)?;
            arg_types.push(arg_type);
        }

        // Build substitution map: type_param -> concrete_type
        let mut subst_map: HashMap<String, String> = HashMap::new();

        // Match each argument type against parameter type
        for (i, arg_type) in arg_types.iter().enumerate() {
            if i >= params.len() {
                break;  // More args than params - will error later
            }

            let (_param_name, param_type_str, _default) = &params[i];

            // Convert BrixType to string for comparison
            let arg_type_str = match arg_type {
                BrixType::Int => "int",
                BrixType::Float => "float",
                BrixType::String => "string",
                BrixType::Matrix => "matrix",
                BrixType::IntMatrix => "intmatrix",
                _ => "unknown",
            }.to_string();

            // If param type is a type parameter, record the substitution
            if type_params.iter().any(|tp| &tp.name == param_type_str) {
                // Check if we already have a substitution for this type param
                if let Some(existing) = subst_map.get(param_type_str) {
                    // If types don't match, apply promotion rules
                    if existing != &arg_type_str {
                        // Int + Float = Float (promote to float)
                        if (existing == "int" && arg_type_str == "float") ||
                           (existing == "float" && arg_type_str == "int") {
                            subst_map.insert(param_type_str.clone(), "float".to_string());
                        }
                    }
                } else {
                    subst_map.insert(param_type_str.clone(), arg_type_str);
                }
            }
        }

        // Build final type_args vector in the order of type_params
        let mut type_args = Vec::new();
        for type_param in type_params {
            if let Some(concrete_type) = subst_map.get(&type_param.name) {
                type_args.push(concrete_type.clone());
            } else {
                return Err(CodegenError::General(
                    format!("Could not infer type for type parameter '{}'", type_param.name)
                ));
            }
        }

        Ok(type_args)
    }

    /// Monomorphize and compile a generic function
    fn monomorphize_function(
        &mut self,
        func_name: &str,
        type_args: &[String],
        parent_function: inkwell::values::FunctionValue<'ctx>,
    ) -> CodegenResult<String> {
        // Check cache first
        let cache_key = (func_name.to_string(), type_args.to_vec());
        if let Some(mangled_name) = self.monomorphized_cache.get(&cache_key) {
            return Ok(mangled_name.clone());
        }

        // Get generic function definition
        let generic_stmt = self.generic_functions.get(func_name)
            .ok_or_else(|| CodegenError::UndefinedSymbol {
                name: func_name.to_string(),
                context: "generic function call".to_string(),
                span: None,
            })?
            .clone();

        // Extract function details
        let (type_params, params, return_type, body) = match &generic_stmt.kind {
            StmtKind::FunctionDef {
                type_params,
                params,
                return_type,
                body,
                ..
            } => (type_params, params, return_type, body),
            _ => return Err(CodegenError::General(
                format!("Stored generic '{}' is not a function", func_name)
            )),
        };

        // Validate type argument count
        if type_args.len() != type_params.len() {
            return Err(CodegenError::General(
                format!("Generic function '{}' expects {} type arguments, got {}",
                    func_name, type_params.len(), type_args.len())
            ));
        }

        // Generate mangled name
        let mangled_name = self.mangle_generic_name(func_name, type_args);

        // Substitute types in parameters and return type
        let specialized_params = self.substitute_params(params, type_params, type_args);
        let specialized_return_type = self.substitute_return_type(return_type, type_params, type_args);

        // Compile specialized function
        self.compile_function_def(
            &mangled_name,
            &[],  // No type params in specialized version
            &specialized_params,
            &specialized_return_type,
            body,
            &generic_stmt,
            parent_function,
        )?;

        // Cache the result
        self.monomorphized_cache.insert(cache_key, mangled_name.clone());

        Ok(mangled_name)
    }

    /// Monomorphize and compile a generic struct with concrete type arguments
    fn monomorphize_struct(
        &mut self,
        struct_name: &str,
        type_args: &[String],
    ) -> CodegenResult<String> {
        // Check cache first
        let cache_key = (struct_name.to_string(), type_args.to_vec());
        if let Some(mangled_name) = self.monomorphized_cache.get(&cache_key) {
            return Ok(mangled_name.clone());
        }

        // Get generic struct definition
        let generic_struct = self.generic_structs.get(struct_name)
            .ok_or_else(|| CodegenError::UndefinedSymbol {
                name: struct_name.to_string(),
                context: "generic struct instantiation".to_string(),
                span: None,
            })?
            .clone();

        // Validate type argument count
        if type_args.len() != generic_struct.type_params.len() {
            return Err(CodegenError::General(
                format!("Generic struct '{}' expects {} type arguments, got {}",
                    struct_name, generic_struct.type_params.len(), type_args.len())
            ));
        }

        // Generate mangled name: Box<int> -> Box_int
        let mangled_name = self.mangle_generic_name(struct_name, type_args);

        // Substitute type parameters in fields
        let specialized_fields: Vec<(String, String, Option<Expr>)> = generic_struct
            .fields
            .iter()
            .map(|(field_name, field_type, default)| {
                let substituted_type = self.substitute_type(
                    field_type,
                    &generic_struct.type_params,
                    type_args,
                );
                (field_name.clone(), substituted_type, default.clone())
            })
            .collect();

        // Convert field types to BrixType
        let field_metadata: Vec<(String, BrixType, Option<Expr>)> = specialized_fields
            .iter()
            .map(|(field_name, field_type, default)| {
                (field_name.clone(), self.string_to_brix_type(field_type), default.clone())
            })
            .collect();

        // Create specialized LLVM struct type
        let struct_type = self.context.opaque_struct_type(&mangled_name);

        // Define struct body
        let field_llvm_types: Vec<BasicTypeEnum> = field_metadata
            .iter()
            .map(|(_, brix_type, _)| self.brix_type_to_llvm(brix_type))
            .collect();

        struct_type.set_body(&field_llvm_types, false);

        // Store in registries
        self.struct_defs.insert(mangled_name.clone(), field_metadata);
        self.struct_types.insert(mangled_name.clone(), struct_type);

        // Monomorphize associated methods
        if let Some(methods) = self.generic_methods.get(struct_name).cloned() {
            for method in methods {
                self.monomorphize_method(&mangled_name, &generic_struct.type_params, type_args, &method)?;
            }
        }

        // Cache the result
        self.monomorphized_cache.insert(cache_key, mangled_name.clone());

        Ok(mangled_name)
    }

    /// Monomorphize and compile a method for a specialized struct
    fn monomorphize_method(
        &mut self,
        specialized_struct_name: &str,
        type_params: &[parser::ast::TypeParam],
        type_args: &[String],
        method: &MethodDef,
    ) -> CodegenResult<()> {
        // Substitute type parameters in method signature
        let specialized_params: Vec<(String, String, Option<Expr>)> = method
            .params
            .iter()
            .map(|(param_name, param_type, default)| {
                let substituted_type = self.substitute_type(param_type, type_params, type_args);
                (param_name.clone(), substituted_type, default.clone())
            })
            .collect();

        let specialized_return_type = method.return_type.as_ref().map(|ret_types| {
            ret_types
                .iter()
                .map(|ret_type| self.substitute_type(ret_type, type_params, type_args))
                .collect()
        });

        // Compile the method with specialized struct name as receiver
        // The receiver_type is now the specialized struct name (e.g., "Box_int")
        self.compile_method_def(
            &method.receiver_name,
            specialized_struct_name,
            &method.method_name,
            &specialized_params,
            &specialized_return_type,
            &method.body,
            self.current_function.ok_or_else(|| CodegenError::General(
                "No current function during method monomorphization".to_string()
            ))?,
        )
    }

    // --- MAIN COMPILATION ---

    pub fn compile_program(&mut self, program: &Program) -> CodegenResult<()> {
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[], false);
        let function = self.module.add_function("main", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");

        self.builder.position_at_end(basic_block);
        self.current_function = Some(function);

        // Compile all statements, propagating errors
        for stmt in &program.statements {
            self.compile_stmt(stmt, function)?;
        }

        // ARC: Release all ref-counted variables before exiting main
        self.release_function_scope_vars()?;

        // Build return instruction
        self.builder
            .build_return(Some(&i64_type.const_int(0, false)))
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_return".to_string(),
                details: "Failed to build return instruction in main".to_string(),
                            span: None,
            })?;

        Ok(())
    }

    fn compile_lvalue_addr(&mut self, expr: &Expr) -> CodegenResult<(PointerValue<'ctx>, BrixType)> {
        match &expr.kind {
            ExprKind::Identifier(name) => {
                if let Some((ptr, var_type)) = self.variables.get(name) {
                    Ok((*ptr, var_type.clone()))
                } else {
                    Err(CodegenError::UndefinedSymbol {
                        name: name.clone(),
                        context: "Assignment target".to_string(),
                        span: Some(expr.span.clone()),
                    })
                }
            }

            ExprKind::Index { array, indices } => {
                let (target_val, target_type) = self.compile_expr(array)?;

                // Support both Matrix and IntMatrix for lvalue assignment
                if target_type != BrixType::Matrix && target_type != BrixType::IntMatrix {
                    return Err(CodegenError::TypeError {
                        expected: "Matrix or IntMatrix".to_string(),
                        found: format!("{:?}", target_type),
                        context: "Index assignment target".to_string(),
                                            span: None,
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
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "cols")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed to get matrix columns pointer".to_string(),
                                            span: None,
                    })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load matrix columns".to_string(),
                                            span: None,
                    })?
                    .into_int_value();

                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 3, "data")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed to get matrix data pointer".to_string(),
                                            span: None,
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
                                            span: None,
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
                                                    span: None,
                        })?;
                    self.builder
                        .build_int_add(row_offset, col_val.into_int_value(), "final_off")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_int_add".to_string(),
                            details: "Failed to calculate final offset".to_string(),
                                                    span: None,
                        })?
                } else {
                    return Err(CodegenError::InvalidOperation {
                        operation: "Matrix indexing".to_string(),
                        reason: format!("Invalid number of indices: {}", indices.len()),
                                            span: None,
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
                                                            span: None,
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
                                                            span: None,
                            })?;
                        Ok((item_ptr, BrixType::Float))
                    }
                }
            }

            _ => {
                Err(CodegenError::InvalidOperation {
                    operation: "Assignment".to_string(),
                    reason: "Invalid expression for the left side of an assignment".to_string(),
                                    span: None,
                })
            }
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt, function: inkwell::values::FunctionValue<'ctx>) -> CodegenResult<()> {
        match &stmt.kind {
            StmtKind::VariableDecl {
                name,
                type_hint,
                value,
                is_const: _,
            } => {
                self.compile_variable_decl_stmt(name, type_hint, value)?;
                Ok(())
            }

            StmtKind::DestructuringDecl {
                names,
                value,
                is_const: _,
            } => {
                self.compile_destructuring_decl_stmt(names, value)?;
                Ok(())
            }

            StmtKind::Assignment { target, value } => {
                self.compile_assignment_stmt(target, value)?;
                Ok(())
            }

            StmtKind::Printf { format, args } => {
                self.compile_printf_stmt(format, args)?;
                Ok(())
            }

            StmtKind::Print { expr } => {
                self.compile_print_stmt(expr)?;
                Ok(())
            }

            StmtKind::Println { expr } => {
                self.compile_println_stmt(expr)?;
                Ok(())
            }

            StmtKind::Expr(expr) => {
                self.compile_expr_stmt(expr)?;
                Ok(())
            }

            StmtKind::Block(statements) => {
                self.compile_block_stmt(statements, function)?;
                Ok(())
            }

            StmtKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.compile_if_stmt(condition, then_block, else_block, function)?;
                Ok(())
            }

            StmtKind::While { condition, body } => {
                self.compile_while_stmt(condition, body, function)?;
                Ok(())
            }

            StmtKind::For {
                var_names,
                iterable,
                body,
            } => {
                // For ranges, we only support single variable
                if let ExprKind::Range { start, end, step } = &iterable.kind {
                    if var_names.len() != 1 {
                        return Err(CodegenError::InvalidOperation {
                            operation: "Range iteration".to_string(),
                            reason: "Range iteration supports only single variable".to_string(),
                                                    span: Some(stmt.span.clone()),
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
                                                    span: Some(stmt.span.clone()),
                        })?;

                    let old_var = self.variables.remove(var_name);
                    self.variables
                        .insert(var_name.clone(), (i_alloca, BrixType::Int));

                    // 2. Basic blocks
                    let cond_bb = self.context.append_basic_block(function, "for_cond");
                    let body_bb = self.context.append_basic_block(function, "for_body");
                    let inc_bb = self.context.append_basic_block(function, "for_inc");
                    let after_bb = self.context.append_basic_block(function, "for_after");

                    self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                    // --- BLOCK: COND ---
                    self.builder.position_at_end(cond_bb);
                    let cur_i = self
                        .builder
                        .build_load(self.context.i64_type(), i_alloca, "i_val")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                        .into_int_value();

                    let loop_cond = self
                        .builder
                        .build_int_compare(IntPredicate::SLE, cur_i, end_int, "loop_cond")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                    self.builder
                        .build_conditional_branch(loop_cond, body_bb, after_bb)
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                    // --- BLOCK: BODY ---
                    self.builder.position_at_end(body_bb);
                    self.compile_stmt(body, function)?;
                    self.builder.build_unconditional_branch(inc_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                    // --- BLOCK: INC ---
                    self.builder.position_at_end(inc_bb);
                    let tmp_i = self
                        .builder
                        .build_load(self.context.i64_type(), i_alloca, "i_load")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                        .into_int_value();
                    let next_i = self
                        .builder
                        .build_int_add(tmp_i, step_val, "i_next")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                    self.builder.build_store(i_alloca, next_i).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                    self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

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
                                .build_struct_gep(matrix_type, matrix_ptr, 1, "rows")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            let cols_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 2, "cols")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                            let rows = self
                                .builder
                                .build_load(i64_type, rows_ptr, "rows")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                                .into_int_value();
                            let cols = self
                                .builder
                                .build_load(i64_type, cols_ptr, "cols")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
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
                                    self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?,
                                    false,
                                )
                            };

                            let idx_alloca =
                                self.create_entry_block_alloca(i64_type.into(), "_hidden_idx")?;
                            self.builder
                                .build_store(idx_alloca, i64_type.const_int(0, false))
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

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

                            self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                            // --- COND ---
                            self.builder.position_at_end(cond_bb);
                            let cur_idx = self
                                .builder
                                .build_load(i64_type, idx_alloca, "cur_idx")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                                .into_int_value();
                            let loop_cond = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::SLT,
                                    cur_idx,
                                    total_len,
                                    "check_idx",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            self.builder
                                .build_conditional_branch(loop_cond, body_bb, after_bb)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                            // --- BODY ---
                            self.builder.position_at_end(body_bb);

                            let data_ptr_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 3, "data_ptr")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            let data_base = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_ptr_ptr,
                                    "data_base",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
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
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                        let col_offset = self
                                            .builder
                                            .build_int_add(
                                                offset,
                                                i64_type.const_int(j as u64, false),
                                                "elem_offset",
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                                        let elem_ptr = self
                                            .builder
                                            .build_gep(
                                                self.context.f64_type(),
                                                data_base,
                                                &[col_offset],
                                                &format!("elem_{}_ptr", j),
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                        let elem_val = self
                                            .builder
                                            .build_load(
                                                self.context.f64_type(),
                                                elem_ptr,
                                                &format!("elem_{}", j),
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                        self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
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
                                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                    let elem_val = self
                                        .builder
                                        .build_load(self.context.f64_type(), elem_ptr, "elem_val")
                                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                    self.builder.build_store(var_allocas[0], elem_val).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                }
                            }

                            self.compile_stmt(body, function)?;
                            self.builder.build_unconditional_branch(inc_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                            // --- INC ---
                            self.builder.position_at_end(inc_bb);
                            let tmp_idx = self
                                .builder
                                .build_load(i64_type, idx_alloca, "idx_load")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                                .into_int_value();
                            let next_idx = self
                                .builder
                                .build_int_add(tmp_idx, i64_type.const_int(1, false), "idx_next")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

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
                                .build_struct_gep(matrix_type, matrix_ptr, 1, "rows")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            let cols_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 2, "cols")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                            let rows = self
                                .builder
                                .build_load(i64_type, rows_ptr, "rows")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                                .into_int_value();
                            let cols = self
                                .builder
                                .build_load(i64_type, cols_ptr, "cols")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                                .into_int_value();

                            let (total_len, is_destructuring) = if var_names.len() > 1 {
                                // Destructuring: iterate rows, assuming cols matches var_names.len()
                                // TODO: Add runtime check for cols == var_names.len()
                                (rows, true)
                            } else {
                                (
                                    self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?,
                                    false,
                                )
                            };

                            let idx_alloca =
                                self.create_entry_block_alloca(i64_type.into(), "_hidden_idx")?;
                            self.builder
                                .build_store(idx_alloca, i64_type.const_int(0, false))
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

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

                            self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                            // --- COND ---
                            self.builder.position_at_end(cond_bb);
                            let cur_idx = self
                                .builder
                                .build_load(i64_type, idx_alloca, "cur_idx")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                                .into_int_value();
                            let loop_cond = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::SLT,
                                    cur_idx,
                                    total_len,
                                    "check_idx",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            self.builder
                                .build_conditional_branch(loop_cond, body_bb, after_bb)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                            // --- BODY ---
                            self.builder.position_at_end(body_bb);

                            let data_ptr_ptr = self
                                .builder
                                .build_struct_gep(matrix_type, matrix_ptr, 3, "data_ptr")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            let data_base = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_ptr_ptr,
                                    "data_base",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                                .into_pointer_value();

                            if is_destructuring {
                                for (j, var_alloca) in var_allocas.iter().enumerate() {
                                    unsafe {
                                        let offset = self
                                            .builder
                                            .build_int_mul(cur_idx, cols, "row_offset")
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                        let col_offset = self
                                            .builder
                                            .build_int_add(
                                                offset,
                                                i64_type.const_int(j as u64, false),
                                                "elem_offset",
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                                        let elem_ptr = self
                                            .builder
                                            .build_gep(
                                                i64_type,
                                                data_base,
                                                &[col_offset],
                                                &format!("elem_{}_ptr", j),
                                            )
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                        let elem_val = self
                                            .builder
                                            .build_load(i64_type, elem_ptr, &format!("elem_{}", j))
                                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                        self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                    }
                                }
                            } else {
                                unsafe {
                                    let elem_ptr = self
                                        .builder
                                        .build_gep(i64_type, data_base, &[cur_idx], "elem_ptr")
                                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                    let elem_val = self
                                        .builder
                                        .build_load(i64_type, elem_ptr, "elem_val")
                                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                    self.builder.build_store(var_allocas[0], elem_val).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                                }
                            }

                            self.compile_stmt(body, function)?;
                            self.builder.build_unconditional_branch(inc_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

                            // --- INC ---
                            self.builder.position_at_end(inc_bb);
                            let tmp_idx = self
                                .builder
                                .build_load(i64_type, idx_alloca, "idx_load")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?
                                .into_int_value();
                            let next_idx = self
                                .builder
                                .build_int_add(tmp_idx, i64_type.const_int(1, false), "idx_next")
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;
                            self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in for loop".to_string(), span: None })?;

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
                                                            span: Some(stmt.span.clone()),
                            })
                        }
                    }
                }
            }

            StmtKind::Import { module, alias } => {
                self.compile_import_stmt(module, alias)?;
                Ok(())
            }

            StmtKind::TypeAlias { name, definition } => {
                // Store type alias in symbol table
                self.type_aliases.insert(name.clone(), definition.clone());
                Ok(())
            }

            StmtKind::FunctionDef {
                name,
                type_params,
                params,
                return_type,
                body,
            } => {
                self.compile_function_def(name, type_params, params, return_type, body, stmt, function)?;
                Ok(())
            }

            StmtKind::StructDef(struct_def) => {
                self.compile_struct_def(&struct_def.name, &struct_def.type_params, &struct_def.fields, struct_def)?;
                Ok(())
            }

            StmtKind::MethodDef(method_def) => {
                self.compile_method_def(
                    &method_def.receiver_name,
                    &method_def.receiver_type,
                    &method_def.method_name,
                    &method_def.params,
                    &method_def.return_type,
                    &method_def.body,
                    function,
                )?;
                Ok(())
            }

            StmtKind::Return { values } => {
                self.compile_return_stmt(values)?;
                Ok(())
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        match &expr.kind {
            ExprKind::Literal(lit) => self.compile_literal_expr(lit),

            ExprKind::Identifier(name) => {
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
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
                            Ok((val, brix_type.clone()))
                        }

                        BrixType::Int => {
                            let val = self
                                .builder
                                .build_load(self.context.i64_type(), *ptr, name)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
                            Ok((val, BrixType::Int))
                        }
                        BrixType::Atom => {
                            let val = self
                                .builder
                                .build_load(self.context.i64_type(), *ptr, name)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
                            Ok((val, BrixType::Atom))
                        }
                        BrixType::Float => {
                            let val = self
                                .builder
                                .build_load(self.context.f64_type(), *ptr, name)
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
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
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
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
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
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
                                .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
                            Ok((val, BrixType::ComplexMatrix))
                        }
                        BrixType::Tuple(types) => {
                            // Check if this is a closure (Tuple with 3 Int fields = {ref_count, fn_ptr, env_ptr})
                            if types.len() == 3 && types[0] == BrixType::Int && types[1] == BrixType::Int && types[2] == BrixType::Int {
                                // This is a closure! Load it and retain (ARC)
                                let ptr_type = self.context.ptr_type(AddressSpace::default());
                                let closure_ptr = self.builder
                                    .build_load(ptr_type, *ptr, name)
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_load".to_string(),
                                        details: format!("Failed to load closure variable '{}'", name),
                                        span: Some(expr.span.clone()),
                                    })?
                                    .into_pointer_value();

                                // ARC: Retain the closure when loading from variable (copying reference)
                                let retained_closure = self.closure_retain(closure_ptr)?;

                                Ok((retained_closure.into(), BrixType::Tuple(types.clone())))
                            } else {
                                // Regular tuple - load the tuple struct
                                let struct_type =
                                    self.brix_type_to_llvm(&BrixType::Tuple(types.clone()));
                                let val = self.builder.build_load(struct_type, *ptr, name).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
                                Ok((val, BrixType::Tuple(types.clone())))
                            }
                        }
                        BrixType::Complex => {
                            // Load the complex struct { f64 real, f64 imag }
                            let complex_type = self.brix_type_to_llvm(&BrixType::Complex);
                            let val = self.builder.build_load(complex_type, *ptr, name).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
                            Ok((val, BrixType::Complex))
                        }
                        BrixType::Nil => {
                            // Load nil (null pointer)
                            let ptr_type = self.context.ptr_type(AddressSpace::default());
                            let val = self.builder.build_load(ptr_type, *ptr, name).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
                            Ok((val, BrixType::Nil))
                        }
                        BrixType::Error => {
                            // Load error (pointer to BrixError struct)
                            let ptr_type = self.context.ptr_type(AddressSpace::default());
                            let val = self.builder.build_load(ptr_type, *ptr, name).map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in compile_expr".to_string(), span: None })?;
                            Ok((val, BrixType::Error))
                        }
                        BrixType::Struct(struct_name) => {
                            // Load struct value from the alloca'd memory
                            let struct_type = self.struct_types.get(struct_name).ok_or_else(|| {
                                CodegenError::UndefinedSymbol {
                                    name: struct_name.clone(),
                                    context: "Loading struct variable".to_string(),
                                    span: Some(expr.span.clone()),
                                }
                            })?;
                            let val = self.builder.build_load(*struct_type, *ptr, name)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!("Failed to load struct variable '{}'", name),
                                    span: Some(expr.span.clone()),
                                })?;
                            Ok((val, BrixType::Struct(struct_name.clone())))
                        }
                        BrixType::Optional(_) => {
                            // Optional is now Union(T, nil), this should never be reached
                            panic!("Optional type should have been converted to Union")
                        }
                        BrixType::Union(_) => {
                            // Load Union value (tagged union struct)
                            let llvm_type = self.brix_type_to_llvm(brix_type);
                            let val = self.builder.build_load(llvm_type, *ptr, name)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!("Failed to load Union variable '{}'", name),
                                    span: Some(expr.span.clone()),
                                })?;
                            Ok((val, brix_type.clone()))
                        }
                        BrixType::Intersection(_) => {
                            // Load Intersection value (merged struct)
                            let llvm_type = self.brix_type_to_llvm(brix_type);
                            let val = self.builder.build_load(llvm_type, *ptr, name)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!("Failed to load Intersection variable '{}'", name),
                                    span: Some(expr.span.clone()),
                                })?;
                            Ok((val, brix_type.clone()))
                        }
                        _ => {
                            Err(CodegenError::TypeError {
                                expected: "Nil, Error, or Atom".to_string(),
                                found: format!("{:?}", brix_type),
                                context: "Identifier compilation - type not supported".to_string(),
                                span: Some(expr.span.clone()),
                            })
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

                        Err(CodegenError::UndefinedSymbol {
                            name: name.clone(),
                            context: "Variable lookup in expression".to_string(),
                            span: Some(expr.span.clone()),
                        })
                    }
                }
            }

            ExprKind::Unary { op, expr } => {
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
                                                                    span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;

                        // Extend i1 to i64
                        let result = self
                            .builder
                            .build_int_z_extend(is_zero, self.context.i64_type(), "not_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_z_extend".to_string(),
                                details: "Failed to extend NOT result to i64".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            Ok((neg.into(), BrixType::Int))
                        } else if val_type == BrixType::Float {
                            let neg = self
                                .builder
                                .build_float_neg(val.into_float_value(), "neg_float")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_float_neg".to_string(),
                                    details: "Failed to negate float".to_string(),
                                                                    span: Some(expr.span.clone()),
                                })?;
                            Ok((neg.into(), BrixType::Float))
                        } else {
                            Err(CodegenError::TypeError {
                                expected: "Int or Float".to_string(),
                                found: format!("{:?}", val_type),
                                context: "Unary negation".to_string(),
                                                            span: Some(expr.span.clone()),
                            })
                        }
                    }
                }
            }

            ExprKind::Binary { op, lhs, rhs } => {
                // --- ELVIS OPERATOR (v1.4) ---
                // a ?: b → returns a if a is not nil, otherwise returns b
                if matches!(op, BinaryOp::Elvis) {
                    let (lhs_val, lhs_type) = self.compile_expr(lhs)?;

                    let parent_fn = self.builder.get_insert_block()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_insert_block".to_string(),
                            details: "No current block for Elvis operator".to_string(),
                            span: Some(expr.span.clone()),
                        })?
                        .get_parent()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_parent".to_string(),
                            details: "Block has no parent for Elvis operator".to_string(),
                            span: Some(expr.span.clone()),
                        })?;

                    let lhs_not_nil_bb = self.context.append_basic_block(parent_fn, "elvis_lhs");
                    let rhs_bb = self.context.append_basic_block(parent_fn, "elvis_rhs");
                    let merge_bb = self.context.append_basic_block(parent_fn, "elvis_merge");

                    let _entry_bb = self.builder.get_insert_block()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_insert_block".to_string(),
                            details: "No current block for Elvis entry".to_string(),
                            span: Some(expr.span.clone()),
                        })?;

                    // Check if lhs is nil
                    let is_nil = if let BrixType::Union(types) = &lhs_type {
                        // For Union types, check if tag == nil_index
                        if let Some(nil_index) = types.iter().position(|t| t == &BrixType::Nil) {
                            let tag_val = self.builder.build_extract_value(lhs_val.into_struct_value(), 0, "extract_tag")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_extract_value".to_string(),
                                    details: "Failed to extract tag from union in Elvis".to_string(),
                                    span: Some(expr.span.clone()),
                                })?.into_int_value();

                            let nil_tag = self.context.i64_type().const_int(nil_index as u64, false);
                            self.builder.build_int_compare(IntPredicate::EQ, tag_val, nil_tag, "is_nil")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_int_compare".to_string(),
                                    details: "Failed to compare tag with nil in Elvis".to_string(),
                                    span: Some(expr.span.clone()),
                                })?
                        } else {
                            // Union without nil variant - never nil
                            self.context.bool_type().const_int(0, false)
                        }
                    } else if Self::is_ref_counted(&lhs_type) {
                        // For ref-counted types, check if pointer is null
                        let ptr_val = lhs_val.into_pointer_value();
                        self.builder.build_is_null(ptr_val, "is_nil")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_is_null".to_string(),
                                details: "Failed to check null in Elvis".to_string(),
                                span: Some(expr.span.clone()),
                            })?
                    } else if lhs_type == BrixType::Nil {
                        // Literal nil - always nil
                        self.context.bool_type().const_int(1, false)
                    } else {
                        // Non-nullable type - never nil
                        self.context.bool_type().const_int(0, false)
                    };

                    // Branch: if nil, go to rhs_bb; if not nil, go to lhs_not_nil_bb
                    self.builder.build_conditional_branch(is_nil, rhs_bb, lhs_not_nil_bb)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_conditional_branch".to_string(),
                            details: "Failed to branch in Elvis".to_string(),
                            span: Some(expr.span.clone()),
                        })?;

                    // LHS block: lhs is not nil, extract value if it's a Union
                    self.builder.position_at_end(lhs_not_nil_bb);

                    // If lhs is Union, extract the actual value (field 1)
                    let lhs_result = if let BrixType::Union(types) = &lhs_type {
                        // Extract inner value from Union
                        let inner_val = self.builder.build_extract_value(lhs_val.into_struct_value(), 1, "extract_value")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_extract_value".to_string(),
                                details: "Failed to extract value from union in Elvis".to_string(),
                                span: Some(expr.span.clone()),
                            })?;

                        // Determine inner type (first non-nil type)
                        let inner_type = types.iter().find(|t| t != &&BrixType::Nil)
                            .cloned()
                            .unwrap_or(BrixType::Nil);

                        (inner_val, inner_type)
                    } else {
                        (lhs_val, lhs_type.clone())
                    };

                    self.builder.build_unconditional_branch(merge_bb)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_unconditional_branch".to_string(),
                            details: "Failed to branch to merge in Elvis lhs".to_string(),
                            span: Some(expr.span.clone()),
                        })?;

                    // RHS block: lhs is nil, evaluate and use rhs
                    self.builder.position_at_end(rhs_bb);
                    let (rhs_val, _rhs_type) = self.compile_expr(rhs)?;
                    self.builder.build_unconditional_branch(merge_bb)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_unconditional_branch".to_string(),
                            details: "Failed to branch to merge in Elvis rhs".to_string(),
                            span: Some(expr.span.clone()),
                        })?;
                    let rhs_end_bb = self.builder.get_insert_block()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_insert_block".to_string(),
                            details: "No block after Elvis rhs".to_string(),
                            span: Some(expr.span.clone()),
                        })?;

                    // Merge block: PHI node to select result
                    self.builder.position_at_end(merge_bb);

                    // Result type should match the inner type (non-Union)
                    let result_type = lhs_result.1.clone();

                    let phi_type = self.brix_type_to_llvm(&result_type);
                    let phi = self.builder.build_phi(phi_type, "elvis_result")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_phi".to_string(),
                            details: "Failed to build PHI for Elvis result".to_string(),
                            span: Some(expr.span.clone()),
                        })?;

                    phi.add_incoming(&[(&lhs_result.0, lhs_not_nil_bb), (&rhs_val, rhs_end_bb)]);
                    return Ok((phi.as_basic_value(), result_type));
                }

                if matches!(op, BinaryOp::LogicalAnd) || matches!(op, BinaryOp::LogicalOr) {
                    let (lhs_val, _) = self.compile_expr(lhs)?;
                    let lhs_int = lhs_val.into_int_value();

                    let parent_fn = self
                        .builder
                        .get_insert_block()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_insert_block".to_string(),
                            details: "No current block for logical operator".to_string(),
                                                    span: Some(expr.span.clone()),
                        })?
                        .get_parent()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_parent".to_string(),
                            details: "Block has no parent for logical operator".to_string(),
                                                    span: Some(expr.span.clone()),
                        })?;
                    let rhs_bb = self.context.append_basic_block(parent_fn, "logic_rhs");
                    let merge_bb = self.context.append_basic_block(parent_fn, "logic_merge");

                    let entry_bb = self.builder.get_insert_block()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_insert_block".to_string(),
                            details: "No current block for logical entry".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;

                            self.builder
                                .build_conditional_branch(lhs_bool, rhs_bb, merge_bb)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_conditional_branch".to_string(),
                                    details: "Failed to branch for LogicalAnd".to_string(),
                                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;

                            self.builder
                                .build_conditional_branch(lhs_bool, merge_bb, rhs_bb)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_conditional_branch".to_string(),
                                    details: "Failed to branch for LogicalOr".to_string(),
                                                                    span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
                        })?;
                    let rhs_end_bb = self.builder.get_insert_block()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "get_insert_block".to_string(),
                            details: "No block after logical rhs".to_string(),
                                                    span: Some(expr.span.clone()),
                        })?;

                    self.builder.position_at_end(merge_bb);
                    let phi = self
                        .builder
                        .build_phi(self.context.i64_type(), "logic_result")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_phi".to_string(),
                            details: "Failed to build PHI for logical result".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            let promoted = call.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "promoted matrix value".to_string(),
                                    context: "IntMatrix promotion lhs".to_string(),
                                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            let promoted = call.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "promoted matrix value".to_string(),
                                    context: "IntMatrix promotion rhs".to_string(),
                                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "matrix op result".to_string(),
                                context: "Matrix op scalar".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                });
                            }
                        };

                        let scalar_val = if lhs_type == BrixType::Int {
                            self.builder
                                .build_signed_int_to_float(lhs_val.into_int_value(), self.context.f64_type(), "int_to_float")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_signed_int_to_float".to_string(),
                                    details: "Failed to convert int to float for scalar-matrix op".to_string(),
                                                                    span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "scalar-matrix op result".to_string(),
                                context: "scalar op Matrix".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "matrix-matrix op result".to_string(),
                                context: "Matrix op Matrix".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "intmatrix op result".to_string(),
                                context: "IntMatrix op Int".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "scalar-intmatrix op result".to_string(),
                                context: "Int op IntMatrix".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "intmatrix-intmatrix op result".to_string(),
                                context: "IntMatrix op IntMatrix".to_string(),
                                                            span: Some(expr.span.clone()),
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

                // --- UNION NIL COMPARISON (v1.4) ---
                // Handle: union_var == nil or union_var != nil
                // Since Optional is now Union(T, nil), this handles Optional comparisons too
                if (matches!(op, BinaryOp::Eq) || matches!(op, BinaryOp::NotEq)) {
                    let (union_val, union_type, is_eq) = if matches!(&lhs_type, BrixType::Union(_)) && rhs_type == BrixType::Nil {
                        (lhs_val, &lhs_type, matches!(op, BinaryOp::Eq))
                    } else if matches!(&rhs_type, BrixType::Union(_)) && lhs_type == BrixType::Nil {
                        (rhs_val, &rhs_type, matches!(op, BinaryOp::Eq))
                    } else {
                        // Not a Union-nil comparison, continue to regular logic
                        (BasicValueEnum::IntValue(self.context.i64_type().const_zero()), &lhs_type, false)
                    };

                    if let BrixType::Union(types) = union_type {
                        // Check if Union contains Nil
                        if let Some(nil_index) = types.iter().position(|t| t == &BrixType::Nil) {
                            // Extract tag from union (field 0)
                            let tag_val = self.builder.build_extract_value(union_val.into_struct_value(), 0, "extract_tag")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_extract_value".to_string(),
                                    details: "Failed to extract tag from union".to_string(),
                                    span: Some(expr.span.clone()),
                                })?.into_int_value();

                            // Compare tag with nil index
                            let nil_tag = self.context.i64_type().const_int(nil_index as u64, false);
                            let predicate = if is_eq {
                                IntPredicate::EQ  // x == nil → tag == nil_index
                            } else {
                                IntPredicate::NE  // x != nil → tag != nil_index
                            };

                            let cmp = self.builder.build_int_compare(predicate, tag_val, nil_tag, "union_nil_cmp")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_int_compare".to_string(),
                                    details: "Failed to compare Union tag with nil".to_string(),
                                    span: Some(expr.span.clone()),
                                })?;

                            // Extend i1 to i64 for consistency
                            let result = self.builder.build_int_z_extend(cmp, self.context.i64_type(), "union_nil_cmp_ext")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_int_z_extend".to_string(),
                                    details: "Failed to extend Union nil comparison result".to_string(),
                                    span: Some(expr.span.clone()),
                                })?;

                            return Ok((result.into(), BrixType::Int));
                        }
                    }
                }

                // Optional is now Union(T, nil)
                // No special Optional comparison needed

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
                                                    span: Some(expr.span.clone()),
                        })?;

                    // Extend i1 to i64 for consistency
                    let result = self
                        .builder
                        .build_int_z_extend(cmp, self.context.i64_type(), "nil_cmp_ext")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_int_z_extend".to_string(),
                            details: "Failed to extend nil comparison result".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
                        })?
                        .into_float_value();
                    let rhs_imag = self
                        .builder
                        .build_extract_value(rhs_struct, 1, "rhs_imag")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_extract_value".to_string(),
                            details: "Failed to extract imag part from complex rhs".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
                        })?;

                    let complex_val = self
                        .builder
                        .build_insert_value(complex_val, final_imag, 1, "complex_full")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_insert_value".to_string(),
                            details: "Failed to insert imag part into complex".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?;
                                call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "complex_powi result".to_string(),
                                        context: "Complex power (int exp)".to_string(),
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?;
                                call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "complex_powf result".to_string(),
                                        context: "Complex power (float exp)".to_string(),
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?;
                                call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "complex_pow result".to_string(),
                                        context: "Complex power (complex exp)".to_string(),
                                                                            span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        self.builder
                            .build_insert_value(inner, zero, 1, "imag")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_insert_value".to_string(),
                                details: "Failed to insert imag part for lhs complex promotion".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        self.builder
                            .build_insert_value(inner, zero, 1, "imag")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_insert_value".to_string(),
                                details: "Failed to insert imag part for rhs complex promotion".to_string(),
                                                            span: Some(expr.span.clone()),
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
                            return Err(CodegenError::InvalidOperation {
                                operation: format!("{:?}", op),
                                reason: "Operator not supported for complex numbers".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
                        })?;
                    let result = call.try_as_basic_value().left()
                        .ok_or_else(|| CodegenError::MissingValue {
                            what: "complex op result".to_string(),
                            context: "Complex arithmetic".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            let result = res.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "str_concat result".to_string(),
                                    context: "String concatenation".to_string(),
                                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            let result = res.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "str_eq result".to_string(),
                                    context: "String equality".to_string(),
                                                                    span: Some(expr.span.clone()),
                                })?;
                            return Ok((result, BrixType::Int));
                        }
                        _ => {
                            return Err(CodegenError::InvalidOperation {
                                operation: format!("{:?}", op),
                                reason: "Only + and == are supported for strings".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
                        }
                    }
                }

                // --- Numbers (Int and Float) ---
                // Validate that both operands are numeric types
                let is_numeric = |t: &BrixType| matches!(t, BrixType::Int | BrixType::Float);

                if !is_numeric(&lhs_type) || !is_numeric(&rhs_type) {
                    return Err(CodegenError::TypeError {
                        expected: "Int or Float".to_string(),
                        found: format!("{:?} and {:?}", lhs_type, rhs_type),
                        context: format!("Binary operation {:?} requires numeric operands", op),
                        span: Some(expr.span.clone()),
                    });
                }

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
                                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?
                    } else {
                        rhs_val.into_float_value()
                    };

                    let val = self.compile_float_op(op, l_float, r_float).ok_or_else(|| {
                        CodegenError::InvalidOperation {
                            operation: format!("{:?}", op),
                            reason: "unsupported float operation".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
                        }
                    })?;
                    Ok((val, BrixType::Int))
                }
            }

            ExprKind::Call { func, args } => {
                // ---- TEST LIBRARY DETECTION (must be before generic module call resolution) ----
                if let Some(result) = self.try_compile_test_call(func, args, &expr.span) {
                    return result;
                }

                // Handle math.function() calls (e.g., math.sin, math.cos, math.sum, etc.)
                if let ExprKind::FieldAccess { target, field } = &func.kind {
                    if let ExprKind::Identifier(_module_name) = &target.kind {
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
                                                                                            span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            let result = call.try_as_basic_value().left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "math function result".to_string(),
                                    context: format!("math.{}", fn_name),
                                                                    span: Some(expr.span.clone()),
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

                    // Check if this is a method call (e.g., obj.method(args))
                    if let ExprKind::FieldAccess { target, field } = &func.kind {
                        // Special handling for method calls on struct identifiers
                        // We need the pointer to the struct, not the loaded value
                        if let ExprKind::Identifier(var_name) = &target.kind {
                            // Clone the values we need before mutable borrows
                            let receiver_info = self.variables.get(var_name).cloned();

                            if let Some((receiver_ptr, receiver_type)) = receiver_info {
                                if let BrixType::Struct(struct_name) = receiver_type {
                                    // Build mangled method name: StructName_methodname
                                    let mangled_name = format!("{}_{}", struct_name, field);

                                    // Look up the method function
                                    if let Some(llvm_fn) = self.module.get_function(&mangled_name) {
                                        // Compile method arguments
                                        let mut llvm_args = Vec::new();

                                        // First argument is the receiver (pointer to struct)
                                        llvm_args.push(receiver_ptr.into());

                                // Add remaining arguments
                                for arg in args {
                                    let (arg_val, _) = self.compile_expr(arg)?;
                                    llvm_args.push(arg_val.into());
                                }

                                // Call the method
                                let call = self
                                    .builder
                                    .build_call(llvm_fn, &llvm_args, "method_call")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_call".to_string(),
                                        details: format!(
                                            "Failed to call method '{}.{}'",
                                            struct_name, field
                                        ),
                                        span: Some(expr.span.clone()),
                                    })?;

                                // Check if method returns void
                                if llvm_fn.get_type().get_return_type().is_none() {
                                    return Ok((
                                        self.context.i64_type().const_int(0, false).into(),
                                        BrixType::Void,
                                    ));
                                }

                                let result = call.try_as_basic_value().left().ok_or_else(|| {
                                    CodegenError::MissingValue {
                                        what: "method result".to_string(),
                                        context: format!("{}.{}", struct_name, field),
                                        span: Some(expr.span.clone()),
                                    }
                                })?;

                                // TODO: Determine return type from method signature
                                // For now, we'll use a simple heuristic based on LLVM type
                                let return_type = if result.is_int_value() {
                                    BrixType::Int
                                } else if result.is_float_value() {
                                    BrixType::Float
                                } else if result.is_pointer_value() {
                                    // Could be String, Matrix, Error, etc.
                                    // For now default to Matrix
                                    BrixType::Matrix
                                } else {
                                    BrixType::Void
                                };

                                        return Ok((result, return_type));
                                    } else {
                                        return Err(CodegenError::UndefinedSymbol {
                                            name: format!("{}.{}", struct_name, field),
                                            context: "method call".to_string(),
                                            span: Some(expr.span.clone()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                if let ExprKind::Identifier(fn_name) = &func.kind {
                    if fn_name == "typeof" {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "typeof".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
                        }
                        let (_, arg_type) = self.compile_expr(&args[0])?;

                        let type_str = match &arg_type {
                            BrixType::Int => "int".to_string(),
                            BrixType::Float => "float".to_string(),
                            BrixType::String => "string".to_string(),
                            BrixType::Matrix => "matrix".to_string(),
                            BrixType::IntMatrix => "intmatrix".to_string(),
                            BrixType::Complex => "complex".to_string(),
                            BrixType::ComplexArray => "complexarray".to_string(),
                            BrixType::ComplexMatrix => "complexmatrix".to_string(),
                            BrixType::FloatPtr => "float_ptr".to_string(),
                            BrixType::Void => "void".to_string(),
                            BrixType::Tuple(_) => "tuple".to_string(),
                            BrixType::Nil => "nil".to_string(),
                            BrixType::Error => "error".to_string(),
                            BrixType::Atom => "atom".to_string(),
                            BrixType::Struct(name) => name.clone(),
                            BrixType::Optional(_) => {
                                // Optional is now Union(T, nil), should never be reached
                                panic!("Optional type should have been converted to Union")
                            }
                            BrixType::Union(types) => {
                                // Format as "int | float | string"
                                types.iter().map(|t| match t {
                                    BrixType::Int => "int",
                                    BrixType::Float => "float",
                                    BrixType::String => "string",
                                    BrixType::Nil => "nil",
                                    BrixType::Struct(name) => name.as_str(),
                                    _ => "unknown",
                                }).collect::<Vec<_>>().join(" | ")
                            }
                            BrixType::Intersection(types) => {
                                // Format as "Point & Label"
                                types.iter().map(|t| match t {
                                    BrixType::Struct(name) => name.as_str(),
                                    _ => "unknown",
                                }).collect::<Vec<_>>().join(" & ")
                            }
                        };

                        return self
                            .compile_expr(&Expr::dummy(ExprKind::Literal(Literal::String(type_str))));
                    }

                    // Conversion functions
                    if fn_name == "int" {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "int()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
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
                                    .build_struct_gep(str_type, struct_ptr, 2, "str_data_ptr")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_struct_gep".to_string(),
                                        details: "Failed to get string data ptr for int()".to_string(),
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?;

                                let call = self
                                    .builder
                                    .build_call(atoi_fn, &[data_ptr.into()], "atoi_result")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_call".to_string(),
                                        details: "Failed to call atoi".to_string(),
                                                                            span: Some(expr.span.clone()),
                                    })?;
                                let i32_result = call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "atoi result".to_string(),
                                        context: "int() conversion".to_string(),
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?
                                    .into()
                            }
                            _ => {
                                return Err(CodegenError::TypeError {
                                    expected: "int-convertible type (Int, Float, Boolean)".to_string(),
                                    found: format!("{:?}", val_type),
                                    context: "int() conversion".to_string(),
                                                                    span: Some(expr.span.clone()),
                                });
                            }
                        };

                        return Ok((result, BrixType::Int));
                    }

                    if fn_name == "float" {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "float()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                                                                            span: Some(expr.span.clone()),
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
                                    .build_struct_gep(str_type, struct_ptr, 2, "str_data_ptr")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_struct_gep".to_string(),
                                        details: "Failed to get string data ptr for float()".to_string(),
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?;

                                let call = self.builder
                                    .build_call(atof_fn, &[data_ptr.into()], "atof_result")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_call".to_string(),
                                        details: "Failed to call atof".to_string(),
                                                                            span: Some(expr.span.clone()),
                                    })?;
                                call.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "atof result".to_string(),
                                        context: "float() conversion".to_string(),
                                                                            span: Some(expr.span.clone()),
                                    })?
                            }
                            _ => {
                                return Err(CodegenError::TypeError {
                                    expected: "float-convertible type (Int, Float, String)".to_string(),
                                    found: format!("{:?}", val_type),
                                    context: "float() conversion".to_string(),
                                                                    span: Some(expr.span.clone()),
                                });
                            }
                        };

                        return Ok((result, BrixType::Float));
                    }

                    if fn_name == "string" {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "string()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        // Reuse value_to_string() which already handles all types
                        let result = self.value_to_string(val, &val_type, None)?;
                        return Ok((result, BrixType::String));
                    }

                    if fn_name == "bool" {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "bool()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                                                                            span: Some(expr.span.clone()),
                                    })?;

                                // Extend i1 to i64
                                self.builder
                                    .build_int_z_extend(cmp, self.context.i64_type(), "bool_extend")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_z_extend".to_string(),
                                        details: "Failed to extend bool result".to_string(),
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?;

                                // Extend i1 to i64
                                self.builder
                                    .build_int_z_extend(cmp, self.context.i64_type(), "bool_extend")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_z_extend".to_string(),
                                        details: "Failed to extend float bool result".to_string(),
                                                                            span: Some(expr.span.clone()),
                                    })?
                                    .into()
                            }
                            BrixType::String => {
                                // String to bool: len > 0
                                let struct_ptr = val.into_pointer_value();
                                let str_type = self.get_string_type();
                                let len_ptr = self
                                    .builder
                                    .build_struct_gep(str_type, struct_ptr, 1, "str_len_ptr")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_struct_gep".to_string(),
                                        details: "Failed to get string len ptr for bool()".to_string(),
                                                                            span: Some(expr.span.clone()),
                                    })?;
                                let len_val = self
                                    .builder
                                    .build_load(self.context.i64_type(), len_ptr, "str_len")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_load".to_string(),
                                        details: "Failed to load string length for bool()".to_string(),
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?;

                                // Extend i1 to i64
                                self.builder
                                    .build_int_z_extend(cmp, self.context.i64_type(), "bool_extend")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_int_z_extend".to_string(),
                                        details: "Failed to extend string bool result".to_string(),
                                                                            span: Some(expr.span.clone()),
                                    })?
                                    .into()
                            }
                            _ => {
                                return Err(CodegenError::TypeError {
                                    expected: "Int, Float, or String".to_string(),
                                    found: format!("{:?}", val_type),
                                    context: "bool() conversion".to_string(),
                                                                    span: Some(expr.span.clone()),
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_nil()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_atom()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_boolean()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?;

                                let is_bool = self
                                    .builder
                                    .build_or(is_zero, is_one, "is_bool_or")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_or".to_string(),
                                        details: "Failed to OR for is_boolean()".to_string(),
                                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_integer()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_float()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_number()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_string()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_list()".to_string(),
                                reason: "expects exactly 1 argument".to_string(),
                                                            span: Some(expr.span.clone()),
                            });
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_tuple()".to_string(),
                                reason: format!("Expected 1 argument, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "is_function()".to_string(),
                                reason: format!("Expected 1 argument, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
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
                            return Err(CodegenError::InvalidOperation {
                                operation: "uppercase()".to_string(),
                                reason: format!("Expected 1 argument, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            return Err(CodegenError::TypeError {
                                expected: "String".to_string(),
                                found: format!("{:?}", val_type),
                                context: "uppercase()".to_string(),
                                span: Some(expr.span.clone()),
                            });
                        }

                        let uppercase_fn = self.get_uppercase();
                        let call = self
                            .builder
                            .build_call(uppercase_fn, &[val.into()], "uppercase_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call uppercase()".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "uppercase()".to_string(),
                                                            span: Some(expr.span.clone()),
                            }
                        })?;

                        return Ok((result, BrixType::String));
                    }

                    // lowercase(str) - Convert string to lowercase
                    if fn_name == "lowercase" {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "lowercase()".to_string(),
                                reason: format!("Expected 1 argument, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            return Err(CodegenError::TypeError {
                                expected: "String".to_string(),
                                found: format!("{:?}", val_type),
                                context: "lowercase()".to_string(),
                                span: Some(expr.span.clone()),
                            });
                        }

                        let lowercase_fn = self.get_lowercase();
                        let call = self
                            .builder
                            .build_call(lowercase_fn, &[val.into()], "lowercase_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call lowercase()".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "lowercase()".to_string(),
                                                            span: Some(expr.span.clone()),
                            }
                        })?;

                        return Ok((result, BrixType::String));
                    }

                    // capitalize(str) - Capitalize first character
                    if fn_name == "capitalize" {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "capitalize()".to_string(),
                                reason: format!("Expected 1 argument, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            return Err(CodegenError::TypeError {
                                expected: "String".to_string(),
                                found: format!("{:?}", val_type),
                                context: "capitalize()".to_string(),
                                span: Some(expr.span.clone()),
                            });
                        }

                        let capitalize_fn = self.get_capitalize();
                        let call = self
                            .builder
                            .build_call(capitalize_fn, &[val.into()], "capitalize_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call capitalize()".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "capitalize()".to_string(),
                                                            span: Some(expr.span.clone()),
                            }
                        })?;

                        return Ok((result, BrixType::String));
                    }

                    // byte_size(str) - Get byte size of string
                    if fn_name == "byte_size" {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "byte_size()".to_string(),
                                reason: format!("Expected 1 argument, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            return Err(CodegenError::TypeError {
                                expected: "String".to_string(),
                                found: format!("{:?}", val_type),
                                context: "byte_size()".to_string(),
                                span: Some(expr.span.clone()),
                            });
                        }

                        let byte_size_fn = self.get_byte_size();
                        let call = self
                            .builder
                            .build_call(byte_size_fn, &[val.into()], "byte_size_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call byte_size()".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "byte_size()".to_string(),
                                                            span: Some(expr.span.clone()),
                            }
                        })?;

                        return Ok((result, BrixType::Int));
                    }

                    // length(str) - Get number of characters (UTF-8 aware)
                    if fn_name == "length" {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "length()".to_string(),
                                reason: format!("Expected 1 argument, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
                        }
                        let (val, val_type) = self.compile_expr(&args[0])?;

                        if val_type != BrixType::String {
                            return Err(CodegenError::TypeError {
                                expected: "String".to_string(),
                                found: format!("{:?}", val_type),
                                context: "length()".to_string(),
                                span: Some(expr.span.clone()),
                            });
                        }

                        let length_fn = self.get_length();
                        let call = self
                            .builder
                            .build_call(length_fn, &[val.into()], "length_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call length()".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "length()".to_string(),
                                                            span: Some(expr.span.clone()),
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
                            return Err(CodegenError::TypeError {
                                expected: "String for all arguments".to_string(),
                                found: "Mixed types".to_string(),
                                context: "replace()".to_string(),
                                span: Some(expr.span.clone()),
                            });
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "replace()".to_string(),
                                                            span: Some(expr.span.clone()),
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
                            return Err(CodegenError::TypeError {
                                expected: "String for all arguments".to_string(),
                                found: "Mixed types".to_string(),
                                context: "replace_all()".to_string(),
                                span: Some(expr.span.clone()),
                            });
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "replace_all()".to_string(),
                                                            span: Some(expr.span.clone()),
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
                            return Err(CodegenError::TypeError {
                                expected: "String".to_string(),
                                found: format!("{:?}", msg_type),
                                context: "error()".to_string(),
                                span: Some(expr.span.clone()),
                            });
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let char_ptr = self
                            .builder
                            .build_load(ptr_type, data_ptr_ptr, "str_data")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load string data for error()".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?
                            .into_pointer_value();

                        // Call brix_error_new(char_ptr)
                        let call = self
                            .builder
                            .build_call(error_new_fn, &[char_ptr.into()], "error_new")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call brix_error_new()".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?;
                        let error_ptr = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "return value".to_string(),
                                context: "brix_error_new()".to_string(),
                                                            span: Some(expr.span.clone()),
                            }
                        })?;

                        return Ok((error_ptr, BrixType::Error));
                    }

                    // === COMPLEX NUMBER FUNCTIONS ===

                    // complex(re, im) - constructor
                    if fn_name == "complex" {
                        if args.len() != 2 {
                            return Err(CodegenError::InvalidOperation {
                                operation: "complex()".to_string(),
                                reason: format!("Expected 2 arguments, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
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
                                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
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
                            return Err(CodegenError::TypeError {
                                expected: "Complex".to_string(),
                                found: format!("{:?}", val_type),
                                context: "real()".to_string(),
                                span: Some(expr.span.clone()),
                            });
                        }

                        let real_part = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 0, "real_part")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_extract_value".to_string(),
                                details: "Failed to extract real part".to_string(),
                                                            span: Some(expr.span.clone()),
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
                            return Err(CodegenError::TypeError {
                                expected: "Complex".to_string(),
                                found: format!("{:?}", val_type),
                                context: "imag()".to_string(),
                                span: Some(expr.span.clone()),
                            });
                        }

                        let imag_part = self
                            .builder
                            .build_extract_value(val.into_struct_value(), 1, "imag_part")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_extract_value".to_string(),
                                details: "Failed to extract imaginary part".to_string(),
                                                            span: Some(expr.span.clone()),
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
                            return Err(CodegenError::InvalidOperation {
                                operation: format!("{}()", fn_name),
                                reason: format!("Expected 1 argument, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
                        }

                        let (val, val_type) = self.compile_expr(&args[0])?;
                        if val_type != BrixType::Complex {
                            return Err(CodegenError::TypeError {
                                expected: "Complex".to_string(),
                                found: format!("{:?}", val_type),
                                context: format!("{}()", fn_name),
                                span: Some(expr.span.clone()),
                            });
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call_site
                            .try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: format!("complex_{} return value", fn_name),
                                context: "complex function call".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?;

                        return Ok((result, BrixType::Complex));
                    }

                    // Single-argument complex functions that return float
                    let complex_to_float_fns = ["abs", "abs2", "angle"];
                    if complex_to_float_fns.contains(&fn_name.as_str()) {
                        if args.len() != 1 {
                            return Err(CodegenError::InvalidOperation {
                                operation: format!("{}()", fn_name),
                                reason: format!("Expected 1 argument, got {}", args.len()),
                                span: Some(expr.span.clone()),
                            });
                        }

                        let (val, val_type) = self.compile_expr(&args[0])?;
                        if val_type != BrixType::Complex {
                            return Err(CodegenError::TypeError {
                                expected: "Complex".to_string(),
                                found: format!("{:?}", val_type),
                                context: format!("{}()", fn_name),
                                span: Some(expr.span.clone()),
                            });
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let result = call_site
                            .try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: format!("complex_{} return value", fn_name),
                                context: "complex to float function call".to_string(),
                                                            span: Some(expr.span.clone()),
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

                // Check if it's a generic function call (with type inference)
                if let ExprKind::Identifier(fn_name) = &func.kind {
                    // Check if this is a generic function
                    if self.generic_functions.contains_key(fn_name) {
                        // Get parent function for context
                        let parent_function = self.current_function.ok_or_else(|| {
                            CodegenError::General("Generic call outside function context".to_string())
                        })?;

                        // Infer type arguments from argument types
                        let type_args = self.infer_generic_types(fn_name, args)?;

                        // Monomorphize the function
                        let specialized_name = self.monomorphize_function(fn_name, &type_args, parent_function)?;

                        // Get the specialized function
                        let llvm_fn = self.module.get_function(&specialized_name)
                            .ok_or_else(|| CodegenError::General(
                                format!("Specialized function '{}' not found after monomorphization", specialized_name)
                            ))?;

                        // Get parameter types from the generic function to cast arguments correctly
                        let generic_stmt = self.generic_functions.get(fn_name).unwrap().clone();
                        let (type_params, params) = match &generic_stmt.kind {
                            StmtKind::FunctionDef { type_params, params, .. } => (type_params, params),
                            _ => unreachable!(),
                        };

                        // Compile arguments with proper type casting
                        let mut llvm_args: Vec<BasicMetadataValueEnum> = Vec::new();
                        for (i, arg) in args.iter().enumerate() {
                            let (mut arg_val, arg_type) = self.compile_expr(arg)?;

                            // Get expected type for this parameter
                            if i < params.len() {
                                let (_param_name, param_type_str, _default) = &params[i];
                                let expected_type = self.substitute_type(param_type_str, type_params, &type_args);

                                // Cast int to float if needed
                                if arg_type == BrixType::Int && expected_type == "float" {
                                    arg_val = self.builder
                                        .build_signed_int_to_float(
                                            arg_val.into_int_value(),
                                            self.context.f64_type(),
                                            "int_to_float_cast",
                                        )
                                        .map_err(|_| CodegenError::LLVMError {
                                            operation: "build_signed_int_to_float".to_string(),
                                            details: "Failed to cast int to float for generic call".to_string(),
                                            span: Some(expr.span.clone()),
                                        })?
                                        .into();
                                }
                            }

                            llvm_args.push(arg_val.into());
                        }

                        // Call the specialized function
                        let call_result = self.builder
                            .build_call(llvm_fn, &llvm_args, "generic_inferred_call")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: format!("Failed to call specialized function '{}'", specialized_name),
                                span: Some(expr.span.clone()),
                            })?;

                        // Get return value
                        if let Some(ret_val) = call_result.try_as_basic_value().left() {
                            // Infer return type from LLVM function signature
                            let ret_type = if let Some((_, Some(ret_types))) = self.functions.get(&specialized_name) {
                                if ret_types.len() == 1 {
                                    ret_types[0].clone()
                                } else {
                                    BrixType::Tuple(ret_types.clone())
                                }
                            } else {
                                BrixType::Int  // Fallback
                            };
                            return Ok((ret_val, ret_type));
                        } else {
                            // Void function
                            return Ok((self.context.i64_type().const_int(0, false).into(), BrixType::Void));
                        }
                    }
                }

                // Check if func is a closure (indirect call)
                // Try to compile func and check if it's a closure
                // Closures can be: variables (Identifier), field access, or other expressions

                // First, try to compile the func expression to see its type
                let func_result = self.compile_expr(func);

                if let Ok((func_val, func_type)) = func_result {
                    // Check if this is a closure call
                    // Closures are represented as Tuple(Int, Int, Int) - {ref_count, fn_ptr, env_ptr}
                    if let BrixType::Tuple(ref fields) = func_type {
                        if fields.len() == 3 && fields[0] == BrixType::Int && fields[1] == BrixType::Int && fields[2] == BrixType::Int {
                        // This is a closure! Perform indirect call

                        // func_val might be a pointer (from closure_retain) or a struct value
                        // If it's a pointer, we need to load it
                        let closure_struct = if func_val.is_pointer_value() {
                            // Load the closure struct from the pointer
                            let closure_ptr = func_val.into_pointer_value();
                            let closure_struct_type = self.context.struct_type(&[
                                self.context.i64_type().into(),                         // ref_count
                                self.context.ptr_type(AddressSpace::default()).into(), // fn_ptr
                                self.context.ptr_type(AddressSpace::default()).into(), // env_ptr
                            ], false);

                            self.builder
                                .build_load(closure_struct_type, closure_ptr, "closure_struct")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: "Failed to load closure struct from pointer".to_string(),
                                    span: Some(expr.span.clone()),
                                })?
                                .into_struct_value()
                        } else {
                            // Already a struct value
                            func_val.into_struct_value()
                        };

                        // Extract fn_ptr (field 1)
                        let fn_ptr = self.builder
                            .build_extract_value(closure_struct, 1, "fn_ptr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_extract_value".to_string(),
                                details: "Failed to extract fn_ptr from closure".to_string(),
                                span: Some(expr.span.clone()),
                            })?
                            .into_pointer_value();

                        // Extract env_ptr (field 2)
                        let env_ptr = self.builder
                            .build_extract_value(closure_struct, 2, "env_ptr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_extract_value".to_string(),
                                details: "Failed to extract env_ptr from closure".to_string(),
                                span: Some(expr.span.clone()),
                            })?
                            .into_pointer_value();

                        // Compile arguments
                        let mut llvm_args: Vec<BasicMetadataValueEnum> = Vec::new();

                        // First argument is always env_ptr
                        llvm_args.push(env_ptr.into());

                        // Add user arguments
                        for arg in args {
                            let (arg_val, _) = self.compile_expr(arg)?;
                            llvm_args.push(arg_val.into());
                        }

                        // TODO: Get proper function type from closure metadata
                        // For now, we'll infer it from arguments and assume int return
                        // This is a limitation - we need to store closure signature in BrixType

                        // Build parameter types: env_ptr + user args
                        let mut param_types: Vec<BasicMetadataTypeEnum> = Vec::new();
                        param_types.push(self.context.ptr_type(AddressSpace::default()).into()); // env_ptr

                        for _ in args {
                            // HACK: Assume all params are i64 for now
                            // TODO: Store proper signature in BrixType::Closure
                            param_types.push(self.context.i64_type().into());
                        }

                        // Assume return type is i64 for now
                        // TODO: Get from closure signature
                        let return_type = self.context.i64_type();
                        let fn_type = return_type.fn_type(&param_types, false);

                        // Perform indirect call
                        let call_result = self.builder
                            .build_indirect_call(
                                fn_type,
                                fn_ptr,
                                &llvm_args,
                                "closure_call"
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_indirect_call".to_string(),
                                details: "Failed to call closure".to_string(),
                                span: Some(expr.span.clone()),
                            })?;

                        let result = call_result.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "closure call result".to_string(),
                                context: "closure invocation".to_string(),
                                span: Some(expr.span.clone()),
                            })?;

                        // TODO: Return proper type from closure signature
                        return Ok((result, BrixType::Int));
                        }
                    }

                    // Not a closure, fall through to check other function types below
                }

                // Check if it's a user-defined function
                if let ExprKind::Identifier(fn_name) = &func.kind {
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
                                                                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
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
                                                                            span: Some(expr.span.clone()),
                                    })?;
                                return Ok((result, ret_types[0].clone()));
                            } else {
                                // Multiple returns - return struct as Tuple type
                                let result = call_result.try_as_basic_value().left()
                                    .ok_or_else(|| CodegenError::MissingValue {
                                        what: "function return value".to_string(),
                                        context: format!("call to '{}'", fn_name),
                                                                            span: Some(expr.span.clone()),
                                    })?;
                                let tuple_type = BrixType::Tuple(ret_types.clone());
                                return Ok((result, tuple_type));
                            }
                        }
                    }
                }

                eprintln!("Error: Unknown function: {:?}", func);
                Err(CodegenError::MissingValue { what: "expression value".to_string(), context: "compile_expr".to_string(), span: None })
            }

            ExprKind::GenericCall { func, type_args, args } => {
                // Get function name
                let func_name = match &func.kind {
                    ExprKind::Identifier(name) => name.clone(),
                    _ => return Err(CodegenError::General(
                        "Generic calls only supported on identifiers (e.g., swap<int, float>)".to_string()
                    )),
                };

                // Get parent function for context
                let parent_function = self.current_function.ok_or_else(|| {
                    CodegenError::General("Generic call outside function context".to_string())
                })?;

                // Monomorphize the function
                let specialized_name = self.monomorphize_function(&func_name, type_args, parent_function)?;

                // Get the specialized function
                let llvm_fn = self.module.get_function(&specialized_name)
                    .ok_or_else(|| CodegenError::General(
                        format!("Specialized function '{}' not found after monomorphization", specialized_name)
                    ))?;

                // Compile arguments
                let mut llvm_args: Vec<BasicMetadataValueEnum> = Vec::new();
                for arg in args {
                    let (arg_val, _arg_type) = self.compile_expr(arg)?;
                    llvm_args.push(arg_val.into());
                }

                // Call the specialized function
                let call_result = self.builder
                    .build_call(llvm_fn, &llvm_args, "generic_call")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: format!("Failed to call specialized function '{}'", specialized_name),
                        span: Some(expr.span.clone()),
                    })?;

                // Get return value
                if let Some(ret_val) = call_result.try_as_basic_value().left() {
                    // Infer return type from LLVM function signature
                    let ret_type = if let Some((_, Some(ret_types))) = self.functions.get(&specialized_name) {
                        if ret_types.len() == 1 {
                            ret_types[0].clone()
                        } else {
                            BrixType::Tuple(ret_types.clone())
                        }
                    } else {
                        BrixType::Int  // Fallback
                    };
                    Ok((ret_val, ret_type))
                } else {
                    // Void function
                    Ok((self.context.i64_type().const_int(0, false).into(), BrixType::Void))
                }
            }

            ExprKind::FieldAccess { target, field } => {
                // Special handling for struct field access on identifiers
                // We need the pointer to the struct, not the loaded value
                if let ExprKind::Identifier(var_name) = &target.kind {
                    // Check if this is a module constant access (e.g., math.pi)
                    let const_name = format!("{}.{}", var_name, field);
                    if let Some((ptr, brix_type)) = self.variables.get(&const_name) {
                        // Load the constant value
                        let loaded_val = self
                            .builder
                            .build_load(self.context.f64_type(), *ptr, &const_name)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: format!("Failed to load constant '{}'", const_name),
                                                            span: Some(expr.span.clone()),
                            })?;
                        return Ok((loaded_val, brix_type.clone()));
                    }

                    // Check if it's a struct variable - if so, get pointer from symbol table
                    if let Some((target_ptr, target_type)) = self.variables.get(var_name) {
                        if let BrixType::Struct(struct_name) = target_type {
                            // We have a struct variable - use the pointer directly
                            let struct_def = self.struct_defs.get(struct_name).ok_or_else(|| {
                                CodegenError::UndefinedSymbol {
                                    name: struct_name.clone(),
                                    context: "struct field access".to_string(),
                                    span: Some(expr.span.clone()),
                                }
                            })?;

                            // Find the field index and type
                            let (field_index, field_type) = struct_def
                                .iter()
                                .enumerate()
                                .find(|(_, (name, _, _))| name == field)
                                .map(|(idx, (_, ty, _))| (idx as u32, ty.clone()))
                                .ok_or_else(|| CodegenError::General(format!(
                                    "Struct '{}' has no field '{}'",
                                    struct_name, field
                                )))?;

                            // Get the LLVM struct type
                            let llvm_struct_type = self.struct_types.get(struct_name).ok_or_else(|| {
                                CodegenError::General(format!("Struct type '{}' not found", struct_name))
                            })?;

                            // Get pointer to the field
                            let field_ptr = self
                                .builder
                                .build_struct_gep(*llvm_struct_type, *target_ptr, field_index, "field_ptr")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_struct_gep".to_string(),
                                    details: format!(
                                        "Failed to get field '{}' pointer from struct '{}'",
                                        field, struct_name
                                    ),
                                    span: Some(expr.span.clone()),
                                })?;

                            // Load the field value
                            let field_llvm_type = self.brix_type_to_llvm(&field_type);
                            let field_val = self
                                .builder
                                .build_load(field_llvm_type, field_ptr, "load_field")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!(
                                        "Failed to load field '{}' from struct '{}'",
                                        field, struct_name
                                    ),
                                    span: Some(expr.span.clone()),
                                })?;

                            return Ok((field_val, field_type));
                        }
                    }
                }

                // For non-identifier targets (or non-struct identifiers), compile normally
                let (target_val, target_type) = self.compile_expr(target)?;

                // TODO: Handle struct field access for expressions that return structs
                // (not just identifiers). Would need to alloca + store the struct value first.
                if let BrixType::Struct(_) = &target_type {
                    return Err(CodegenError::General(
                        "Field access on struct-valued expressions not yet supported. Use a variable.".to_string()
                    ));
                }

                if target_type == BrixType::String {
                    if field == "len" {
                        let ptr = target_val.into_pointer_value();
                        let str_type = self.get_string_type();
                        let len_ptr = self
                            .builder
                            .build_struct_gep(str_type, ptr, 1, "len_ptr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_struct_gep".to_string(),
                                details: "Failed to get string len pointer".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?;
                        let len_val = self
                            .builder
                            .build_load(self.context.i64_type(), len_ptr, "len_val")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load string length".to_string(),
                                                            span: Some(expr.span.clone()),
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
                        "rows" => 1,
                        "cols" => 2,
                        "data" => 3,
                        _ => return Err(CodegenError::General(format!("unknown field '{}'", field))),
                    };

                    let field_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, target_ptr, index, "field_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_struct_gep".to_string(),
                            details: format!("Failed to get matrix field '{}' pointer", field),
                                                    span: Some(expr.span.clone()),
                        })?;

                    let val = match index {
                        1 | 2 => {
                            let v = self
                                .builder
                                .build_load(self.context.i64_type(), field_ptr, "load_field")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!("Failed to load matrix field '{}'", field),
                                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            (v, BrixType::FloatPtr)
                        }
                    };
                    return Ok(val);
                }
                eprintln!("Type error: Access field on non-matrix.");
                Err(CodegenError::MissingValue { what: "expression value".to_string(), context: "compile_expr".to_string(), span: None })
            }

            ExprKind::Index { array, indices } => {
                let (target_val, target_type) = self.compile_expr(array)?;

                // Check if indexing a tuple
                if let BrixType::Tuple(types) = &target_type {
                    // Tuple indexing: result[0], result[1], etc.
                    if indices.len() != 1 {
                        eprintln!("Error: Tuple indexing requires exactly one index");
                        return Err(CodegenError::General("compilation error".to_string()));
                    }

                    // Extract index (must be a constant integer)
                    if let ExprKind::Literal(Literal::Int(idx)) = &indices[0].kind {
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
                                                            span: Some(expr.span.clone()),
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
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "cols")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed to get matrix cols pointer".to_string(),
                                            span: Some(expr.span.clone()),
                    })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load matrix cols".to_string(),
                                            span: Some(expr.span.clone()),
                    })?
                    .into_int_value();

                // Get data pointer (field 2 for both)
                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 3, "data")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed to get matrix data pointer".to_string(),
                                            span: Some(expr.span.clone()),
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
                                            span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
                        })?;
                    self.builder
                        .build_int_add(row_offset, col_val.into_int_value(), "final_off")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_int_add".to_string(),
                            details: "Failed to compute final offset for matrix indexing".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let val = self.builder.build_load(i64_type, item_ptr, "val")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load IntMatrix element".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        let val = self.builder.build_load(f64, item_ptr, "val")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load Matrix element".to_string(),
                                                            span: Some(expr.span.clone()),
                            })?;
                        Ok((val, BrixType::Float))
                    }
                }
            }

            ExprKind::Array(elements) => {
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
                                                    span: Some(expr.span.clone()),
                        })?;
                    let new_intmatrix_ptr = call
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| CodegenError::MissingValue {
                            what: "intmatrix_new return value".to_string(),
                            context: "array literal".to_string(),
                                                    span: Some(expr.span.clone()),
                        })?
                        .into_pointer_value();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(intmatrix_type, new_intmatrix_ptr, 3, "data_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_struct_gep".to_string(),
                            details: "Failed to get IntMatrix data pointer".to_string(),
                                                    span: Some(expr.span.clone()),
                        })?;
                    let data_ptr = self
                        .builder
                        .build_load(ptr_type, data_ptr_ptr, "data_base")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed to load IntMatrix data pointer".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            self.builder
                                .build_store(elem_ptr, val.into_int_value())
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_store".to_string(),
                                    details: format!("Failed to store IntMatrix element {}", i),
                                                                    span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
                        })?;
                    let new_matrix_ptr = call
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| CodegenError::MissingValue {
                            what: "matrix_new return value".to_string(),
                            context: "array literal".to_string(),
                                                    span: Some(expr.span.clone()),
                        })?
                        .into_pointer_value();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, new_matrix_ptr, 3, "data_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_struct_gep".to_string(),
                            details: "Failed to get Matrix data pointer".to_string(),
                                                    span: Some(expr.span.clone()),
                        })?;
                    let data_ptr = self
                        .builder
                        .build_load(ptr_type, data_ptr_ptr, "data_base")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed to load Matrix data pointer".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            self.builder.build_store(elem_ptr, float_val)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_store".to_string(),
                                    details: format!("Failed to store Matrix element {}", i),
                                                                    span: Some(expr.span.clone()),
                                })?;
                        }
                    }

                    Ok((new_matrix_ptr.as_basic_value_enum(), BrixType::Matrix))
                }
            }

            ExprKind::Range { .. } => self.compile_range_expr(),

            ExprKind::ListComprehension { expr, generators } => {
                self.compile_list_comprehension(expr, generators)
            }

            ExprKind::StaticInit {
                element_type,
                dimensions,
            } => self.compile_static_init_expr(element_type, dimensions),

            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => self.compile_ternary_expr(condition, then_expr, else_expr),

            ExprKind::Match { value, arms } => {
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
                                            span: Some(expr.span.clone()),
                    })?
                    .get_parent()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_parent".to_string(),
                        details: "Current block has no parent function in match expression".to_string(),
                                            span: Some(expr.span.clone()),
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
                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;
                        self.builder.build_store(ptr, match_val)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "Failed to store match value".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?;

                        // pattern_matches AND guard
                        self.builder
                            .build_and(pattern_matches, guard_bool, "pattern_and_guard")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_and".to_string(),
                                details: "Failed to AND pattern match with guard".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
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
                                            span: Some(expr.span.clone()),
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
                                                            span: Some(expr.span.clone()),
                            })?
                            .into()
                    } else {
                        body_val
                    };

                    let current_bb = self.builder.get_insert_block().ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_insert_block".to_string(),
                        details: "No current block after match arm body".to_string(),
                                            span: Some(expr.span.clone()),
                    })?;
                    phi_incoming.push((coerced_val, current_bb));

                    // Jump to merge block
                    self.builder.build_unconditional_branch(merge_bb).map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed to branch to merge block from match arm".to_string(),
                                            span: Some(expr.span.clone()),
                    })?;
                }

                // Position at merge block and create PHI node
                self.builder.position_at_end(merge_bb);

                let final_type = result_type.ok_or_else(|| CodegenError::MissingValue {
                    what: "result type".to_string(),
                    context: "match expression has no arms or no result type".to_string(),
                                    span: Some(expr.span.clone()),
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
                                    span: Some(expr.span.clone()),
                })?;

                for (val, bb) in phi_incoming {
                    phi.add_incoming(&[(&val, bb)]);
                }

                Ok((phi.as_basic_value(), final_type))
            }

            ExprKind::Increment { expr, is_prefix } => {
                // Get the address of the l-value
                let (var_ptr, _) = self.compile_lvalue_addr(expr)?;

                // Load current value
                let current_val = self
                    .builder
                    .build_load(self.context.i64_type(), var_ptr, "load_for_inc")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load value for increment".to_string(),
                                            span: Some(expr.span.clone()),
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
                                            span: Some(expr.span.clone()),
                    })?;

                // Store new value
                self.builder.build_store(var_ptr, new_val).map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "Failed to store incremented value".to_string(),
                                    span: Some(expr.span.clone()),
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

            ExprKind::Decrement { expr, is_prefix } => {
                // Get the address of the l-value
                let (var_ptr, _) = self.compile_lvalue_addr(expr)?;

                // Load current value
                let current_val = self
                    .builder
                    .build_load(self.context.i64_type(), var_ptr, "load_for_dec")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load value for decrement".to_string(),
                                            span: Some(expr.span.clone()),
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
                                            span: Some(expr.span.clone()),
                    })?;

                // Store new value
                self.builder.build_store(var_ptr, new_val).map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "Failed to store decremented value".to_string(),
                                    span: Some(expr.span.clone()),
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

            ExprKind::FString { parts } => {
                use parser::ast::FStringPart;

                if parts.is_empty() {
                    // Empty f-string -> empty string
                    let raw_str = self
                        .builder
                        .build_global_string_ptr("", "empty_fstr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_global_string_ptr".to_string(),
                            details: "Failed to create empty f-string literal".to_string(),
                                                    span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
                        })?;
                    let val = call.try_as_basic_value().left().ok_or_else(|| CodegenError::MissingValue {
                        what: "return value from str_new".to_string(),
                        context: "empty f-string".to_string(),
                                            span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
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
                                                                    span: Some(expr.span.clone()),
                                })?;
                            call.try_as_basic_value().left().ok_or_else(|| CodegenError::MissingValue {
                                what: "return value from str_new".to_string(),
                                context: "f-string text part".to_string(),
                                                            span: Some(expr.span.clone()),
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
                                                    span: Some(expr.span.clone()),
                        })?;
                    result = call.try_as_basic_value().left().ok_or_else(|| CodegenError::MissingValue {
                        what: "return value from str_concat".to_string(),
                        context: "f-string concatenation".to_string(),
                                            span: Some(expr.span.clone()),
                    })?;
                }

                Ok((result, BrixType::String))
            }

            ExprKind::StructInit {
                struct_name,
                type_args,
                fields,
            } => self.compile_struct_init(struct_name, type_args, fields, expr),

            ExprKind::Closure(closure) => self.compile_closure(closure, expr),

            #[allow(unreachable_patterns)]
            _ => {
                eprintln!("Expression not implemented");
                Err(CodegenError::MissingValue { what: "expression value".to_string(), context: "compile_expr".to_string(), span: None })
            }
        }
    }

    // --- HELPER FUNCTIONS ---

    fn compile_input_call(&self, args: &[Expr]) -> Option<(BasicValueEnum<'ctx>, BrixType)> {
        let arg_str = if args.len() > 0 {
            if let ExprKind::Literal(Literal::String(s)) = &args[0].kind {
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Call sprintf
                self.builder
                    .build_call(
                        sprintf_fn,
                        &[buffer.into(), fmt_str.as_pointer_value().into(), val.into()],
                        "sprintf_int",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

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
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "Failed to call str_new in value_to_string".to_string(), span: None })?;
                let value = call.try_as_basic_value().left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "str_new did not return a value".to_string(),
                                            span: None,
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Call sprintf
                self.builder
                    .build_call(
                        sprintf_fn,
                        &[buffer.into(), fmt_str.as_pointer_value().into(), val.into()],
                        "sprintf_float",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let value = call.try_as_basic_value().left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "str_new did not return a value".to_string(),
                                            span: None,
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
                        .build_struct_gep(matrix_type, matrix_ptr, 1, "rows_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                    let rows = self
                        .builder
                        .build_load(i64_type, rows_ptr, "rows")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                        .into_int_value();

                    let cols_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, matrix_ptr, 2, "cols_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                    let cols = self
                        .builder
                        .build_load(i64_type, cols_ptr, "cols")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                        .into_int_value();

                    (rows, cols)
                };

                // Load data pointer
                let data_ptr = {
                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, matrix_ptr, 3, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                    self.builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "data_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let result_alloca = self.create_entry_block_alloca(ptr_type.into(), "array_str")?;
                let initial_str = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[open_bracket.as_pointer_value().into()],
                        "init_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                self.builder
                    .build_store(result_alloca, initial_str)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Calculate total length
                let total_len = self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;

                // Loop through elements
                let block = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_insert_block".to_string(),
                        details: "No current basic block".to_string(),
                                            span: None,
                    })?;
                let parent_fn = block.get_parent()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_parent".to_string(),
                        details: "Block has no parent function".to_string(),
                                            span: None,
                    })?;
                let loop_cond = self.context.append_basic_block(parent_fn, "array_str_cond");
                let loop_body = self.context.append_basic_block(parent_fn, "array_str_body");
                let loop_after = self
                    .context
                    .append_basic_block(parent_fn, "array_str_after");

                let idx_alloca = self.create_entry_block_alloca(i64_type.into(), "array_idx")?;
                self.builder
                    .build_store(idx_alloca, i64_type.const_int(0, false))
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                self.builder.build_unconditional_branch(loop_cond).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;

                // Condition: idx < total_len
                self.builder.position_at_end(loop_cond);
                let idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx, total_len, "cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                self.builder
                    .build_conditional_branch(cond, loop_body, loop_after)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Body: append element
                self.builder.position_at_end(loop_body);

                // Load current result string
                let current_str = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Load element
                let elem_val = if is_int {
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(i64_type, data_ptr, &[idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                        self.builder.build_load(i64_type, elem_ptr, "elem").map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?
                    }
                } else {
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(self.context.f64_type(), data_ptr, &[idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                        self.builder
                            .build_load(self.context.f64_type(), elem_ptr, "elem")
                            .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;

                // Store concatenated result
                self.builder
                    .build_store(result_alloca, concatenated)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Add comma if not last element
                let next_idx = self
                    .builder
                    .build_int_add(idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let is_not_last = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, next_idx, total_len, "is_not_last")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                let add_comma_bb = self.context.append_basic_block(parent_fn, "add_comma");
                let continue_bb = self.context.append_basic_block(parent_fn, "continue_loop");

                self.builder
                    .build_conditional_branch(is_not_last, add_comma_bb, continue_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Add comma
                self.builder.position_at_end(add_comma_bb);
                let current_with_elem = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_with_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let comma_str = self.builder.build_global_string_ptr(", ", "comma").map_err(|_| CodegenError::LLVMError { operation: "build_global_string_ptr".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;
                let comma_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[comma_str.as_pointer_value().into()],
                        "comma_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                let with_comma = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_with_elem.into(), comma_brix.into()],
                        "with_comma",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                self.builder.build_store(result_alloca, with_comma).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;
                self.builder
                    .build_unconditional_branch(continue_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Continue: increment and loop
                self.builder.position_at_end(continue_bb);
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;
                self.builder.build_unconditional_branch(loop_cond).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;

                // After loop: append "]"
                self.builder.position_at_end(loop_after);
                let final_result = self
                    .builder
                    .build_load(ptr_type, result_alloca, "final_result")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let close_bracket = self
                    .builder
                    .build_global_string_ptr("]", "close_bracket")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let close_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[close_bracket.as_pointer_value().into()],
                        "close_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                let final_str = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[final_result.into(), close_brix.into()],
                        "final_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let c_str = call.try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "Function did not return a value".to_string(),
                                            span: None,
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
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
                        .build_struct_gep(complexmatrix_type, matrix_ptr, 1, "rows_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                    let rows = self
                        .builder
                        .build_load(i64_type, rows_ptr, "rows")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                        .into_int_value();

                    let cols_ptr = self
                        .builder
                        .build_struct_gep(complexmatrix_type, matrix_ptr, 2, "cols_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                    let cols = self
                        .builder
                        .build_load(i64_type, cols_ptr, "cols")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                        .into_int_value();

                    (rows, cols)
                };

                // Load data pointer (Complex*)
                let data_ptr = {
                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(complexmatrix_type, matrix_ptr, 3, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                    self.builder
                        .build_load(ptr_type, data_ptr_ptr, "data_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let result_alloca = self.create_entry_block_alloca(ptr_type.into(), "cmatrix_str")?;
                let initial_str = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[open_bracket.as_pointer_value().into()],
                        "init_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                self.builder
                    .build_store(result_alloca, initial_str)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Calculate total length
                let total_len = self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;

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
                                            span: None,
                    })?;
                let parent_fn = block.get_parent()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "get_parent".to_string(),
                        details: "Block has no parent function".to_string(),
                                            span: None,
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                self.builder.build_unconditional_branch(loop_cond).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;

                // Condition: idx < total_len
                self.builder.position_at_end(loop_cond);
                let idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx, total_len, "cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                self.builder
                    .build_conditional_branch(cond, loop_body, loop_after)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Body: append element
                self.builder.position_at_end(loop_body);

                // Check if we're at the start of a new row (idx % cols == 0)
                let col_pos = self
                    .builder
                    .build_int_unsigned_rem(idx, cols, "col_pos")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let is_row_start = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        col_pos,
                        i64_type.const_int(0, false),
                        "is_row_start",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // If start of row, add "["
                let after_row_start_bb = self
                    .context
                    .append_basic_block(parent_fn, "after_row_start");
                let add_row_start_bb = self.context.append_basic_block(parent_fn, "add_row_start");
                self.builder
                    .build_conditional_branch(is_row_start, add_row_start_bb, after_row_start_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                self.builder.position_at_end(add_row_start_bb);
                let current_with_row_bracket = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_str_2")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let row_open = self
                    .builder
                    .build_global_string_ptr("[", "row_open")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let row_open_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[row_open.as_pointer_value().into()],
                        "row_open_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                let with_row_open = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_with_row_bracket.into(), row_open_brix.into()],
                        "with_row_open",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                self.builder
                    .build_store(result_alloca, with_row_open)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                self.builder
                    .build_unconditional_branch(after_row_start_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                self.builder.position_at_end(after_row_start_bb);
                let current_str = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_str_3")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Load Complex element (struct with 2 f64s)
                let complex_elem = unsafe {
                    let elem_ptr = self
                        .builder
                        .build_gep(complex_type, data_ptr, &[idx], "elem_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                    self.builder
                        .build_load(complex_type, elem_ptr, "complex_elem")
                        .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                };

                // Convert Complex to C string
                let call = self
                    .builder
                    .build_call(
                        complex_to_string_fn,
                        &[complex_elem.into()],
                        "complex_c_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let c_str = call.try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "Function did not return a value".to_string(),
                                            span: None,
                    })?
                    .into_pointer_value();

                // Convert C string to BrixString
                let elem_str = self
                    .builder
                    .build_call(str_new_fn, &[c_str.into()], "elem_brix_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;

                // Concatenate element
                let concatenated = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_str.into(), elem_str.into()],
                        "concat",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                self.builder
                    .build_store(result_alloca, concatenated)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Determine what to add after element: ", " or "]" or "], "
                let next_idx = self
                    .builder
                    .build_int_add(idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let is_last_elem = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, next_idx, total_len, "is_last_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Check if we're at end of row (next_idx % cols == 0)
                let next_col_pos = self
                    .builder
                    .build_int_unsigned_rem(next_idx, cols, "next_col_pos")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let is_row_end = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        next_col_pos,
                        i64_type.const_int(0, false),
                        "is_row_end",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                let add_separator_bb = self.context.append_basic_block(parent_fn, "add_separator");
                let continue_bb = self.context.append_basic_block(parent_fn, "continue_loop");

                // Skip separator if it's the very last element
                self.builder
                    .build_conditional_branch(is_last_elem, continue_bb, add_separator_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Add separator ("]" or "], " or ", ")
                self.builder.position_at_end(add_separator_bb);

                let row_end_bb = self.context.append_basic_block(parent_fn, "row_end");
                let elem_comma_bb = self.context.append_basic_block(parent_fn, "elem_comma");
                self.builder
                    .build_conditional_branch(is_row_end, row_end_bb, elem_comma_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // End of row: add "]" and maybe ", "
                self.builder.position_at_end(row_end_bb);
                let current_for_row_end = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_for_row_end")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let row_close = self
                    .builder
                    .build_global_string_ptr("]", "row_close")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let row_close_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[row_close.as_pointer_value().into()],
                        "row_close_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                let with_row_close = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_for_row_end.into(), row_close_brix.into()],
                        "with_row_close",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                self.builder
                    .build_store(result_alloca, with_row_close)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Add ", " between rows if not last row
                let current_after_bracket = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_after_bracket")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let comma_str = self.builder.build_global_string_ptr(", ", "comma").map_err(|_| CodegenError::LLVMError { operation: "build_global_string_ptr".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;
                let comma_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[comma_str.as_pointer_value().into()],
                        "comma_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                let with_comma = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_after_bracket.into(), comma_brix.into()],
                        "with_comma",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                self.builder.build_store(result_alloca, with_comma).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;
                self.builder
                    .build_unconditional_branch(continue_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Not end of row: just add ", "
                self.builder.position_at_end(elem_comma_bb);
                let current_for_comma = self
                    .builder
                    .build_load(ptr_type, result_alloca, "current_for_comma")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let elem_comma = self
                    .builder
                    .build_global_string_ptr(", ", "elem_comma")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let elem_comma_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[elem_comma.as_pointer_value().into()],
                        "elem_comma_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                let with_elem_comma = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[current_for_comma.into(), elem_comma_brix.into()],
                        "with_elem_comma",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                self.builder
                    .build_store(result_alloca, with_elem_comma)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                self.builder
                    .build_unconditional_branch(continue_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;

                // Continue: increment and loop
                self.builder.position_at_end(continue_bb);
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;
                self.builder.build_unconditional_branch(loop_cond).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "LLVM operation failed in value_to_string".to_string(), span: None })?;

                // After loop: append "]"
                self.builder.position_at_end(loop_after);
                let final_result = self
                    .builder
                    .build_load(ptr_type, result_alloca, "final_result")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let close_bracket = self
                    .builder
                    .build_global_string_ptr("]", "close_bracket")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let close_brix = self
                    .builder
                    .build_call(
                        str_new_fn,
                        &[close_bracket.as_pointer_value().into()],
                        "close_brix",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;
                let final_str = self
                    .builder
                    .build_call(
                        str_concat_fn,
                        &[final_result.into(), close_brix.into()],
                        "final_str",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;

                Ok(final_str)
            }

            BrixType::Nil => {
                // Convert nil to string "nil"
                let nil_str = self
                    .builder
                    .build_global_string_ptr("nil", "nil_str")
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let msg_char_ptr = call.try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "Function did not return a value".to_string(),
                                            span: None,
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?;
                let name_char_ptr = call.try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "Function did not return a value".to_string(),
                                            span: None,
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
                    .map_err(|_| CodegenError::LLVMError { operation: "unwrap".to_string(), details: "Failed in value_to_string".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                        .ok_or_else(|| CodegenError::LLVMError {
                            operation: "try_as_basic_value".to_string(),
                            details: "Function did not return a value".to_string(),
                                                    span: None,
                        })?;

                Ok(brix_string)
            }

            BrixType::Union(types) => {
                if types.is_empty() {
                    return Err(CodegenError::MissingValue {
                        what: "value_to_string conversion".to_string(),
                        context: "Empty Union type".to_string(),
                        span: None
                    });
                }

                // Extract tag from union (field 0)
                let tag_val = self.builder.build_extract_value(val.into_struct_value(), 0, "extract_tag")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_extract_value".to_string(),
                        details: "Failed to extract tag from union".to_string(),
                        span: None,
                    })?.into_int_value();

                // Extract value from union (field 1)
                let inner_val = self.builder.build_extract_value(val.into_struct_value(), 1, "extract_value")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_extract_value".to_string(),
                        details: "Failed to extract value from union".to_string(),
                        span: None,
                    })?;

                // Use if/else chain based on tag (simpler than switch)
                let current_fn = self.current_function.expect("No current function");
                let merge_block = self.context.append_basic_block(current_fn, "union_print_merge");

                // Allocate space for result string
                let result_ptr = self.builder.build_alloca(self.context.ptr_type(AddressSpace::default()), "union_str_result")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_alloca".to_string(),
                        details: "Failed to allocate union string result".to_string(),
                        span: None,
                    })?;

                // Build if/else chain
                for (i, typ) in types.iter().enumerate() {
                    let case_block = self.context.append_basic_block(current_fn, &format!("union_case_{}", i));
                    let next_block = if i < types.len() - 1 {
                        self.context.append_basic_block(current_fn, &format!("union_check_{}", i + 1))
                    } else {
                        merge_block
                    };

                    // Check if tag == i
                    let case_val = self.context.i64_type().const_int(i as u64, false);
                    let is_case = self.builder.build_int_compare(
                        inkwell::IntPredicate::EQ,
                        tag_val,
                        case_val,
                        "is_case"
                    ).map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed to compare union tag".to_string(),
                        span: None,
                    })?;

                    self.builder.build_conditional_branch(is_case, case_block, next_block)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_conditional_branch".to_string(),
                            details: "Failed to branch on union tag".to_string(),
                            span: None,
                        })?;

                    // Case block: convert to string
                    self.builder.position_at_end(case_block);
                    let str_val = self.value_to_string(inner_val, typ, format)?;
                    self.builder.build_store(result_ptr, str_val)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "Failed to store union string result".to_string(),
                            span: None,
                        })?;
                    self.builder.build_unconditional_branch(merge_block)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_unconditional_branch".to_string(),
                            details: "Failed to branch to merge".to_string(),
                            span: None,
                        })?;

                    // Position for next check
                    if i < types.len() - 1 {
                        self.builder.position_at_end(next_block);
                    }
                }

                // Merge block: load result
                self.builder.position_at_end(merge_block);
                let result = self.builder.build_load(self.context.ptr_type(AddressSpace::default()), result_ptr, "union_str")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load union string result".to_string(),
                        span: None,
                    })?;

                Ok(result)
            }

            BrixType::Optional(_) => {
                // Optional is now Union(T, nil), should never be reached
                panic!("Optional type should have been converted to Union")
            }

            BrixType::Intersection(_) => {
                // For Intersection, just print as struct for now
                eprintln!("value_to_string for Intersection not fully implemented");
                let str_ptr = self.builder.build_global_string_ptr("<Intersection>", "intersection_str")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_global_string_ptr".to_string(),
                        details: "Failed to create Intersection string".to_string(),
                        span: None,
                    })?;

                let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                    let ptr_type = self.context.ptr_type(AddressSpace::default());
                    let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                    self.module.add_function("str_new", fn_type, Some(Linkage::External))
                });

                let brix_string = self.builder.build_call(str_new_fn, &[str_ptr.as_pointer_value().into()], "intersection_brix_str")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call str_new".to_string(),
                        span: None,
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "str_new did not return a value".to_string(),
                        span: None,
                    })?;

                Ok(brix_string)
            }

            _ => {
                eprintln!("value_to_string not implemented for type: {:?}", typ);
                Err(CodegenError::MissingValue { what: "value_to_string conversion".to_string(), context: format!("{:?}", typ), span: None })
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
        // Struct { ref_count: i64, rows: i64, cols: i64, data: f64* }
        self.context
            .struct_type(&[i64_type.into(), i64_type.into(), i64_type.into(), ptr_type.into()], false)
    }

    fn get_intmatrix_type(&self) -> inkwell::types::StructType<'ctx> {
        // Same structure as Matrix: { ref_count: i64, rows: i64, cols: i64, data: i64* }
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        self.context
            .struct_type(&[i64_type.into(), i64_type.into(), i64_type.into(), ptr_type.into()], false)
    }

    fn get_string_type(&self) -> inkwell::types::StructType<'ctx> {
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        // Struct { ref_count: i64, len: i64, data: char* }
        self.context
            .struct_type(&[i64_type.into(), i64_type.into(), ptr_type.into()], false)
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
                            span: None,
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
                            span: None,
            })?;

        call.try_as_basic_value().left()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "try_as_basic_value".to_string(),
                details: "matrix_new did not return a value".to_string(),
                            span: None,
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
                            span: None,
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
                            span: None,
            })?;

        call.try_as_basic_value().left()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "try_as_basic_value".to_string(),
                details: "intmatrix_new did not return a value".to_string(),
                            span: None,
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
            BinaryOp::Div => {
                // Check for division by zero at runtime
                let zero = self.context.i64_type().const_int(0, false);
                let is_zero = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    rhs,
                    zero,
                    "div_check"
                ).ok()?;

                let parent_fn = self.builder.get_insert_block()?.get_parent()?;
                let then_block = self.context.append_basic_block(parent_fn, "div_zero_error");
                let else_block = self.context.append_basic_block(parent_fn, "div_ok");

                self.builder.build_conditional_branch(is_zero, then_block, else_block).ok()?;

                // Then block: call error handler and exit
                self.builder.position_at_end(then_block);
                let void_type = self.context.void_type();
                let fn_type = void_type.fn_type(&[], false);
                let error_fn = self.module.get_function("brix_division_by_zero_error")
                    .unwrap_or_else(|| {
                        self.module.add_function("brix_division_by_zero_error", fn_type, Some(inkwell::module::Linkage::External))
                    });
                self.builder.build_call(error_fn, &[], "").ok()?;
                self.builder.build_unreachable().ok()?;

                // Else block: perform division
                self.builder.position_at_end(else_block);
                Some(
                    self.builder
                        .build_int_signed_div(lhs, rhs, "tmp_div")
                        .ok()?
                        .as_basic_value_enum(),
                )
            },
            BinaryOp::Mod => {
                // Check for modulo by zero at runtime
                let zero = self.context.i64_type().const_int(0, false);
                let is_zero = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    rhs,
                    zero,
                    "mod_check"
                ).ok()?;

                let parent_fn = self.builder.get_insert_block()?.get_parent()?;
                let then_block = self.context.append_basic_block(parent_fn, "mod_zero_error");
                let else_block = self.context.append_basic_block(parent_fn, "mod_ok");

                self.builder.build_conditional_branch(is_zero, then_block, else_block).ok()?;

                // Then block: call error handler and exit
                self.builder.position_at_end(then_block);
                let void_type = self.context.void_type();
                let fn_type = void_type.fn_type(&[], false);
                let error_fn = self.module.get_function("brix_division_by_zero_error")
                    .unwrap_or_else(|| {
                        self.module.add_function("brix_division_by_zero_error", fn_type, Some(inkwell::module::Linkage::External))
                    });
                self.builder.build_call(error_fn, &[], "").ok()?;
                self.builder.build_unreachable().ok()?;

                // Else block: perform modulo
                self.builder.position_at_end(else_block);
                Some(
                    self.builder
                        .build_int_signed_rem(lhs, rhs, "tmp_mod")
                        .ok()?
                        .as_basic_value_enum(),
                )
            },
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
                            span: None,
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
                            1,
                            "rows_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get rows_ptr in list comprehension".to_string(), span: None })?;
                    let rows = self
                        .builder
                        .build_load(i64_type, rows_ptr, "rows")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load rows in list comprehension".to_string(), span: None })?
                        .into_int_value();

                    // Load cols (field 2)
                    let cols_ptr = self
                        .builder
                        .build_struct_gep(
                            if iterable_type == BrixType::Matrix {
                                self.get_matrix_type()
                            } else {
                                self.get_intmatrix_type()
                            },
                            matrix_ptr,
                            2,
                            "cols_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get cols_ptr in list comprehension".to_string(), span: None })?;
                    let cols = self
                        .builder
                        .build_load(i64_type, cols_ptr, "cols")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cols in list comprehension".to_string(), span: None })?
                        .into_int_value();

                    self.builder.build_int_mul(rows, cols, "len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute len in list comprehension".to_string(), span: None })?
                }
                _ => {
                    eprintln!(
                        "Error: List comprehension only supports Matrix/IntMatrix iterables for now"
                    );
                    return Err(CodegenError::InvalidOperation {
                        operation: "list comprehension".to_string(),
                        reason: "only supports Matrix/IntMatrix iterables for now".to_string(),
                                            span: None,
                    });
                }
            };

            total_size = self
                .builder
                .build_int_mul(total_size, len, "total_size")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_int_mul".to_string(),
                    details: "Failed to compute total size for list comprehension".to_string(),
                                    span: None,
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
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call intmatrix_new for temp array".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "intmatrix_new return value".to_string(), context: "list comprehension temp array".to_string(), span: None })?;
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
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call matrix_new for temp array".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "matrix_new return value".to_string(), context: "list comprehension temp array".to_string(), span: None })?;
                (result, BrixType::Matrix)
            }
            _ => {
                eprintln!("Error: List comprehension result type must be Int or Float for now");
                return Err(CodegenError::InvalidOperation {
                    operation: "list comprehension".to_string(),
                    reason: "result type must be Int or Float for now".to_string(),
                                    span: None,
                });
            }
        };

        // Step 4: Create counter variable
        let count_alloca = self.create_entry_block_alloca(i64_type.into(), "comp_count")?;
        self.builder
            .build_store(count_alloca, i64_type.const_int(0, false))
            .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to initialize comp_count".to_string(), span: None })?;

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
            .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load final_count".to_string(), span: None })?
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
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call intmatrix_new for result array".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "intmatrix_new return value".to_string(), context: "list comprehension result array".to_string(), span: None })?;
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
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call matrix_new for result array".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "matrix_new return value".to_string(), context: "list comprehension result array".to_string(), span: None })?;
                (result, BrixType::Matrix)
            }
            _ => unreachable!(),
        };

        // Step 8: Copy elements from temp to result
        let parent_fn = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::LLVMError { operation: "get_insert_block".to_string(), details: "no current block in list comprehension copy".to_string(), span: None })?
            .get_parent()
            .ok_or_else(|| CodegenError::LLVMError { operation: "get_parent".to_string(), details: "block has no parent function in list comprehension copy".to_string(), span: None })?;
        let copy_cond_bb = self.context.append_basic_block(parent_fn, "copy_cond");
        let copy_body_bb = self.context.append_basic_block(parent_fn, "copy_body");
        let copy_after_bb = self.context.append_basic_block(parent_fn, "copy_after");

        // Initialize copy index
        let copy_idx_alloca = self.create_entry_block_alloca(i64_type.into(), "copy_idx")?;
        self.builder
            .build_store(copy_idx_alloca, i64_type.const_int(0, false))
            .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to initialize copy_idx".to_string(), span: None })?;
        self.builder
            .build_unconditional_branch(copy_cond_bb)
            .map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to copy_cond".to_string(), span: None })?;

        // Copy condition: idx < final_count
        self.builder.position_at_end(copy_cond_bb);
        let copy_idx = self
            .builder
            .build_load(i64_type, copy_idx_alloca, "copy_idx")
            .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load copy_idx".to_string(), span: None })?
            .into_int_value();
        let copy_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, copy_idx, final_count, "copy_cond")
            .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare copy_idx < final_count".to_string(), span: None })?;
        self.builder
            .build_conditional_branch(copy_cond, copy_body_bb, copy_after_bb)
            .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch in copy loop".to_string(), span: None })?;

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
                    .build_struct_gep(matrix_type, temp_matrix_ptr, 3, "temp_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get temp data_ptr_ptr in copy loop".to_string(), span: None })?;
                let temp_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        temp_data_ptr_ptr,
                        "temp_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load temp data_ptr in copy loop".to_string(), span: None })?
                    .into_pointer_value();

                // Get result data pointer
                let result_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, result_matrix_ptr, 3, "result_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get result data_ptr_ptr in copy loop".to_string(), span: None })?;
                let result_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        result_data_ptr_ptr,
                        "result_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load result data_ptr in copy loop".to_string(), span: None })?
                    .into_pointer_value();

                // Load temp[idx]
                let temp_elem_ptr = self
                    .builder
                    .build_gep(f64_type, temp_data_ptr, &[copy_idx], "temp_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get temp elem_ptr in copy loop".to_string(), span: None })?;
                let temp_elem = self
                    .builder
                    .build_load(f64_type, temp_elem_ptr, "temp_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load temp elem in copy loop".to_string(), span: None })?;

                // Store to result[idx]
                let result_elem_ptr = self
                    .builder
                    .build_gep(f64_type, result_data_ptr, &[copy_idx], "result_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get result elem_ptr in copy loop".to_string(), span: None })?;
                self.builder
                    .build_store(result_elem_ptr, temp_elem)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store temp elem to result in copy loop".to_string(), span: None })?;
            } else {
                let matrix_type = self.get_intmatrix_type();

                // Get temp data pointer
                let temp_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, temp_matrix_ptr, 3, "temp_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get int temp data_ptr_ptr in copy loop".to_string(), span: None })?;
                let temp_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        temp_data_ptr_ptr,
                        "temp_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int temp data_ptr in copy loop".to_string(), span: None })?
                    .into_pointer_value();

                // Get result data pointer
                let result_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, result_matrix_ptr, 3, "result_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get int result data_ptr_ptr in copy loop".to_string(), span: None })?;
                let result_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        result_data_ptr_ptr,
                        "result_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int result data_ptr in copy loop".to_string(), span: None })?
                    .into_pointer_value();

                // Load temp[idx]
                let temp_elem_ptr = self
                    .builder
                    .build_gep(i64_type, temp_data_ptr, &[copy_idx], "temp_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get int temp elem_ptr in copy loop".to_string(), span: None })?;
                let temp_elem = self
                    .builder
                    .build_load(i64_type, temp_elem_ptr, "temp_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int temp elem in copy loop".to_string(), span: None })?;

                // Store to result[idx]
                let result_elem_ptr = self
                    .builder
                    .build_gep(i64_type, result_data_ptr, &[copy_idx], "result_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get int result elem_ptr in copy loop".to_string(), span: None })?;
                self.builder
                    .build_store(result_elem_ptr, temp_elem)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store int temp elem to result in copy loop".to_string(), span: None })?;
            }
        }

        // Increment copy_idx
        let next_copy_idx = self
            .builder
            .build_int_add(copy_idx, i64_type.const_int(1, false), "next_copy_idx")
            .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment copy_idx".to_string(), span: None })?;
        self.builder
            .build_store(copy_idx_alloca, next_copy_idx)
            .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next copy_idx".to_string(), span: None })?;
        self.builder
            .build_unconditional_branch(copy_cond_bb)
            .map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch back to copy_cond".to_string(), span: None })?;

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
                .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load count in comp loop".to_string(), span: None })?
                .into_int_value();

            // Get data pointer from temp_array
            let temp_matrix_ptr = temp_array.into_pointer_value();

            unsafe {
                if temp_type == BrixType::Matrix {
                    let matrix_type = self.get_matrix_type();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, temp_matrix_ptr, 3, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get data_ptr_ptr in comp loop base case".to_string(), span: None })?;
                    let data_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "data_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load data_ptr in comp loop base case".to_string(), span: None })?
                        .into_pointer_value();

                    // Convert expr_val to correct type if needed
                    let val_to_store = if expr_type == BrixType::Float {
                        expr_val
                    } else if expr_type == BrixType::Int {
                        // int -> float
                        let int_val = expr_val.into_int_value();
                        self.builder
                            .build_signed_int_to_float(int_val, f64_type, "int_to_float")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_signed_int_to_float".to_string(), details: "failed to convert int to float in comp loop".to_string(), span: None })?
                            .into()
                    } else {
                        eprintln!("Error: Type mismatch in list comprehension");
                        return Err(CodegenError::TypeError { expected: "Float or Int".to_string(), found: format!("{:?}", expr_type), context: "list comprehension expression".to_string(), span: None });
                    };

                    // Store at temp_array[count]
                    let elem_ptr = self
                        .builder
                        .build_gep(f64_type, data_ptr, &[count], "elem_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop base case".to_string(), span: None })?;
                    self.builder.build_store(elem_ptr, val_to_store).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store value in comp loop base case".to_string(), span: None })?;
                } else {
                    // IntMatrix
                    let matrix_type = self.get_intmatrix_type();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, temp_matrix_ptr, 3, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get int data_ptr_ptr in comp loop base case".to_string(), span: None })?;
                    let data_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "data_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int data_ptr in comp loop base case".to_string(), span: None })?
                        .into_pointer_value();

                    // Ensure type is Int
                    if expr_type != BrixType::Int {
                        eprintln!(
                            "Error: Type mismatch in list comprehension (expected Int for IntMatrix)"
                        );
                        return Err(CodegenError::TypeError { expected: "Int".to_string(), found: format!("{:?}", expr_type), context: "list comprehension IntMatrix expression".to_string(), span: None });
                    }

                    // Store at temp_array[count]
                    let elem_ptr = self
                        .builder
                        .build_gep(i64_type, data_ptr, &[count], "elem_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get int elem_ptr in comp loop base case".to_string(), span: None })?;
                    self.builder.build_store(elem_ptr, expr_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store int value in comp loop base case".to_string(), span: None })?;
                }
            }

            // Increment count
            let next_count = self
                .builder
                .build_int_add(count, i64_type.const_int(1, false), "next_count")
                .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment count in comp loop".to_string(), span: None })?;
            self.builder.build_store(count_alloca, next_count).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next_count in comp loop".to_string(), span: None })?;

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
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "rows_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get rows_ptr in comp loop Matrix".to_string(), span: None })?;
                let rows = self
                    .builder
                    .build_load(i64_type, rows_ptr, "rows")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load rows in comp loop Matrix".to_string(), span: None })?
                    .into_int_value();

                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "cols_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get cols_ptr in comp loop Matrix".to_string(), span: None })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cols in comp loop Matrix".to_string(), span: None })?
                    .into_int_value();

                // Load data pointer
                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 3, "data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get data_ptr_ptr in comp loop Matrix".to_string(), span: None })?;
                let data_base = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data_base",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load data_base in comp loop Matrix".to_string(), span: None })?
                    .into_pointer_value();

                // Determine if destructuring
                let (total_len, is_destructuring) = if generator.var_names.len() > 1 {
                    (rows, true)
                } else {
                    (
                        self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute total_len in comp loop Matrix".to_string(), span: None })?,
                        false,
                    )
                };

                // Create loop blocks
                let parent_fn = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_insert_block".to_string(), details: "no current block in comp loop Matrix".to_string(), span: None })?
                    .get_parent()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_parent".to_string(), details: "block has no parent in comp loop Matrix".to_string(), span: None })?;
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
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to init loop idx in comp loop Matrix".to_string(), span: None })?;

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
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to cond in comp loop Matrix".to_string(), span: None })?;

                // Condition: idx < total_len
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "cur_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cur_idx in comp loop Matrix".to_string(), span: None })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total_len, "loop_cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare idx < total_len in comp loop Matrix".to_string(), span: None })?;
                self.builder
                    .build_conditional_branch(cond, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch in comp loop Matrix".to_string(), span: None })?;

                // Body: load variables
                self.builder.position_at_end(body_bb);

                if is_destructuring {
                    // Load row elements
                    for (j, var_alloca) in var_allocas.iter().enumerate() {
                        unsafe {
                            let offset = self
                                .builder
                                .build_int_mul(cur_idx, cols, "row_offset")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute row_offset in comp loop Matrix".to_string(), span: None })?;
                            let col_offset = self
                                .builder
                                .build_int_add(
                                    offset,
                                    i64_type.const_int(j as u64, false),
                                    "elem_offset",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to compute elem_offset in comp loop Matrix".to_string(), span: None })?;

                            let elem_ptr = self
                                .builder
                                .build_gep(
                                    f64_type,
                                    data_base,
                                    &[col_offset],
                                    &format!("elem_{}_ptr", j),
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop Matrix destructuring".to_string(), span: None })?;
                            let elem_val = self
                                .builder
                                .build_load(f64_type, elem_ptr, &format!("elem_{}", j))
                                .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop Matrix destructuring".to_string(), span: None })?;
                            self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop Matrix destructuring".to_string(), span: None })?;
                        }
                    }
                } else {
                    // Load single element
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(f64_type, data_base, &[cur_idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop Matrix".to_string(), span: None })?;
                        let elem_val = self.builder.build_load(f64_type, elem_ptr, "elem").map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop Matrix".to_string(), span: None })?;
                        let current_var = self.variables.get(&generator.var_names[0]).ok_or_else(|| CodegenError::UndefinedSymbol { name: generator.var_names[0].clone(), context: "comp loop Matrix variable lookup".to_string(), span: None })?.0;
                        self.builder.build_store(current_var, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop Matrix".to_string(), span: None })?;
                    }
                }

                // Jump to check block (for conditions)
                self.builder.build_unconditional_branch(check_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to check in comp loop Matrix".to_string(), span: None })?;

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
                            .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare condition in comp loop Matrix".to_string(), span: None })?;

                        combined_cond = Some(if let Some(prev) = combined_cond {
                            self.builder
                                .build_and(prev, cond_bool, "combined_cond")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_and".to_string(), details: "failed to combine conditions in comp loop Matrix".to_string(), span: None })?
                        } else {
                            cond_bool
                        });
                    }

                    // If conditions pass, recurse to next generator or evaluate expr
                    let recurse_bb = self
                        .context
                        .append_basic_block(parent_fn, &format!("comp_recurse_{}", gen_idx));
                    let combined = combined_cond.ok_or_else(|| CodegenError::MissingValue { what: "combined_cond".to_string(), context: "comp loop Matrix conditions".to_string(), span: None })?;
                    self.builder
                        .build_conditional_branch(combined, recurse_bb, incr_bb)
                        .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch on condition in comp loop Matrix".to_string(), span: None })?;

                    self.builder.position_at_end(recurse_bb);
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr in comp loop Matrix".to_string(), span: None })?;
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
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr (no cond) in comp loop Matrix".to_string(), span: None })?;
                }

                // Increment block
                self.builder.position_at_end(incr_bb);
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment idx in comp loop Matrix".to_string(), span: None })?;
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next_idx in comp loop Matrix".to_string(), span: None })?;
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to loop back in comp loop Matrix".to_string(), span: None })?;

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
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "rows_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get rows_ptr in comp loop IntMatrix".to_string(), span: None })?;
                let rows = self
                    .builder
                    .build_load(i64_type, rows_ptr, "rows")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load rows in comp loop IntMatrix".to_string(), span: None })?
                    .into_int_value();

                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "cols_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get cols_ptr in comp loop IntMatrix".to_string(), span: None })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cols in comp loop IntMatrix".to_string(), span: None })?
                    .into_int_value();

                // Load data pointer
                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 3, "data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get data_ptr_ptr in comp loop IntMatrix".to_string(), span: None })?;
                let data_base = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data_base",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load data_base in comp loop IntMatrix".to_string(), span: None })?
                    .into_pointer_value();

                // Determine if destructuring
                let (total_len, is_destructuring) = if generator.var_names.len() > 1 {
                    (rows, true)
                } else {
                    (
                        self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute total_len in comp loop IntMatrix".to_string(), span: None })?,
                        false,
                    )
                };

                // Create loop blocks
                let parent_fn = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_insert_block".to_string(), details: "no current block in comp loop IntMatrix".to_string(), span: None })?
                    .get_parent()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_parent".to_string(), details: "block has no parent in comp loop IntMatrix".to_string(), span: None })?;
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
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to init loop idx in comp loop IntMatrix".to_string(), span: None })?;

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
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to cond in comp loop IntMatrix".to_string(), span: None })?;

                // Condition: idx < total_len
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "cur_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cur_idx in comp loop IntMatrix".to_string(), span: None })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total_len, "loop_cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare idx < total_len in comp loop IntMatrix".to_string(), span: None })?;
                self.builder
                    .build_conditional_branch(cond, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch in comp loop IntMatrix".to_string(), span: None })?;

                // Body: load variables
                self.builder.position_at_end(body_bb);

                if is_destructuring {
                    // Load row elements
                    for (j, var_alloca) in var_allocas.iter().enumerate() {
                        unsafe {
                            let offset = self
                                .builder
                                .build_int_mul(cur_idx, cols, "row_offset")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute row_offset in comp loop IntMatrix".to_string(), span: None })?;
                            let col_offset = self
                                .builder
                                .build_int_add(
                                    offset,
                                    i64_type.const_int(j as u64, false),
                                    "elem_offset",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to compute elem_offset in comp loop IntMatrix".to_string(), span: None })?;

                            let elem_ptr = self
                                .builder
                                .build_gep(
                                    i64_type,
                                    data_base,
                                    &[col_offset],
                                    &format!("elem_{}_ptr", j),
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop IntMatrix destructuring".to_string(), span: None })?;
                            let elem_val = self
                                .builder
                                .build_load(i64_type, elem_ptr, &format!("elem_{}", j))
                                .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop IntMatrix destructuring".to_string(), span: None })?;
                            self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop IntMatrix destructuring".to_string(), span: None })?;
                        }
                    }
                } else {
                    // Load single element
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(i64_type, data_base, &[cur_idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop IntMatrix".to_string(), span: None })?;
                        let elem_val = self.builder.build_load(i64_type, elem_ptr, "elem").map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop IntMatrix".to_string(), span: None })?;
                        let current_var = self.variables.get(&generator.var_names[0]).ok_or_else(|| CodegenError::UndefinedSymbol { name: generator.var_names[0].clone(), context: "comp loop IntMatrix variable lookup".to_string(), span: None })?.0;
                        self.builder.build_store(current_var, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop IntMatrix".to_string(), span: None })?;
                    }
                }

                // Jump to check block (for conditions)
                self.builder.build_unconditional_branch(check_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to check in comp loop IntMatrix".to_string(), span: None })?;

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
                            .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare condition in comp loop IntMatrix".to_string(), span: None })?;

                        combined_cond = Some(if let Some(prev) = combined_cond {
                            self.builder
                                .build_and(prev, cond_bool, "combined_cond")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_and".to_string(), details: "failed to combine conditions in comp loop IntMatrix".to_string(), span: None })?
                        } else {
                            cond_bool
                        });
                    }

                    // If conditions pass, recurse to next generator or evaluate expr
                    let recurse_bb = self
                        .context
                        .append_basic_block(parent_fn, &format!("comp_recurse_{}", gen_idx));
                    let combined = combined_cond.ok_or_else(|| CodegenError::MissingValue { what: "combined_cond".to_string(), context: "comp loop IntMatrix conditions".to_string(), span: None })?;
                    self.builder
                        .build_conditional_branch(combined, recurse_bb, incr_bb)
                        .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch on condition in comp loop IntMatrix".to_string(), span: None })?;

                    self.builder.position_at_end(recurse_bb);
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr in comp loop IntMatrix".to_string(), span: None })?;
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
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr (no cond) in comp loop IntMatrix".to_string(), span: None })?;
                }

                // Increment block
                self.builder.position_at_end(incr_bb);
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment idx in comp loop IntMatrix".to_string(), span: None })?;
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next_idx in comp loop IntMatrix".to_string(), span: None })?;
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to loop back in comp loop IntMatrix".to_string(), span: None })?;

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
                                    span: None,
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
                                                            span: None,
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
                                                            span: None,
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
                                                            span: None,
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
                                                            span: None,
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
                                                            span: None,
                            })?;
                        let literal_str = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "str_new result".to_string(),
                                context: "pattern string literal".to_string(),
                                                            span: None,
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
                                                            span: None,
                            })?;
                        let result = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "str_eq result".to_string(),
                                context: "pattern string comparison".to_string(),
                                                            span: None,
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
                                                            span: None,
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
                                                            span: None,
                            })?;
                        let pattern_atom_id = call.try_as_basic_value().left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "atom_intern result".to_string(),
                                context: "pattern atom comparison".to_string(),
                                                            span: None,
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
                                                            span: None,
                            })?;

                        Ok(cmp)
                    }
                    _ => {
                        Err(CodegenError::TypeError {
                            expected: format!("{:?}", value_type),
                            found: format!("{:?}", lit),
                            context: "Pattern literal type mismatch".to_string(),
                                                    span: None,
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
                                                    span: None,
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

    // ==========================================
    // TEST LIBRARY CODEGEN (v1.5)
    // ==========================================

    /// Convert a span byte-offset to a 1-based line number using source text.
    fn span_to_line(&self, span: &parser::ast::Span) -> u32 {
        let end = span.start.min(self.source.len());
        self.source[..end]
            .chars()
            .filter(|&c| c == '\n')
            .count() as u32
            + 1
    }

    /// Build a global string ptr for a `&str` literal (used for file/label constants).
    fn build_str_global(&self, s: &str, name: &str) -> CodegenResult<inkwell::values::PointerValue<'ctx>> {
        let gv = self.builder.build_global_string_ptr(s, name)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_global_string_ptr".to_string(),
                details: format!("Failed to create string constant '{}'", name),
                span: None,
            })?;
        Ok(gv.as_pointer_value())
    }

    /// Declare a void test matcher function on-demand.
    fn declare_test_matcher_void(
        &self,
        name: &str,
        param_types: &[inkwell::types::BasicMetadataTypeEnum<'ctx>],
    ) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function(name) { return f; }
        let fn_type = self.context.void_type().fn_type(param_types, false);
        self.module.add_function(name, fn_type, Some(inkwell::module::Linkage::External))
    }

    /// Top-level dispatcher: returns Some(result) if the expression is a test library call.
    fn try_compile_test_call(
        &mut self,
        func: &parser::ast::Expr,
        args: &[parser::ast::Expr],
        span: &parser::ast::Span,
    ) -> Option<CodegenResult<(inkwell::values::BasicValueEnum<'ctx>, BrixType)>> {
        use parser::ast::ExprKind;

        // ── Pattern A: test.expect(actual).matcher(expected) ──
        if let ExprKind::FieldAccess { target: fa_target, field: matcher_name } = &func.kind {
            // A1: test.expect(actual).matcher(expected)   (not negated)
            if let ExprKind::Call { func: inner_func, args: expect_args } = &fa_target.kind {
                if let ExprKind::FieldAccess { target: mod_target, field: expect_field } = &inner_func.kind {
                    if let ExprKind::Identifier(mod_name) = &mod_target.kind {
                        if mod_name == "test" && expect_field == "expect" && expect_args.len() == 1 {
                            let actual = expect_args[0].clone();
                            let matcher = matcher_name.clone();
                            let m_args: Vec<_> = args.to_vec();
                            return Some(self.compile_test_matcher(&actual, &matcher, &m_args, false, span));
                        }
                    }
                }
            }
            // A2: test.expect(actual).not.matcher(expected)   (negated)
            if let ExprKind::FieldAccess { target: not_target, field: not_field } = &fa_target.kind {
                if not_field == "not" {
                    if let ExprKind::Call { func: inner_func, args: expect_args } = &not_target.kind {
                        if let ExprKind::FieldAccess { target: mod_target, field: expect_field } = &inner_func.kind {
                            if let ExprKind::Identifier(mod_name) = &mod_target.kind {
                                if mod_name == "test" && expect_field == "expect" && expect_args.len() == 1 {
                                    let actual = expect_args[0].clone();
                                    let matcher = matcher_name.clone();
                                    let m_args: Vec<_> = args.to_vec();
                                    return Some(self.compile_test_matcher(&actual, &matcher, &m_args, true, span));
                                }
                            }
                        }
                    }
                }
            }
        }

        // ── Pattern B: test.describe / test.it / test.beforeAll etc. ──
        if let ExprKind::FieldAccess { target, field } = &func.kind {
            if let ExprKind::Identifier(mod_name) = &target.kind {
                if mod_name == "test" {
                    let method = field.clone();
                    let m_args: Vec<_> = args.to_vec();
                    return Some(self.compile_test_module_call(&method, &m_args, span));
                }
            }
        }

        None
    }

    /// Compile a top-level test module call: test.describe(), test.it(), test.beforeAll(), etc.
    fn compile_test_module_call(
        &mut self,
        method: &str,
        args: &[parser::ast::Expr],
        span: &parser::ast::Span,
    ) -> CodegenResult<(inkwell::values::BasicValueEnum<'ctx>, BrixType)> {
        use inkwell::AddressSpace;
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let void_type = self.context.void_type();
        let dummy_val: inkwell::values::BasicValueEnum<'ctx> =
            self.context.i64_type().const_int(0, false).into();

        match method {
            "describe" => {
                // test.describe("title", closure)
                if args.len() < 2 {
                    return Err(CodegenError::InvalidOperation {
                        operation: "test.describe".to_string(),
                        reason: "requires two arguments: (title, closure)".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let (title_val, _) = self.compile_expr(&args[0])?;
                let (closure_val, _) = self.compile_expr(&args[1])?;

                let fn_type = void_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
                let describe_fn = self.module.get_function("test_describe_start")
                    .unwrap_or_else(|| {
                        self.module.add_function("test_describe_start", fn_type, Some(inkwell::module::Linkage::External))
                    });
                self.builder.build_call(
                    describe_fn,
                    &[title_val.into(), closure_val.into()],
                    "test_describe",
                ).map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "Failed to call test_describe_start".to_string(),
                    span: Some(span.clone()),
                })?;
                Ok((dummy_val, BrixType::Nil))
            }

            "it" => {
                // test.it("title", closure)
                if args.len() < 2 {
                    return Err(CodegenError::InvalidOperation {
                        operation: "test.it".to_string(),
                        reason: "requires two arguments: (title, closure)".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let (title_val, _) = self.compile_expr(&args[0])?;
                let (closure_val, _) = self.compile_expr(&args[1])?;

                let fn_type = void_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
                let it_fn = self.module.get_function("test_it_register")
                    .unwrap_or_else(|| {
                        self.module.add_function("test_it_register", fn_type, Some(inkwell::module::Linkage::External))
                    });
                self.builder.build_call(
                    it_fn,
                    &[title_val.into(), closure_val.into()],
                    "test_it",
                ).map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "Failed to call test_it_register".to_string(),
                    span: Some(span.clone()),
                })?;
                Ok((dummy_val, BrixType::Nil))
            }

            "beforeAll" => self.compile_test_hook_register("test_before_all_register", args, span),
            "afterAll"  => self.compile_test_hook_register("test_after_all_register",  args, span),
            "beforeEach"=> self.compile_test_hook_register("test_before_each_register", args, span),
            "afterEach" => self.compile_test_hook_register("test_after_each_register",  args, span),

            _ => {
                // Unknown test method - fall through (return nil so codegen continues)
                Ok((dummy_val, BrixType::Nil))
            }
        }
    }

    /// Compile a lifecycle hook registration call.
    fn compile_test_hook_register(
        &mut self,
        c_fn_name: &str,
        args: &[parser::ast::Expr],
        span: &parser::ast::Span,
    ) -> CodegenResult<(inkwell::values::BasicValueEnum<'ctx>, BrixType)> {
        use inkwell::AddressSpace;
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let void_type = self.context.void_type();
        let dummy_val: inkwell::values::BasicValueEnum<'ctx> =
            self.context.i64_type().const_int(0, false).into();

        if args.is_empty() {
            return Err(CodegenError::InvalidOperation {
                operation: c_fn_name.to_string(),
                reason: "requires one argument: (closure)".to_string(),
                span: Some(span.clone()),
            });
        }
        let (closure_val, _) = self.compile_expr(&args[0])?;

        let fn_type = void_type.fn_type(&[ptr_type.into()], false);
        let hook_fn = self.module.get_function(c_fn_name)
            .unwrap_or_else(|| {
                self.module.add_function(c_fn_name, fn_type, Some(inkwell::module::Linkage::External))
            });
        self.builder.build_call(hook_fn, &[closure_val.into()], "hook_reg")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: format!("Failed to call {}", c_fn_name),
                span: Some(span.clone()),
            })?;
        Ok((dummy_val, BrixType::Nil))
    }

    /// Compile a test matcher call: test.expect(actual).matcher(args).
    /// `negated` = true for `test.expect(x).not.matcher(y)`.
    fn compile_test_matcher(
        &mut self,
        actual_expr: &parser::ast::Expr,
        matcher_name: &str,
        matcher_args: &[parser::ast::Expr],
        negated: bool,
        span: &parser::ast::Span,
    ) -> CodegenResult<(inkwell::values::BasicValueEnum<'ctx>, BrixType)> {
        use inkwell::AddressSpace;
        use inkwell::types::BasicMetadataTypeEnum;

        let ptr_type  = self.context.ptr_type(AddressSpace::default());
        let i64_type  = self.context.i64_type();
        let f64_type  = self.context.f64_type();
        let i32_type  = self.context.i32_type();
        let dummy_val: inkwell::values::BasicValueEnum<'ctx> = i64_type.const_int(0, false).into();

        let (actual_val, actual_type) = self.compile_expr(actual_expr)?;

        // Prepare file/line arguments
        let filename = self.filename.clone();
        let line_no  = self.span_to_line(span);
        let file_ptr = self.build_str_global(&filename, "tf")?;
        let line_val = i32_type.const_int(line_no as u64, false);

        let not_prefix = if negated { "not_" } else { "" };

        match matcher_name {
            // ──────────────────────────────────────────────────────────────
            "toBe" => {
                if matcher_args.is_empty() {
                    return Err(CodegenError::InvalidOperation {
                        operation: "toBe".to_string(),
                        reason: "requires one argument".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let (expected_val, expected_type) = self.compile_expr(&matcher_args[0])?;

                match &actual_type {
                    BrixType::Int | BrixType::Atom => {
                        let fn_name = format!("test_expect_{}toBe_int", not_prefix);
                        let exp = if expected_type == BrixType::Float {
                            // truncate to i64 for comparison
                            self.builder.build_float_to_signed_int(
                                expected_val.into_float_value(), i64_type, "f2i"
                            ).map_err(|_| CodegenError::LLVMError {
                                operation: "build_float_to_signed_int".to_string(),
                                details: "".to_string(), span: Some(span.clone()),
                            })?.into()
                        } else { expected_val };
                        let f = self.declare_test_matcher_void(
                            &fn_name,
                            &[i64_type.into(), i64_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        self.builder.build_call(f, &[actual_val.into(), exp.into(), file_ptr.into(), line_val.into()], "tbm")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                    }
                    BrixType::Float => {
                        let fn_name = format!("test_expect_{}toBe_float", not_prefix);
                        let exp = if expected_type == BrixType::Int {
                            self.builder.build_signed_int_to_float(
                                expected_val.into_int_value(), f64_type, "i2f"
                            ).map_err(|_| CodegenError::LLVMError {
                                operation: "build_signed_int_to_float".to_string(),
                                details: "".to_string(), span: Some(span.clone()),
                            })?.into()
                        } else { expected_val };
                        let f = self.declare_test_matcher_void(
                            &fn_name,
                            &[f64_type.into(), f64_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        self.builder.build_call(f, &[actual_val.into(), exp.into(), file_ptr.into(), line_val.into()], "tbm")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                    }
                    BrixType::String => {
                        let fn_name = format!("test_expect_{}toBe_string", not_prefix);
                        let f = self.declare_test_matcher_void(
                            &fn_name,
                            &[ptr_type.into(), ptr_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        self.builder.build_call(f, &[actual_val.into(), expected_val.into(), file_ptr.into(), line_val.into()], "tbm")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                    }
                    _ => {
                        // Boolean or generic int-like: treat as int
                        let fn_name = format!("test_expect_{}toBe_int", not_prefix);
                        let f = self.declare_test_matcher_void(
                            &fn_name,
                            &[i64_type.into(), i64_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        let act = if actual_val.is_int_value() { actual_val } else { i64_type.const_int(0, false).into() };
                        let exp = if expected_val.is_int_value() { expected_val } else { i64_type.const_int(0, false).into() };
                        self.builder.build_call(f, &[act.into(), exp.into(), file_ptr.into(), line_val.into()], "tbm")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                    }
                }
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toEqual" => {
                if matcher_args.is_empty() {
                    return Ok((dummy_val, BrixType::Nil));
                }
                let (expected_val, _) = self.compile_expr(&matcher_args[0])?;
                let (fn_name, params): (&str, Vec<BasicMetadataTypeEnum<'ctx>>) = match &actual_type {
                    BrixType::IntMatrix =>
                        ("test_expect_toEqual_int_array",
                         vec![ptr_type.into(), ptr_type.into(), ptr_type.into(), i32_type.into()]),
                    _ =>
                        ("test_expect_toEqual_float_array",
                         vec![ptr_type.into(), ptr_type.into(), ptr_type.into(), i32_type.into()]),
                };
                let f = self.declare_test_matcher_void(fn_name, &params);
                self.builder.build_call(f, &[actual_val.into(), expected_val.into(), file_ptr.into(), line_val.into()], "teq")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.to_string(), span: Some(span.clone()) })?;
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toBeCloseTo" => {
                if matcher_args.is_empty() {
                    return Ok((dummy_val, BrixType::Nil));
                }
                let (expected_val, expected_type) = self.compile_expr(&matcher_args[0])?;
                // Ensure both are f64
                let act_f64 = if actual_type == BrixType::Int {
                    self.builder.build_signed_int_to_float(actual_val.into_int_value(), f64_type, "a2f")
                        .map_err(|_| CodegenError::LLVMError { operation: "i2f".to_string(), details: "".to_string(), span: Some(span.clone()) })?.into()
                } else { actual_val };
                let exp_f64 = if expected_type == BrixType::Int {
                    self.builder.build_signed_int_to_float(expected_val.into_int_value(), f64_type, "e2f")
                        .map_err(|_| CodegenError::LLVMError { operation: "i2f".to_string(), details: "".to_string(), span: Some(span.clone()) })?.into()
                } else { expected_val };
                let f = self.declare_test_matcher_void(
                    "test_expect_toBeCloseTo",
                    &[f64_type.into(), f64_type.into(), ptr_type.into(), i32_type.into()],
                );
                self.builder.build_call(f, &[act_f64.into(), exp_f64.into(), file_ptr.into(), line_val.into()], "tbc")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "test_expect_toBeCloseTo".to_string(), span: Some(span.clone()) })?;
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toBeTruthy" => {
                let fn_name = if negated { "test_expect_toBeFalsy" } else { "test_expect_toBeTruthy" };
                let act = if actual_val.is_int_value() { actual_val } else { i64_type.const_int(0, false).into() };
                let f = self.declare_test_matcher_void(fn_name, &[i64_type.into(), ptr_type.into(), i32_type.into()]);
                self.builder.build_call(f, &[act.into(), file_ptr.into(), line_val.into()], "tbt")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.to_string(), span: Some(span.clone()) })?;
                Ok((dummy_val, BrixType::Nil))
            }

            "toBeFalsy" => {
                let fn_name = if negated { "test_expect_toBeTruthy" } else { "test_expect_toBeFalsy" };
                let act = if actual_val.is_int_value() { actual_val } else { i64_type.const_int(0, false).into() };
                let f = self.declare_test_matcher_void(fn_name, &[i64_type.into(), ptr_type.into(), i32_type.into()]);
                self.builder.build_call(f, &[act.into(), file_ptr.into(), line_val.into()], "tbf")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.to_string(), span: Some(span.clone()) })?;
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toBeGreaterThan" | "toBeLessThan" | "toBeGreaterThanOrEqual" | "toBeLessThanOrEqual" => {
                if matcher_args.is_empty() {
                    return Ok((dummy_val, BrixType::Nil));
                }
                let (threshold_val, threshold_type) = self.compile_expr(&matcher_args[0])?;

                let use_float = actual_type == BrixType::Float || threshold_type == BrixType::Float;
                let (fn_name, params): (String, Vec<BasicMetadataTypeEnum<'ctx>>) = if use_float {
                    let suffix = match matcher_name {
                        "toBeGreaterThan"        => "toBeGreaterThan_float",
                        "toBeLessThan"           => "toBeLessThan_float",
                        "toBeGreaterThanOrEqual" => "toBeGreaterThanOrEqual_float",
                        _                        => "toBeLessThanOrEqual_float",
                    };
                    (format!("test_expect_{}", suffix),
                     vec![f64_type.into(), f64_type.into(), ptr_type.into(), i32_type.into()])
                } else {
                    let suffix = match matcher_name {
                        "toBeGreaterThan"        => "toBeGreaterThan_int",
                        "toBeLessThan"           => "toBeLessThan_int",
                        "toBeGreaterThanOrEqual" => "toBeGreaterThanOrEqual_int",
                        _                        => "toBeLessThanOrEqual_int",
                    };
                    (format!("test_expect_{}", suffix),
                     vec![i64_type.into(), i64_type.into(), ptr_type.into(), i32_type.into()])
                };

                let act_v = if use_float && actual_type == BrixType::Int {
                    self.builder.build_signed_int_to_float(actual_val.into_int_value(), f64_type, "a2f")
                        .map_err(|_| CodegenError::LLVMError { operation: "i2f".to_string(), details: "".to_string(), span: Some(span.clone()) })?.into()
                } else { actual_val };
                let thr_v = if use_float && threshold_type == BrixType::Int {
                    self.builder.build_signed_int_to_float(threshold_val.into_int_value(), f64_type, "t2f")
                        .map_err(|_| CodegenError::LLVMError { operation: "i2f".to_string(), details: "".to_string(), span: Some(span.clone()) })?.into()
                } else { threshold_val };

                let f = self.declare_test_matcher_void(&fn_name, &params);
                self.builder.build_call(f, &[act_v.into(), thr_v.into(), file_ptr.into(), line_val.into()], "tcmp")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toContain" => {
                if matcher_args.is_empty() {
                    return Ok((dummy_val, BrixType::Nil));
                }
                let (elem_val, elem_type) = self.compile_expr(&matcher_args[0])?;

                match &actual_type {
                    BrixType::String => {
                        // toContain(substring): both are strings
                        let f = self.declare_test_matcher_void(
                            "test_expect_toContain_string",
                            &[ptr_type.into(), ptr_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        self.builder.build_call(f, &[actual_val.into(), elem_val.into(), file_ptr.into(), line_val.into()], "tc")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "toContain_string".to_string(), span: Some(span.clone()) })?;
                    }
                    BrixType::IntMatrix => {
                        let f = self.declare_test_matcher_void(
                            "test_expect_toContain_int_array",
                            &[ptr_type.into(), i64_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        self.builder.build_call(f, &[actual_val.into(), elem_val.into(), file_ptr.into(), line_val.into()], "tc")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "toContain_int_array".to_string(), span: Some(span.clone()) })?;
                    }
                    BrixType::Matrix => {
                        let elem_f = if elem_type == BrixType::Int {
                            self.builder.build_signed_int_to_float(elem_val.into_int_value(), f64_type, "e2f")
                                .map_err(|_| CodegenError::LLVMError { operation: "i2f".to_string(), details: "".to_string(), span: Some(span.clone()) })?.into()
                        } else { elem_val };
                        let f = self.declare_test_matcher_void(
                            "test_expect_toContain_float_array",
                            &[ptr_type.into(), f64_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        self.builder.build_call(f, &[actual_val.into(), elem_f.into(), file_ptr.into(), line_val.into()], "tc")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "toContain_float_array".to_string(), span: Some(span.clone()) })?;
                    }
                    _ => {}
                }
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toHaveLength" => {
                if matcher_args.is_empty() {
                    return Ok((dummy_val, BrixType::Nil));
                }
                let (len_val, _) = self.compile_expr(&matcher_args[0])?;
                let len_i64 = if len_val.is_int_value() { len_val }
                              else { i64_type.const_int(0, false).into() };

                let fn_name = match &actual_type {
                    BrixType::IntMatrix => "test_expect_toHaveLength_int_array",
                    BrixType::Matrix    => "test_expect_toHaveLength_float_array",
                    _                   => "test_expect_toHaveLength_string",
                };
                let f = self.declare_test_matcher_void(fn_name,
                    &[ptr_type.into(), i64_type.into(), ptr_type.into(), i32_type.into()]);
                self.builder.build_call(f, &[actual_val.into(), len_i64.into(), file_ptr.into(), line_val.into()], "thl")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.to_string(), span: Some(span.clone()) })?;
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toBeNil" => {
                // For optional/nil: if actual is a pointer, check if null.
                // For union types: check the tag. For v1.5, use simple null check.
                let fn_name = if negated { "test_expect_not_toBeNil" } else { "test_expect_toBeNil" };
                // Encode nil-ness as i64: 1 = nil, 0 = not nil
                let is_nil = if actual_val.is_pointer_value() {
                    let null_check = self.builder.build_is_null(actual_val.into_pointer_value(), "is_nil")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_is_null".to_string(), details: "".to_string(), span: Some(span.clone()) })?;
                    self.builder.build_int_z_extend(null_check, i64_type, "nil_i64")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_int_z_extend".to_string(), details: "".to_string(), span: Some(span.clone()) })?
                        .into()
                } else if actual_val.is_int_value() {
                    // For int-based nil (e.g. union tag 0 = value, 1 = nil), use actual directly
                    actual_val
                } else {
                    i64_type.const_int(0, false).into()
                };
                let f = self.declare_test_matcher_void(fn_name,
                    &[i64_type.into(), ptr_type.into(), i32_type.into()]);
                self.builder.build_call(f, &[is_nil.into(), file_ptr.into(), line_val.into()], "tbn")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.to_string(), span: Some(span.clone()) })?;
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            _ => {
                // Unknown matcher - silently skip (don't crash the compiler)
                Ok((dummy_val, BrixType::Nil))
            }
        }
    }
}
