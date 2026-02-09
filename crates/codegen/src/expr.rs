// Expression compilation
//
// This module contains logic for compiling Brix expressions.
//
// REFACTORING NOTE (v1.2):
// - Extracted from lib.rs (originally ~263 lines)
// - Uses trait pattern (ExpressionCompiler) for organization
// - Handles self-contained, low-dependency expressions
//
// Refactored expressions:
// - Literal - All literal types (Int, Float, String, Bool, Complex, Nil, Atom)
// - Ternary - Conditional operator with PHI nodes
// - StaticInit - Syntactic sugar for zeros()/izeros()
// - Range - Error handler (only valid in for loops)
//
// Still in lib.rs:
// - Binary/Unary operators (postponed to Phase 5, see operators.rs)
// - Call, Identifier, FieldAccess, Index (highly coupled with symbol table)
// - Array, Match, Increment/Decrement, FString (complex logic)
// - ListComprehension (has dedicated method)

use crate::{BrixType, Compiler, CodegenError, CodegenResult};
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use parser::ast::{Expr, Literal};

/// Trait for expression compilation helper methods
pub trait ExpressionCompiler<'ctx> {
    /// Compile literal expression (Int, Float, String, Bool, Complex, Nil, Atom)
    fn compile_literal_expr(&self, lit: &Literal) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)>;

    /// Compile range expression (error, only valid in for loops)
    fn compile_range_expr(&self) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)>;

    /// Compile ternary expression (condition ? then : else)
    fn compile_ternary_expr(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)>;

    /// Compile static initialization (int[5], float[2,3])
    fn compile_static_init_expr(
        &mut self,
        element_type: &str,
        dimensions: &[Expr],
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)>;
}

impl<'a, 'ctx> ExpressionCompiler<'ctx> for Compiler<'a, 'ctx> {
    fn compile_literal_expr(&self, lit: &Literal) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        match lit {
            Literal::Int(n) => {
                let val = self.context.i64_type().const_int(*n as u64, false);
                Ok((val.into(), BrixType::Int))
            }
            Literal::Float(n) => {
                let val = self.context.f64_type().const_float(*n);
                Ok((val.into(), BrixType::Float))
            }
            Literal::String(s) => {
                let raw_str = self.builder.build_global_string_ptr(s, "raw_str")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_global_string_ptr".to_string(),
                        details: format!("Failed to create global string for '{}'", s),
                                            span: None,
                    })?;

                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                });

                let call = self
                    .builder
                    .build_call(str_new_fn, &[raw_str.as_pointer_value().into()], "new_str")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call str_new".to_string(),
                                            span: None,
                    })?;

                let value = call.try_as_basic_value().left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "str_new call did not return a value".to_string(),
                                            span: None,
                    })?;

                Ok((value, BrixType::String))
            }
            Literal::Bool(b) => {
                let bool_val = self.context.bool_type().const_int(*b as u64, false);
                let int_val = self
                    .builder
                    .build_int_z_extend(bool_val, self.context.i64_type(), "bool_ext")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_z_extend".to_string(),
                        details: "Failed to extend boolean to i64".to_string(),
                                            span: None,
                    })?;
                Ok((int_val.into(), BrixType::Int))
            }
            Literal::Complex(real, imag) => {
                // Create complex number as struct { f64, f64 }
                let f64_type = self.context.f64_type();
                let real_val = f64_type.const_float(*real);
                let imag_val = f64_type.const_float(*imag);

                let complex_type = self
                    .context
                    .struct_type(&[f64_type.into(), f64_type.into()], false);
                let complex_val =
                    complex_type.const_named_struct(&[real_val.into(), imag_val.into()]);

                Ok((complex_val.into(), BrixType::Complex))
            }
            Literal::Nil => {
                // Nil is represented as a null pointer
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let null_ptr = ptr_type.const_null();
                Ok((null_ptr.into(), BrixType::Nil))
            }
            Literal::Atom(name) => {
                // Atom: call atom_intern() to get unique ID
                // Declare atom_intern(const char*) -> i64
                let i64_type = self.context.i64_type();
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
                let atom_intern_fn =
                    self.module.get_function("atom_intern").unwrap_or_else(|| {
                        self.module.add_function(
                            "atom_intern",
                            fn_type,
                            Some(Linkage::External),
                        )
                    });

                // Create string literal for atom name
                let name_cstr = self
                    .builder
                    .build_global_string_ptr(name, "atom_name_str")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_global_string_ptr".to_string(),
                        details: format!("Failed to create global string for atom '{}'", name),
                                            span: None,
                    })?;

                // Call atom_intern(name)
                let call_site = self
                    .builder
                    .build_call(
                        atom_intern_fn,
                        &[name_cstr.as_pointer_value().into()],
                        "atom_id",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: format!("Failed to call atom_intern for '{}'", name),
                                            span: None,
                    })?;

                let atom_val = call_site.try_as_basic_value().left()
                    .ok_or_else(|| CodegenError::LLVMError {
                        operation: "try_as_basic_value".to_string(),
                        details: "atom_intern did not return a value".to_string(),
                                            span: None,
                    })?;

                Ok((atom_val, BrixType::Atom))
            }
        }
    }

    fn compile_range_expr(&self) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        Err(CodegenError::InvalidOperation {
            operation: "Range".to_string(),
            reason: "Ranges cannot be assigned to variables, use only inside 'for' loops".to_string(),
                    span: None,
        })
    }

    fn compile_ternary_expr(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        // Compile condition
        let (cond_val, _) = self.compile_expr(condition)?;
        let cond_int = cond_val.into_int_value();

        // Convert to boolean
        let i64_type = self.context.i64_type();
        let zero = i64_type.const_int(0, false);
        let cond_bool = self
            .builder
            .build_int_compare(IntPredicate::NE, cond_int, zero, "terncond")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_int_compare".to_string(),
                details: "Failed to compare ternary condition with zero".to_string(),
                            span: None,
            })?;

        // Get parent function
        let block = self.builder.get_insert_block()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "get_insert_block".to_string(),
                details: "No current basic block for ternary expression".to_string(),
                            span: None,
            })?;

        let parent_fn = block.get_parent()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "get_parent".to_string(),
                details: "Basic block has no parent function".to_string(),
                            span: None,
            })?;

        // Create basic blocks
        let then_bb = self.context.append_basic_block(parent_fn, "tern_then");
        let else_bb = self.context.append_basic_block(parent_fn, "tern_else");
        let merge_bb = self.context.append_basic_block(parent_fn, "tern_merge");

        // Conditional branch
        self.builder
            .build_conditional_branch(cond_bool, then_bb, else_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_conditional_branch".to_string(),
                details: "Failed to build conditional branch for ternary".to_string(),
                            span: None,
            })?;

        // Compile then branch
        self.builder.position_at_end(then_bb);
        let (then_val, then_type) = self.compile_expr(then_expr)?;
        self.builder.build_unconditional_branch(merge_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_unconditional_branch".to_string(),
                details: "Failed to build branch from then block".to_string(),
                            span: None,
            })?;
        let then_end_bb = self.builder.get_insert_block()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "get_insert_block".to_string(),
                details: "No insert block after then expression".to_string(),
                            span: None,
            })?;

        // Compile else branch
        self.builder.position_at_end(else_bb);
        let (else_val, else_type) = self.compile_expr(else_expr)?;
        self.builder.build_unconditional_branch(merge_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_unconditional_branch".to_string(),
                details: "Failed to build branch from else block".to_string(),
                            span: None,
            })?;
        let else_end_bb = self.builder.get_insert_block()
            .ok_or_else(|| CodegenError::LLVMError {
                operation: "get_insert_block".to_string(),
                details: "No insert block after else expression".to_string(),
                            span: None,
            })?;

        // Merge with PHI node
        self.builder.position_at_end(merge_bb);

        // Determine result type (promote int to float if needed)
        let result_type = if then_type == BrixType::Float || else_type == BrixType::Float {
            BrixType::Float
        } else if then_type == BrixType::String || else_type == BrixType::String {
            BrixType::String
        } else {
            then_type.clone()
        };

        // Cast values to same type if needed
        let final_then_val = if then_type == BrixType::Int && result_type == BrixType::Float
        {
            self.builder
                .build_signed_int_to_float(
                    then_val.into_int_value(),
                    self.context.f64_type(),
                    "then_cast",
                )
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_signed_int_to_float".to_string(),
                    details: "Failed to cast then value to float".to_string(),
                                    span: None,
                })?
                .into()
        } else {
            then_val
        };

        let final_else_val = if else_type == BrixType::Int && result_type == BrixType::Float
        {
            self.builder
                .build_signed_int_to_float(
                    else_val.into_int_value(),
                    self.context.f64_type(),
                    "else_cast",
                )
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_signed_int_to_float".to_string(),
                    details: "Failed to cast else value to float".to_string(),
                                    span: None,
                })?
                .into()
        } else {
            else_val
        };

        // Create PHI node
        let phi_type: BasicTypeEnum = match result_type {
            BrixType::Int => self.context.i64_type().into(),
            BrixType::Float => self.context.f64_type().into(),
            BrixType::String | BrixType::Matrix | BrixType::FloatPtr => self
                .context
                .ptr_type(AddressSpace::default())
                .into(),
            _ => self.context.i64_type().into(),
        };

        let phi = self.builder.build_phi(phi_type, "tern_result")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_phi".to_string(),
                details: "Failed to build PHI node for ternary result".to_string(),
                            span: None,
            })?;

        phi.add_incoming(&[
            (&final_then_val, then_end_bb),
            (&final_else_val, else_end_bb),
        ]);

        Ok((phi.as_basic_value(), result_type))
    }

    fn compile_static_init_expr(
        &mut self,
        element_type: &str,
        dimensions: &[Expr],
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        // Static initialization: int[5], float[2,3]
        // This is syntactic sugar for zeros() and izeros()
        if element_type == "int" {
            let val = self.compile_izeros(dimensions)?;
            Ok((val, BrixType::IntMatrix))
        } else if element_type == "float" {
            let val = self.compile_zeros(dimensions)?;
            Ok((val, BrixType::Matrix))
        } else {
            Err(CodegenError::TypeError {
                expected: "int or float".to_string(),
                found: element_type.to_string(),
                context: "StaticInit".to_string(),
                            span: None,
            })
        }
    }
}
