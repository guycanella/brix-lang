// Test library functions for Brix (v1.5)
//
// Declares all test runtime functions as external LLVM declarations, and
// (since refactor Extraction 2) hosts the Test Library call/matcher
// compilation logic: try_compile_test_call, compile_test_module_call,
// compile_test_hook_register, compile_test_matcher, declare_test_matcher_void.
// These link to the C implementations in runtime.c SECTION 8.
//
// API:
//   test.describe("Suite", () -> { ... })  → test_describe_start(ptr, ptr)
//   test.it("name", () -> { ... })         → test_it_register(ptr, ptr)
//   test.beforeAll(() -> { ... })          → test_before_all_register(ptr)
//   test.afterAll(() -> { ... })           → test_after_all_register(ptr)
//   test.beforeEach(() -> { ... })         → test_before_each_register(ptr)
//   test.afterEach(() -> { ... })          → test_after_each_register(ptr)
//   test.expect(x).toBe(y)                 → compile_test_matcher (this file)

use crate::{Compiler, BrixType, CodegenError, CodegenResult};
use inkwell::module::Linkage;
use inkwell::types::BasicType;
use inkwell::values::FunctionValue;
use inkwell::{AddressSpace, IntPredicate};
use parser::ast::{ExprKind, Literal};

/// Trait for test library function declarations
pub trait TestFunctions<'ctx> {
    /// Declare a void function that takes two ptr arguments
    fn declare_test_fn_ptr_ptr(&self, name: &str) -> FunctionValue<'ctx>;

    /// Declare a void function that takes one ptr argument
    fn declare_test_fn_ptr(&self, name: &str) -> FunctionValue<'ctx>;

    /// Register all test framework functions
    fn register_test_functions(&mut self, prefix: &str);
}

impl<'a, 'ctx> TestFunctions<'ctx> for Compiler<'a, 'ctx> {
    fn declare_test_fn_ptr_ptr(&self, name: &str) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let void_type = self.context.void_type();
        let fn_type = void_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        self.module.add_function(name, fn_type, Some(Linkage::External))
    }

    fn declare_test_fn_ptr(&self, name: &str) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let void_type = self.context.void_type();
        let fn_type = void_type.fn_type(&[ptr_type.into()], false);
        self.module.add_function(name, fn_type, Some(Linkage::External))
    }

    fn register_test_functions(&mut self, _prefix: &str) {
        // Suite management
        // test_describe_start(BrixString* title, void* closure_ptr) -> void
        self.declare_test_fn_ptr_ptr("test_describe_start");

        // test_it_register(BrixString* title, void* closure_ptr) -> void
        self.declare_test_fn_ptr_ptr("test_it_register");

        // Lifecycle hooks (each takes a single void* closure_ptr)
        self.declare_test_fn_ptr("test_before_all_register");
        self.declare_test_fn_ptr("test_after_all_register");
        self.declare_test_fn_ptr("test_before_each_register");
        self.declare_test_fn_ptr("test_after_each_register");

        // test_it_async(BrixString* title, void* state_ptr) -> void
        // Used for async closures in test.it() (v1.6 Phase 3d)
        self.declare_test_fn_ptr_ptr("test_it_async");

        // Matcher functions are declared on-demand in compile_test_matcher()
        // because they depend on the actual value type being matched
    }
}

// --- Test Library call/matcher compilation (moved from lib.rs, refactor Extraction 2) ---
impl<'a, 'ctx> Compiler<'a, 'ctx> {
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
    pub(crate) fn try_compile_test_call(
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
                // test.it("title", closure_or_async_closure)
                if args.len() < 2 {
                    return Err(CodegenError::InvalidOperation {
                        operation: "test.it".to_string(),
                        reason: "requires two arguments: (title, closure)".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let (title_val, _) = self.compile_expr(&args[0])?;
                let (callback_val, callback_type) = self.compile_expr(&args[1])?;

                let fn_type = void_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);

                if matches!(callback_type, BrixType::AsyncFuture) {
                    let it_fn = self.module.get_function("test_it_async")
                        .unwrap_or_else(|| {
                            self.module.add_function("test_it_async", fn_type, Some(inkwell::module::Linkage::External))
                        });
                    self.builder.build_call(
                        it_fn,
                        &[title_val.into(), callback_val.into()],
                        "test_it_async",
                    ).map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call test_it_async".to_string(),
                        span: Some(span.clone()),
                    })?;
                } else {
                    let it_fn = self.module.get_function("test_it_register")
                        .unwrap_or_else(|| {
                            self.module.add_function("test_it_register", fn_type, Some(inkwell::module::Linkage::External))
                        });
                    self.builder.build_call(
                        it_fn,
                        &[title_val.into(), callback_val.into()],
                        "test_it",
                    ).map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call test_it_register".to_string(),
                        span: Some(span.clone()),
                    })?;
                }
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
            "toStartWith" => {
                if matcher_args.is_empty() {
                    return Err(CodegenError::InvalidOperation {
                        operation: "toStartWith".to_string(),
                        reason: "requires one string argument".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let (prefix_val, prefix_type) = self.compile_expr(&matcher_args[0])?;
                if actual_type != BrixType::String || prefix_type != BrixType::String {
                    return Err(CodegenError::TypeError {
                        expected: "string receiver and string prefix".to_string(),
                        found: format!("{:?} receiver and {:?} prefix", actual_type, prefix_type),
                        context: "toStartWith".to_string(),
                        span: Some(span.clone()),
                    });
                }
                match &actual_type {
                    BrixType::String => {
                        let fn_name = format!("test_expect_{}toStartWith_string", not_prefix);
                        let f = self.declare_test_matcher_void(
                            &fn_name,
                            &[ptr_type.into(), ptr_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        self.builder.build_call(f, &[actual_val.into(), prefix_val.into(), file_ptr.into(), line_val.into()], "tsw")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                    }
                    _ => unreachable!("toStartWith type check should reject non-string receivers"),
                }
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toEndWith" => {
                if matcher_args.is_empty() {
                    return Err(CodegenError::InvalidOperation {
                        operation: "toEndWith".to_string(),
                        reason: "requires one string argument".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let (suffix_val, suffix_type) = self.compile_expr(&matcher_args[0])?;
                if actual_type != BrixType::String || suffix_type != BrixType::String {
                    return Err(CodegenError::TypeError {
                        expected: "string receiver and string suffix".to_string(),
                        found: format!("{:?} receiver and {:?} suffix", actual_type, suffix_type),
                        context: "toEndWith".to_string(),
                        span: Some(span.clone()),
                    });
                }
                match &actual_type {
                    BrixType::String => {
                        let fn_name = format!("test_expect_{}toEndWith_string", not_prefix);
                        let f = self.declare_test_matcher_void(
                            &fn_name,
                            &[ptr_type.into(), ptr_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        self.builder.build_call(f, &[actual_val.into(), suffix_val.into(), file_ptr.into(), line_val.into()], "tew")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                    }
                    _ => unreachable!("toEndWith type check should reject non-string receivers"),
                }
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toMatch" => {
                if matcher_args.is_empty() {
                    return Err(CodegenError::InvalidOperation {
                        operation: "toMatch".to_string(),
                        reason: "requires one string pattern argument".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let (pattern_val, pattern_type) = self.compile_expr(&matcher_args[0])?;
                if actual_type != BrixType::String || pattern_type != BrixType::String {
                    return Err(CodegenError::TypeError {
                        expected: "string receiver and string pattern".to_string(),
                        found: format!("{:?} receiver and {:?} pattern", actual_type, pattern_type),
                        context: "toMatch".to_string(),
                        span: Some(span.clone()),
                    });
                }
                match &actual_type {
                    BrixType::String => {
                        let fn_name = format!("test_expect_{}matches_string", not_prefix);
                        let f = self.declare_test_matcher_void(
                            &fn_name,
                            &[ptr_type.into(), ptr_type.into(), ptr_type.into(), i32_type.into()],
                        );
                        self.builder.build_call(f, &[actual_val.into(), pattern_val.into(), file_ptr.into(), line_val.into()], "tm")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                    }
                    _ => unreachable!("toMatch type check should reject non-string receivers"),
                }
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toHaveProperty" => {
                // Resolved at compile-time: check the static struct definition.
                if matcher_args.is_empty() {
                    return Err(CodegenError::InvalidOperation {
                        operation: "toHaveProperty".to_string(),
                        reason: "requires one string-literal argument".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let struct_name = match &actual_type {
                    BrixType::Struct(name) => name.clone(),
                    other => {
                        return Err(CodegenError::TypeError {
                            expected: "struct".to_string(),
                            found: format!("{:?}", other),
                            context: "toHaveProperty".to_string(),
                            span: Some(span.clone()),
                        });
                    }
                };
                let prop_name = match &matcher_args[0].kind {
                    ExprKind::Literal(Literal::String(s)) => s.clone(),
                    _ => {
                        return Err(CodegenError::TypeError {
                            expected: "string literal".to_string(),
                            found: "non-literal expression".to_string(),
                            context: "toHaveProperty property name".to_string(),
                            span: Some(span.clone()),
                        });
                    }
                };
                let has_prop = match self.struct_defs.get(&struct_name) {
                    Some(fields) => fields.iter().any(|(n, _, _)| n == &prop_name),
                    None => {
                        return Err(CodegenError::UndefinedSymbol {
                            name: struct_name.clone(),
                            context: "toHaveProperty struct definition".to_string(),
                            span: Some(span.clone()),
                        });
                    }
                };
                let has_prop_val = i32_type.const_int(has_prop as u64, false);
                let (prop_name_ptr, _) = self.compile_expr(&matcher_args[0])?;
                let fn_name = format!("test_expect_{}has_property", not_prefix);
                let f = self.declare_test_matcher_void(
                    &fn_name,
                    &[i32_type.into(), ptr_type.into(), ptr_type.into(), i32_type.into()],
                );
                self.builder.build_call(f, &[has_prop_val.into(), prop_name_ptr.into(), file_ptr.into(), line_val.into()], "thp")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                Ok((dummy_val, BrixType::Nil))
            }

            // ──────────────────────────────────────────────────────────────
            "toBeNil" => {
                // For optional/nil: if actual is a pointer, check if null.
                // For union types (struct): extract tag field (field 0) and check if == 1 (nil tag).
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
                } else if actual_val.is_struct_value() {
                    // For struct-based union types: extract tag (field 0) and check if == 1 (nil tag)
                    let nil_tag_val = i64_type.const_int(1, false);
                    let tag = self.builder.build_extract_value(actual_val.into_struct_value(), 0, "union_tag")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_extract_value".to_string(), details: "Failed to extract union tag for toBeNil".to_string(), span: Some(span.clone()) })?;
                    let is_nil_cmp = self.builder.build_int_compare(IntPredicate::EQ, tag.into_int_value(), nil_tag_val, "is_nil_cmp")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "".to_string(), span: Some(span.clone()) })?;
                    self.builder.build_int_z_extend(is_nil_cmp, i64_type, "nil_i64")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_int_z_extend".to_string(), details: "".to_string(), span: Some(span.clone()) })?
                        .into()
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
            "toThrow" => {
                // RESTRICTED SCOPE (v1.7 Grupo H): only a synchronous,
                // zero-parameter closure *literal* is supported.
                let closure = match &actual_expr.kind {
                    ExprKind::Closure(c) => c,
                    _ => {
                        return Err(CodegenError::TypeError {
                            expected: "closure literal".to_string(),
                            found: format!("{:?}", actual_type),
                            context: "toThrow only supports a synchronous, zero-parameter closure literal for now".to_string(),
                            span: Some(span.clone()),
                        });
                    }
                };
                if !closure.params.is_empty() {
                    return Err(CodegenError::TypeError {
                        expected: "zero-parameter closure".to_string(),
                        found: format!("closure with {} parameter(s)", closure.params.len()),
                        context: "toThrow only supports a synchronous, zero-parameter closure literal for now".to_string(),
                        span: Some(span.clone()),
                    });
                }
                if closure.is_async {
                    return Err(CodegenError::TypeError {
                        expected: "synchronous closure".to_string(),
                        found: "async closure".to_string(),
                        context: "toThrow only supports a synchronous, zero-parameter closure literal for now".to_string(),
                        span: Some(span.clone()),
                    });
                }
                if !matcher_args.is_empty() {
                    return Err(CodegenError::InvalidOperation {
                        operation: "toThrow".to_string(),
                        reason: "toThrow() takes no arguments".to_string(),
                        span: Some(span.clone()),
                    });
                }

                // Extract (fn_ptr, env_ptr) from the already-compiled closure value.
                let (fn_ptr, env_ptr) = self.load_closure_fn_env(actual_val, span)?;

                // Determine the closure's return type so the indirect call's
                // fn_type is valid (single param env_ptr: ptr, return = inferred).
                let ret_brix_type = if let Some(rt) = &closure.return_type {
                    self.string_to_brix_type(rt)
                } else {
                    self.infer_return_type_from_body(&closure.body, &closure.params)
                        .unwrap_or(BrixType::Void)
                };
                let closure_fn_type = if ret_brix_type == BrixType::Void {
                    self.context.void_type().fn_type(&[ptr_type.into()], false)
                } else {
                    self.brix_type_to_llvm(&ret_brix_type)
                        .fn_type(&[ptr_type.into()], false)
                };

                // Declare libc externals (idempotent).
                let fork_fn = self.module.get_function("fork").unwrap_or_else(|| {
                    self.module.add_function("fork", i32_type.fn_type(&[], false), Some(Linkage::External))
                });
                let fflush_fn = self.module.get_function("fflush").unwrap_or_else(|| {
                    self.module.add_function("fflush", i32_type.fn_type(&[ptr_type.into()], false), Some(Linkage::External))
                });
                let exit_fn = self.module.get_function("_exit").unwrap_or_else(|| {
                    self.module.add_function("_exit", self.context.void_type().fn_type(&[i32_type.into()], false), Some(Linkage::External))
                });
                let wait_fn = self.module.get_function("brix_wait_for_child").unwrap_or_else(|| {
                    self.module.add_function("brix_wait_for_child", i32_type.fn_type(&[i32_type.into()], false), Some(Linkage::External))
                });

                // Flush all open streams before forking so buffered output
                // isn't duplicated in the child.
                self.builder.build_call(fflush_fn, &[ptr_type.const_null().into()], "fflush_all")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "fflush".to_string(), span: Some(span.clone()) })?;

                // pid = fork()
                let pid = self.builder.build_call(fork_fn, &[], "fork_pid")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "fork".to_string(), span: Some(span.clone()) })?
                    .try_as_basic_value().left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "fork() result".to_string(), context: "toThrow".to_string(), span: Some(span.clone()) })?
                    .into_int_value();

                let parent_fn = self.current_function()?;
                let child_bb  = self.context.append_basic_block(parent_fn, "throw_child_bb");
                let parent_bb = self.context.append_basic_block(parent_fn, "throw_parent_bb");
                let merge_bb  = self.context.append_basic_block(parent_fn, "throw_merge_bb");

                let is_child = self.builder.build_int_compare(IntPredicate::EQ, pid, i32_type.const_int(0, false), "is_child")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "fork pid".to_string(), span: Some(span.clone()) })?;
                self.builder.build_conditional_branch(is_child, child_bb, parent_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "fork branch".to_string(), span: Some(span.clone()) })?;

                // Child: run the closure body. If it returns normally (no panic),
                // exit cleanly with 0; brix_panic() calls exit(1) on its own path.
                self.builder.position_at_end(child_bb);
                self.builder.build_indirect_call(closure_fn_type, fn_ptr, &[env_ptr.into()], "throw_call")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_indirect_call".to_string(), details: "toThrow closure".to_string(), span: Some(span.clone()) })?;
                self.builder.build_call(exit_fn, &[i32_type.const_int(0, false).into()], "child_exit")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "_exit".to_string(), span: Some(span.clone()) })?;
                self.builder.build_unreachable()
                    .map_err(|_| CodegenError::LLVMError { operation: "build_unreachable".to_string(), details: "after _exit".to_string(), span: Some(span.clone()) })?;

                // Parent: wait for the child, then dispatch to the matcher.
                self.builder.position_at_end(parent_bb);
                let threw = self.builder.build_call(wait_fn, &[pid.into()], "threw")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "brix_wait_for_child".to_string(), span: Some(span.clone()) })?
                    .try_as_basic_value().left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "brix_wait_for_child result".to_string(), context: "toThrow".to_string(), span: Some(span.clone()) })?;
                let fn_name = format!("test_expect_{}to_throw", not_prefix);
                let f = self.declare_test_matcher_void(&fn_name, &[i32_type.into(), ptr_type.into(), i32_type.into()]);
                self.builder.build_call(f, &[threw.into(), file_ptr.into(), line_val.into()], "ttm")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: fn_name.clone(), span: Some(span.clone()) })?;
                self.builder.build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "parent to merge".to_string(), span: Some(span.clone()) })?;

                self.builder.position_at_end(merge_bb);
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
