// Closure compilation + ARC for Brix
//
// Hosts (since refactor Extraction 6) the closure-specific codegen:
//   compile_closure (env struct + closure fn + { fn_ptr, env_ptr } value),
//   closure_retain / closure_release (closure ARC), load_closure_fn_env
//   (env extraction), infer_closure_return_type, is_closure_type.
// The general per-type ARC dispatch (is_ref_counted / insert_retain /
// insert_release / release_function_scope_vars) stays in lib.rs.
//
// Implemented as an inherent impl block on Compiler, reaching the sibling
// helpers still in lib.rs (compile_expr, symbol table, closure_analysis, etc.).

use crate::helpers::HelperFunctions;
use crate::{BrixType, CodegenError, CodegenResult, Compiler, Span};
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;
use parser::ast::{Closure, Expr, ExprKind};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    /// Returns true when the BrixType is the closure representation (Tuple(Int,Int,Int)).
    /// Closures are always heap-allocated and must be passed/stored as pointers, not by value.
    pub(crate) fn is_closure_type(t: &BrixType) -> bool {
        matches!(t, BrixType::Tuple(types)
            if types.len() == 3
                && types[0] == BrixType::Int
                && types[1] == BrixType::Int
                && types[2] == BrixType::Int)
    }

    /// Compile a closure expression
    ///
    /// Creates:
    /// 1. Environment struct with captured variables (as pointers)
    /// 2. Closure function that receives env_ptr as first parameter
    /// 3. Returns a struct { fn_ptr, env_ptr }
    pub(crate) fn compile_closure(
        &mut self,
        closure: &Closure,
        expr: &Expr,
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        if closure.is_async {
            return self.compile_async_closure(closure, expr);
        }

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
            let llvm_param_type: BasicMetadataTypeEnum = if Compiler::is_closure_type(&brix_type) {
                self.context.ptr_type(AddressSpace::default()).into()
            } else {
                self.brix_type_to_llvm(&brix_type).into()
            };
            param_types.push(llvm_param_type);
        }

        // Return type. When there's no explicit annotation, infer it from the body's
        // return statements — the SAME inference every call site (compile_iterator_method's
        // "map", compile_test_matcher's "toThrow") already performs independently to decide
        // the type it calls this closure AS. Defaulting to Void here unconditionally (as
        // before) made compile_closure() declare a `void`-returning LLVM function while its
        // body still emitted `ret <value>` for any closure with a return statement, and every
        // caller then built its indirect-call fn_type from ITS OWN inferred type instead of
        // the function's real (void) declared type. That mismatch is undefined behavior at
        // the LLVM IR level; it only "worked" because callers and the body agreed on a type
        // that the function's own `define void` header never verified against.
        // infer_return_type_from_body() returns None when there's no return statement at all
        // (a purely side-effecting closure), for which Void is still the correct answer — the
        // "no terminator -> ret void" fallback further down handles that case.
        let return_brix_type = if let Some(ret_type_str) = &closure.return_type {
            self.string_to_brix_type(ret_type_str)
        } else {
            self.infer_return_type_from_body(&closure.body, &closure.params)
                .unwrap_or(BrixType::Void)
        };

        let fn_type = if return_brix_type == BrixType::Void {
            self.context.void_type().fn_type(&param_types, false)
        } else {
            self.brix_type_to_llvm(&return_brix_type)
                .fn_type(&param_types, false)
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
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            for (i, var_name) in closure.captured_vars.iter().enumerate() {
                // Get pointer to field in environment struct
                let field_ptr = self
                    .builder
                    .build_struct_gep(env_type, env_ptr, i as u32, &format!("{}_ptr", var_name))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: format!("Failed to access captured variable {}", var_name),
                        span: Some(expr.span.clone()),
                    })?;

                // Get the type of the captured variable from outer scope
                let var_type = prev_variables
                    .get(var_name)
                    .map(|(_, t)| t.clone())
                    .ok_or_else(|| CodegenError::UndefinedSymbol {
                        name: var_name.clone(),
                        context: "closure captured variable".to_string(),
                        span: Some(expr.span.clone()),
                    })?;

                let alloca_for_var = if Compiler::is_closure_type(&var_type) {
                    // env[i] holds the retained BrixClosure* directly (capture-by-value).
                    // Load it and store into a fresh local alloca so the identifier handler
                    // can do its standard load(alloca) -> closure_retain() flow.
                    let closure_val = self
                        .builder
                        .build_load(ptr_type, field_ptr, &format!("{}_cls", var_name))
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: format!("Failed to load captured closure {}", var_name),
                            span: Some(expr.span.clone()),
                        })?
                        .into_pointer_value();
                    let local_alloca = self.create_entry_block_alloca(ptr_type.into(), var_name)?;
                    self.builder
                        .build_store(local_alloca, closure_val)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: format!(
                                "Failed to store captured closure into local alloca: {}",
                                var_name
                            ),
                            span: Some(expr.span.clone()),
                        })?;
                    local_alloca
                } else {
                    // env[i] holds &alloca (capture-by-reference for non-closure vars).
                    // Load the alloca address and use it directly.
                    self.builder
                        .build_load(ptr_type, field_ptr, var_name)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: format!("Failed to load captured variable {}", var_name),
                            span: Some(expr.span.clone()),
                        })?
                        .into_pointer_value()
                };

                // Add to current scope
                self.variables
                    .insert(var_name.clone(), (alloca_for_var, var_type));
            }
        }

        // Add closure parameters to scope
        for (i, (param_name, _param_type)) in closure.params.iter().enumerate() {
            let param_val = closure_fn.get_nth_param((i + 1) as u32).unwrap(); // +1 for env_ptr
            let param_type = &param_brix_types[i];

            // Allocate space for parameter and store it
            let param_llvm_type = if Compiler::is_closure_type(param_type) {
                self.context.ptr_type(AddressSpace::default()).into()
            } else {
                self.brix_type_to_llvm(param_type)
            };
            let param_ptr = self.create_entry_block_alloca(param_llvm_type, param_name)?;
            self.builder
                .build_store(param_ptr, param_val)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: format!("Failed to store parameter {}", param_name),
                    span: Some(expr.span.clone()),
                })?;

            self.variables
                .insert(param_name.clone(), (param_ptr, param_type.clone()));
        }

        // 5. Re-register module constants inside closure scope
        // Math constants (pi, e, tau, etc.) are stack-allocated in the outer function
        // and don't exist in the closure's LLVM function. Re-create them here.
        {
            use crate::builtins::math::MathFunctions;
            for (module_name, prefix) in self.imported_modules.clone() {
                if module_name == "math" {
                    self.register_math_constants(&prefix);
                }
            }
        }

        // 6. Compile closure body
        self.compile_stmt(&closure.body, closure_fn)?;

        // If no return was emitted and return type is void, add ret void
        if return_brix_type == BrixType::Void {
            let current_block = self.builder.get_insert_block().unwrap();
            if current_block.get_terminator().is_none() {
                self.builder
                    .build_return(None)
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
                self.module
                    .add_function("brix_malloc", malloc_fn_type, Some(Linkage::External))
            });

            // Calculate size of environment struct
            let env_size = env_type.size_of().ok_or_else(|| CodegenError::LLVMError {
                operation: "size_of".to_string(),
                details: "Failed to get size of environment struct".to_string(),
                span: Some(expr.span.clone()),
            })?;

            // Call brix_malloc(size)
            let malloc_call = self
                .builder
                .build_call(malloc_fn, &[env_size.into()], "env_malloc")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "Failed to call brix_malloc".to_string(),
                    span: Some(expr.span.clone()),
                })?;

            let env_ptr_raw = malloc_call
                .try_as_basic_value()
                .left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "brix_malloc result".to_string(),
                    context: "closure environment allocation".to_string(),
                    span: Some(expr.span.clone()),
                })?
                .into_pointer_value();

            // Cast i8* to env_type*
            let env_alloca = env_ptr_raw; // No cast needed - LLVM treats all pointers uniformly

            // Store pointers to captured variables
            // For closure-typed captures: retain the VALUE and store the BrixClosure* directly (capture-by-value).
            // For other captures: store the alloca address (capture-by-reference).
            let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
            for (i, var_name) in closure.captured_vars.iter().enumerate() {
                let (var_ptr, var_captured_type) = self
                    .variables
                    .get(var_name)
                    .map(|(p, t)| (*p, t.clone()))
                    .ok_or_else(|| CodegenError::UndefinedSymbol {
                        name: var_name.clone(),
                        context: "closure environment".to_string(),
                        span: Some(expr.span.clone()),
                    })?;

                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        env_type,
                        env_alloca,
                        i as u32,
                        &format!("{}_field", var_name),
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: format!("Failed to get field pointer for {}", var_name),
                        span: Some(expr.span.clone()),
                    })?;

                if Compiler::is_closure_type(&var_captured_type) {
                    // Capture-by-value: load the current BrixClosure* from the alloca, retain it,
                    // and store the retained ptr in env[i].
                    let closure_val = self
                        .builder
                        .build_load(ptr_type, var_ptr, &format!("{}_val", var_name))
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: format!(
                                "Failed to load closure value for capture: {}",
                                var_name
                            ),
                            span: Some(expr.span.clone()),
                        })?
                        .into_pointer_value();
                    let retained = self.closure_retain(closure_val)?;
                    self.builder.build_store(field_ptr, retained).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: format!(
                                "Failed to store retained closure ptr for {}",
                                var_name
                            ),
                            span: Some(expr.span.clone()),
                        }
                    })?;
                } else {
                    // Capture-by-reference: store the alloca address (as before).
                    self.builder.build_store(field_ptr, var_ptr).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: format!("Failed to store pointer for {}", var_name),
                            span: Some(expr.span.clone()),
                        }
                    })?;
                }
            }

            env_alloca
        } else {
            // No captured variables, use null pointer
            self.context.ptr_type(AddressSpace::default()).const_null()
        };

        // 7a. Generate env_destructor if any captured variable is itself a closure.
        // The destructor is stored in the closure struct (field 3) and called by
        // closure_release() in runtime.c before freeing the env, so that captured
        // closures are properly reference-counted instead of being raw-freed.
        let destructor_ptr: inkwell::values::PointerValue<'ctx> = {
            let ptr_type = self.context.ptr_type(AddressSpace::default());

            // Identify which captured vars are closures (by checking outer-scope types).
            // At this point self.variables holds the restored outer scope (prev_variables).
            let captured_closure_indices: Vec<usize> = closure
                .captured_vars
                .iter()
                .enumerate()
                .filter(|(_, var_name)| {
                    self.variables
                        .get(var_name.as_str())
                        .map(|(_, t)| Compiler::is_closure_type(t))
                        .unwrap_or(false)
                })
                .map(|(i, _)| i)
                .collect();

            if !captured_closure_indices.is_empty() {
                let destructor_name = format!("{}_env_dtor", closure_name);
                let destructor_fn_type =
                    self.context.void_type().fn_type(&[ptr_type.into()], false);
                let destructor_fn =
                    self.module
                        .add_function(&destructor_name, destructor_fn_type, None);

                // Save builder context, switch to destructor function
                let saved_fn = self.current_function;
                let saved_bb = self.builder.get_insert_block();

                let dest_entry = self.context.append_basic_block(destructor_fn, "entry");
                self.builder.position_at_end(dest_entry);

                let env_arg = destructor_fn.get_nth_param(0).unwrap().into_pointer_value();
                let env_type = env_struct_type.unwrap();

                // Declare closure_release: void closure_release(void*)
                let release_fn_type = self.context.void_type().fn_type(&[ptr_type.into()], false);
                let release_fn = self
                    .module
                    .get_function("closure_release")
                    .unwrap_or_else(|| {
                        self.module.add_function(
                            "closure_release",
                            release_fn_type,
                            Some(Linkage::External),
                        )
                    });

                for field_idx in &captured_closure_indices {
                    // env[field_idx] now holds the retained BrixClosure* directly (capture-by-value).
                    // Single load is sufficient — no double dereference.
                    let field_ptr = self
                        .builder
                        .build_struct_gep(env_type, env_arg, *field_idx as u32, "cls_field_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_struct_gep".to_string(),
                            details: "destructor: gep env field".to_string(),
                            span: Some(expr.span.clone()),
                        })?;
                    let closure_val = self
                        .builder
                        .build_load(ptr_type, field_ptr, "cls_val")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "destructor: load closure ptr".to_string(),
                            span: Some(expr.span.clone()),
                        })?
                        .into_pointer_value();
                    self.builder
                        .build_call(release_fn, &[closure_val.into()], "")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_call".to_string(),
                            details: "destructor: closure_release".to_string(),
                            span: Some(expr.span.clone()),
                        })?;
                }

                self.builder
                    .build_return(None)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_return".to_string(),
                        details: "destructor: ret void".to_string(),
                        span: Some(expr.span.clone()),
                    })?;

                // Restore builder context
                self.current_function = saved_fn;
                if let Some(bb) = saved_bb {
                    self.builder.position_at_end(bb);
                }

                destructor_fn.as_global_value().as_pointer_value()
            } else {
                // No captured closures → null destructor
                self.context.ptr_type(AddressSpace::default()).const_null()
            }
        };

        // 7. Create closure struct { ref_count, fn_ptr, env_ptr, env_destructor } on HEAP
        // ARC: ref_count tracks how many references exist to this closure
        // env_destructor is a function pointer (or null) that releases captured closures
        let closure_struct_type = self.context.struct_type(
            &[
                self.context.i64_type().into(), // field 0: ref_count
                self.context.ptr_type(AddressSpace::default()).into(), // field 1: fn_ptr
                self.context.ptr_type(AddressSpace::default()).into(), // field 2: env_ptr
                self.context.ptr_type(AddressSpace::default()).into(), // field 3: env_destructor
            ],
            false,
        );

        // Allocate closure struct on heap (for ARC to work)
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let malloc_fn_type = ptr_type.fn_type(&[i64_type.into()], false);
        let malloc_fn = self.module.get_function("brix_malloc").unwrap_or_else(|| {
            self.module
                .add_function("brix_malloc", malloc_fn_type, Some(Linkage::External))
        });

        let closure_size =
            closure_struct_type
                .size_of()
                .ok_or_else(|| CodegenError::LLVMError {
                    operation: "size_of".to_string(),
                    details: "Failed to get size of closure struct".to_string(),
                    span: Some(expr.span.clone()),
                })?;

        let closure_malloc = self
            .builder
            .build_call(malloc_fn, &[closure_size.into()], "closure_malloc")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call brix_malloc for closure".to_string(),
                span: Some(expr.span.clone()),
            })?;

        let closure_ptr = closure_malloc
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CodegenError::MissingValue {
                what: "brix_malloc result for closure".to_string(),
                context: "closure struct allocation".to_string(),
                span: Some(expr.span.clone()),
            })?
            .into_pointer_value();

        // Store ref_count = 1 (initial reference)
        let ref_count_field = self
            .builder
            .build_struct_gep(closure_struct_type, closure_ptr, 0, "ref_count_field")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get ref_count field".to_string(),
                span: Some(expr.span.clone()),
            })?;

        let one = self.context.i64_type().const_int(1, false);
        self.builder
            .build_store(ref_count_field, one)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: "Failed to store initial ref_count".to_string(),
                span: Some(expr.span.clone()),
            })?;

        // Store function pointer (field 1)
        let fn_ptr_field = self
            .builder
            .build_struct_gep(closure_struct_type, closure_ptr, 1, "fn_ptr_field")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get fn_ptr field".to_string(),
                span: Some(expr.span.clone()),
            })?;

        let fn_ptr = closure_fn.as_global_value().as_pointer_value();
        self.builder
            .build_store(fn_ptr_field, fn_ptr)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: "Failed to store function pointer".to_string(),
                span: Some(expr.span.clone()),
            })?;

        // Store environment pointer (field 2)
        let env_ptr_field = self
            .builder
            .build_struct_gep(closure_struct_type, closure_ptr, 2, "env_ptr_field")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get env_ptr field".to_string(),
                span: Some(expr.span.clone()),
            })?;

        self.builder
            .build_store(env_ptr_field, env_ptr_in_caller)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: "Failed to store environment pointer".to_string(),
                span: Some(expr.span.clone()),
            })?;

        // Store env_destructor (field 3)
        let destructor_field = self
            .builder
            .build_struct_gep(closure_struct_type, closure_ptr, 3, "destructor_field")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get env_destructor field".to_string(),
                span: Some(expr.span.clone()),
            })?;
        self.builder
            .build_store(destructor_field, destructor_ptr)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_store".to_string(),
                details: "Failed to store env_destructor".to_string(),
                span: Some(expr.span.clone()),
            })?;

        // Return the closure pointer directly (not loaded - we want the pointer)
        let closure_val = closure_ptr.into();

        // Return closure as a special type
        // For now, we'll use a Tuple type to represent closure with ARC
        Ok((
            closure_val,
            BrixType::Tuple(vec![
                BrixType::Int, // ref_count
                BrixType::Int, // fn_ptr
                BrixType::Int, // env_ptr
            ]),
        ))
    }

    // --- ARC: AUTOMATIC REFERENCE COUNTING FOR CLOSURES ---

    /// Call closure_retain() to increment ref_count
    /// Returns the same closure pointer
    pub(crate) fn closure_retain(
        &self,
        closure_ptr: PointerValue<'ctx>,
    ) -> CodegenResult<PointerValue<'ctx>> {
        // Declare closure_retain: void* closure_retain(void* closure_ptr)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let retain_fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        let retain_fn = self
            .module
            .get_function("closure_retain")
            .unwrap_or_else(|| {
                self.module
                    .add_function("closure_retain", retain_fn_type, Some(Linkage::External))
            });

        // Call closure_retain(closure_ptr)
        let result = self
            .builder
            .build_call(retain_fn, &[closure_ptr.into()], "retain_call")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "Failed to call closure_retain".to_string(),
                span: None,
            })?;

        let retained_ptr = result
            .try_as_basic_value()
            .left()
            .ok_or_else(|| CodegenError::MissingValue {
                what: "closure_retain result".to_string(),
                context: "ARC retain".to_string(),
                span: None,
            })?
            .into_pointer_value();

        Ok(retained_ptr)
    }

    /// Call closure_release() to decrement ref_count and free if zero
    pub(crate) fn closure_release(&self, closure_ptr: PointerValue<'ctx>) -> CodegenResult<()> {
        // Declare closure_release: void closure_release(void* closure_ptr)
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let void_type = self.context.void_type();
        let release_fn_type = void_type.fn_type(&[ptr_type.into()], false);
        let release_fn = self
            .module
            .get_function("closure_release")
            .unwrap_or_else(|| {
                self.module.add_function(
                    "closure_release",
                    release_fn_type,
                    Some(Linkage::External),
                )
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

    /// Infer the return type of a closure or named function used as a callback.
    pub(crate) fn infer_closure_return_type(&self, callback_expr: &Expr) -> BrixType {
        match &callback_expr.kind {
            ExprKind::Closure(c) => {
                if let Some(ret_type_str) = &c.return_type {
                    self.string_to_brix_type(ret_type_str)
                } else if let Some(inferred) = self.infer_return_type_from_body(&c.body, &c.params)
                {
                    inferred
                } else {
                    BrixType::Int
                }
            }
            ExprKind::Identifier(name) => self
                .functions
                .get(name)
                .and_then(|(_, ret)| ret.as_ref())
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or(BrixType::Int),
            _ => BrixType::Int,
        }
    }

    /// Extract (fn_ptr, env_ptr) from a closure value (pointer or struct).
    pub(crate) fn load_closure_fn_env(
        &self,
        closure_val: BasicValueEnum<'ctx>,
        span: &Span,
    ) -> CodegenResult<(
        inkwell::values::PointerValue<'ctx>,
        inkwell::values::PointerValue<'ctx>,
    )> {
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let closure_struct_type = self.context.struct_type(
            &[
                self.context.i64_type().into(),
                ptr_type.into(),
                ptr_type.into(),
            ],
            false,
        );
        let closure_struct = if closure_val.is_pointer_value() {
            self.builder
                .build_load(
                    closure_struct_type,
                    closure_val.into_pointer_value(),
                    "iter_cls",
                )
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_load".to_string(),
                    details: "Failed to load closure struct for iterator".to_string(),
                    span: Some(span.clone()),
                })?
                .into_struct_value()
        } else {
            closure_val.into_struct_value()
        };
        let fn_ptr = self
            .builder
            .build_extract_value(closure_struct, 1, "iter_fn_ptr")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_extract_value".to_string(),
                details: "Failed to extract fn_ptr from iterator closure".to_string(),
                span: Some(span.clone()),
            })?
            .into_pointer_value();
        let env_ptr = self
            .builder
            .build_extract_value(closure_struct, 2, "iter_env_ptr")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_extract_value".to_string(),
                details: "Failed to extract env_ptr from iterator closure".to_string(),
                span: Some(span.clone()),
            })?
            .into_pointer_value();
        Ok((fn_ptr, env_ptr))
    }
}
