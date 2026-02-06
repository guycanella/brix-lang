// Statistics functions (sum, mean, std, etc.)
//
// This module contains declarations and compilation logic for statistics functions.

use crate::Compiler;
use inkwell::module::Linkage;
use inkwell::AddressSpace;

/// Trait for statistics function declarations
pub trait StatsFunctions<'ctx> {
    /// Declare a statistics function with signature: f64 function(Matrix*)
    fn declare_stats_function(&self, name: &str) -> inkwell::values::FunctionValue<'ctx>;
}

impl<'a, 'ctx> StatsFunctions<'ctx> for Compiler<'a, 'ctx> {
    fn declare_stats_function(&self, name: &str) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function(name) {
            return fn_val;
        }
        let f64_type = self.context.f64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = f64_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function(name, fn_type, Some(Linkage::External))
    }
}
