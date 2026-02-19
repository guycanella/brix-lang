// Test library functions for Brix (v1.5)
//
// Declares all test runtime functions as external LLVM declarations.
// These link to the C implementations in runtime.c SECTION 8.
//
// API:
//   test.describe("Suite", () -> { ... })  → test_describe_start(ptr, ptr)
//   test.it("name", () -> { ... })         → test_it_register(ptr, ptr)
//   test.beforeAll(() -> { ... })          → test_before_all_register(ptr)
//   test.afterAll(() -> { ... })           → test_after_all_register(ptr)
//   test.beforeEach(() -> { ... })         → test_before_each_register(ptr)
//   test.afterEach(() -> { ... })          → test_after_each_register(ptr)
//   test.expect(x).toBe(y)                 → special codegen in lib.rs

use crate::Compiler;
use inkwell::module::Linkage;
use inkwell::values::FunctionValue;
use inkwell::AddressSpace;

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

        // Matcher functions are declared on-demand in compile_test_matcher()
        // because they depend on the actual value type being matched
    }
}
