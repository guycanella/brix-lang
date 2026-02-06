// LLVM helper functions
//
// This module contains utility functions for LLVM code generation.
//
// REFACTORING NOTE (v1.2):
// - Extracted from lib.rs (originally 88 lines)
// - Uses trait pattern for clean separation
// - All functions available on Compiler via HelperFunctions trait
//
// Functions provided:
// - create_entry_block_alloca() - Allocate in function entry block
// - get_printf(), get_scanf(), get_sprintf() - C stdio functions
// - get_atoi(), get_atof() - String conversion functions

use crate::{Compiler, CodegenError, CodegenResult};
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::PointerValue;
use inkwell::AddressSpace;

/// Trait for LLVM helper functions
pub trait HelperFunctions<'ctx> {
    /// Create an alloca instruction in the entry block of the current function
    fn create_entry_block_alloca(&self, ty: BasicTypeEnum<'ctx>, name: &str) -> CodegenResult<PointerValue<'ctx>>;

    /// Get or declare printf function
    fn get_printf(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare scanf function
    fn get_scanf(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare sprintf function
    fn get_sprintf(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare atoi function (string to int)
    fn get_atoi(&self) -> inkwell::values::FunctionValue<'ctx>;

    /// Get or declare atof function (string to float)
    fn get_atof(&self) -> inkwell::values::FunctionValue<'ctx>;
}

impl<'a, 'ctx> HelperFunctions<'ctx> for Compiler<'a, 'ctx> {
    fn create_entry_block_alloca(&self, ty: BasicTypeEnum<'ctx>, name: &str) -> CodegenResult<PointerValue<'ctx>> {
        let builder = self.context.create_builder();

        let block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "get_insert_block".to_string(),
                details: "No current basic block".to_string(),
            })?;

        let parent = block
            .get_parent()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "get_parent".to_string(),
                details: "Basic block has no parent function".to_string(),
            })?;

        let entry = parent
            .get_first_basic_block()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "get_first_basic_block".to_string(),
                details: "Function has no entry block".to_string(),
            })?;

        match entry.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry),
        }

        builder.build_alloca(ty, name).map_err(|_| CodegenError::LLVMError {
            operation: "build_alloca".to_string(),
            details: format!("Failed to allocate variable '{}'", name),
        })
    }

    fn get_printf(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function("printf") {
            return fn_val;
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = i32_type.fn_type(&[ptr_type.into()], true);
        self.module
            .add_function("printf", fn_type, Some(Linkage::External))
    }

    fn get_scanf(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function("scanf") {
            return fn_val;
        }
        let i32_type = self.context.i32_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let fn_type = i32_type.fn_type(&[ptr_type.into()], true);
        self.module
            .add_function("scanf", fn_type, Some(Linkage::External))
    }

    fn get_sprintf(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("sprintf") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i32_type = self.context.i32_type();

        // int sprintf(char *str, const char *format, ...)
        let fn_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], true); // variadic

        self.module
            .add_function("sprintf", fn_type, Some(Linkage::External))
    }

    fn get_atoi(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("atoi") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i32_type = self.context.i32_type();

        // int atoi(const char *str)
        let fn_type = i32_type.fn_type(&[ptr_type.into()], false);

        self.module
            .add_function("atoi", fn_type, Some(Linkage::External))
    }

    fn get_atof(&self) -> inkwell::values::FunctionValue<'ctx> {
        if let Some(func) = self.module.get_function("atof") {
            return func;
        }

        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let f64_type = self.context.f64_type();

        // double atof(const char *str)
        let fn_type = f64_type.fn_type(&[ptr_type.into()], false);

        self.module
            .add_function("atof", fn_type, Some(Linkage::External))
    }
}
