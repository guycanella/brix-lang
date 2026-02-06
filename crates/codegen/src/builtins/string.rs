// String functions (uppercase, lowercase, replace, etc.)
//
// This module contains declarations and compilation logic for string functions.

use crate::Compiler;
use inkwell::module::Linkage;
use inkwell::AddressSpace;

/// Trait for string function declarations
pub trait StringFunctions<'ctx> {
    /// Get or declare uppercase function: BrixString* brix_uppercase(BrixString*)
    fn get_uppercase(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare lowercase function: BrixString* brix_lowercase(BrixString*)
    fn get_lowercase(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare capitalize function: BrixString* brix_capitalize(BrixString*)
    fn get_capitalize(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare byte_size function: long brix_byte_size(BrixString*)
    fn get_byte_size(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare length function: long brix_length(BrixString*)
    fn get_length(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare replace function: BrixString* brix_replace(BrixString*, BrixString*, BrixString*)
    fn get_replace(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare replace_all function: BrixString* brix_replace_all(BrixString*, BrixString*, BrixString*)
    fn get_replace_all(&self) -> inkwell::values::FunctionValue<'ctx>;
}

impl<'a, 'ctx> StringFunctions<'ctx> for Compiler<'a, 'ctx> {
    fn get_uppercase(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_uppercase") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // BrixString* brix_uppercase(BrixString* str)
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);

        self.module
            .add_function("brix_uppercase", fn_type, Some(Linkage::External))
    }

    fn get_lowercase(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_lowercase") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // BrixString* brix_lowercase(BrixString* str)
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);

        self.module
            .add_function("brix_lowercase", fn_type, Some(Linkage::External))
    }

    fn get_capitalize(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_capitalize") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // BrixString* brix_capitalize(BrixString* str)
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);

        self.module
            .add_function("brix_capitalize", fn_type, Some(Linkage::External))
    }

    fn get_byte_size(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_byte_size") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();

        // long brix_byte_size(BrixString* str)
        let fn_type = i64_type.fn_type(&[ptr_type.into()], false);

        self.module
            .add_function("brix_byte_size", fn_type, Some(Linkage::External))
    }

    fn get_length(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_length") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();

        // long brix_length(BrixString* str)
        let fn_type = i64_type.fn_type(&[ptr_type.into()], false);

        self.module
            .add_function("brix_length", fn_type, Some(Linkage::External))
    }

    fn get_replace(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_replace") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // BrixString* brix_replace(BrixString* str, BrixString* old, BrixString* new)
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false);

        self.module
            .add_function("brix_replace", fn_type, Some(Linkage::External))
    }

    fn get_replace_all(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_replace_all") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());

        // BrixString* brix_replace_all(BrixString* str, BrixString* old, BrixString* new)
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false);

        self.module
            .add_function("brix_replace_all", fn_type, Some(Linkage::External))
    }
}
