// Expression compilation
//
// This module contains logic for compiling Brix expressions.

use crate::{BrixType, Compiler};
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use parser::ast::{Expr, Literal};

/// Trait for expression compilation helper methods
pub trait ExpressionCompiler<'ctx> {
    /// Compile literal expression (Int, Float, String, Bool, Complex, Nil, Atom)
    fn compile_literal_expr(&self, lit: &Literal) -> Option<(BasicValueEnum<'ctx>, BrixType)>;

    /// Compile range expression (error, only valid in for loops)
    fn compile_range_expr(&self) -> Option<(BasicValueEnum<'ctx>, BrixType)>;

    /// Compile ternary expression (condition ? then : else)
    fn compile_ternary_expr(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Option<(BasicValueEnum<'ctx>, BrixType)>;

    /// Compile static initialization (int[5], float[2,3])
    fn compile_static_init_expr(
        &mut self,
        element_type: &str,
        dimensions: &[Expr],
    ) -> Option<(BasicValueEnum<'ctx>, BrixType)>;
}

impl<'a, 'ctx> ExpressionCompiler<'ctx> for Compiler<'a, 'ctx> {
    fn compile_literal_expr(&self, lit: &Literal) -> Option<(BasicValueEnum<'ctx>, BrixType)> {
        match lit {
            Literal::Int(n) => {
                let val = self.context.i64_type().const_int(*n as u64, false);
                Some((val.into(), BrixType::Int))
            }
            Literal::Float(n) => {
                let val = self.context.f64_type().const_float(*n);
                Some((val.into(), BrixType::Float))
            }
            Literal::String(s) => {
                let raw_str = self.builder.build_global_string_ptr(s, "raw_str").unwrap();

                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                    self.module
                        .add_function("str_new", fn_type, Some(Linkage::External))
                });

                let call = self
                    .builder
                    .build_call(str_new_fn, &[raw_str.as_pointer_value().into()], "new_str")
                    .unwrap();

                Some((call.try_as_basic_value().left().unwrap(), BrixType::String))
            }
            Literal::Bool(b) => {
                let bool_val = self.context.bool_type().const_int(*b as u64, false);
                let int_val = self
                    .builder
                    .build_int_z_extend(bool_val, self.context.i64_type(), "bool_ext")
                    .unwrap();
                Some((int_val.into(), BrixType::Int))
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

                Some((complex_val.into(), BrixType::Complex))
            }
            Literal::Nil => {
                // Nil is represented as a null pointer
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let null_ptr = ptr_type.const_null();
                Some((null_ptr.into(), BrixType::Nil))
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
                    .unwrap();

                // Call atom_intern(name)
                let atom_id = self
                    .builder
                    .build_call(
                        atom_intern_fn,
                        &[name_cstr.as_pointer_value().into()],
                        "atom_id",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                Some((atom_id.into(), BrixType::Atom))
            }
        }
    }

    fn compile_range_expr(&self) -> Option<(BasicValueEnum<'ctx>, BrixType)> {
        eprintln!(
            "Error: Ranges cannot be assigned to variables, use only inside 'for' loops."
        );
        None
    }

    fn compile_ternary_expr(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Option<(BasicValueEnum<'ctx>, BrixType)> {
        // Compile condition
        let (cond_val, _) = self.compile_expr(condition)?;
        let cond_int = cond_val.into_int_value();

        // Convert to boolean
        let i64_type = self.context.i64_type();
        let zero = i64_type.const_int(0, false);
        let cond_bool = self
            .builder
            .build_int_compare(IntPredicate::NE, cond_int, zero, "terncond")
            .unwrap();

        // Get parent function
        let parent_fn = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

        // Create basic blocks
        let then_bb = self.context.append_basic_block(parent_fn, "tern_then");
        let else_bb = self.context.append_basic_block(parent_fn, "tern_else");
        let merge_bb = self.context.append_basic_block(parent_fn, "tern_merge");

        // Conditional branch
        self.builder
            .build_conditional_branch(cond_bool, then_bb, else_bb)
            .unwrap();

        // Compile then branch
        self.builder.position_at_end(then_bb);
        let (then_val, then_type) = self.compile_expr(then_expr)?;
        self.builder.build_unconditional_branch(merge_bb).unwrap();
        let then_end_bb = self.builder.get_insert_block().unwrap();

        // Compile else branch
        self.builder.position_at_end(else_bb);
        let (else_val, else_type) = self.compile_expr(else_expr)?;
        self.builder.build_unconditional_branch(merge_bb).unwrap();
        let else_end_bb = self.builder.get_insert_block().unwrap();

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
                .unwrap()
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
                .unwrap()
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

        let phi = self.builder.build_phi(phi_type, "tern_result").unwrap();

        phi.add_incoming(&[
            (&final_then_val, then_end_bb),
            (&final_else_val, else_end_bb),
        ]);

        Some((phi.as_basic_value(), result_type))
    }

    fn compile_static_init_expr(
        &mut self,
        element_type: &str,
        dimensions: &[Expr],
    ) -> Option<(BasicValueEnum<'ctx>, BrixType)> {
        // Static initialization: int[5], float[2,3]
        // This is syntactic sugar for zeros() and izeros()
        if element_type == "int" {
            let val = self.compile_izeros(dimensions)?;
            Some((val, BrixType::IntMatrix))
        } else if element_type == "float" {
            let val = self.compile_zeros(dimensions)?;
            Some((val, BrixType::Matrix))
        } else {
            eprintln!("Error: StaticInit only supports 'int' and 'float' types.");
            None
        }
    }
}
