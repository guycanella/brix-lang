// Async codegen for Brix (v1.5 Phase 2+)
//
// Hosts (since refactor Extraction 5) the async state-machine compilation:
//   compile_async_fn_def (async fn -> create_/poll_ artefacts + state struct),
//   compile_async_fn_def_nested, compile_async_closure, compile_async_block.
// The await-point handling lives inline inside these functions; the section
// banner below documents the state-struct layout.
//
// Implemented as an inherent impl block on Compiler, so it reaches the sibling
// helpers still in lib.rs (compile_expr, compile_stmt, closure analysis, etc.).

use crate::helpers::HelperFunctions;
use crate::{
    collect_all_await_points, count_awaits, extract_async_stmts, extract_await_segments, AsyncStmt,
    AwaitPoint, BrixType, CodegenError, CodegenResult, Compiler,
};
use inkwell::IntPredicate;
use parser::ast::{Closure, Expr, Stmt, StmtKind};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    // ==========================================
    // ASYNC FN CODEGEN (v1.5 Phase 2)
    // ==========================================
    //
    // Transforms `async fn name(params) -> T { body }` into three LLVM artefacts:
    //
    //   1. create_{name}(params...) -> i8*
    //      Malloc + populate a state struct; set state = 0; return pointer.
    //
    //   2. poll_{name}(i8* state_ptr) -> { i64 status, i64 value }
    //      State-machine dispatcher.
    //      - status 0 = PENDING  (awaiting a nested async call)
    //      - status 1 = READY    (value is the function's return value as i64)
    //
    //   3. For `async fn main` only: emit a call to `brix_run_to_completion`
    //      inside compile_program's main LLVM function so the runtime drives
    //      the state machine to completion before returning 0.
    //
    // State struct layout (indices into the LLVM struct type):
    //   [0]            state: i64           (current state counter)
    //   [1 .. K]       param_i              (one slot per parameter)
    //   [K+1 .. K+N]   result_i             (one slot per await result)
    //   [K+N+1]        sub_future_ptr: i8*  (only when N > 0)
    //
    // Phase 2 restriction: only recognises `var x := await f(args)` at the
    // top level of a Block body.  Nested control flow spanning an await point
    // will produce a compile error when the await expr reaches compile_expr.
    pub(crate) fn compile_async_fn_def(
        &mut self,
        name: &str,
        params: &[(String, String, Option<Expr>)],
        return_type: &Option<Vec<String>>,
        body: &Stmt,
        _parent_function: inkwell::values::FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        use inkwell::module::Linkage;
        use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
        use inkwell::values::BasicMetadataValueEnum;

        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        // ── 1. Return type ──────────────────────────────────────────────
        let ret_brix_type: BrixType = match return_type {
            None => BrixType::Void,
            Some(ts) if ts.is_empty() => BrixType::Void,
            Some(ts) => self.string_to_brix_type(&ts[0]),
        };

        // ── 2. Extract await points and segments ─────────────────────────
        let (await_points, segments) = extract_await_segments(body);
        let n_awaits = await_points.len();

        // ── 2b. Check for nested awaits (Phase 3a) ───────────────────────
        // Build the recursive AsyncStmt tree. If it contains IfAwait or WhileAwait,
        // delegate to the nested state machine compiler.
        let param_names: Vec<String> = params.iter().map(|(n, _, _)| n.clone()).collect();
        let top_stmts = match &body.kind {
            parser::ast::StmtKind::Block(s) => s.as_slice(),
            _ => std::slice::from_ref(body),
        };
        let async_stmts = extract_async_stmts(top_stmts, &param_names);
        let has_nested = async_stmts
            .iter()
            .any(|s| matches!(s, AsyncStmt::IfAwait { .. } | AsyncStmt::WhileAwait { .. }));

        // ── 3. Register as async fn ──────────────────────────────────────
        self.async_fn_names.insert(name.to_string());

        // Delegate to nested compiler if needed
        if has_nested {
            return self.compile_async_fn_def_nested(
                name,
                params,
                &ret_brix_type,
                &async_stmts,
                _parent_function,
            );
        }

        // ── 4. Build state struct type ───────────────────────────────────
        // Collect BrixTypes for parameters
        let param_brix_types: Vec<BrixType> = params
            .iter()
            .map(|(_, ts, _)| self.string_to_brix_type(ts))
            .collect();

        let mut struct_fields: Vec<BasicTypeEnum<'ctx>> = vec![i64_type.into()]; // field 0: state
        let param_field_start: usize = 1;

        for bt in &param_brix_types {
            let lt: BasicTypeEnum<'ctx> = if Compiler::is_closure_type(bt) {
                ptr_type.into()
            } else {
                self.brix_type_to_llvm(bt)
            };
            struct_fields.push(lt);
        }

        let result_field_start: usize = param_field_start + params.len();

        // Await result types — the awaited functions must already be registered
        let await_ret_types: Vec<BrixType> = await_points
            .iter()
            .map(|ap| {
                self.functions
                    .get(&ap.callee_name)
                    .and_then(|(_, ret)| ret.as_ref())
                    .and_then(|v| v.first())
                    .cloned()
                    .unwrap_or(BrixType::Int)
            })
            .collect();

        for bt in &await_ret_types {
            struct_fields.push(self.brix_type_to_llvm(bt));
        }

        let sub_future_field: usize = result_field_start + n_awaits;
        if n_awaits > 0 {
            struct_fields.push(ptr_type.into()); // sub_future_ptr
        }

        let state_struct_type = self.context.struct_type(&struct_fields, false);

        // ── A. Generate create_{name}(params...) -> i8* ──────────────────
        let create_name_str = format!("create_{}", name);
        {
            let create_param_types: Vec<BasicMetadataTypeEnum<'ctx>> = param_brix_types
                .iter()
                .map(|bt| -> BasicMetadataTypeEnum<'ctx> {
                    if Compiler::is_closure_type(bt) {
                        ptr_type.into()
                    } else {
                        self.brix_type_to_llvm(bt).into()
                    }
                })
                .collect();
            let create_fn_type = ptr_type.fn_type(&create_param_types, false);
            let create_fn = self
                .module
                .add_function(&create_name_str, create_fn_type, None);
            self.async_create_fns.insert(name.to_string(), create_fn);

            let saved_fn = self.current_function;
            self.current_function = Some(create_fn);
            let entry_bb = self.context.append_basic_block(create_fn, "entry");
            self.builder.position_at_end(entry_bb);

            // malloc(sizeof state_struct)
            let malloc_fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            let malloc_fn = self.module.get_function("brix_malloc").unwrap_or_else(|| {
                self.module
                    .add_function("brix_malloc", malloc_fn_type, Some(Linkage::External))
            });
            let struct_size =
                state_struct_type
                    .size_of()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "size_of".to_string(),
                        details: "Failed to get size of async state struct".to_string(),
                        span: None,
                    })?;
            let malloc_call = self
                .builder
                .build_call(malloc_fn, &[struct_size.into()], "sp")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "malloc state".to_string(),
                    span: None,
                })?;
            let state_ptr = malloc_call
                .try_as_basic_value()
                .left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "malloc".to_string(),
                    context: "create_async".to_string(),
                    span: None,
                })?
                .into_pointer_value();

            // state = 0
            let f0 = self
                .builder
                .build_struct_gep(state_struct_type, state_ptr, 0, "sf0")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "struct_gep".to_string(),
                    details: "".to_string(),
                    span: None,
                })?;
            self.builder
                .build_store(f0, i64_type.const_int(0, false))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "state=0".to_string(),
                    span: None,
                })?;

            // store params
            for (i, _) in params.iter().enumerate() {
                let pv = create_fn.get_nth_param(i as u32).unwrap();
                let fi = self
                    .builder
                    .build_struct_gep(
                        state_struct_type,
                        state_ptr,
                        (param_field_start + i) as u32,
                        &format!("pf{}", i),
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: format!("pf{}", i),
                        span: None,
                    })?;
                self.builder
                    .build_store(fi, pv)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: format!("param {}", i),
                        span: None,
                    })?;
            }

            self.builder
                .build_return(Some(&state_ptr))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "create_async".to_string(),
                    span: None,
                })?;

            self.current_function = saved_fn;
            if let Some(prev) = saved_fn {
                if let Some(bb) = prev.get_last_basic_block() {
                    self.builder.position_at_end(bb);
                }
            }
        }

        // ── B. Generate poll_{name}(i8* sp) -> {i64, i64} ───────────────
        let poll_name_str = format!("poll_{}", name);
        let poll_result_type = self
            .context
            .struct_type(&[i64_type.into(), i64_type.into()], false);
        let poll_fn_type = poll_result_type.fn_type(&[ptr_type.into()], false);
        let poll_fn = self.module.add_function(&poll_name_str, poll_fn_type, None);
        self.async_poll_fns.insert(name.to_string(), poll_fn);

        // Register in functions map (for `await name(args)` at call sites)
        let reg_ret = if ret_brix_type == BrixType::Void {
            None
        } else {
            Some(vec![ret_brix_type.clone()])
        };
        self.functions.insert(name.to_string(), (poll_fn, reg_ret));

        {
            let saved_fn = self.current_function;
            let saved_vars = self.variables.clone();
            let saved_scope = self.function_scope_vars.clone();
            self.current_function = Some(poll_fn);
            self.function_scope_vars.clear();

            let entry_bb = self.context.append_basic_block(poll_fn, "entry");
            self.builder.position_at_end(entry_bb);

            let sp = poll_fn.get_nth_param(0).unwrap().into_pointer_value();

            // Pre-allocate all allocas in the entry block (LLVM requirement)
            let param_allocas: Vec<inkwell::values::PointerValue<'ctx>> = {
                let mut v = Vec::new();
                for (i, (pname, _, _)) in params.iter().enumerate() {
                    let lt: BasicTypeEnum<'ctx> = if Compiler::is_closure_type(&param_brix_types[i])
                    {
                        ptr_type.into()
                    } else {
                        self.brix_type_to_llvm(&param_brix_types[i])
                    };
                    v.push(self.create_entry_block_alloca(lt, pname)?);
                }
                v
            };
            let result_allocas: Vec<inkwell::values::PointerValue<'ctx>> = {
                let mut v = Vec::new();
                for (i, ap) in await_points.iter().enumerate() {
                    let lt = self.brix_type_to_llvm(&await_ret_types[i]);
                    v.push(self.create_entry_block_alloca(lt, &ap.result_var)?);
                }
                v
            };
            let async_ret_alloca: Option<inkwell::values::PointerValue<'ctx>> =
                if ret_brix_type != BrixType::Void {
                    Some(self.create_entry_block_alloca(
                        self.brix_type_to_llvm(&ret_brix_type),
                        "async_ret",
                    )?)
                } else {
                    None
                };

            // PENDING constant
            let pending_val = poll_result_type.const_named_struct(&[
                i64_type.const_int(0, false).into(),
                i64_type.const_int(0, false).into(),
            ]);

            // ── Inline helper: load params from struct into allocas ──────
            // (macro used to avoid borrowing issues with &mut self closures)
            macro_rules! load_params {
                () => {
                    for (i, (pname, _, _)) in params.iter().enumerate() {
                        let lt: BasicTypeEnum<'ctx> =
                            if Compiler::is_closure_type(&param_brix_types[i]) {
                                ptr_type.into()
                            } else {
                                self.brix_type_to_llvm(&param_brix_types[i])
                            };
                        let fi = self
                            .builder
                            .build_struct_gep(
                                state_struct_type,
                                sp,
                                (param_field_start + i) as u32,
                                &format!("lpf{}", i),
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: format!("lpf{}", i),
                                span: None,
                            })?;
                        let val = self.builder.build_load(lt, fi, pname).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: pname.clone(),
                                span: None,
                            }
                        })?;
                        self.builder
                            .build_store(param_allocas[i], val)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: pname.clone(),
                                span: None,
                            })?;
                        self.variables.insert(
                            pname.clone(),
                            (param_allocas[i], param_brix_types[i].clone()),
                        );
                    }
                };
            }

            // ── Inline helper: build READY { 1, value } and return ───────
            macro_rules! return_ready {
                ($val:expr) => {{
                    let rs0 = self
                        .builder
                        .build_insert_value(
                            poll_result_type.get_undef(),
                            i64_type.const_int(1, false),
                            0,
                            "rs0",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "insert_value".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;
                    let rs1 = self
                        .builder
                        .build_insert_value(rs0.into_struct_value(), $val, 1, "rs1")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "insert_value".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_return(Some(&rs1))
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "READY".to_string(),
                            span: None,
                        })?;
                }};
            }

            // ── CASE A: No awaits — compile body inline, wrap result ──────
            if n_awaits == 0 {
                let ready_bb = self.context.append_basic_block(poll_fn, "ready");
                load_params!();

                // Compile stmts, intercepting top-level Return
                let mut reached_return = false;
                for stmt in &segments[0] {
                    if let StmtKind::Return { values } = &stmt.kind {
                        if let Some(ra) = async_ret_alloca {
                            if let Some(ret_expr) = values.first() {
                                let (val, _) = self.compile_expr(ret_expr)?;
                                self.builder.build_store(ra, val).map_err(|_| {
                                    CodegenError::LLVMError {
                                        operation: "build_store".to_string(),
                                        details: "ret val".to_string(),
                                        span: None,
                                    }
                                })?;
                            }
                        }
                        self.builder
                            .build_unconditional_branch(ready_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "branch".to_string(),
                                details: "to ready".to_string(),
                                span: None,
                            })?;
                        reached_return = true;
                        break;
                    }
                    self.compile_stmt(stmt, poll_fn)?;
                }
                if !reached_return {
                    if let Some(bb) = self.builder.get_insert_block() {
                        if bb.get_terminator().is_none() {
                            self.builder
                                .build_unconditional_branch(ready_bb)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "branch".to_string(),
                                    details: "fallthrough ready".to_string(),
                                    span: None,
                                })?;
                        }
                    }
                }

                // ready_bb: return READY
                self.builder.position_at_end(ready_bb);
                let ret_i64 = if let Some(ra) = async_ret_alloca {
                    let lt = self.brix_type_to_llvm(&ret_brix_type);
                    self.builder
                        .build_load(lt, ra, "rv")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "rv".to_string(),
                            span: None,
                        })?
                        .into_int_value()
                } else {
                    i64_type.const_int(0, false)
                };
                return_ready!(ret_i64);
            } else {
                // ── CASE B: N awaits — switch-based state machine ─────────

                // Load state from struct[0]
                let sf0 = self
                    .builder
                    .build_struct_gep(state_struct_type, sp, 0, "sf0")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: "state".to_string(),
                        span: None,
                    })?;
                let state_val = self
                    .builder
                    .build_load(i64_type, sf0, "state")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "state".to_string(),
                        span: None,
                    })?
                    .into_int_value();

                // Create state basic blocks + default
                let default_bb = self.context.append_basic_block(poll_fn, "sw_default");
                let state_bbs: Vec<inkwell::basic_block::BasicBlock<'ctx>> = (0..=n_awaits)
                    .map(|i| {
                        self.context
                            .append_basic_block(poll_fn, &format!("state_{}", i))
                    })
                    .collect();

                let cases: Vec<(
                    inkwell::values::IntValue<'ctx>,
                    inkwell::basic_block::BasicBlock<'ctx>,
                )> = state_bbs
                    .iter()
                    .enumerate()
                    .map(|(i, bb)| (i64_type.const_int(i as u64, false), *bb))
                    .collect();
                self.builder
                    .build_switch(state_val, default_bb, &cases)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_switch".to_string(),
                        details: "state machine".to_string(),
                        span: None,
                    })?;

                // default → PENDING (unreachable in well-formed programs)
                self.builder.position_at_end(default_bb);
                self.builder.build_return(Some(&pending_val)).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_return".to_string(),
                        details: "default".to_string(),
                        span: None,
                    }
                })?;

                // ── State 0: run segments[0], launch await_points[0] ──────
                self.builder.position_at_end(state_bbs[0]);
                load_params!();

                for stmt in &segments[0] {
                    self.compile_stmt(stmt, poll_fn)?;
                }

                {
                    let ap = &await_points[0];
                    let sf_ptr = if ap.is_variable_await {
                        // Variable await: load the future from the variable (it's already an i8*)
                        let var_entry = self.variables.get(&ap.callee_name).ok_or_else(|| {
                            CodegenError::UndefinedSymbol {
                                name: ap.callee_name.clone(),
                                context: "async variable await state 0".to_string(),
                                span: None,
                            }
                        })?;
                        let (var_alloca, _var_type) = var_entry.clone();
                        self.builder
                            .build_load(ptr_type, var_alloca, "sf_var")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "sf_var".to_string(),
                                span: None,
                            })?
                            .into_pointer_value()
                    } else {
                        // Function call await: call create_{callee}(args)
                        let create_fn = self
                            .async_create_fns
                            .get(&ap.callee_name)
                            .copied()
                            .ok_or_else(|| CodegenError::UndefinedSymbol {
                                name: format!("create_{}", ap.callee_name),
                                context: "async state 0".to_string(),
                                span: None,
                            })?;
                        let mut arg_vals: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
                        for arg_expr in &ap.callee_args.clone() {
                            let (v, _) = self.compile_expr(arg_expr)?;
                            arg_vals.push(v.into());
                        }
                        let cc = self
                            .builder
                            .build_call(create_fn, &arg_vals, "sf0")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "create sub_future".to_string(),
                                span: None,
                            })?;
                        cc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "create_callee".to_string(),
                                context: "state 0".to_string(),
                                span: None,
                            })?
                            .into_pointer_value()
                    };

                    let sf_field = self
                        .builder
                        .build_struct_gep(state_struct_type, sp, sub_future_field as u32, "sff")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "sf_field".to_string(),
                            span: None,
                        })?;
                    self.builder.build_store(sf_field, sf_ptr).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "sf".to_string(),
                            span: None,
                        }
                    })?;
                }

                // state = 1
                let sf_state = self
                    .builder
                    .build_struct_gep(state_struct_type, sp, 0, "sfs")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: "state".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(sf_state, i64_type.const_int(1, false))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "state=1".to_string(),
                        span: None,
                    })?;
                self.builder.build_return(Some(&pending_val)).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_return".to_string(),
                        details: "PENDING s0".to_string(),
                        span: None,
                    }
                })?;

                // ── States 1..=N: poll previous sub_future ────────────────
                for k in 1..=n_awaits {
                    self.builder.position_at_end(state_bbs[k]);

                    // Load sub_future ptr from struct
                    let sff = self
                        .builder
                        .build_struct_gep(state_struct_type, sp, sub_future_field as u32, "sff")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "sff".to_string(),
                            span: None,
                        })?;
                    let sf_ptr = self
                        .builder
                        .build_load(ptr_type, sff, "sf")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "sf".to_string(),
                            span: None,
                        })?
                        .into_pointer_value();

                    // Call poll_{callee_{k-1}}(sf_ptr) → {status, value}
                    // For variable awaits, use indirect call via poll_fn_ptr at sub_future[0]
                    let poll_result_type_local = self
                        .context
                        .struct_type(&[i64_type.into(), i64_type.into()], false);
                    let poll_fn_type_local =
                        poll_result_type_local.fn_type(&[ptr_type.into()], false);
                    let pr = if await_points[k - 1].is_variable_await {
                        // Indirect call: load poll_fn_ptr from sub_future[0]
                        let poll_fn_ptr = self
                            .builder
                            .build_load(ptr_type, sf_ptr, "pfp")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "poll_fn_ptr".to_string(),
                                span: None,
                            })?
                            .into_pointer_value();
                        let pc = self
                            .builder
                            .build_indirect_call(
                                poll_fn_type_local,
                                poll_fn_ptr,
                                &[sf_ptr.into()],
                                "pr",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_indirect_call".to_string(),
                                details: "poll variable future".to_string(),
                                span: None,
                            })?;
                        pc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "poll_var_future".to_string(),
                                context: format!("state {}", k),
                                span: None,
                            })?
                            .into_struct_value()
                    } else {
                        let poll_callee_fn = self
                            .async_poll_fns
                            .get(&await_points[k - 1].callee_name)
                            .copied()
                            .ok_or_else(|| CodegenError::UndefinedSymbol {
                                name: format!("poll_{}", await_points[k - 1].callee_name),
                                context: format!("await state {}", k),
                                span: None,
                            })?;
                        let pc = self
                            .builder
                            .build_call(poll_callee_fn, &[sf_ptr.into()], "pr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "poll callee".to_string(),
                                span: None,
                            })?;
                        pc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "poll_callee".to_string(),
                                context: format!("state {}", k),
                                span: None,
                            })?
                            .into_struct_value()
                    };

                    // Extract status
                    let status = self
                        .builder
                        .build_extract_value(pr, 0, "pr_status")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "extract_value".to_string(),
                            details: "status".to_string(),
                            span: None,
                        })?
                        .into_int_value();

                    let is_ready = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            status,
                            i64_type.const_int(1, false),
                            "is_ready",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "int_compare".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;

                    let ready_bb = self
                        .context
                        .append_basic_block(poll_fn, &format!("ready_{}", k));
                    let pending_bb = self
                        .context
                        .append_basic_block(poll_fn, &format!("pending_{}", k));

                    self.builder
                        .build_conditional_branch(is_ready, ready_bb, pending_bb)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "cond_branch".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;

                    // pending_k → return PENDING
                    self.builder.position_at_end(pending_bb);
                    self.builder.build_return(Some(&pending_val)).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "pending".to_string(),
                            span: None,
                        }
                    })?;

                    // ready_k: store result, load vars, run segments[k]
                    self.builder.position_at_end(ready_bb);

                    let result_val = self
                        .builder
                        .build_extract_value(pr, 1, "pr_val")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "extract_value".to_string(),
                            details: "value".to_string(),
                            span: None,
                        })?
                        .into_int_value();

                    // Persist result in struct[result_field_{k-1}]
                    let rfield = self
                        .builder
                        .build_struct_gep(
                            state_struct_type,
                            sp,
                            (result_field_start + k - 1) as u32,
                            &format!("rf{}", k - 1),
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "rfield".to_string(),
                            span: None,
                        })?;
                    self.builder.build_store(rfield, result_val).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "result".to_string(),
                            span: None,
                        }
                    })?;

                    // Store result in alloca + register variable
                    self.builder
                        .build_store(result_allocas[k - 1], result_val)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "result alloca".to_string(),
                            span: None,
                        })?;
                    self.variables.insert(
                        await_points[k - 1].result_var.clone(),
                        (result_allocas[k - 1], await_ret_types[k - 1].clone()),
                    );

                    // Load params + previous results from struct into allocas/variables
                    load_params!();
                    for prev in 0..(k - 1) {
                        let lt = self.brix_type_to_llvm(&await_ret_types[prev]);
                        let fi = self
                            .builder
                            .build_struct_gep(
                                state_struct_type,
                                sp,
                                (result_field_start + prev) as u32,
                                &format!("lprf{}", prev),
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: "prev result".to_string(),
                                span: None,
                            })?;
                        let pv = self
                            .builder
                            .build_load(lt, fi, &await_points[prev].result_var)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "prev result".to_string(),
                                span: None,
                            })?;
                        self.builder
                            .build_store(result_allocas[prev], pv)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "prev result alloca".to_string(),
                                span: None,
                            })?;
                        self.variables.insert(
                            await_points[prev].result_var.clone(),
                            (result_allocas[prev], await_ret_types[prev].clone()),
                        );
                    }

                    if k == n_awaits {
                        // Final segment: intercept Return
                        let mut seg_returned = false;
                        for stmt in &segments[k] {
                            if let StmtKind::Return { values } = &stmt.kind {
                                if let Some(ra) = async_ret_alloca {
                                    if let Some(re) = values.first() {
                                        let (v, _) = self.compile_expr(re)?;
                                        self.builder.build_store(ra, v).map_err(|_| {
                                            CodegenError::LLVMError {
                                                operation: "build_store".to_string(),
                                                details: "final ret".to_string(),
                                                span: None,
                                            }
                                        })?;
                                    }
                                }
                                seg_returned = true;
                                break;
                            }
                            self.compile_stmt(stmt, poll_fn)?;
                        }

                        let ret_i64 = if let Some(ra) = async_ret_alloca {
                            let lt = self.brix_type_to_llvm(&ret_brix_type);
                            if let Some(bb) = self.builder.get_insert_block() {
                                if bb.get_terminator().is_none() || seg_returned {
                                    self.builder
                                        .build_load(lt, ra, "rv")
                                        .map_err(|_| CodegenError::LLVMError {
                                            operation: "build_load".to_string(),
                                            details: "rv".to_string(),
                                            span: None,
                                        })?
                                        .into_int_value()
                                } else {
                                    i64_type.const_int(0, false)
                                }
                            } else {
                                i64_type.const_int(0, false)
                            }
                        } else {
                            i64_type.const_int(0, false)
                        };

                        if let Some(bb) = self.builder.get_insert_block() {
                            if bb.get_terminator().is_none() {
                                return_ready!(ret_i64);
                            }
                        }
                    } else {
                        // Intermediate segment: compile, then start next sub_future
                        for stmt in &segments[k] {
                            self.compile_stmt(stmt, poll_fn)?;
                        }

                        let next_ap = &await_points[k];
                        let nsf_ptr = if next_ap.is_variable_await {
                            // Variable await: load the future from the variable
                            let var_entry =
                                self.variables.get(&next_ap.callee_name).ok_or_else(|| {
                                    CodegenError::UndefinedSymbol {
                                        name: next_ap.callee_name.clone(),
                                        context: format!("async variable await state {}", k),
                                        span: None,
                                    }
                                })?;
                            let (var_alloca, _) = var_entry.clone();
                            self.builder
                                .build_load(ptr_type, var_alloca, "nsf_var")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: "nsf_var".to_string(),
                                    span: None,
                                })?
                                .into_pointer_value()
                        } else {
                            let create_next_fn = self
                                .async_create_fns
                                .get(&next_ap.callee_name)
                                .copied()
                                .ok_or_else(|| CodegenError::UndefinedSymbol {
                                    name: format!("create_{}", next_ap.callee_name),
                                    context: format!("await state {}", k),
                                    span: None,
                                })?;
                            let args_cloned: Vec<Expr> = next_ap.callee_args.clone();
                            let mut av: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
                            for ae in &args_cloned {
                                let (v, _) = self.compile_expr(ae)?;
                                av.push(v.into());
                            }
                            let nc = self
                                .builder
                                .build_call(create_next_fn, &av, "nsf")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_call".to_string(),
                                    details: "create next sf".to_string(),
                                    span: None,
                                })?;
                            nc.try_as_basic_value()
                                .left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "create_next".to_string(),
                                    context: format!("state {}", k),
                                    span: None,
                                })?
                                .into_pointer_value()
                        };

                        let sff2 = self
                            .builder
                            .build_struct_gep(
                                state_struct_type,
                                sp,
                                sub_future_field as u32,
                                "sff2",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: "sff2".to_string(),
                                span: None,
                            })?;
                        self.builder.build_store(sff2, nsf_ptr).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "nsf".to_string(),
                                span: None,
                            }
                        })?;

                        let sfs2 = self
                            .builder
                            .build_struct_gep(state_struct_type, sp, 0, "sfs2")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: "state".to_string(),
                                span: None,
                            })?;
                        self.builder
                            .build_store(sfs2, i64_type.const_int((k + 1) as u64, false))
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "state+1".to_string(),
                                span: None,
                            })?;
                        self.builder.build_return(Some(&pending_val)).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_return".to_string(),
                                details: "PENDING inter".to_string(),
                                span: None,
                            }
                        })?;
                    }
                }
            }

            // Restore state after poll_fn generation
            self.current_function = saved_fn;
            self.variables = saved_vars;
            self.function_scope_vars = saved_scope;
            if let Some(prev) = saved_fn {
                if let Some(bb) = prev.get_last_basic_block() {
                    self.builder.position_at_end(bb);
                }
            }
        }

        // ── C. async fn main: emit drive code in compile_program's main ──
        if name == "main" {
            let prt_local = self
                .context
                .struct_type(&[i64_type.into(), i64_type.into()], false);
            let run_fn_type = prt_local.fn_type(&[ptr_type.into(), ptr_type.into()], false);
            let run_fn = self
                .module
                .get_function("brix_run_to_completion")
                .unwrap_or_else(|| {
                    self.module.add_function(
                        "brix_run_to_completion",
                        run_fn_type,
                        Some(Linkage::External),
                    )
                });

            let create_fn_ref = self.async_create_fns.get(name).copied().ok_or_else(|| {
                CodegenError::UndefinedSymbol {
                    name: create_name_str.clone(),
                    context: "async main drive".to_string(),
                    span: None,
                }
            })?;
            let poll_fn_ref = self.async_poll_fns.get(name).copied().ok_or_else(|| {
                CodegenError::UndefinedSymbol {
                    name: poll_name_str.clone(),
                    context: "async main drive".to_string(),
                    span: None,
                }
            })?;

            let cc = self
                .builder
                .build_call(create_fn_ref, &[], "main_sp")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "create_main".to_string(),
                    span: None,
                })?;
            let main_sp = cc
                .try_as_basic_value()
                .left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "create_main".to_string(),
                    context: "async main".to_string(),
                    span: None,
                })?
                .into_pointer_value();

            let poll_ptr = poll_fn_ref.as_global_value().as_pointer_value();

            self.builder
                .build_call(run_fn, &[main_sp.into(), poll_ptr.into()], "")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "brix_run_to_completion".to_string(),
                    span: None,
                })?;
        }

        Ok(())
    }

    // --- ASYNC FN WITH NESTED CONTROL FLOW (v1.6 Phase 3a) ---
    //
    // State struct layout:
    //   [0]          state: i64
    //   [1..P]       params (one per fn param)
    //   [P+1..P+V]   live vars (await results + while live vars)
    //   [P+V+1]      sub_future_ptr: i8*
    fn compile_async_fn_def_nested(
        &mut self,
        name: &str,
        params: &[(String, String, Option<Expr>)],
        ret_brix_type: &BrixType,
        async_stmts: &[AsyncStmt],
        _parent_function: inkwell::values::FunctionValue<'ctx>,
    ) -> CodegenResult<()> {
        use inkwell::module::Linkage;
        use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
        use inkwell::values::BasicMetadataValueEnum;
        use parser::ast::StmtKind;
        use std::collections::HashMap as StdHashMap;

        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        // ── 1. Collect all await points ─────────────────────────────────
        let total_awaits = count_awaits(async_stmts);
        let all_await_points = collect_all_await_points(async_stmts);

        // ── 2. Parameter types ──────────────────────────────────────────
        let param_brix_types: Vec<BrixType> = params
            .iter()
            .map(|(_, ts, _)| self.string_to_brix_type(ts))
            .collect();

        // ── 3. Build var_field_map: name → (field_index, BrixType) ──────
        let mut var_field_map: StdHashMap<String, (u32, BrixType)> = StdHashMap::new();
        let mut next_field: u32 = 1; // field 0 is state

        // Step 1: params
        for (i, (pname, _, _)) in params.iter().enumerate() {
            var_field_map.insert(pname.clone(), (next_field, param_brix_types[i].clone()));
            next_field += 1;
        }

        // Step 2: result_vars from each await
        for ap in &all_await_points {
            if !var_field_map.contains_key(&ap.result_var) {
                let vtype = if ap.is_variable_await {
                    BrixType::Int
                } else {
                    self.functions
                        .get(&ap.callee_name)
                        .and_then(|(_, ret)| ret.as_ref())
                        .and_then(|v| v.first())
                        .cloned()
                        .unwrap_or(BrixType::Int)
                };
                var_field_map.insert(ap.result_var.clone(), (next_field, vtype));
                next_field += 1;
            }
        }

        // Step 3: Collect ALL variables that need preservation across state boundaries:
        //   - live_vars from WhileAwaits
        //   - vars declared in Stmts blocks (may be used after an await in a different state)
        fn collect_all_live_vars(stmts: &[AsyncStmt], out: &mut Vec<String>) {
            for s in stmts {
                match s {
                    AsyncStmt::WhileAwait {
                        live_vars,
                        body_stmts,
                        ..
                    } => {
                        for v in live_vars {
                            if !out.contains(v) {
                                out.push(v.clone());
                            }
                        }
                        collect_all_live_vars(body_stmts, out);
                    }
                    AsyncStmt::IfAwait {
                        then_stmts,
                        else_stmts,
                        ..
                    } => {
                        collect_all_live_vars(then_stmts, out);
                        collect_all_live_vars(else_stmts, out);
                    }
                    AsyncStmt::Stmts(code) => {
                        for stmt in code {
                            if let parser::ast::StmtKind::VariableDecl { name, .. } = &stmt.kind {
                                if !out.contains(name) {
                                    out.push(name.clone());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut all_live_vars = Vec::new();
        collect_all_live_vars(async_stmts, &mut all_live_vars);

        for varname in &all_live_vars {
            if !var_field_map.contains_key(varname) {
                var_field_map.insert(varname.clone(), (next_field, BrixType::Int));
                next_field += 1;
            }
        }

        let sub_future_field: u32 = next_field;

        // ── 4. Build state struct type ──────────────────────────────────
        let mut struct_fields: Vec<BasicTypeEnum<'ctx>> = vec![i64_type.into()]; // field 0: state
        for idx in 1..next_field {
            let btype = var_field_map
                .values()
                .find(|(fi, _)| *fi == idx)
                .map(|(_, bt)| bt.clone())
                .unwrap_or(BrixType::Int);
            let lt: BasicTypeEnum<'ctx> = if Compiler::is_closure_type(&btype) {
                ptr_type.into()
            } else {
                self.brix_type_to_llvm(&btype)
            };
            struct_fields.push(lt);
        }
        struct_fields.push(ptr_type.into()); // sub_future_ptr

        let state_struct_type = self.context.struct_type(&struct_fields, false);

        // ── 5. Generate create_{name}(params...) -> i8* ─────────────────
        let create_name_str = format!("create_{}", name);
        {
            let create_param_types: Vec<BasicMetadataTypeEnum<'ctx>> = param_brix_types
                .iter()
                .map(|bt| -> BasicMetadataTypeEnum<'ctx> {
                    if Compiler::is_closure_type(bt) {
                        ptr_type.into()
                    } else {
                        self.brix_type_to_llvm(bt).into()
                    }
                })
                .collect();
            let create_fn_type = ptr_type.fn_type(&create_param_types, false);
            let create_fn = self
                .module
                .add_function(&create_name_str, create_fn_type, None);
            self.async_create_fns.insert(name.to_string(), create_fn);

            let saved_fn = self.current_function;
            self.current_function = Some(create_fn);
            let entry_bb = self.context.append_basic_block(create_fn, "entry");
            self.builder.position_at_end(entry_bb);

            let malloc_fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            let malloc_fn = self.module.get_function("brix_malloc").unwrap_or_else(|| {
                self.module
                    .add_function("brix_malloc", malloc_fn_type, Some(Linkage::External))
            });
            let struct_size =
                state_struct_type
                    .size_of()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "size_of".to_string(),
                        details: "nested async state struct".to_string(),
                        span: None,
                    })?;
            let mc = self
                .builder
                .build_call(malloc_fn, &[struct_size.into()], "sp")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "malloc".to_string(),
                    span: None,
                })?;
            let state_ptr = mc
                .try_as_basic_value()
                .left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "malloc".to_string(),
                    context: "create_async_nested".to_string(),
                    span: None,
                })?
                .into_pointer_value();

            // state = 0
            let f0 = self
                .builder
                .build_struct_gep(state_struct_type, state_ptr, 0, "sf0")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "struct_gep".to_string(),
                    details: "state=0".to_string(),
                    span: None,
                })?;
            self.builder
                .build_store(f0, i64_type.const_int(0, false))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "state=0".to_string(),
                    span: None,
                })?;

            // store params
            for (i, (pname, _, _)) in params.iter().enumerate() {
                let pv = create_fn.get_nth_param(i as u32).unwrap();
                let fi = var_field_map.get(pname).unwrap().0;
                let gep = self
                    .builder
                    .build_struct_gep(state_struct_type, state_ptr, fi, &format!("pf{}", i))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: format!("pf{}", i),
                        span: None,
                    })?;
                self.builder
                    .build_store(gep, pv)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: format!("param {}", i),
                        span: None,
                    })?;
            }

            self.builder
                .build_return(Some(&state_ptr))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "create_async_nested".to_string(),
                    span: None,
                })?;

            self.current_function = saved_fn;
            if let Some(prev) = saved_fn {
                if let Some(bb) = prev.get_last_basic_block() {
                    self.builder.position_at_end(bb);
                }
            }
        }

        // ── 6. Generate poll_{name}(i8*) -> {i64, i64} ──────────────────
        let poll_name_str = format!("poll_{}", name);
        let poll_result_type = self
            .context
            .struct_type(&[i64_type.into(), i64_type.into()], false);
        let poll_fn_type = poll_result_type.fn_type(&[ptr_type.into()], false);
        let poll_fn = self.module.add_function(&poll_name_str, poll_fn_type, None);
        self.async_poll_fns.insert(name.to_string(), poll_fn);

        let reg_ret = if *ret_brix_type == BrixType::Void {
            None
        } else {
            Some(vec![ret_brix_type.clone()])
        };
        self.functions.insert(name.to_string(), (poll_fn, reg_ret));

        {
            let saved_fn = self.current_function;
            let saved_vars = self.variables.clone();
            let saved_scope = self.function_scope_vars.clone();
            self.current_function = Some(poll_fn);
            self.function_scope_vars.clear();

            let entry_bb = self.context.append_basic_block(poll_fn, "entry");
            self.builder.position_at_end(entry_bb);

            let sp = poll_fn.get_nth_param(0).unwrap().into_pointer_value();

            // Pre-allocate allocas for all vars
            let mut var_allocas: StdHashMap<String, inkwell::values::PointerValue<'ctx>> =
                StdHashMap::new();
            for (vname, (_, vtype)) in &var_field_map {
                let lt: BasicTypeEnum<'ctx> = if Compiler::is_closure_type(vtype) {
                    ptr_type.into()
                } else {
                    self.brix_type_to_llvm(vtype)
                };
                let alloca = self.create_entry_block_alloca(lt, vname)?;
                var_allocas.insert(vname.clone(), alloca);
            }

            let async_ret_alloca: Option<inkwell::values::PointerValue<'ctx>> =
                if *ret_brix_type != BrixType::Void {
                    Some(self.create_entry_block_alloca(
                        self.brix_type_to_llvm(ret_brix_type),
                        "async_ret",
                    )?)
                } else {
                    None
                };

            let pending_val = poll_result_type.const_named_struct(&[
                i64_type.const_int(0, false).into(),
                i64_type.const_int(0, false).into(),
            ]);

            // ── Macro: save live vars to struct ─────────────────────────
            macro_rules! save_live_vars {
                () => {
                    for (vname, (field_idx, vtype)) in &var_field_map {
                        if self.variables.contains_key(vname) {
                            let lt: BasicTypeEnum<'ctx> = if Compiler::is_closure_type(vtype) {
                                ptr_type.into()
                            } else {
                                self.brix_type_to_llvm(vtype)
                            };
                            let fi = self
                                .builder
                                .build_struct_gep(
                                    state_struct_type,
                                    sp,
                                    *field_idx,
                                    &format!("sv_{}", vname),
                                )
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "struct_gep".to_string(),
                                    details: format!("save {}", vname),
                                    span: None,
                                })?;
                            let (alloca, _) = self.variables.get(vname).unwrap().clone();
                            let val = self
                                .builder
                                .build_load(lt, alloca, &format!("sv_l_{}", vname))
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!("save {}", vname),
                                    span: None,
                                })?;
                            self.builder.build_store(fi, val).map_err(|_| {
                                CodegenError::LLVMError {
                                    operation: "build_store".to_string(),
                                    details: format!("save {}", vname),
                                    span: None,
                                }
                            })?;
                        }
                    }
                };
            }

            // ── Macro: restore live vars from struct ────────────────────
            macro_rules! restore_live_vars {
                () => {
                    for (vname, (field_idx, vtype)) in &var_field_map {
                        let lt: BasicTypeEnum<'ctx> = if Compiler::is_closure_type(vtype) {
                            ptr_type.into()
                        } else {
                            self.brix_type_to_llvm(vtype)
                        };
                        let fi = self
                            .builder
                            .build_struct_gep(
                                state_struct_type,
                                sp,
                                *field_idx,
                                &format!("rv_{}", vname),
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: format!("restore {}", vname),
                                span: None,
                            })?;
                        let val = self.builder.build_load(lt, fi, vname).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: format!("restore {}", vname),
                                span: None,
                            }
                        })?;
                        let alloca = if let Some(a) = var_allocas.get(vname) {
                            *a
                        } else {
                            let a = self.create_entry_block_alloca(lt, vname)?;
                            a
                        };
                        self.builder.build_store(alloca, val).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: format!("restore {}", vname),
                                span: None,
                            }
                        })?;
                        self.variables
                            .insert(vname.clone(), (alloca, vtype.clone()));
                    }
                };
            }

            // ── Macro: return READY { 1, value } ────────────────────────
            macro_rules! return_ready {
                ($val:expr) => {{
                    let rs0 = self
                        .builder
                        .build_insert_value(
                            poll_result_type.get_undef(),
                            i64_type.const_int(1, false),
                            0,
                            "rs0",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "insert_value".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;
                    let rs1 = self
                        .builder
                        .build_insert_value(rs0.into_struct_value(), $val, 1, "rs1")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "insert_value".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_return(Some(&rs1))
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "READY".to_string(),
                            span: None,
                        })?;
                }};
            }

            // ── Inline helper: launch an await ──────────────────────────
            macro_rules! launch_await {
                ($ap:expr, $next_state:expr) => {{
                    let ap: &AwaitPoint = $ap;
                    let sf_ptr = if ap.is_variable_await {
                        let var_entry = self.variables.get(&ap.callee_name).ok_or_else(|| {
                            CodegenError::UndefinedSymbol {
                                name: ap.callee_name.clone(),
                                context: "async nested variable await".to_string(),
                                span: None,
                            }
                        })?;
                        let (var_alloca, _) = var_entry.clone();
                        self.builder
                            .build_load(ptr_type, var_alloca, "sf_var")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "sf_var".to_string(),
                                span: None,
                            })?
                            .into_pointer_value()
                    } else {
                        let create_fn = self
                            .async_create_fns
                            .get(&ap.callee_name)
                            .copied()
                            .ok_or_else(|| CodegenError::UndefinedSymbol {
                                name: format!("create_{}", ap.callee_name),
                                context: "async nested launch".to_string(),
                                span: None,
                            })?;
                        let mut arg_vals: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
                        let args_c = ap.callee_args.clone();
                        for arg_expr in &args_c {
                            let (v, _) = self.compile_expr(arg_expr)?;
                            arg_vals.push(v.into());
                        }
                        let cc = self
                            .builder
                            .build_call(create_fn, &arg_vals, "sf_launch")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "create sub_future".to_string(),
                                span: None,
                            })?;
                        cc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "create_callee".to_string(),
                                context: "async nested launch".to_string(),
                                span: None,
                            })?
                            .into_pointer_value()
                    };
                    let sff = self
                        .builder
                        .build_struct_gep(state_struct_type, sp, sub_future_field, "sff")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "sff".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_store(sff, sf_ptr)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "sf".to_string(),
                            span: None,
                        })?;

                    let sfs = self
                        .builder
                        .build_struct_gep(state_struct_type, sp, 0, "sfs")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "state".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_store(sfs, i64_type.const_int($next_state as u64, false))
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "state=next".to_string(),
                            span: None,
                        })?;
                    self.builder.build_return(Some(&pending_val)).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "PENDING launch".to_string(),
                            span: None,
                        }
                    })?;
                }};
            }

            // ── Inline helper: poll sub_future and branch ───────────────
            macro_rules! poll_sub_future {
                ($ap:expr, $ready_bb:expr, $pending_bb:expr) => {{
                    let ap: &AwaitPoint = $ap;
                    let sff = self
                        .builder
                        .build_struct_gep(state_struct_type, sp, sub_future_field, "sff")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "sff".to_string(),
                            span: None,
                        })?;
                    let sf_ptr = self
                        .builder
                        .build_load(ptr_type, sff, "sf")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "sf".to_string(),
                            span: None,
                        })?
                        .into_pointer_value();

                    let poll_result_type_local = self
                        .context
                        .struct_type(&[i64_type.into(), i64_type.into()], false);
                    let poll_fn_type_local =
                        poll_result_type_local.fn_type(&[ptr_type.into()], false);

                    let pr = if ap.is_variable_await {
                        let pfp = self
                            .builder
                            .build_load(ptr_type, sf_ptr, "pfp")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "poll_fn_ptr".to_string(),
                                span: None,
                            })?
                            .into_pointer_value();
                        let pc = self
                            .builder
                            .build_indirect_call(poll_fn_type_local, pfp, &[sf_ptr.into()], "pr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_indirect_call".to_string(),
                                details: "poll var".to_string(),
                                span: None,
                            })?;
                        pc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "poll_var".to_string(),
                                context: "nested poll".to_string(),
                                span: None,
                            })?
                            .into_struct_value()
                    } else {
                        let poll_callee = self
                            .async_poll_fns
                            .get(&ap.callee_name)
                            .copied()
                            .ok_or_else(|| CodegenError::UndefinedSymbol {
                                name: format!("poll_{}", ap.callee_name),
                                context: "nested poll".to_string(),
                                span: None,
                            })?;
                        let pc = self
                            .builder
                            .build_call(poll_callee, &[sf_ptr.into()], "pr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "poll callee".to_string(),
                                span: None,
                            })?;
                        pc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "poll_callee".to_string(),
                                context: "nested poll".to_string(),
                                span: None,
                            })?
                            .into_struct_value()
                    };

                    let status = self
                        .builder
                        .build_extract_value(pr, 0, "pr_status")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "extract_value".to_string(),
                            details: "status".to_string(),
                            span: None,
                        })?
                        .into_int_value();
                    let is_ready = self
                        .builder
                        .build_int_compare(
                            inkwell::IntPredicate::EQ,
                            status,
                            i64_type.const_int(1, false),
                            "is_ready",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "int_compare".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_conditional_branch(is_ready, $ready_bb, $pending_bb)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "cond_branch".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;

                    // pending path
                    self.builder.position_at_end($pending_bb);
                    self.builder.build_return(Some(&pending_val)).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "pending".to_string(),
                            span: None,
                        }
                    })?;

                    // ready path — store result
                    self.builder.position_at_end($ready_bb);
                    let result_val = self
                        .builder
                        .build_extract_value(pr, 1, "pr_val")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "extract_value".to_string(),
                            details: "value".to_string(),
                            span: None,
                        })?
                        .into_int_value();
                    let result_vtype = if ap.is_variable_await {
                        BrixType::Int
                    } else {
                        self.functions
                            .get(&ap.callee_name)
                            .and_then(|(_, ret)| ret.as_ref())
                            .and_then(|v| v.first())
                            .cloned()
                            .unwrap_or(BrixType::Int)
                    };
                    if let Some(alloca) = var_allocas.get(&ap.result_var) {
                        self.builder.build_store(*alloca, result_val).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "result".to_string(),
                                span: None,
                            }
                        })?;
                        self.variables
                            .insert(ap.result_var.clone(), (*alloca, result_vtype.clone()));
                    } else {
                        let lt = self.brix_type_to_llvm(&result_vtype);
                        let alloca = self.create_entry_block_alloca(lt, &ap.result_var)?;
                        self.builder.build_store(alloca, result_val).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "result alloca".to_string(),
                                span: None,
                            }
                        })?;
                        self.variables
                            .insert(ap.result_var.clone(), (alloca, result_vtype));
                    }
                }};
            }

            // ── Inline helper: compile stmts, intercept Return, always emit READY ─
            macro_rules! compile_tail_with_return {
                ($stmts:expr) => {{
                    for stmt in $stmts {
                        if let StmtKind::Return { values } = &stmt.kind {
                            if let Some(ra) = async_ret_alloca {
                                if let Some(ret_expr) = values.first() {
                                    let (val, _) = self.compile_expr(ret_expr)?;
                                    self.builder.build_store(ra, val).map_err(|_| {
                                        CodegenError::LLVMError {
                                            operation: "build_store".to_string(),
                                            details: "ret val".to_string(),
                                            span: None,
                                        }
                                    })?;
                                }
                            }
                            break;
                        }
                        self.compile_stmt(stmt, poll_fn)?;
                    }
                    if let Some(bb) = self.builder.get_insert_block() {
                        if bb.get_terminator().is_none() {
                            let ret_i64 = if let Some(ra) = async_ret_alloca {
                                let lt = self.brix_type_to_llvm(ret_brix_type);
                                self.builder
                                    .build_load(lt, ra, "rv")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_load".to_string(),
                                        details: "rv".to_string(),
                                        span: None,
                                    })?
                                    .into_int_value()
                            } else {
                                i64_type.const_int(0, false)
                            };
                            return_ready!(ret_i64);
                        }
                    }
                }};
            }

            // ── Build switch ────────────────────────────────────────────
            let sf0 = self
                .builder
                .build_struct_gep(state_struct_type, sp, 0, "sf0")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "struct_gep".to_string(),
                    details: "state".to_string(),
                    span: None,
                })?;
            let state_val = self
                .builder
                .build_load(i64_type, sf0, "state")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_load".to_string(),
                    details: "state".to_string(),
                    span: None,
                })?
                .into_int_value();

            let default_bb = self.context.append_basic_block(poll_fn, "sw_default");
            let state_bbs: Vec<inkwell::basic_block::BasicBlock<'ctx>> = (0..=total_awaits)
                .map(|i| {
                    self.context
                        .append_basic_block(poll_fn, &format!("state_{}", i))
                })
                .collect();

            let cases: Vec<(
                inkwell::values::IntValue<'ctx>,
                inkwell::basic_block::BasicBlock<'ctx>,
            )> = state_bbs
                .iter()
                .enumerate()
                .map(|(i, bb)| (i64_type.const_int(i as u64, false), *bb))
                .collect();
            self.builder
                .build_switch(state_val, default_bb, &cases)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_switch".to_string(),
                    details: "".to_string(),
                    span: None,
                })?;

            // default → PENDING
            self.builder.position_at_end(default_bb);
            self.builder
                .build_return(Some(&pending_val))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "default".to_string(),
                    span: None,
                })?;

            // ── State 0: load params, process async_stmts ───────────────
            self.builder.position_at_end(state_bbs[0]);

            // Load params from struct
            for (pname, _, _) in params.iter() {
                let (field_idx, vtype) = var_field_map.get(pname).unwrap().clone();
                let lt: BasicTypeEnum<'ctx> = if Compiler::is_closure_type(&vtype) {
                    ptr_type.into()
                } else {
                    self.brix_type_to_llvm(&vtype)
                };
                let fi = self
                    .builder
                    .build_struct_gep(state_struct_type, sp, field_idx, &format!("lp_{}", pname))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: "load param".to_string(),
                        span: None,
                    })?;
                let val = self.builder.build_load(lt, fi, pname).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: pname.clone(),
                        span: None,
                    }
                })?;
                let alloca = var_allocas.get(pname).copied().unwrap();
                self.builder
                    .build_store(alloca, val)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: pname.clone(),
                        span: None,
                    })?;
                self.variables.insert(pname.clone(), (alloca, vtype));
            }

            // ── Process the async_stmts tree ────────────────────────────
            // We track which await index we're on (0-based, corresponds to state_bbs[await_idx+1])
            let mut await_idx: usize = 0;

            // Flatten the processing: walk the top-level async_stmts list
            let mut i = 0;
            while i < async_stmts.len() {
                match &async_stmts[i] {
                    AsyncStmt::Stmts(stmts) => {
                        for stmt in stmts {
                            self.compile_stmt(stmt, poll_fn)?;
                        }
                        i += 1;
                    }

                    AsyncStmt::TopLevelAwait(ap) => {
                        save_live_vars!();
                        let next_state = await_idx + 1;
                        launch_await!(ap, next_state);

                        // Fill state_bbs[next_state]: poll, then continue with remaining stmts
                        self.builder.position_at_end(state_bbs[next_state]);
                        restore_live_vars!();
                        let ready_bb = self
                            .context
                            .append_basic_block(poll_fn, &format!("ready_{}", next_state));
                        let pending_bb = self
                            .context
                            .append_basic_block(poll_fn, &format!("pending_{}", next_state));
                        poll_sub_future!(ap, ready_bb, pending_bb);

                        await_idx += 1;
                        i += 1;
                        // Continue processing remaining stmts from this state
                    }

                    AsyncStmt::IfAwait {
                        cond,
                        then_stmts,
                        else_stmts,
                    } => {
                        let then_awaits = count_awaits(then_stmts);
                        let else_awaits = count_awaits(else_stmts);
                        let _then_aps = collect_all_await_points(then_stmts);
                        let _else_aps = collect_all_await_points(else_stmts);

                        // Collect post-if stmts (everything after this IfAwait in the top-level list)
                        let post_if_stmts: Vec<AsyncStmt> = async_stmts[i + 1..].to_vec();

                        // Evaluate condition
                        let (cond_val, _) = self.compile_expr(cond)?;
                        let cond_int = cond_val.into_int_value();
                        let cond_bool = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::NE,
                                cond_int,
                                i64_type.const_int(0, false),
                                "cond",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "int_compare".to_string(),
                                details: "if cond".to_string(),
                                span: None,
                            })?;

                        let then_bb = self.context.append_basic_block(poll_fn, "if_then");
                        let else_bb = self.context.append_basic_block(poll_fn, "if_else");

                        self.builder
                            .build_conditional_branch(cond_bool, then_bb, else_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "cond_branch".to_string(),
                                details: "if".to_string(),
                                span: None,
                            })?;

                        // State indices for then-branch: await_idx .. await_idx + then_awaits - 1
                        let then_start_await = await_idx;
                        // State indices for else-branch: await_idx + then_awaits .. await_idx + then_awaits + else_awaits - 1
                        let else_start_await = await_idx + then_awaits;

                        // ── Then branch ─────────────────────────────────
                        self.builder.position_at_end(then_bb);
                        if then_awaits > 0 {
                            // Compile then stmts until first await, launch it
                            let mut then_await_count = 0;
                            for ts in then_stmts {
                                match ts {
                                    AsyncStmt::Stmts(code) => {
                                        for s in code {
                                            self.compile_stmt(s, poll_fn)?;
                                        }
                                    }
                                    AsyncStmt::TopLevelAwait(ap) => {
                                        save_live_vars!();
                                        let state_num = then_start_await + then_await_count + 1;
                                        launch_await!(ap, state_num);

                                        // Fill states for then-branch awaits
                                        let remaining_then: Vec<&AsyncStmt> = then_stmts
                                            .iter()
                                            .skip(
                                                then_stmts
                                                    .iter()
                                                    .position(|x| std::ptr::eq(x, ts))
                                                    .unwrap()
                                                    + 1,
                                            )
                                            .collect();

                                        // Fill state_bbs for this then-branch await
                                        self.builder.position_at_end(state_bbs[state_num]);
                                        restore_live_vars!();
                                        let rbb = self.context.append_basic_block(
                                            poll_fn,
                                            &format!("then_ready_{}", state_num),
                                        );
                                        let pbb = self.context.append_basic_block(
                                            poll_fn,
                                            &format!("then_pending_{}", state_num),
                                        );
                                        poll_sub_future!(ap, rbb, pbb);

                                        then_await_count += 1;

                                        // Process remaining then stmts
                                        let mut hit_another_await = false;
                                        for rts in &remaining_then {
                                            match rts {
                                                AsyncStmt::Stmts(code) => {
                                                    for s in code {
                                                        self.compile_stmt(s, poll_fn)?;
                                                    }
                                                }
                                                AsyncStmt::TopLevelAwait(ap2) => {
                                                    save_live_vars!();
                                                    let ns2 =
                                                        then_start_await + then_await_count + 1;
                                                    launch_await!(ap2, ns2);

                                                    self.builder.position_at_end(state_bbs[ns2]);
                                                    restore_live_vars!();
                                                    let rbb2 = self.context.append_basic_block(
                                                        poll_fn,
                                                        &format!("then_ready_{}", ns2),
                                                    );
                                                    let pbb2 = self.context.append_basic_block(
                                                        poll_fn,
                                                        &format!("then_pending_{}", ns2),
                                                    );
                                                    poll_sub_future!(ap2, rbb2, pbb2);

                                                    then_await_count += 1;
                                                    hit_another_await = true;
                                                }
                                                _ => {}
                                            }
                                        }

                                        // After then-branch is done, compile post-if stmts
                                        if !hit_another_await || then_await_count == then_awaits {
                                            // Compile the post-if tail
                                            for pis in &post_if_stmts {
                                                match pis {
                                                    AsyncStmt::Stmts(code) => {
                                                        compile_tail_with_return!(code);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }

                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            // No awaits in then — compile inline
                            for ts in then_stmts {
                                if let AsyncStmt::Stmts(code) = ts {
                                    for s in code {
                                        self.compile_stmt(s, poll_fn)?;
                                    }
                                }
                            }
                            // Compile post-if stmts and return READY
                            for pis in &post_if_stmts {
                                if let AsyncStmt::Stmts(code) = pis {
                                    compile_tail_with_return!(code);
                                }
                            }
                        }

                        // ── Else branch ─────────────────────────────────
                        self.builder.position_at_end(else_bb);
                        if else_awaits > 0 {
                            let mut else_await_count = 0;
                            for es in else_stmts {
                                match es {
                                    AsyncStmt::Stmts(code) => {
                                        for s in code {
                                            self.compile_stmt(s, poll_fn)?;
                                        }
                                    }
                                    AsyncStmt::TopLevelAwait(ap) => {
                                        save_live_vars!();
                                        let state_num = else_start_await + else_await_count + 1;
                                        launch_await!(ap, state_num);

                                        self.builder.position_at_end(state_bbs[state_num]);
                                        restore_live_vars!();
                                        let rbb = self.context.append_basic_block(
                                            poll_fn,
                                            &format!("else_ready_{}", state_num),
                                        );
                                        let pbb = self.context.append_basic_block(
                                            poll_fn,
                                            &format!("else_pending_{}", state_num),
                                        );
                                        poll_sub_future!(ap, rbb, pbb);

                                        else_await_count += 1;

                                        // Process remaining else stmts...
                                        let remaining_else: Vec<&AsyncStmt> = else_stmts
                                            .iter()
                                            .skip(
                                                else_stmts
                                                    .iter()
                                                    .position(|x| std::ptr::eq(x, es))
                                                    .unwrap()
                                                    + 1,
                                            )
                                            .collect();
                                        for res in &remaining_else {
                                            match res {
                                                AsyncStmt::Stmts(code) => {
                                                    for s in code {
                                                        self.compile_stmt(s, poll_fn)?;
                                                    }
                                                }
                                                AsyncStmt::TopLevelAwait(ap2) => {
                                                    save_live_vars!();
                                                    let ns2 =
                                                        else_start_await + else_await_count + 1;
                                                    launch_await!(ap2, ns2);

                                                    self.builder.position_at_end(state_bbs[ns2]);
                                                    restore_live_vars!();
                                                    let rbb2 = self.context.append_basic_block(
                                                        poll_fn,
                                                        &format!("else_ready_{}", ns2),
                                                    );
                                                    let pbb2 = self.context.append_basic_block(
                                                        poll_fn,
                                                        &format!("else_pending_{}", ns2),
                                                    );
                                                    poll_sub_future!(ap2, rbb2, pbb2);
                                                    else_await_count += 1;
                                                }
                                                _ => {}
                                            }
                                        }

                                        // Compile post-if tail
                                        for pis in &post_if_stmts {
                                            if let AsyncStmt::Stmts(code) = pis {
                                                compile_tail_with_return!(code);
                                            }
                                        }
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            // No awaits in else — compile inline
                            for es in else_stmts {
                                if let AsyncStmt::Stmts(code) = es {
                                    for s in code {
                                        self.compile_stmt(s, poll_fn)?;
                                    }
                                }
                            }
                            // Compile post-if stmts
                            for pis in &post_if_stmts {
                                if let AsyncStmt::Stmts(code) = pis {
                                    compile_tail_with_return!(code);
                                }
                            }
                        }

                        await_idx += then_awaits + else_awaits;
                        // All remaining stmts handled in branch tails
                        i = async_stmts.len();
                    }

                    AsyncStmt::WhileAwait {
                        cond,
                        body_stmts,
                        live_vars: _,
                    } => {
                        let body_awaits = count_awaits(body_stmts);
                        let _body_aps = collect_all_await_points(body_stmts);

                        save_live_vars!();

                        // Evaluate condition
                        let (cond_val, _) = self.compile_expr(cond)?;
                        let cond_int = cond_val.into_int_value();
                        let cond_bool = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::NE,
                                cond_int,
                                i64_type.const_int(0, false),
                                "while_cond",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "int_compare".to_string(),
                                details: "while cond".to_string(),
                                span: None,
                            })?;

                        let while_body_bb = self.context.append_basic_block(poll_fn, "while_body");
                        let while_done_bb = self.context.append_basic_block(poll_fn, "while_done");
                        let after_while_bb =
                            self.context.append_basic_block(poll_fn, "after_while");

                        self.builder
                            .build_conditional_branch(cond_bool, while_body_bb, while_done_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "cond_branch".to_string(),
                                details: "while".to_string(),
                                span: None,
                            })?;

                        // ── while_done: branch to after_while merge block ──
                        self.builder.position_at_end(while_done_bb);
                        self.builder
                            .build_unconditional_branch(after_while_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_unconditional_branch".to_string(),
                                details: "while_done → after_while".to_string(),
                                span: None,
                            })?;

                        // ── while_body: compile body until first await ─────
                        self.builder.position_at_end(while_body_bb);

                        let while_start_await = await_idx;
                        let first_body_state = while_start_await + 1;

                        // Split body_stmts into segments between awaits
                        let mut body_segments: Vec<Vec<Stmt>> = vec![Vec::new()];
                        let mut body_await_list: Vec<AwaitPoint> = Vec::new();
                        for bs in body_stmts {
                            match bs {
                                AsyncStmt::Stmts(code) => {
                                    body_segments
                                        .last_mut()
                                        .unwrap()
                                        .extend(code.iter().cloned());
                                }
                                AsyncStmt::TopLevelAwait(ap) => {
                                    body_await_list.push(ap.clone());
                                    body_segments.push(Vec::new());
                                }
                                _ => {}
                            }
                        }

                        // Compile body_segments[0] (code before first await in body)
                        for s in &body_segments[0] {
                            self.compile_stmt(s, poll_fn)?;
                        }
                        save_live_vars!();
                        launch_await!(&body_await_list[0], first_body_state);

                        // ── States for while body awaits ────────────────
                        for k in 0..body_awaits {
                            let state_num = while_start_await + k + 1;
                            self.builder.position_at_end(state_bbs[state_num]);
                            restore_live_vars!();

                            let rbb = self
                                .context
                                .append_basic_block(poll_fn, &format!("while_ready_{}", state_num));
                            let pbb = self.context.append_basic_block(
                                poll_fn,
                                &format!("while_pending_{}", state_num),
                            );
                            poll_sub_future!(&body_await_list[k], rbb, pbb);

                            if k < body_awaits - 1 {
                                // Intermediate: compile segment, launch next await
                                for s in &body_segments[k + 1] {
                                    self.compile_stmt(s, poll_fn)?;
                                }
                                save_live_vars!();
                                let next_state = while_start_await + k + 2;
                                launch_await!(&body_await_list[k + 1], next_state);
                            } else {
                                // Last await in body: compile remaining, re-eval condition
                                for s in &body_segments[k + 1] {
                                    self.compile_stmt(s, poll_fn)?;
                                }
                                save_live_vars!();

                                // Re-evaluate while condition
                                let (cond_val2, _) = self.compile_expr(cond)?;
                                let cond_int2 = cond_val2.into_int_value();
                                let cond_bool2 = self
                                    .builder
                                    .build_int_compare(
                                        inkwell::IntPredicate::NE,
                                        cond_int2,
                                        i64_type.const_int(0, false),
                                        "re_cond",
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "int_compare".to_string(),
                                        details: "re_cond".to_string(),
                                        span: None,
                                    })?;

                                let reloop_bb = self.context.append_basic_block(poll_fn, "reloop");
                                let exit_bb =
                                    self.context.append_basic_block(poll_fn, "while_exit");

                                self.builder
                                    .build_conditional_branch(cond_bool2, reloop_bb, exit_bb)
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "cond_branch".to_string(),
                                        details: "reloop".to_string(),
                                        span: None,
                                    })?;

                                // reloop: re-compile body_segments[0], launch first body await (self-loop)
                                self.builder.position_at_end(reloop_bb);
                                for s in &body_segments[0] {
                                    self.compile_stmt(s, poll_fn)?;
                                }
                                save_live_vars!();
                                launch_await!(&body_await_list[0], first_body_state);

                                // exit: branch to after_while merge block
                                self.builder.position_at_end(exit_bb);
                                self.builder
                                    .build_unconditional_branch(after_while_bb)
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_unconditional_branch".to_string(),
                                        details: "exit → after_while".to_string(),
                                        span: None,
                                    })?;
                            }
                        }

                        self.builder.position_at_end(after_while_bb);
                        await_idx += body_awaits;
                        i += 1;
                    }
                }
            }

            // If we got through all stmts without hitting an await (shouldn't happen for nested),
            // emit READY
            if let Some(bb) = self.builder.get_insert_block() {
                if bb.get_terminator().is_none() {
                    let ret_i64 = if let Some(ra) = async_ret_alloca {
                        let lt = self.brix_type_to_llvm(ret_brix_type);
                        self.builder
                            .build_load(lt, ra, "rv_final")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "rv_final".to_string(),
                                span: None,
                            })?
                            .into_int_value()
                    } else {
                        i64_type.const_int(0, false)
                    };
                    return_ready!(ret_i64);
                }
            }

            // Restore compiler state
            self.current_function = saved_fn;
            self.variables = saved_vars;
            self.function_scope_vars = saved_scope;
            if let Some(prev) = saved_fn {
                if let Some(bb) = prev.get_last_basic_block() {
                    self.builder.position_at_end(bb);
                }
            }
        }

        // ── C. async fn main: emit drive code ───────────────────────────
        if name == "main" {
            let prt_local = self
                .context
                .struct_type(&[i64_type.into(), i64_type.into()], false);
            let run_fn_type = prt_local.fn_type(&[ptr_type.into(), ptr_type.into()], false);
            let run_fn = self
                .module
                .get_function("brix_run_to_completion")
                .unwrap_or_else(|| {
                    self.module.add_function(
                        "brix_run_to_completion",
                        run_fn_type,
                        Some(Linkage::External),
                    )
                });

            let create_fn_ref = self.async_create_fns.get(name).copied().ok_or_else(|| {
                CodegenError::UndefinedSymbol {
                    name: create_name_str.clone(),
                    context: "async main drive".to_string(),
                    span: None,
                }
            })?;
            let poll_fn_ref = self.async_poll_fns.get(name).copied().ok_or_else(|| {
                CodegenError::UndefinedSymbol {
                    name: poll_name_str.clone(),
                    context: "async main drive".to_string(),
                    span: None,
                }
            })?;

            let cc = self
                .builder
                .build_call(create_fn_ref, &[], "main_sp")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "create_main".to_string(),
                    span: None,
                })?;
            let main_sp = cc
                .try_as_basic_value()
                .left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "create_main".to_string(),
                    context: "async main".to_string(),
                    span: None,
                })?
                .into_pointer_value();

            let poll_ptr = poll_fn_ref.as_global_value().as_pointer_value();
            self.builder
                .build_call(run_fn, &[main_sp.into(), poll_ptr.into()], "")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "brix_run_to_completion".to_string(),
                    span: None,
                })?;
        }

        Ok(())
    }

    // --- ASYNC CLOSURE (v1.6 Phase 3c) ---
    //
    // Compile an `async (params) -> { body }` closure expression.
    // Similar to compile_async_block but with closure params stored in the state struct.
    //
    // State struct layout:
    //   [0]          poll_fn_ptr: i8*
    //   [1]          state: i64
    //   [2 .. P+1]   params (one per closure param)
    //   [P+2 .. P+M+1]  result_i: one per await in body
    //   [P+M+2]      sub_future_ptr: i8* (only if M > 0)
    //
    // Returns: (state_ptr as BasicValueEnum, BrixType::AsyncFuture)
    pub(crate) fn compile_async_closure(
        &mut self,
        closure: &Closure,
        _expr: &Expr,
    ) -> CodegenResult<(inkwell::values::BasicValueEnum<'ctx>, BrixType)> {
        use inkwell::module::Linkage;
        use inkwell::types::BasicTypeEnum;
        use inkwell::values::BasicMetadataValueEnum;
        use parser::ast::StmtKind;

        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        // ── 1. Unique name ──────────────────────────────────────────────
        self.closure_counter += 1;
        let counter = self.closure_counter;
        let block_name = format!("async_closure_{}", counter);

        // ── 2. Extract await points and segments ─────────────────────────
        let (await_points, segments) = extract_await_segments(&closure.body);
        let n_awaits = await_points.len();

        // ── 3. Param types ──────────────────────────────────────────────
        let param_brix_types: Vec<BrixType> = closure
            .params
            .iter()
            .map(|(_, ts)| self.string_to_brix_type(ts))
            .collect();

        // ── 4. Infer return type ─────────────────────────────────────────
        let ret_brix_type: BrixType = if let Some(ref rt) = closure.return_type {
            self.string_to_brix_type(rt)
        } else {
            self.infer_return_type_from_body(&closure.body, &[])
                .unwrap_or(BrixType::Int)
        };

        // ── 5. Await result types ────────────────────────────────────────
        let await_ret_types: Vec<BrixType> = await_points
            .iter()
            .map(|ap| {
                if ap.is_variable_await {
                    ret_brix_type.clone()
                } else {
                    self.functions
                        .get(&ap.callee_name)
                        .and_then(|(_, ret)| ret.as_ref())
                        .and_then(|v| v.first())
                        .cloned()
                        .unwrap_or(BrixType::Int)
                }
            })
            .collect();

        // ── 6. Build state struct type ──────────────────────────────────
        let mut struct_fields: Vec<BasicTypeEnum<'ctx>> = vec![
            ptr_type.into(), // field 0: poll_fn_ptr
            i64_type.into(), // field 1: state
        ];

        let state_field: usize = 1;
        let param_field_start: usize = 2;

        for bt in &param_brix_types {
            let lt: BasicTypeEnum<'ctx> = if Compiler::is_closure_type(bt) {
                ptr_type.into()
            } else {
                self.brix_type_to_llvm(bt)
            };
            struct_fields.push(lt);
        }

        let result_field_start: usize = param_field_start + closure.params.len();

        for bt in &await_ret_types {
            struct_fields.push(self.brix_type_to_llvm(bt));
        }

        let sub_future_field: usize = result_field_start + n_awaits;
        if n_awaits > 0 {
            struct_fields.push(ptr_type.into());
        }

        let state_struct_type = self.context.struct_type(&struct_fields, false);

        // ── 7. Generate poll function ───────────────────────────────────
        let poll_name_str = format!("poll_{}", block_name);
        let poll_result_type = self
            .context
            .struct_type(&[i64_type.into(), i64_type.into()], false);
        let poll_fn_type = poll_result_type.fn_type(&[ptr_type.into()], false);
        let poll_fn = self.module.add_function(&poll_name_str, poll_fn_type, None);
        self.async_poll_fns.insert(block_name.clone(), poll_fn);

        let saved_insert_block = self.builder.get_insert_block();

        {
            let saved_fn = self.current_function;
            let saved_vars = self.variables.clone();
            let saved_scope = self.function_scope_vars.clone();
            self.current_function = Some(poll_fn);
            self.function_scope_vars.clear();

            let entry_bb = self.context.append_basic_block(poll_fn, "entry");
            self.builder.position_at_end(entry_bb);

            let sp = poll_fn.get_nth_param(0).unwrap().into_pointer_value();

            // Pre-allocate param allocas
            let param_allocas: Vec<inkwell::values::PointerValue<'ctx>> = {
                let mut v = Vec::new();
                for (i, (pname, _)) in closure.params.iter().enumerate() {
                    let lt: BasicTypeEnum<'ctx> = if Compiler::is_closure_type(&param_brix_types[i])
                    {
                        ptr_type.into()
                    } else {
                        self.brix_type_to_llvm(&param_brix_types[i])
                    };
                    v.push(self.create_entry_block_alloca(lt, pname)?);
                }
                v
            };

            let result_allocas: Vec<inkwell::values::PointerValue<'ctx>> = {
                let mut v = Vec::new();
                for (i, ap) in await_points.iter().enumerate() {
                    let lt = self.brix_type_to_llvm(&await_ret_types[i]);
                    v.push(self.create_entry_block_alloca(lt, &ap.result_var)?);
                }
                v
            };

            let async_ret_alloca: Option<inkwell::values::PointerValue<'ctx>> =
                if ret_brix_type != BrixType::Void {
                    Some(self.create_entry_block_alloca(
                        self.brix_type_to_llvm(&ret_brix_type),
                        "async_ret",
                    )?)
                } else {
                    None
                };

            let pending_val = poll_result_type.const_named_struct(&[
                i64_type.const_int(0, false).into(),
                i64_type.const_int(0, false).into(),
            ]);

            macro_rules! return_ready {
                ($val:expr) => {{
                    let rs0 = self
                        .builder
                        .build_insert_value(
                            poll_result_type.get_undef(),
                            i64_type.const_int(1, false),
                            0,
                            "rs0",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "insert_value".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;
                    let rs1 = self
                        .builder
                        .build_insert_value(rs0.into_struct_value(), $val, 1, "rs1")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "insert_value".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_return(Some(&rs1))
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "READY".to_string(),
                            span: None,
                        })?;
                }};
            }

            // Load params from struct
            macro_rules! load_params {
                () => {
                    for (i, (pname, _)) in closure.params.iter().enumerate() {
                        let lt: BasicTypeEnum<'ctx> =
                            if Compiler::is_closure_type(&param_brix_types[i]) {
                                ptr_type.into()
                            } else {
                                self.brix_type_to_llvm(&param_brix_types[i])
                            };
                        let fi = self
                            .builder
                            .build_struct_gep(
                                state_struct_type,
                                sp,
                                (param_field_start + i) as u32,
                                &format!("lpf{}", i),
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: format!("lpf{}", i),
                                span: None,
                            })?;
                        let val = self.builder.build_load(lt, fi, pname).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: pname.clone(),
                                span: None,
                            }
                        })?;
                        self.builder
                            .build_store(param_allocas[i], val)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: pname.clone(),
                                span: None,
                            })?;
                        self.variables.insert(
                            pname.clone(),
                            (param_allocas[i], param_brix_types[i].clone()),
                        );
                    }
                };
            }

            // ── CASE A: No awaits ───────────────────────────────────────
            if n_awaits == 0 {
                let ready_bb = self.context.append_basic_block(poll_fn, "ready");
                load_params!();

                let mut reached_return = false;
                for stmt in &segments[0] {
                    if let StmtKind::Return { values } = &stmt.kind {
                        if let Some(ra) = async_ret_alloca {
                            if let Some(ret_expr) = values.first() {
                                let (val, _) = self.compile_expr(ret_expr)?;
                                self.builder.build_store(ra, val).map_err(|_| {
                                    CodegenError::LLVMError {
                                        operation: "build_store".to_string(),
                                        details: "ret val".to_string(),
                                        span: None,
                                    }
                                })?;
                            }
                        }
                        self.builder
                            .build_unconditional_branch(ready_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "branch".to_string(),
                                details: "to ready".to_string(),
                                span: None,
                            })?;
                        reached_return = true;
                        break;
                    }
                    self.compile_stmt(stmt, poll_fn)?;
                }
                if !reached_return {
                    if let Some(bb) = self.builder.get_insert_block() {
                        if bb.get_terminator().is_none() {
                            self.builder
                                .build_unconditional_branch(ready_bb)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "branch".to_string(),
                                    details: "fallthrough".to_string(),
                                    span: None,
                                })?;
                        }
                    }
                }

                self.builder.position_at_end(ready_bb);
                let ret_i64 = if let Some(ra) = async_ret_alloca {
                    let lt = self.brix_type_to_llvm(&ret_brix_type);
                    self.builder
                        .build_load(lt, ra, "rv")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "rv".to_string(),
                            span: None,
                        })?
                        .into_int_value()
                } else {
                    i64_type.const_int(0, false)
                };
                return_ready!(ret_i64);
            } else {
                // ── CASE B: N awaits — state machine ────────────────────
                let sf_state_field_ptr = self
                    .builder
                    .build_struct_gep(state_struct_type, sp, state_field as u32, "sf_state")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: "state".to_string(),
                        span: None,
                    })?;
                let state_val = self
                    .builder
                    .build_load(i64_type, sf_state_field_ptr, "state")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "state".to_string(),
                        span: None,
                    })?
                    .into_int_value();

                let default_bb = self.context.append_basic_block(poll_fn, "sw_default");
                let state_bbs: Vec<inkwell::basic_block::BasicBlock<'ctx>> = (0..=n_awaits)
                    .map(|i| {
                        self.context
                            .append_basic_block(poll_fn, &format!("state_{}", i))
                    })
                    .collect();

                let cases: Vec<(
                    inkwell::values::IntValue<'ctx>,
                    inkwell::basic_block::BasicBlock<'ctx>,
                )> = state_bbs
                    .iter()
                    .enumerate()
                    .map(|(i, bb)| (i64_type.const_int(i as u64, false), *bb))
                    .collect();
                self.builder
                    .build_switch(state_val, default_bb, &cases)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_switch".to_string(),
                        details: "".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(default_bb);
                self.builder.build_return(Some(&pending_val)).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_return".to_string(),
                        details: "default".to_string(),
                        span: None,
                    }
                })?;

                // State 0: load params, run segments[0], launch first await
                self.builder.position_at_end(state_bbs[0]);
                load_params!();

                for stmt in &segments[0] {
                    self.compile_stmt(stmt, poll_fn)?;
                }

                {
                    let ap = &await_points[0];
                    let sf_ptr = if ap.is_variable_await {
                        let var_entry = self.variables.get(&ap.callee_name).ok_or_else(|| {
                            CodegenError::UndefinedSymbol {
                                name: ap.callee_name.clone(),
                                context: "async_closure var await s0".to_string(),
                                span: None,
                            }
                        })?;
                        let (va, _) = var_entry.clone();
                        self.builder
                            .build_load(ptr_type, va, "sf_var")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "sf_var".to_string(),
                                span: None,
                            })?
                            .into_pointer_value()
                    } else {
                        let create_fn = self
                            .async_create_fns
                            .get(&ap.callee_name)
                            .copied()
                            .ok_or_else(|| CodegenError::UndefinedSymbol {
                                name: format!("create_{}", ap.callee_name),
                                context: "async_closure s0".to_string(),
                                span: None,
                            })?;
                        let mut av: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
                        for ae in &ap.callee_args.clone() {
                            let (v, _) = self.compile_expr(ae)?;
                            av.push(v.into());
                        }
                        let cc = self
                            .builder
                            .build_call(create_fn, &av, "sf0")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "create sf".to_string(),
                                span: None,
                            })?;
                        cc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "create_callee".to_string(),
                                context: "async_closure s0".to_string(),
                                span: None,
                            })?
                            .into_pointer_value()
                    };

                    let sff = self
                        .builder
                        .build_struct_gep(state_struct_type, sp, sub_future_field as u32, "sff")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "sff".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_store(sff, sf_ptr)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "sf".to_string(),
                            span: None,
                        })?;
                }

                let sf_sp = self
                    .builder
                    .build_struct_gep(state_struct_type, sp, state_field as u32, "sfs")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: "state".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(sf_sp, i64_type.const_int(1, false))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "state=1".to_string(),
                        span: None,
                    })?;
                self.builder.build_return(Some(&pending_val)).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_return".to_string(),
                        details: "PENDING s0".to_string(),
                        span: None,
                    }
                })?;

                // States 1..N: poll + continue
                for k in 1..=n_awaits {
                    self.builder.position_at_end(state_bbs[k]);

                    let sff = self
                        .builder
                        .build_struct_gep(state_struct_type, sp, sub_future_field as u32, "sff")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "sff".to_string(),
                            span: None,
                        })?;
                    let sf_ptr = self
                        .builder
                        .build_load(ptr_type, sff, "sf")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "sf".to_string(),
                            span: None,
                        })?
                        .into_pointer_value();

                    let poll_fn_type_local = poll_result_type.fn_type(&[ptr_type.into()], false);
                    let pr = if await_points[k - 1].is_variable_await {
                        let pfp = self
                            .builder
                            .build_load(ptr_type, sf_ptr, "pfp")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "pfp".to_string(),
                                span: None,
                            })?
                            .into_pointer_value();
                        let pc = self
                            .builder
                            .build_indirect_call(poll_fn_type_local, pfp, &[sf_ptr.into()], "pr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_indirect_call".to_string(),
                                details: "poll var".to_string(),
                                span: None,
                            })?;
                        pc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "poll_var".to_string(),
                                context: format!("async_closure s{}", k),
                                span: None,
                            })?
                            .into_struct_value()
                    } else {
                        let poll_callee = self
                            .async_poll_fns
                            .get(&await_points[k - 1].callee_name)
                            .copied()
                            .ok_or_else(|| CodegenError::UndefinedSymbol {
                                name: format!("poll_{}", await_points[k - 1].callee_name),
                                context: format!("async_closure s{}", k),
                                span: None,
                            })?;
                        let pc = self
                            .builder
                            .build_call(poll_callee, &[sf_ptr.into()], "pr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "poll".to_string(),
                                span: None,
                            })?;
                        pc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "poll".to_string(),
                                context: format!("async_closure s{}", k),
                                span: None,
                            })?
                            .into_struct_value()
                    };

                    let status = self
                        .builder
                        .build_extract_value(pr, 0, "pr_status")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "extract_value".to_string(),
                            details: "status".to_string(),
                            span: None,
                        })?
                        .into_int_value();
                    let is_ready = self
                        .builder
                        .build_int_compare(
                            inkwell::IntPredicate::EQ,
                            status,
                            i64_type.const_int(1, false),
                            "is_ready",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "int_compare".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;

                    let ready_bb = self
                        .context
                        .append_basic_block(poll_fn, &format!("ready_{}", k));
                    let pending_bb = self
                        .context
                        .append_basic_block(poll_fn, &format!("pending_{}", k));

                    self.builder
                        .build_conditional_branch(is_ready, ready_bb, pending_bb)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "cond_branch".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;

                    self.builder.position_at_end(pending_bb);
                    self.builder.build_return(Some(&pending_val)).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "pending".to_string(),
                            span: None,
                        }
                    })?;

                    self.builder.position_at_end(ready_bb);
                    let result_val = self
                        .builder
                        .build_extract_value(pr, 1, "pr_val")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "extract_value".to_string(),
                            details: "value".to_string(),
                            span: None,
                        })?
                        .into_int_value();

                    let rfield = self
                        .builder
                        .build_struct_gep(
                            state_struct_type,
                            sp,
                            (result_field_start + k - 1) as u32,
                            &format!("rf{}", k - 1),
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "rfield".to_string(),
                            span: None,
                        })?;
                    self.builder.build_store(rfield, result_val).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "result".to_string(),
                            span: None,
                        }
                    })?;

                    self.builder
                        .build_store(result_allocas[k - 1], result_val)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "result alloca".to_string(),
                            span: None,
                        })?;
                    self.variables.insert(
                        await_points[k - 1].result_var.clone(),
                        (result_allocas[k - 1], await_ret_types[k - 1].clone()),
                    );

                    load_params!();
                    for prev in 0..(k - 1) {
                        let lt = self.brix_type_to_llvm(&await_ret_types[prev]);
                        let fi = self
                            .builder
                            .build_struct_gep(
                                state_struct_type,
                                sp,
                                (result_field_start + prev) as u32,
                                &format!("lprf{}", prev),
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: "prev result".to_string(),
                                span: None,
                            })?;
                        let pv = self
                            .builder
                            .build_load(lt, fi, &await_points[prev].result_var)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "prev result".to_string(),
                                span: None,
                            })?;
                        self.builder
                            .build_store(result_allocas[prev], pv)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "prev result alloca".to_string(),
                                span: None,
                            })?;
                        self.variables.insert(
                            await_points[prev].result_var.clone(),
                            (result_allocas[prev], await_ret_types[prev].clone()),
                        );
                    }

                    if k == n_awaits {
                        let mut seg_returned = false;
                        for stmt in &segments[k] {
                            if let StmtKind::Return { values } = &stmt.kind {
                                if let Some(ra) = async_ret_alloca {
                                    if let Some(re) = values.first() {
                                        let (v, _) = self.compile_expr(re)?;
                                        self.builder.build_store(ra, v).map_err(|_| {
                                            CodegenError::LLVMError {
                                                operation: "build_store".to_string(),
                                                details: "final ret".to_string(),
                                                span: None,
                                            }
                                        })?;
                                    }
                                }
                                seg_returned = true;
                                break;
                            }
                            self.compile_stmt(stmt, poll_fn)?;
                        }
                        let ret_i64 = if let Some(ra) = async_ret_alloca {
                            let lt = self.brix_type_to_llvm(&ret_brix_type);
                            if let Some(bb) = self.builder.get_insert_block() {
                                if bb.get_terminator().is_none() || seg_returned {
                                    self.builder
                                        .build_load(lt, ra, "rv")
                                        .map_err(|_| CodegenError::LLVMError {
                                            operation: "build_load".to_string(),
                                            details: "rv".to_string(),
                                            span: None,
                                        })?
                                        .into_int_value()
                                } else {
                                    i64_type.const_int(0, false)
                                }
                            } else {
                                i64_type.const_int(0, false)
                            }
                        } else {
                            i64_type.const_int(0, false)
                        };
                        if let Some(bb) = self.builder.get_insert_block() {
                            if bb.get_terminator().is_none() {
                                return_ready!(ret_i64);
                            }
                        }
                    } else {
                        for stmt in &segments[k] {
                            self.compile_stmt(stmt, poll_fn)?;
                        }
                        let next_ap = &await_points[k];
                        let nsf_ptr = if next_ap.is_variable_await {
                            let var_entry =
                                self.variables.get(&next_ap.callee_name).ok_or_else(|| {
                                    CodegenError::UndefinedSymbol {
                                        name: next_ap.callee_name.clone(),
                                        context: format!("async_closure s{}", k),
                                        span: None,
                                    }
                                })?;
                            let (va, _) = var_entry.clone();
                            self.builder
                                .build_load(ptr_type, va, "nsf_var")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: "nsf_var".to_string(),
                                    span: None,
                                })?
                                .into_pointer_value()
                        } else {
                            let create_next = self
                                .async_create_fns
                                .get(&next_ap.callee_name)
                                .copied()
                                .ok_or_else(|| CodegenError::UndefinedSymbol {
                                    name: format!("create_{}", next_ap.callee_name),
                                    context: format!("async_closure s{}", k),
                                    span: None,
                                })?;
                            let ac = next_ap.callee_args.clone();
                            let mut av: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
                            for ae in &ac {
                                let (v, _) = self.compile_expr(ae)?;
                                av.push(v.into());
                            }
                            let nc =
                                self.builder
                                    .build_call(create_next, &av, "nsf")
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_call".to_string(),
                                        details: "create next".to_string(),
                                        span: None,
                                    })?;
                            nc.try_as_basic_value()
                                .left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "create_next".to_string(),
                                    context: format!("async_closure s{}", k),
                                    span: None,
                                })?
                                .into_pointer_value()
                        };
                        let sff2 = self
                            .builder
                            .build_struct_gep(
                                state_struct_type,
                                sp,
                                sub_future_field as u32,
                                "sff2",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: "sff2".to_string(),
                                span: None,
                            })?;
                        self.builder.build_store(sff2, nsf_ptr).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "nsf".to_string(),
                                span: None,
                            }
                        })?;
                        let sfs2 = self
                            .builder
                            .build_struct_gep(state_struct_type, sp, state_field as u32, "sfs2")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: "state".to_string(),
                                span: None,
                            })?;
                        self.builder
                            .build_store(sfs2, i64_type.const_int((k + 1) as u64, false))
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "state+1".to_string(),
                                span: None,
                            })?;
                        self.builder.build_return(Some(&pending_val)).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_return".to_string(),
                                details: "PENDING inter".to_string(),
                                span: None,
                            }
                        })?;
                    }
                }
            }

            self.current_function = saved_fn;
            self.variables = saved_vars;
            self.function_scope_vars = saved_scope;
            if let Some(bb) = saved_insert_block {
                self.builder.position_at_end(bb);
            } else if let Some(prev) = saved_fn {
                if let Some(bb) = prev.get_last_basic_block() {
                    self.builder.position_at_end(bb);
                }
            }
        }

        // ── 8. Generate create function ─────────────────────────────────
        let create_name_str = format!("create_{}", block_name);
        {
            // create_async_closure_N(params...) -> i8*
            let create_param_types: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> =
                param_brix_types
                    .iter()
                    .map(|bt| -> inkwell::types::BasicMetadataTypeEnum<'ctx> {
                        if Compiler::is_closure_type(bt) {
                            ptr_type.into()
                        } else {
                            self.brix_type_to_llvm(bt).into()
                        }
                    })
                    .collect();
            let create_fn_type = ptr_type.fn_type(&create_param_types, false);
            let create_fn = self
                .module
                .add_function(&create_name_str, create_fn_type, None);
            self.async_create_fns.insert(block_name.clone(), create_fn);

            let saved_fn = self.current_function;
            self.current_function = Some(create_fn);
            let entry_bb = self.context.append_basic_block(create_fn, "entry");
            self.builder.position_at_end(entry_bb);

            let malloc_fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            let malloc_fn = self.module.get_function("brix_malloc").unwrap_or_else(|| {
                self.module
                    .add_function("brix_malloc", malloc_fn_type, Some(Linkage::External))
            });
            let struct_size =
                state_struct_type
                    .size_of()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "size_of".to_string(),
                        details: "async closure state struct".to_string(),
                        span: None,
                    })?;
            let mc = self
                .builder
                .build_call(malloc_fn, &[struct_size.into()], "sp")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "malloc".to_string(),
                    span: None,
                })?;
            let state_ptr = mc
                .try_as_basic_value()
                .left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "malloc".to_string(),
                    context: "create_async_closure".to_string(),
                    span: None,
                })?
                .into_pointer_value();

            // Store poll_fn at field [0]
            let poll_fn_field = self
                .builder
                .build_struct_gep(state_struct_type, state_ptr, 0, "pff")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "struct_gep".to_string(),
                    details: "poll_fn_field".to_string(),
                    span: None,
                })?;
            let poll_fn_ref = self
                .async_poll_fns
                .get(&block_name)
                .copied()
                .ok_or_else(|| CodegenError::UndefinedSymbol {
                    name: poll_name_str.clone(),
                    context: "create_async_closure".to_string(),
                    span: None,
                })?;
            let poll_fn_ptr_val = poll_fn_ref.as_global_value().as_pointer_value();
            self.builder
                .build_store(poll_fn_field, poll_fn_ptr_val)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "poll_fn".to_string(),
                    span: None,
                })?;

            // state = 0 at field [1]
            let state_f1 = self
                .builder
                .build_struct_gep(state_struct_type, state_ptr, state_field as u32, "sf1")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "struct_gep".to_string(),
                    details: "state=0".to_string(),
                    span: None,
                })?;
            self.builder
                .build_store(state_f1, i64_type.const_int(0, false))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "state=0".to_string(),
                    span: None,
                })?;

            // Store params
            for (i, _) in closure.params.iter().enumerate() {
                let pv = create_fn.get_nth_param(i as u32).unwrap();
                let fi = self
                    .builder
                    .build_struct_gep(
                        state_struct_type,
                        state_ptr,
                        (param_field_start + i) as u32,
                        &format!("cpf{}", i),
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: format!("cpf{}", i),
                        span: None,
                    })?;
                self.builder
                    .build_store(fi, pv)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: format!("cparam {}", i),
                        span: None,
                    })?;
            }

            self.builder
                .build_return(Some(&state_ptr))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "create_async_closure".to_string(),
                    span: None,
                })?;

            self.current_function = saved_fn;
            if let Some(bb) = saved_insert_block {
                self.builder.position_at_end(bb);
            } else if let Some(prev) = saved_fn {
                if let Some(bb) = prev.get_last_basic_block() {
                    self.builder.position_at_end(bb);
                }
            }
        }

        // ── 9. Emit call to create_async_closure_N() at current call site ─
        let create_fn_ref = self
            .async_create_fns
            .get(&block_name)
            .copied()
            .ok_or_else(|| CodegenError::UndefinedSymbol {
                name: create_name_str.clone(),
                context: "compile_async_closure".to_string(),
                span: None,
            })?;
        let cc = self
            .builder
            .build_call(create_fn_ref, &[], "cls_sp")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "create_async_closure".to_string(),
                span: None,
            })?;
        let state_ptr_val =
            cc.try_as_basic_value()
                .left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "create_async_closure".to_string(),
                    context: "compile_async_closure".to_string(),
                    span: None,
                })?;

        Ok((state_ptr_val, BrixType::AsyncFuture))
    }

    // --- ASYNC BLOCK (v1.6 Phase 3b) ---
    //
    // Compile an `async { ... }` block expression.
    //
    // State struct layout (different from async fn — no params, poll_fn_ptr embedded):
    //   [0]          poll_fn_ptr: i8*    (so variable-await can call indirectly)
    //   [1]          state: i64          (current state counter, starts at 0)
    //   [2 .. M+1]   result_i: one per await in block body
    //   [M+2]        sub_future_ptr: i8* (only if M > 0)
    //
    // Returns: (state_ptr as BasicValueEnum, BrixType::AsyncFuture)
    pub(crate) fn compile_async_block(
        &mut self,
        body: &Stmt,
    ) -> CodegenResult<(inkwell::values::BasicValueEnum<'ctx>, BrixType)> {
        use inkwell::module::Linkage;
        use inkwell::types::BasicTypeEnum;
        use inkwell::values::BasicMetadataValueEnum;
        use parser::ast::StmtKind;

        let i64_type = self.context.i64_type();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());

        // ── 1. Unique name for this block ────────────────────────────────
        self.closure_counter += 1;
        let counter = self.closure_counter;
        let block_name = format!("async_block_{}", counter);

        // ── 2. Extract await points and segments ─────────────────────────
        let (await_points, segments) = extract_await_segments(body);
        let n_awaits = await_points.len();

        // ── 3. Infer return type ─────────────────────────────────────────
        let ret_brix_type: BrixType = self
            .infer_return_type_from_body(body, &[])
            .unwrap_or(BrixType::Int);

        // ── 4. Await result types ────────────────────────────────────────
        // For variable awaits the return type defaults to Int (the async block's result type)
        let await_ret_types: Vec<BrixType> = await_points
            .iter()
            .map(|ap| {
                if ap.is_variable_await {
                    ret_brix_type.clone()
                } else {
                    self.functions
                        .get(&ap.callee_name)
                        .and_then(|(_, ret)| ret.as_ref())
                        .and_then(|v| v.first())
                        .cloned()
                        .unwrap_or(BrixType::Int)
                }
            })
            .collect();

        // ── 5. Build state struct type ───────────────────────────────────
        // Layout: [poll_fn_ptr, state, ...results..., optional sub_future_ptr]
        let mut struct_fields: Vec<BasicTypeEnum<'ctx>> = vec![
            ptr_type.into(), // field 0: poll_fn_ptr
            i64_type.into(), // field 1: state
        ];

        let state_field: usize = 1; // index of state field
        let result_field_start: usize = 2; // first result field

        for bt in &await_ret_types {
            struct_fields.push(self.brix_type_to_llvm(bt));
        }

        let sub_future_field: usize = result_field_start + n_awaits;
        if n_awaits > 0 {
            struct_fields.push(ptr_type.into()); // sub_future_ptr
        }

        let state_struct_type = self.context.struct_type(&struct_fields, false);

        // ── 6. Generate poll_async_block_N function ──────────────────────
        let poll_name_str = format!("poll_{}", block_name);
        let poll_result_type = self
            .context
            .struct_type(&[i64_type.into(), i64_type.into()], false);
        let poll_fn_type = poll_result_type.fn_type(&[ptr_type.into()], false);
        let poll_fn = self.module.add_function(&poll_name_str, poll_fn_type, None);
        self.async_poll_fns.insert(block_name.clone(), poll_fn);

        // Save the current insertion block so we can restore it precisely after generating
        // the nested poll/create functions. Using get_last_basic_block() is not sufficient
        // because the state machine may have created multiple BBs and we must return to
        // the exact BB we were building (e.g., state_0), not the last one (state_N).
        let saved_insert_block = self.builder.get_insert_block();

        {
            let saved_fn = self.current_function;
            let saved_vars = self.variables.clone();
            let saved_scope = self.function_scope_vars.clone();
            self.current_function = Some(poll_fn);
            self.function_scope_vars.clear();

            let entry_bb = self.context.append_basic_block(poll_fn, "entry");
            self.builder.position_at_end(entry_bb);

            let sp = poll_fn.get_nth_param(0).unwrap().into_pointer_value();

            // Pre-allocate result allocas in the entry block
            let result_allocas: Vec<inkwell::values::PointerValue<'ctx>> = {
                let mut v = Vec::new();
                for (i, ap) in await_points.iter().enumerate() {
                    let lt = self.brix_type_to_llvm(&await_ret_types[i]);
                    v.push(self.create_entry_block_alloca(lt, &ap.result_var)?);
                }
                v
            };
            let async_ret_alloca: Option<inkwell::values::PointerValue<'ctx>> =
                if ret_brix_type != BrixType::Void {
                    Some(self.create_entry_block_alloca(
                        self.brix_type_to_llvm(&ret_brix_type),
                        "async_ret",
                    )?)
                } else {
                    None
                };

            // PENDING constant
            let pending_val = poll_result_type.const_named_struct(&[
                i64_type.const_int(0, false).into(),
                i64_type.const_int(0, false).into(),
            ]);

            // Helper macro: build READY { 1, value } and return
            macro_rules! return_ready {
                ($val:expr) => {{
                    let rs0 = self
                        .builder
                        .build_insert_value(
                            poll_result_type.get_undef(),
                            i64_type.const_int(1, false),
                            0,
                            "rs0",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "insert_value".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;
                    let rs1 = self
                        .builder
                        .build_insert_value(rs0.into_struct_value(), $val, 1, "rs1")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "insert_value".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_return(Some(&rs1))
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "READY".to_string(),
                            span: None,
                        })?;
                }};
            }

            // ── CASE A: No awaits — compile body inline ───────────────────
            if n_awaits == 0 {
                let ready_bb = self.context.append_basic_block(poll_fn, "ready");

                let mut reached_return = false;
                for stmt in &segments[0] {
                    if let StmtKind::Return { values } = &stmt.kind {
                        if let Some(ra) = async_ret_alloca {
                            if let Some(ret_expr) = values.first() {
                                let (val, _) = self.compile_expr(ret_expr)?;
                                self.builder.build_store(ra, val).map_err(|_| {
                                    CodegenError::LLVMError {
                                        operation: "build_store".to_string(),
                                        details: "ret val".to_string(),
                                        span: None,
                                    }
                                })?;
                            }
                        }
                        self.builder
                            .build_unconditional_branch(ready_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "branch".to_string(),
                                details: "to ready".to_string(),
                                span: None,
                            })?;
                        reached_return = true;
                        break;
                    }
                    self.compile_stmt(stmt, poll_fn)?;
                }
                if !reached_return {
                    if let Some(bb) = self.builder.get_insert_block() {
                        if bb.get_terminator().is_none() {
                            self.builder
                                .build_unconditional_branch(ready_bb)
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "branch".to_string(),
                                    details: "fallthrough ready".to_string(),
                                    span: None,
                                })?;
                        }
                    }
                }

                self.builder.position_at_end(ready_bb);
                let ret_i64 = if let Some(ra) = async_ret_alloca {
                    let lt = self.brix_type_to_llvm(&ret_brix_type);
                    self.builder
                        .build_load(lt, ra, "rv")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "rv".to_string(),
                            span: None,
                        })?
                        .into_int_value()
                } else {
                    i64_type.const_int(0, false)
                };
                return_ready!(ret_i64);
            } else {
                // ── CASE B: N awaits — switch-based state machine ─────────
                // State is at struct field [1] (not [0] like in async fn)

                let sf_state_field_ptr = self
                    .builder
                    .build_struct_gep(state_struct_type, sp, state_field as u32, "sf_state")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: "state".to_string(),
                        span: None,
                    })?;
                let state_val = self
                    .builder
                    .build_load(i64_type, sf_state_field_ptr, "state")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "state".to_string(),
                        span: None,
                    })?
                    .into_int_value();

                let default_bb = self.context.append_basic_block(poll_fn, "sw_default");
                let state_bbs: Vec<inkwell::basic_block::BasicBlock<'ctx>> = (0..=n_awaits)
                    .map(|i| {
                        self.context
                            .append_basic_block(poll_fn, &format!("state_{}", i))
                    })
                    .collect();

                let cases: Vec<(
                    inkwell::values::IntValue<'ctx>,
                    inkwell::basic_block::BasicBlock<'ctx>,
                )> = state_bbs
                    .iter()
                    .enumerate()
                    .map(|(i, bb)| (i64_type.const_int(i as u64, false), *bb))
                    .collect();
                self.builder
                    .build_switch(state_val, default_bb, &cases)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_switch".to_string(),
                        details: "state machine".to_string(),
                        span: None,
                    })?;

                // default → PENDING
                self.builder.position_at_end(default_bb);
                self.builder.build_return(Some(&pending_val)).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_return".to_string(),
                        details: "default".to_string(),
                        span: None,
                    }
                })?;

                // ── State 0: run segments[0], launch first await ────────
                self.builder.position_at_end(state_bbs[0]);

                for stmt in &segments[0] {
                    self.compile_stmt(stmt, poll_fn)?;
                }

                {
                    let ap = &await_points[0];
                    let sf_ptr = if ap.is_variable_await {
                        let var_entry = self.variables.get(&ap.callee_name).ok_or_else(|| {
                            CodegenError::UndefinedSymbol {
                                name: ap.callee_name.clone(),
                                context: "async_block variable await state 0".to_string(),
                                span: None,
                            }
                        })?;
                        let (var_alloca, _) = var_entry.clone();
                        self.builder
                            .build_load(ptr_type, var_alloca, "sf_var")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "sf_var".to_string(),
                                span: None,
                            })?
                            .into_pointer_value()
                    } else {
                        let create_fn = self
                            .async_create_fns
                            .get(&ap.callee_name)
                            .copied()
                            .ok_or_else(|| CodegenError::UndefinedSymbol {
                                name: format!("create_{}", ap.callee_name),
                                context: "async_block state 0".to_string(),
                                span: None,
                            })?;
                        let mut arg_vals: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
                        for arg_expr in &ap.callee_args.clone() {
                            let (v, _) = self.compile_expr(arg_expr)?;
                            arg_vals.push(v.into());
                        }
                        let cc = self
                            .builder
                            .build_call(create_fn, &arg_vals, "sf0")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "create sub_future".to_string(),
                                span: None,
                            })?;
                        cc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "create_callee".to_string(),
                                context: "async_block state 0".to_string(),
                                span: None,
                            })?
                            .into_pointer_value()
                    };

                    let sf_field = self
                        .builder
                        .build_struct_gep(state_struct_type, sp, sub_future_field as u32, "sff")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "sf_field".to_string(),
                            span: None,
                        })?;
                    self.builder.build_store(sf_field, sf_ptr).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "sf".to_string(),
                            span: None,
                        }
                    })?;
                }

                // state = 1 (stored at field[1])
                let sf_state_ptr = self
                    .builder
                    .build_struct_gep(state_struct_type, sp, state_field as u32, "sfs")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "struct_gep".to_string(),
                        details: "state".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(sf_state_ptr, i64_type.const_int(1, false))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "state=1".to_string(),
                        span: None,
                    })?;
                self.builder.build_return(Some(&pending_val)).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_return".to_string(),
                        details: "PENDING s0".to_string(),
                        span: None,
                    }
                })?;

                // ── States 1..=N: poll previous sub_future ────────────────
                for k in 1..=n_awaits {
                    self.builder.position_at_end(state_bbs[k]);

                    let sff = self
                        .builder
                        .build_struct_gep(state_struct_type, sp, sub_future_field as u32, "sff")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "sff".to_string(),
                            span: None,
                        })?;
                    let sf_ptr = self
                        .builder
                        .build_load(ptr_type, sff, "sf")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "sf".to_string(),
                            span: None,
                        })?
                        .into_pointer_value();

                    // Poll the sub_future — always indirect for async blocks (poll_fn_ptr at sf[0])
                    let poll_fn_type_local = poll_result_type.fn_type(&[ptr_type.into()], false);
                    let pr = if await_points[k - 1].is_variable_await {
                        // Variable await: poll_fn_ptr is at sub_future[0]
                        let poll_fn_ptr = self
                            .builder
                            .build_load(ptr_type, sf_ptr, "pfp")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "poll_fn_ptr".to_string(),
                                span: None,
                            })?
                            .into_pointer_value();
                        let pc = self
                            .builder
                            .build_indirect_call(
                                poll_fn_type_local,
                                poll_fn_ptr,
                                &[sf_ptr.into()],
                                "pr",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_indirect_call".to_string(),
                                details: "poll var future".to_string(),
                                span: None,
                            })?;
                        pc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "poll_var".to_string(),
                                context: format!("async_block state {}", k),
                                span: None,
                            })?
                            .into_struct_value()
                    } else {
                        let poll_callee_fn = self
                            .async_poll_fns
                            .get(&await_points[k - 1].callee_name)
                            .copied()
                            .ok_or_else(|| CodegenError::UndefinedSymbol {
                                name: format!("poll_{}", await_points[k - 1].callee_name),
                                context: format!("async_block await state {}", k),
                                span: None,
                            })?;
                        let pc = self
                            .builder
                            .build_call(poll_callee_fn, &[sf_ptr.into()], "pr")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "poll callee".to_string(),
                                span: None,
                            })?;
                        pc.try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "poll_callee".to_string(),
                                context: format!("async_block state {}", k),
                                span: None,
                            })?
                            .into_struct_value()
                    };

                    let status = self
                        .builder
                        .build_extract_value(pr, 0, "pr_status")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "extract_value".to_string(),
                            details: "status".to_string(),
                            span: None,
                        })?
                        .into_int_value();

                    let is_ready = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            status,
                            i64_type.const_int(1, false),
                            "is_ready",
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "int_compare".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;

                    let ready_bb = self
                        .context
                        .append_basic_block(poll_fn, &format!("ready_{}", k));
                    let pending_bb = self
                        .context
                        .append_basic_block(poll_fn, &format!("pending_{}", k));

                    self.builder
                        .build_conditional_branch(is_ready, ready_bb, pending_bb)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "cond_branch".to_string(),
                            details: "".to_string(),
                            span: None,
                        })?;

                    self.builder.position_at_end(pending_bb);
                    self.builder.build_return(Some(&pending_val)).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_return".to_string(),
                            details: "pending".to_string(),
                            span: None,
                        }
                    })?;

                    self.builder.position_at_end(ready_bb);

                    let result_val = self
                        .builder
                        .build_extract_value(pr, 1, "pr_val")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "extract_value".to_string(),
                            details: "value".to_string(),
                            span: None,
                        })?
                        .into_int_value();

                    // Persist result in struct[result_field_start + k - 1]
                    let rfield = self
                        .builder
                        .build_struct_gep(
                            state_struct_type,
                            sp,
                            (result_field_start + k - 1) as u32,
                            &format!("rf{}", k - 1),
                        )
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "struct_gep".to_string(),
                            details: "rfield".to_string(),
                            span: None,
                        })?;
                    self.builder.build_store(rfield, result_val).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "result".to_string(),
                            span: None,
                        }
                    })?;

                    self.builder
                        .build_store(result_allocas[k - 1], result_val)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "result alloca".to_string(),
                            span: None,
                        })?;
                    self.variables.insert(
                        await_points[k - 1].result_var.clone(),
                        (result_allocas[k - 1], await_ret_types[k - 1].clone()),
                    );

                    // Reload previous results into variables
                    for prev in 0..(k - 1) {
                        let lt = self.brix_type_to_llvm(&await_ret_types[prev]);
                        let fi = self
                            .builder
                            .build_struct_gep(
                                state_struct_type,
                                sp,
                                (result_field_start + prev) as u32,
                                &format!("lprf{}", prev),
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: "prev result".to_string(),
                                span: None,
                            })?;
                        let pv = self
                            .builder
                            .build_load(lt, fi, &await_points[prev].result_var)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "prev result".to_string(),
                                span: None,
                            })?;
                        self.builder
                            .build_store(result_allocas[prev], pv)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "prev result alloca".to_string(),
                                span: None,
                            })?;
                        self.variables.insert(
                            await_points[prev].result_var.clone(),
                            (result_allocas[prev], await_ret_types[prev].clone()),
                        );
                    }

                    if k == n_awaits {
                        // Final segment: intercept Return
                        let mut seg_returned = false;
                        for stmt in &segments[k] {
                            if let StmtKind::Return { values } = &stmt.kind {
                                if let Some(ra) = async_ret_alloca {
                                    if let Some(re) = values.first() {
                                        let (v, _) = self.compile_expr(re)?;
                                        self.builder.build_store(ra, v).map_err(|_| {
                                            CodegenError::LLVMError {
                                                operation: "build_store".to_string(),
                                                details: "final ret".to_string(),
                                                span: None,
                                            }
                                        })?;
                                    }
                                }
                                seg_returned = true;
                                break;
                            }
                            self.compile_stmt(stmt, poll_fn)?;
                        }

                        let ret_i64 = if let Some(ra) = async_ret_alloca {
                            let lt = self.brix_type_to_llvm(&ret_brix_type);
                            if let Some(bb) = self.builder.get_insert_block() {
                                if bb.get_terminator().is_none() || seg_returned {
                                    self.builder
                                        .build_load(lt, ra, "rv")
                                        .map_err(|_| CodegenError::LLVMError {
                                            operation: "build_load".to_string(),
                                            details: "rv".to_string(),
                                            span: None,
                                        })?
                                        .into_int_value()
                                } else {
                                    i64_type.const_int(0, false)
                                }
                            } else {
                                i64_type.const_int(0, false)
                            }
                        } else {
                            i64_type.const_int(0, false)
                        };

                        if let Some(bb) = self.builder.get_insert_block() {
                            if bb.get_terminator().is_none() {
                                return_ready!(ret_i64);
                            }
                        }
                    } else {
                        // Intermediate segment: compile, then start next sub_future
                        for stmt in &segments[k] {
                            self.compile_stmt(stmt, poll_fn)?;
                        }

                        let next_ap = &await_points[k];
                        let nsf_ptr = if next_ap.is_variable_await {
                            let var_entry =
                                self.variables.get(&next_ap.callee_name).ok_or_else(|| {
                                    CodegenError::UndefinedSymbol {
                                        name: next_ap.callee_name.clone(),
                                        context: format!("async_block variable await state {}", k),
                                        span: None,
                                    }
                                })?;
                            let (var_alloca, _) = var_entry.clone();
                            self.builder
                                .build_load(ptr_type, var_alloca, "nsf_var")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: "nsf_var".to_string(),
                                    span: None,
                                })?
                                .into_pointer_value()
                        } else {
                            let create_next_fn = self
                                .async_create_fns
                                .get(&next_ap.callee_name)
                                .copied()
                                .ok_or_else(|| CodegenError::UndefinedSymbol {
                                    name: format!("create_{}", next_ap.callee_name),
                                    context: format!("async_block await state {}", k),
                                    span: None,
                                })?;
                            let args_cloned: Vec<Expr> = next_ap.callee_args.clone();
                            let mut av: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
                            for ae in &args_cloned {
                                let (v, _) = self.compile_expr(ae)?;
                                av.push(v.into());
                            }
                            let nc = self
                                .builder
                                .build_call(create_next_fn, &av, "nsf")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_call".to_string(),
                                    details: "create next sf".to_string(),
                                    span: None,
                                })?;
                            nc.try_as_basic_value()
                                .left()
                                .ok_or_else(|| CodegenError::MissingValue {
                                    what: "create_next".to_string(),
                                    context: format!("async_block state {}", k),
                                    span: None,
                                })?
                                .into_pointer_value()
                        };

                        let sff2 = self
                            .builder
                            .build_struct_gep(
                                state_struct_type,
                                sp,
                                sub_future_field as u32,
                                "sff2",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: "sff2".to_string(),
                                span: None,
                            })?;
                        self.builder.build_store(sff2, nsf_ptr).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "nsf".to_string(),
                                span: None,
                            }
                        })?;

                        let sfs2 = self
                            .builder
                            .build_struct_gep(state_struct_type, sp, state_field as u32, "sfs2")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "struct_gep".to_string(),
                                details: "state".to_string(),
                                span: None,
                            })?;
                        self.builder
                            .build_store(sfs2, i64_type.const_int((k + 1) as u64, false))
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: "state+1".to_string(),
                                span: None,
                            })?;
                        self.builder.build_return(Some(&pending_val)).map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_return".to_string(),
                                details: "PENDING inter".to_string(),
                                span: None,
                            }
                        })?;
                    }
                }
            }

            // Restore state — reposition to the EXACT BB we were in before entering
            // the poll fn generation (not just the last BB of the parent function).
            self.current_function = saved_fn;
            self.variables = saved_vars;
            self.function_scope_vars = saved_scope;
            if let Some(bb) = saved_insert_block {
                self.builder.position_at_end(bb);
            } else if let Some(prev) = saved_fn {
                if let Some(bb) = prev.get_last_basic_block() {
                    self.builder.position_at_end(bb);
                }
            }
        }

        // ── 7. Generate create_async_block_N() -> i8* ───────────────────
        let create_name_str = format!("create_{}", block_name);
        {
            let create_fn_type = ptr_type.fn_type(&[], false);
            let create_fn = self
                .module
                .add_function(&create_name_str, create_fn_type, None);
            self.async_create_fns.insert(block_name.clone(), create_fn);

            let saved_fn = self.current_function;
            self.current_function = Some(create_fn);
            let entry_bb = self.context.append_basic_block(create_fn, "entry");
            self.builder.position_at_end(entry_bb);

            // malloc(sizeof state_struct)
            let malloc_fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            let malloc_fn = self.module.get_function("brix_malloc").unwrap_or_else(|| {
                self.module
                    .add_function("brix_malloc", malloc_fn_type, Some(Linkage::External))
            });
            let struct_size =
                state_struct_type
                    .size_of()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "size_of".to_string(),
                        details: "Failed to get size of async block state struct".to_string(),
                        span: None,
                    })?;
            let malloc_call = self
                .builder
                .build_call(malloc_fn, &[struct_size.into()], "sp")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_call".to_string(),
                    details: "malloc async block state".to_string(),
                    span: None,
                })?;
            let state_ptr = malloc_call
                .try_as_basic_value()
                .left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "malloc".to_string(),
                    context: "create_async_block".to_string(),
                    span: None,
                })?
                .into_pointer_value();

            // Store poll_fn at field [0]
            let poll_fn_field = self
                .builder
                .build_struct_gep(state_struct_type, state_ptr, 0, "pff")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "struct_gep".to_string(),
                    details: "poll_fn_field".to_string(),
                    span: None,
                })?;
            let poll_fn_ref = self
                .async_poll_fns
                .get(&block_name)
                .copied()
                .ok_or_else(|| CodegenError::UndefinedSymbol {
                    name: poll_name_str.clone(),
                    context: "create_async_block".to_string(),
                    span: None,
                })?;
            let poll_fn_ptr = poll_fn_ref.as_global_value().as_pointer_value();
            self.builder
                .build_store(poll_fn_field, poll_fn_ptr)
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "poll_fn".to_string(),
                    span: None,
                })?;

            // state = 0 at field [1]
            let state_f1 = self
                .builder
                .build_struct_gep(state_struct_type, state_ptr, state_field as u32, "sf1")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "struct_gep".to_string(),
                    details: "state=0".to_string(),
                    span: None,
                })?;
            self.builder
                .build_store(state_f1, i64_type.const_int(0, false))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_store".to_string(),
                    details: "state=0".to_string(),
                    span: None,
                })?;

            self.builder
                .build_return(Some(&state_ptr))
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_return".to_string(),
                    details: "create_async_block".to_string(),
                    span: None,
                })?;

            // Restore to the exact insertion block we had before generating this create fn
            self.current_function = saved_fn;
            if let Some(bb) = saved_insert_block {
                self.builder.position_at_end(bb);
            } else if let Some(prev) = saved_fn {
                if let Some(bb) = prev.get_last_basic_block() {
                    self.builder.position_at_end(bb);
                }
            }
        }

        // ── 8. Emit call to create_async_block_N() at the current call site ─
        let create_fn_ref = self
            .async_create_fns
            .get(&block_name)
            .copied()
            .ok_or_else(|| CodegenError::UndefinedSymbol {
                name: create_name_str.clone(),
                context: "compile_async_block".to_string(),
                span: None,
            })?;
        let cc = self
            .builder
            .build_call(create_fn_ref, &[], "blk_sp")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: "create_async_block".to_string(),
                span: None,
            })?;
        let state_ptr_val =
            cc.try_as_basic_value()
                .left()
                .ok_or_else(|| CodegenError::MissingValue {
                    what: "create_async_block".to_string(),
                    context: "compile_async_block".to_string(),
                    span: None,
                })?;

        Ok((state_ptr_val, BrixType::AsyncFuture))
    }
}
