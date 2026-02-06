// Linear algebra functions (det, inv, eigvals, etc.)
//
// This module contains declarations and compilation logic for linear algebra functions.

use crate::Compiler;
use inkwell::module::Linkage;
use inkwell::AddressSpace;

/// Trait for linear algebra function declarations
pub trait LinalgFunctions<'ctx> {
    /// Declare a linear algebra function with signature: Matrix* function(Matrix*)
    fn declare_linalg_function(&self, name: &str) -> inkwell::values::FunctionValue<'ctx>;

    /// Declare eigenvalue functions with signature: ComplexMatrix* function(Matrix*)
    fn declare_eigen_function(&self, name: &str) -> inkwell::values::FunctionValue<'ctx>;

    /// Declare matrix constructor with signature: Matrix* function(i64)
    fn declare_matrix_constructor(&self, name: &str) -> inkwell::values::FunctionValue<'ctx>;
}

impl<'a, 'ctx> LinalgFunctions<'ctx> for Compiler<'a, 'ctx> {
    fn declare_linalg_function(&self, name: &str) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function(name) {
            return fn_val;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function(name, fn_type, Some(Linkage::External))
    }

    fn declare_eigen_function(&self, name: &str) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function(name) {
            return fn_val;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        // ComplexMatrix* function(Matrix* A)
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function(name, fn_type, Some(Linkage::External))
    }

    fn declare_matrix_constructor(&self, name: &str) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function(name) {
            return fn_val;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
        self.module
            .add_function(name, fn_type, Some(Linkage::External))
    }
}
