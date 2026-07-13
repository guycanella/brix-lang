// Matrix array methods (sort, min, max, flatten, unique, reverse, append, prepend)
//
// This module contains declarations for the v1.7 Group B array method runtime
// functions operating on Matrix (f64) and IntMatrix (i64).

use crate::Compiler;
use inkwell::module::Linkage;
use inkwell::AddressSpace;

/// Trait for array-method runtime function declarations (v1.7 Group B).
pub trait MatrixFunctions<'ctx> {
    // ===== Sort =====

    /// Get or declare: Matrix* matrix_sort_asc(Matrix*)
    fn get_matrix_sort_asc(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: Matrix* matrix_sort_desc(Matrix*)
    fn get_matrix_sort_desc(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: IntMatrix* intmatrix_sort_asc(IntMatrix*)
    fn get_intmatrix_sort_asc(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: IntMatrix* intmatrix_sort_desc(IntMatrix*)
    fn get_intmatrix_sort_desc(&self) -> inkwell::values::FunctionValue<'ctx>;

    // ===== Min / Max =====

    /// Get or declare: double brix_matrix_min(Matrix*)
    fn get_brix_matrix_min(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: double brix_matrix_max(Matrix*)
    fn get_brix_matrix_max(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: long brix_intmatrix_min(IntMatrix*)
    fn get_brix_intmatrix_min(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: long brix_intmatrix_max(IntMatrix*)
    fn get_brix_intmatrix_max(&self) -> inkwell::values::FunctionValue<'ctx>;

    // ===== Flatten =====

    /// Get or declare: Matrix* matrix_flatten(Matrix*)
    fn get_matrix_flatten(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: IntMatrix* intmatrix_flatten(IntMatrix*)
    fn get_intmatrix_flatten(&self) -> inkwell::values::FunctionValue<'ctx>;

    // ===== Unique =====

    /// Get or declare: Matrix* matrix_unique(Matrix*)
    fn get_matrix_unique(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: IntMatrix* intmatrix_unique(IntMatrix*)
    fn get_intmatrix_unique(&self) -> inkwell::values::FunctionValue<'ctx>;

    // ===== Reverse =====

    /// Get or declare: Matrix* matrix_reverse(Matrix*)
    fn get_matrix_reverse(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: IntMatrix* intmatrix_reverse(IntMatrix*)
    fn get_intmatrix_reverse(&self) -> inkwell::values::FunctionValue<'ctx>;

    // ===== Append =====

    /// Get or declare: Matrix* matrix_append(Matrix*, double)
    fn get_matrix_append(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: IntMatrix* intmatrix_append(IntMatrix*, long)
    fn get_intmatrix_append(&self) -> inkwell::values::FunctionValue<'ctx>;

    // ===== Prepend =====

    /// Get or declare: Matrix* matrix_prepend(Matrix*, double)
    fn get_matrix_prepend(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: IntMatrix* intmatrix_prepend(IntMatrix*, long)
    fn get_intmatrix_prepend(&self) -> inkwell::values::FunctionValue<'ctx>;

    // ===== Slice (v1.7 Group C) =====

    /// Get or declare: Matrix* matrix_slice(Matrix*, long start, long end)
    fn get_matrix_slice(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare: IntMatrix* intmatrix_slice(IntMatrix*, long start, long end)
    fn get_intmatrix_slice(&self) -> inkwell::values::FunctionValue<'ctx>;
}

impl<'a, 'ctx> MatrixFunctions<'ctx> for Compiler<'a, 'ctx> {
    fn get_matrix_sort_asc(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("matrix_sort_asc") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("matrix_sort_asc", fn_type, Some(Linkage::External))
    }

    fn get_matrix_sort_desc(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("matrix_sort_desc") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("matrix_sort_desc", fn_type, Some(Linkage::External))
    }

    fn get_intmatrix_sort_asc(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("intmatrix_sort_asc") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("intmatrix_sort_asc", fn_type, Some(Linkage::External))
    }

    fn get_intmatrix_sort_desc(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("intmatrix_sort_desc") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("intmatrix_sort_desc", fn_type, Some(Linkage::External))
    }

    fn get_brix_matrix_min(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_matrix_min") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let f64_type = self.context.f64_type();
        let fn_type = f64_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("brix_matrix_min", fn_type, Some(Linkage::External))
    }

    fn get_brix_matrix_max(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_matrix_max") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let f64_type = self.context.f64_type();
        let fn_type = f64_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("brix_matrix_max", fn_type, Some(Linkage::External))
    }

    fn get_brix_intmatrix_min(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_intmatrix_min") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("brix_intmatrix_min", fn_type, Some(Linkage::External))
    }

    fn get_brix_intmatrix_max(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_intmatrix_max") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("brix_intmatrix_max", fn_type, Some(Linkage::External))
    }

    fn get_matrix_flatten(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("matrix_flatten") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("matrix_flatten", fn_type, Some(Linkage::External))
    }

    fn get_intmatrix_flatten(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("intmatrix_flatten") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("intmatrix_flatten", fn_type, Some(Linkage::External))
    }

    fn get_matrix_unique(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("matrix_unique") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("matrix_unique", fn_type, Some(Linkage::External))
    }

    fn get_intmatrix_unique(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("intmatrix_unique") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("intmatrix_unique", fn_type, Some(Linkage::External))
    }

    fn get_matrix_reverse(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("matrix_reverse") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("matrix_reverse", fn_type, Some(Linkage::External))
    }

    fn get_intmatrix_reverse(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("intmatrix_reverse") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module
            .add_function("intmatrix_reverse", fn_type, Some(Linkage::External))
    }

    fn get_matrix_append(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("matrix_append") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let f64_type = self.context.f64_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), f64_type.into()], false);
        self.module
            .add_function("matrix_append", fn_type, Some(Linkage::External))
    }

    fn get_intmatrix_append(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("intmatrix_append") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
        self.module
            .add_function("intmatrix_append", fn_type, Some(Linkage::External))
    }

    fn get_matrix_prepend(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("matrix_prepend") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let f64_type = self.context.f64_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), f64_type.into()], false);
        self.module
            .add_function("matrix_prepend", fn_type, Some(Linkage::External))
    }

    fn get_intmatrix_prepend(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("intmatrix_prepend") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
        self.module
            .add_function("intmatrix_prepend", fn_type, Some(Linkage::External))
    }

    fn get_matrix_slice(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("matrix_slice") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into(), i64_type.into()], false);
        self.module
            .add_function("matrix_slice", fn_type, Some(Linkage::External))
    }

    fn get_intmatrix_slice(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("intmatrix_slice") {
            return func;
        }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into(), i64_type.into()], false);
        self.module
            .add_function("intmatrix_slice", fn_type, Some(Linkage::External))
    }
}
