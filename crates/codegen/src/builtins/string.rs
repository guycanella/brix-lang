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

    // ===== v1.6 String Methods =====

    /// Get or declare trim function: BrixString* brix_str_trim(BrixString*)
    fn get_str_trim(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare ltrim function: BrixString* brix_str_ltrim(BrixString*)
    fn get_str_ltrim(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare rtrim function: BrixString* brix_str_rtrim(BrixString*)
    fn get_str_rtrim(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare starts_with function: long brix_str_starts_with(BrixString*, BrixString*)
    fn get_str_starts_with(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare ends_with function: long brix_str_ends_with(BrixString*, BrixString*)
    fn get_str_ends_with(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare contains function: long brix_str_contains(BrixString*, BrixString*)
    fn get_str_contains(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare substring function: BrixString* brix_str_substring(BrixString*, long, long)
    fn get_str_substring(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare reverse function: BrixString* brix_str_reverse(BrixString*)
    fn get_str_reverse(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare repeat function: BrixString* brix_str_repeat(BrixString*, long)
    fn get_str_repeat(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare index_of function: long brix_str_index_of(BrixString*, BrixString*)
    fn get_str_index_of(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare char_at function: BrixString* brix_str_char_at(BrixString*, long)
    fn get_str_char_at(&self) -> inkwell::values::FunctionValue<'ctx>;
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

    // ===== v1.6 String Methods =====

    fn get_str_trim(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_trim") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module.add_function("brix_str_trim", fn_type, Some(Linkage::External))
    }

    fn get_str_ltrim(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_ltrim") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module.add_function("brix_str_ltrim", fn_type, Some(Linkage::External))
    }

    fn get_str_rtrim(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_rtrim") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module.add_function("brix_str_rtrim", fn_type, Some(Linkage::External))
    }

    fn get_str_starts_with(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_starts_with") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        self.module.add_function("brix_str_starts_with", fn_type, Some(Linkage::External))
    }

    fn get_str_ends_with(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_ends_with") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        self.module.add_function("brix_str_ends_with", fn_type, Some(Linkage::External))
    }

    fn get_str_contains(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_contains") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        self.module.add_function("brix_str_contains", fn_type, Some(Linkage::External))
    }

    fn get_str_substring(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_substring") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        // BrixString* brix_str_substring(BrixString* str, long start, long end)
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into(), i64_type.into()], false);
        self.module.add_function("brix_str_substring", fn_type, Some(Linkage::External))
    }

    fn get_str_reverse(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_reverse") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
        self.module.add_function("brix_str_reverse", fn_type, Some(Linkage::External))
    }

    fn get_str_repeat(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_repeat") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        // BrixString* brix_str_repeat(BrixString* str, long n)
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
        self.module.add_function("brix_str_repeat", fn_type, Some(Linkage::External))
    }

    fn get_str_index_of(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_index_of") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let fn_type = i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
        self.module.add_function("brix_str_index_of", fn_type, Some(Linkage::External))
    }

    fn get_str_char_at(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("brix_str_char_at") { return func; }
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        // BrixString* brix_str_char_at(BrixString* str, long idx)
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
        self.module.add_function("brix_str_char_at", fn_type, Some(Linkage::External))
    }
}
