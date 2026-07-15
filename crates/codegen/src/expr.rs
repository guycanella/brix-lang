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
// - ListComprehension - compile_list_comprehension + generate_comp_loop
//   (moved from lib.rs, refactor Extraction 1)
//
// Still in lib.rs:
// - Binary/Unary operators (postponed to Phase 5, see operators.rs)
// - Call, Identifier, FieldAccess, Index (highly coupled with symbol table)
// - Array, Match, Increment/Decrement, FString (complex logic)

use crate::{BrixType, Compiler, CodegenError, CodegenResult};
use crate::helpers::HelperFunctions;
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, PointerValue};
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

// --- List comprehension (moved from lib.rs, refactor Extraction 1) ---
impl<'a, 'ctx> Compiler<'a, 'ctx> {
    pub(crate) fn compile_list_comprehension(
        &mut self,
        expr: &Expr,
        generators: &[parser::ast::ComprehensionGen],
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        // For now, we'll compile this as a for loop with pre-allocation
        // [expr for x in arr if cond] becomes:
        // temp = zeros(max_size)
        // count = 0
        // for x in arr:
        //     if cond:
        //         temp[count] = expr
        //         count++
        // result = type[count]
        // copy temp to result

        if generators.is_empty() {
            eprintln!("Error: List comprehension must have at least one generator");
            return Err(CodegenError::InvalidOperation {
                operation: "list comprehension".to_string(),
                reason: "must have at least one generator".to_string(),
                            span: None,
            });
        }

        let i64_type = self.context.i64_type();
        let f64_type = self.context.f64_type();

        // Step 1: Determine result type via static inference (v1.7 Grupo I).
        // For each generator, infer its iterable's element type (Int for IntMatrix,
        // Float for Matrix) and bind every one of its var_names to that type — a
        // Matrix/IntMatrix row is always homogeneously typed, so this is correct
        // even for destructuring generators (`for (a, b) in matrix2d`), which
        // generate_comp_loop() below already binds per-element the same way
        // (BrixType::Float for a Matrix iterable, BrixType::Int for IntMatrix).
        // Anything that doesn't statically resolve to IntMatrix/Matrix (e.g. a
        // StringMatrix from .split(), or an iterable we can't infer at all) falls
        // back to "float" here — harmless, since Step 2 below already rejects any
        // non-Matrix/IntMatrix iterable with its own clear error before this
        // (possibly wrong) result_elem_type is ever used for anything observable.
        let mut generator_params: Vec<(String, String)> = Vec::new();
        for generator in generators.iter() {
            let elem_type_str = match self.infer_expr_type_static(&generator.iterable, &[]) {
                Some(BrixType::IntMatrix) => "int",
                _ => "float",
            };
            for var_name in &generator.var_names {
                generator_params.push((var_name.clone(), elem_type_str.to_string()));
            }
        }
        let result_elem_type = self
            .infer_expr_type_static(expr, &generator_params)
            .unwrap_or(BrixType::Float);

        // Step 2: Calculate max size (product of all iterable lengths)
        let mut total_size = i64_type.const_int(1, false);

        for generator in generators.iter() {
            let (iterable_val, iterable_type) = self.compile_expr(&generator.iterable)?;

            let len = match iterable_type {
                BrixType::Matrix | BrixType::IntMatrix => {
                    // Get rows * cols for total element count
                    let matrix_ptr = iterable_val.into_pointer_value();

                    // Load rows (field 0)
                    let rows_ptr = self
                        .builder
                        .build_struct_gep(
                            if iterable_type == BrixType::Matrix {
                                self.get_matrix_type()
                            } else {
                                self.get_intmatrix_type()
                            },
                            matrix_ptr,
                            1,
                            "rows_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get rows_ptr in list comprehension".to_string(), span: None })?;
                    let rows = self
                        .builder
                        .build_load(i64_type, rows_ptr, "rows")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load rows in list comprehension".to_string(), span: None })?
                        .into_int_value();

                    // Load cols (field 2)
                    let cols_ptr = self
                        .builder
                        .build_struct_gep(
                            if iterable_type == BrixType::Matrix {
                                self.get_matrix_type()
                            } else {
                                self.get_intmatrix_type()
                            },
                            matrix_ptr,
                            2,
                            "cols_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get cols_ptr in list comprehension".to_string(), span: None })?;
                    let cols = self
                        .builder
                        .build_load(i64_type, cols_ptr, "cols")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cols in list comprehension".to_string(), span: None })?
                        .into_int_value();

                    self.builder.build_int_mul(rows, cols, "len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute len in list comprehension".to_string(), span: None })?
                }
                _ => {
                    eprintln!(
                        "Error: List comprehension only supports Matrix/IntMatrix iterables for now"
                    );
                    return Err(CodegenError::InvalidOperation {
                        operation: "list comprehension".to_string(),
                        reason: "only supports Matrix/IntMatrix iterables for now".to_string(),
                                            span: None,
                    });
                }
            };

            total_size = self
                .builder
                .build_int_mul(total_size, len, "total_size")
                .map_err(|_| CodegenError::LLVMError {
                    operation: "build_int_mul".to_string(),
                    details: "Failed to compute total size for list comprehension".to_string(),
                                    span: None,
                })?;
        }

        // Step 3: Allocate temporary array with max size
        let (temp_array, temp_type) = match result_elem_type {
            BrixType::Int => {
                // Allocate IntMatrix
                let fn_name = "intmatrix_new";
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let new_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
                    self.module
                        .add_function(fn_name, fn_type, Some(Linkage::External))
                });

                let one = i64_type.const_int(1, false);
                let result = self
                    .builder
                    .build_call(new_fn, &[one.into(), total_size.into()], "temp_array")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call intmatrix_new for temp array".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "intmatrix_new return value".to_string(), context: "list comprehension temp array".to_string(), span: None })?;
                (result, BrixType::IntMatrix)
            }
            BrixType::Float => {
                // Allocate Matrix
                let fn_name = "matrix_new";
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let new_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
                    self.module
                        .add_function(fn_name, fn_type, Some(Linkage::External))
                });

                let one = i64_type.const_int(1, false);
                let result = self
                    .builder
                    .build_call(new_fn, &[one.into(), total_size.into()], "temp_array")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call matrix_new for temp array".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "matrix_new return value".to_string(), context: "list comprehension temp array".to_string(), span: None })?;
                (result, BrixType::Matrix)
            }
            _ => {
                eprintln!("Error: List comprehension result type must be Int or Float for now");
                return Err(CodegenError::InvalidOperation {
                    operation: "list comprehension".to_string(),
                    reason: "result type must be Int or Float for now".to_string(),
                                    span: None,
                });
            }
        };

        // Step 4: Create counter variable
        let count_alloca = self.create_entry_block_alloca(i64_type.into(), "comp_count")?;
        self.builder
            .build_store(count_alloca, i64_type.const_int(0, false))
            .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to initialize comp_count".to_string(), span: None })?;

        // Step 5: Generate nested loops recursively
        self.generate_comp_loop(
            expr,
            generators,
            0,
            &temp_array,
            temp_type.clone(),
            count_alloca,
        )?;

        // Step 6: Load final count
        let final_count = self
            .builder
            .build_load(i64_type, count_alloca, "final_count")
            .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load final_count".to_string(), span: None })?
            .into_int_value();

        // Step 7: Create result array with actual size
        let (result_array, result_type) = match temp_type {
            BrixType::IntMatrix => {
                let fn_name = "intmatrix_new";
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let new_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
                    self.module
                        .add_function(fn_name, fn_type, Some(Linkage::External))
                });

                let one = i64_type.const_int(1, false);
                let result = self
                    .builder
                    .build_call(new_fn, &[one.into(), final_count.into()], "result_array")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call intmatrix_new for result array".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "intmatrix_new return value".to_string(), context: "list comprehension result array".to_string(), span: None })?;
                (result, BrixType::IntMatrix)
            }
            BrixType::Matrix => {
                let fn_name = "matrix_new";
                let ptr_type = self.context.ptr_type(AddressSpace::default());
                let fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let new_fn = self.module.get_function(fn_name).unwrap_or_else(|| {
                    self.module
                        .add_function(fn_name, fn_type, Some(Linkage::External))
                });

                let one = i64_type.const_int(1, false);
                let result = self
                    .builder
                    .build_call(new_fn, &[one.into(), final_count.into()], "result_array")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_call".to_string(), details: "failed to call matrix_new for result array".to_string(), span: None })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue { what: "matrix_new return value".to_string(), context: "list comprehension result array".to_string(), span: None })?;
                (result, BrixType::Matrix)
            }
            _ => unreachable!(),
        };

        // Step 8: Copy elements from temp to result
        let parent_fn = self
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::LLVMError { operation: "get_insert_block".to_string(), details: "no current block in list comprehension copy".to_string(), span: None })?
            .get_parent()
            .ok_or_else(|| CodegenError::LLVMError { operation: "get_parent".to_string(), details: "block has no parent function in list comprehension copy".to_string(), span: None })?;
        let copy_cond_bb = self.context.append_basic_block(parent_fn, "copy_cond");
        let copy_body_bb = self.context.append_basic_block(parent_fn, "copy_body");
        let copy_after_bb = self.context.append_basic_block(parent_fn, "copy_after");

        // Initialize copy index
        let copy_idx_alloca = self.create_entry_block_alloca(i64_type.into(), "copy_idx")?;
        self.builder
            .build_store(copy_idx_alloca, i64_type.const_int(0, false))
            .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to initialize copy_idx".to_string(), span: None })?;
        self.builder
            .build_unconditional_branch(copy_cond_bb)
            .map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to copy_cond".to_string(), span: None })?;

        // Copy condition: idx < final_count
        self.builder.position_at_end(copy_cond_bb);
        let copy_idx = self
            .builder
            .build_load(i64_type, copy_idx_alloca, "copy_idx")
            .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load copy_idx".to_string(), span: None })?
            .into_int_value();
        let copy_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, copy_idx, final_count, "copy_cond")
            .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare copy_idx < final_count".to_string(), span: None })?;
        self.builder
            .build_conditional_branch(copy_cond, copy_body_bb, copy_after_bb)
            .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch in copy loop".to_string(), span: None })?;

        // Copy body: result[idx] = temp[idx]
        self.builder.position_at_end(copy_body_bb);

        unsafe {
            let temp_matrix_ptr = temp_array.into_pointer_value();
            let result_matrix_ptr = result_array.into_pointer_value();

            if temp_type == BrixType::Matrix {
                let matrix_type = self.get_matrix_type();

                // Get temp data pointer
                let temp_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, temp_matrix_ptr, 3, "temp_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get temp data_ptr_ptr in copy loop".to_string(), span: None })?;
                let temp_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        temp_data_ptr_ptr,
                        "temp_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load temp data_ptr in copy loop".to_string(), span: None })?
                    .into_pointer_value();

                // Get result data pointer
                let result_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, result_matrix_ptr, 3, "result_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get result data_ptr_ptr in copy loop".to_string(), span: None })?;
                let result_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        result_data_ptr_ptr,
                        "result_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load result data_ptr in copy loop".to_string(), span: None })?
                    .into_pointer_value();

                // Load temp[idx]
                let temp_elem_ptr = self
                    .builder
                    .build_gep(f64_type, temp_data_ptr, &[copy_idx], "temp_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get temp elem_ptr in copy loop".to_string(), span: None })?;
                let temp_elem = self
                    .builder
                    .build_load(f64_type, temp_elem_ptr, "temp_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load temp elem in copy loop".to_string(), span: None })?;

                // Store to result[idx]
                let result_elem_ptr = self
                    .builder
                    .build_gep(f64_type, result_data_ptr, &[copy_idx], "result_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get result elem_ptr in copy loop".to_string(), span: None })?;
                self.builder
                    .build_store(result_elem_ptr, temp_elem)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store temp elem to result in copy loop".to_string(), span: None })?;
            } else {
                let matrix_type = self.get_intmatrix_type();

                // Get temp data pointer
                let temp_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, temp_matrix_ptr, 3, "temp_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get int temp data_ptr_ptr in copy loop".to_string(), span: None })?;
                let temp_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        temp_data_ptr_ptr,
                        "temp_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int temp data_ptr in copy loop".to_string(), span: None })?
                    .into_pointer_value();

                // Get result data pointer
                let result_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, result_matrix_ptr, 3, "result_data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get int result data_ptr_ptr in copy loop".to_string(), span: None })?;
                let result_data_ptr = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        result_data_ptr_ptr,
                        "result_data_ptr",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int result data_ptr in copy loop".to_string(), span: None })?
                    .into_pointer_value();

                // Load temp[idx]
                let temp_elem_ptr = self
                    .builder
                    .build_gep(i64_type, temp_data_ptr, &[copy_idx], "temp_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get int temp elem_ptr in copy loop".to_string(), span: None })?;
                let temp_elem = self
                    .builder
                    .build_load(i64_type, temp_elem_ptr, "temp_elem")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int temp elem in copy loop".to_string(), span: None })?;

                // Store to result[idx]
                let result_elem_ptr = self
                    .builder
                    .build_gep(i64_type, result_data_ptr, &[copy_idx], "result_elem_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get int result elem_ptr in copy loop".to_string(), span: None })?;
                self.builder
                    .build_store(result_elem_ptr, temp_elem)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store int temp elem to result in copy loop".to_string(), span: None })?;
            }
        }

        // Increment copy_idx
        let next_copy_idx = self
            .builder
            .build_int_add(copy_idx, i64_type.const_int(1, false), "next_copy_idx")
            .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment copy_idx".to_string(), span: None })?;
        self.builder
            .build_store(copy_idx_alloca, next_copy_idx)
            .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next copy_idx".to_string(), span: None })?;
        self.builder
            .build_unconditional_branch(copy_cond_bb)
            .map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch back to copy_cond".to_string(), span: None })?;

        // After copy loop
        self.builder.position_at_end(copy_after_bb);

        Ok((result_array, result_type))
    }

    fn generate_comp_loop(
        &mut self,
        expr: &Expr,
        generators: &[parser::ast::ComprehensionGen],
        gen_idx: usize,
        temp_array: &BasicValueEnum<'ctx>,
        temp_type: BrixType,
        count_alloca: PointerValue<'ctx>,
    ) -> CodegenResult<()> {
        if gen_idx >= generators.len() {
            // Base case: we're inside the innermost loop
            // Evaluate expr and add to temp_array[count++]

            let (expr_val, expr_type) = self.compile_expr(expr)?;

            let i64_type = self.context.i64_type();
            let f64_type = self.context.f64_type();

            // Load current count
            let count = self
                .builder
                .build_load(i64_type, count_alloca, "count")
                .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load count in comp loop".to_string(), span: None })?
                .into_int_value();

            // Get data pointer from temp_array
            let temp_matrix_ptr = temp_array.into_pointer_value();

            unsafe {
                if temp_type == BrixType::Matrix {
                    let matrix_type = self.get_matrix_type();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, temp_matrix_ptr, 3, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get data_ptr_ptr in comp loop base case".to_string(), span: None })?;
                    let data_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "data_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load data_ptr in comp loop base case".to_string(), span: None })?
                        .into_pointer_value();

                    // Convert expr_val to correct type if needed
                    let val_to_store = if expr_type == BrixType::Float {
                        expr_val
                    } else if expr_type == BrixType::Int {
                        // int -> float
                        let int_val = expr_val.into_int_value();
                        self.builder
                            .build_signed_int_to_float(int_val, f64_type, "int_to_float")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_signed_int_to_float".to_string(), details: "failed to convert int to float in comp loop".to_string(), span: None })?
                            .into()
                    } else {
                        eprintln!("Error: Type mismatch in list comprehension");
                        return Err(CodegenError::TypeError { expected: "Float or Int".to_string(), found: format!("{:?}", expr_type), context: "list comprehension expression".to_string(), span: None });
                    };

                    // Store at temp_array[count]
                    let elem_ptr = self
                        .builder
                        .build_gep(f64_type, data_ptr, &[count], "elem_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop base case".to_string(), span: None })?;
                    self.builder.build_store(elem_ptr, val_to_store).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store value in comp loop base case".to_string(), span: None })?;
                } else {
                    // IntMatrix
                    let matrix_type = self.get_intmatrix_type();

                    let data_ptr_ptr = self
                        .builder
                        .build_struct_gep(matrix_type, temp_matrix_ptr, 3, "data_ptr_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get int data_ptr_ptr in comp loop base case".to_string(), span: None })?;
                    let data_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_ptr,
                            "data_ptr",
                        )
                        .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load int data_ptr in comp loop base case".to_string(), span: None })?
                        .into_pointer_value();

                    // Ensure type is Int
                    if expr_type != BrixType::Int {
                        eprintln!(
                            "Error: Type mismatch in list comprehension (expected Int for IntMatrix)"
                        );
                        return Err(CodegenError::TypeError { expected: "Int".to_string(), found: format!("{:?}", expr_type), context: "list comprehension IntMatrix expression".to_string(), span: None });
                    }

                    // Store at temp_array[count]
                    let elem_ptr = self
                        .builder
                        .build_gep(i64_type, data_ptr, &[count], "elem_ptr")
                        .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get int elem_ptr in comp loop base case".to_string(), span: None })?;
                    self.builder.build_store(elem_ptr, expr_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store int value in comp loop base case".to_string(), span: None })?;
                }
            }

            // Increment count
            let next_count = self
                .builder
                .build_int_add(count, i64_type.const_int(1, false), "next_count")
                .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment count in comp loop".to_string(), span: None })?;
            self.builder.build_store(count_alloca, next_count).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next_count in comp loop".to_string(), span: None })?;

            return Ok(());
        }

        // Recursive case: generate this loop level
        let generator = &generators[gen_idx];

        // Compile iterable
        let (iterable_val, iterable_type) = self.compile_expr(&generator.iterable)?;

        match iterable_type {
            BrixType::Matrix => {
                let i64_type = self.context.i64_type();
                let f64_type = self.context.f64_type();

                let matrix_ptr = iterable_val.into_pointer_value();
                let matrix_type = self.get_matrix_type();

                // Load dimensions
                let rows_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "rows_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get rows_ptr in comp loop Matrix".to_string(), span: None })?;
                let rows = self
                    .builder
                    .build_load(i64_type, rows_ptr, "rows")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load rows in comp loop Matrix".to_string(), span: None })?
                    .into_int_value();

                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "cols_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get cols_ptr in comp loop Matrix".to_string(), span: None })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cols in comp loop Matrix".to_string(), span: None })?
                    .into_int_value();

                // Load data pointer
                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 3, "data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get data_ptr_ptr in comp loop Matrix".to_string(), span: None })?;
                let data_base = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data_base",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load data_base in comp loop Matrix".to_string(), span: None })?
                    .into_pointer_value();

                // Determine if destructuring
                let (total_len, is_destructuring) = if generator.var_names.len() > 1 {
                    (rows, true)
                } else {
                    (
                        self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute total_len in comp loop Matrix".to_string(), span: None })?,
                        false,
                    )
                };

                // Create loop blocks
                let parent_fn = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_insert_block".to_string(), details: "no current block in comp loop Matrix".to_string(), span: None })?
                    .get_parent()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_parent".to_string(), details: "block has no parent in comp loop Matrix".to_string(), span: None })?;
                let cond_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_cond_{}", gen_idx));
                let body_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_body_{}", gen_idx));
                let check_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_check_{}", gen_idx));
                let incr_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_incr_{}", gen_idx));
                let after_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_after_{}", gen_idx));

                // Allocate loop index
                let idx_alloca = self
                    .create_entry_block_alloca(i64_type.into(), &format!("comp_idx_{}", gen_idx))?;
                self.builder
                    .build_store(idx_alloca, i64_type.const_int(0, false))
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to init loop idx in comp loop Matrix".to_string(), span: None })?;

                // Allocate variables and save old ones
                let mut var_allocas = Vec::new();
                let mut old_vars = Vec::new();

                if is_destructuring {
                    for var_name in generator.var_names.iter() {
                        let var_alloca = self.create_entry_block_alloca(f64_type.into(), var_name)?;
                        let old_var = self.variables.remove(var_name);
                        self.variables
                            .insert(var_name.clone(), (var_alloca, BrixType::Float));
                        old_vars.push((var_name.clone(), old_var));
                        var_allocas.push(var_alloca);
                    }
                } else {
                    let var_name = &generator.var_names[0];
                    let var_alloca = self.create_entry_block_alloca(f64_type.into(), var_name)?;
                    let old_var = self.variables.remove(var_name);
                    self.variables
                        .insert(var_name.clone(), (var_alloca, BrixType::Float));
                    old_vars.push((var_name.clone(), old_var));
                }

                // Jump to condition
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to cond in comp loop Matrix".to_string(), span: None })?;

                // Condition: idx < total_len
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "cur_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cur_idx in comp loop Matrix".to_string(), span: None })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total_len, "loop_cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare idx < total_len in comp loop Matrix".to_string(), span: None })?;
                self.builder
                    .build_conditional_branch(cond, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch in comp loop Matrix".to_string(), span: None })?;

                // Body: load variables
                self.builder.position_at_end(body_bb);

                if is_destructuring {
                    // Load row elements
                    for (j, var_alloca) in var_allocas.iter().enumerate() {
                        unsafe {
                            let offset = self
                                .builder
                                .build_int_mul(cur_idx, cols, "row_offset")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute row_offset in comp loop Matrix".to_string(), span: None })?;
                            let col_offset = self
                                .builder
                                .build_int_add(
                                    offset,
                                    i64_type.const_int(j as u64, false),
                                    "elem_offset",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to compute elem_offset in comp loop Matrix".to_string(), span: None })?;

                            let elem_ptr = self
                                .builder
                                .build_gep(
                                    f64_type,
                                    data_base,
                                    &[col_offset],
                                    &format!("elem_{}_ptr", j),
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop Matrix destructuring".to_string(), span: None })?;
                            let elem_val = self
                                .builder
                                .build_load(f64_type, elem_ptr, &format!("elem_{}", j))
                                .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop Matrix destructuring".to_string(), span: None })?;
                            self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop Matrix destructuring".to_string(), span: None })?;
                        }
                    }
                } else {
                    // Load single element
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(f64_type, data_base, &[cur_idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop Matrix".to_string(), span: None })?;
                        let elem_val = self.builder.build_load(f64_type, elem_ptr, "elem").map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop Matrix".to_string(), span: None })?;
                        let current_var = self.variables.get(&generator.var_names[0]).ok_or_else(|| CodegenError::UndefinedSymbol { name: generator.var_names[0].clone(), context: "comp loop Matrix variable lookup".to_string(), span: None })?.0;
                        self.builder.build_store(current_var, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop Matrix".to_string(), span: None })?;
                    }
                }

                // Jump to check block (for conditions)
                self.builder.build_unconditional_branch(check_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to check in comp loop Matrix".to_string(), span: None })?;

                // Check block: evaluate all conditions
                self.builder.position_at_end(check_bb);

                if !generator.conditions.is_empty() {
                    let mut combined_cond = None;

                    for condition in &generator.conditions {
                        let (cond_val, _) = self.compile_expr(condition)?;
                        let cond_int = cond_val.into_int_value();
                        let cond_bool = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                cond_int,
                                i64_type.const_int(0, false),
                                "cond_bool",
                            )
                            .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare condition in comp loop Matrix".to_string(), span: None })?;

                        combined_cond = Some(if let Some(prev) = combined_cond {
                            self.builder
                                .build_and(prev, cond_bool, "combined_cond")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_and".to_string(), details: "failed to combine conditions in comp loop Matrix".to_string(), span: None })?
                        } else {
                            cond_bool
                        });
                    }

                    // If conditions pass, recurse to next generator or evaluate expr
                    let recurse_bb = self
                        .context
                        .append_basic_block(parent_fn, &format!("comp_recurse_{}", gen_idx));
                    let combined = combined_cond.ok_or_else(|| CodegenError::MissingValue { what: "combined_cond".to_string(), context: "comp loop Matrix conditions".to_string(), span: None })?;
                    self.builder
                        .build_conditional_branch(combined, recurse_bb, incr_bb)
                        .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch on condition in comp loop Matrix".to_string(), span: None })?;

                    self.builder.position_at_end(recurse_bb);
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr in comp loop Matrix".to_string(), span: None })?;
                } else {
                    // No conditions, just recurse
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr (no cond) in comp loop Matrix".to_string(), span: None })?;
                }

                // Increment block
                self.builder.position_at_end(incr_bb);
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment idx in comp loop Matrix".to_string(), span: None })?;
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next_idx in comp loop Matrix".to_string(), span: None })?;
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to loop back in comp loop Matrix".to_string(), span: None })?;

                // After block: restore variables
                self.builder.position_at_end(after_bb);

                for (var_name, old_var_opt) in old_vars {
                    if let Some(old) = old_var_opt {
                        self.variables.insert(var_name, old);
                    } else {
                        self.variables.remove(&var_name);
                    }
                }

                Ok(())
            }

            BrixType::IntMatrix => {
                let i64_type = self.context.i64_type();

                let matrix_ptr = iterable_val.into_pointer_value();
                let matrix_type = self.get_intmatrix_type();

                // Load dimensions
                let rows_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 1, "rows_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get rows_ptr in comp loop IntMatrix".to_string(), span: None })?;
                let rows = self
                    .builder
                    .build_load(i64_type, rows_ptr, "rows")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load rows in comp loop IntMatrix".to_string(), span: None })?
                    .into_int_value();

                let cols_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 2, "cols_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get cols_ptr in comp loop IntMatrix".to_string(), span: None })?;
                let cols = self
                    .builder
                    .build_load(i64_type, cols_ptr, "cols")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cols in comp loop IntMatrix".to_string(), span: None })?
                    .into_int_value();

                // Load data pointer
                let data_ptr_ptr = self
                    .builder
                    .build_struct_gep(matrix_type, matrix_ptr, 3, "data_ptr_ptr")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_struct_gep".to_string(), details: "failed to get data_ptr_ptr in comp loop IntMatrix".to_string(), span: None })?;
                let data_base = self
                    .builder
                    .build_load(
                        self.context.ptr_type(AddressSpace::default()),
                        data_ptr_ptr,
                        "data_base",
                    )
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load data_base in comp loop IntMatrix".to_string(), span: None })?
                    .into_pointer_value();

                // Determine if destructuring
                let (total_len, is_destructuring) = if generator.var_names.len() > 1 {
                    (rows, true)
                } else {
                    (
                        self.builder.build_int_mul(rows, cols, "total_len").map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute total_len in comp loop IntMatrix".to_string(), span: None })?,
                        false,
                    )
                };

                // Create loop blocks
                let parent_fn = self
                    .builder
                    .get_insert_block()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_insert_block".to_string(), details: "no current block in comp loop IntMatrix".to_string(), span: None })?
                    .get_parent()
                    .ok_or_else(|| CodegenError::LLVMError { operation: "get_parent".to_string(), details: "block has no parent in comp loop IntMatrix".to_string(), span: None })?;
                let cond_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_cond_{}", gen_idx));
                let body_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_body_{}", gen_idx));
                let check_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_check_{}", gen_idx));
                let incr_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_incr_{}", gen_idx));
                let after_bb = self
                    .context
                    .append_basic_block(parent_fn, &format!("comp_after_{}", gen_idx));

                // Allocate loop index
                let idx_alloca = self
                    .create_entry_block_alloca(i64_type.into(), &format!("comp_idx_{}", gen_idx))?;
                self.builder
                    .build_store(idx_alloca, i64_type.const_int(0, false))
                    .map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to init loop idx in comp loop IntMatrix".to_string(), span: None })?;

                // Allocate variables and save old ones
                let mut var_allocas = Vec::new();
                let mut old_vars = Vec::new();

                if is_destructuring {
                    for var_name in generator.var_names.iter() {
                        let var_alloca = self.create_entry_block_alloca(i64_type.into(), var_name)?;
                        let old_var = self.variables.remove(var_name);
                        self.variables
                            .insert(var_name.clone(), (var_alloca, BrixType::Int));
                        old_vars.push((var_name.clone(), old_var));
                        var_allocas.push(var_alloca);
                    }
                } else {
                    let var_name = &generator.var_names[0];
                    let var_alloca = self.create_entry_block_alloca(i64_type.into(), var_name)?;
                    let old_var = self.variables.remove(var_name);
                    self.variables
                        .insert(var_name.clone(), (var_alloca, BrixType::Int));
                    old_vars.push((var_name.clone(), old_var));
                }

                // Jump to condition
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to cond in comp loop IntMatrix".to_string(), span: None })?;

                // Condition: idx < total_len
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "cur_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load cur_idx in comp loop IntMatrix".to_string(), span: None })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total_len, "loop_cond")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare idx < total_len in comp loop IntMatrix".to_string(), span: None })?;
                self.builder
                    .build_conditional_branch(cond, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch in comp loop IntMatrix".to_string(), span: None })?;

                // Body: load variables
                self.builder.position_at_end(body_bb);

                if is_destructuring {
                    // Load row elements
                    for (j, var_alloca) in var_allocas.iter().enumerate() {
                        unsafe {
                            let offset = self
                                .builder
                                .build_int_mul(cur_idx, cols, "row_offset")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_mul".to_string(), details: "failed to compute row_offset in comp loop IntMatrix".to_string(), span: None })?;
                            let col_offset = self
                                .builder
                                .build_int_add(
                                    offset,
                                    i64_type.const_int(j as u64, false),
                                    "elem_offset",
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to compute elem_offset in comp loop IntMatrix".to_string(), span: None })?;

                            let elem_ptr = self
                                .builder
                                .build_gep(
                                    i64_type,
                                    data_base,
                                    &[col_offset],
                                    &format!("elem_{}_ptr", j),
                                )
                                .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop IntMatrix destructuring".to_string(), span: None })?;
                            let elem_val = self
                                .builder
                                .build_load(i64_type, elem_ptr, &format!("elem_{}", j))
                                .map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop IntMatrix destructuring".to_string(), span: None })?;
                            self.builder.build_store(*var_alloca, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop IntMatrix destructuring".to_string(), span: None })?;
                        }
                    }
                } else {
                    // Load single element
                    unsafe {
                        let elem_ptr = self
                            .builder
                            .build_gep(i64_type, data_base, &[cur_idx], "elem_ptr")
                            .map_err(|_| CodegenError::LLVMError { operation: "build_gep".to_string(), details: "failed to get elem_ptr in comp loop IntMatrix".to_string(), span: None })?;
                        let elem_val = self.builder.build_load(i64_type, elem_ptr, "elem").map_err(|_| CodegenError::LLVMError { operation: "build_load".to_string(), details: "failed to load elem in comp loop IntMatrix".to_string(), span: None })?;
                        let current_var = self.variables.get(&generator.var_names[0]).ok_or_else(|| CodegenError::UndefinedSymbol { name: generator.var_names[0].clone(), context: "comp loop IntMatrix variable lookup".to_string(), span: None })?.0;
                        self.builder.build_store(current_var, elem_val).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store elem in comp loop IntMatrix".to_string(), span: None })?;
                    }
                }

                // Jump to check block (for conditions)
                self.builder.build_unconditional_branch(check_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to check in comp loop IntMatrix".to_string(), span: None })?;

                // Check block: evaluate all conditions
                self.builder.position_at_end(check_bb);

                if !generator.conditions.is_empty() {
                    let mut combined_cond = None;

                    for condition in &generator.conditions {
                        let (cond_val, _) = self.compile_expr(condition)?;
                        let cond_int = cond_val.into_int_value();
                        let cond_bool = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                cond_int,
                                i64_type.const_int(0, false),
                                "cond_bool",
                            )
                            .map_err(|_| CodegenError::LLVMError { operation: "build_int_compare".to_string(), details: "failed to compare condition in comp loop IntMatrix".to_string(), span: None })?;

                        combined_cond = Some(if let Some(prev) = combined_cond {
                            self.builder
                                .build_and(prev, cond_bool, "combined_cond")
                                .map_err(|_| CodegenError::LLVMError { operation: "build_and".to_string(), details: "failed to combine conditions in comp loop IntMatrix".to_string(), span: None })?
                        } else {
                            cond_bool
                        });
                    }

                    // If conditions pass, recurse to next generator or evaluate expr
                    let recurse_bb = self
                        .context
                        .append_basic_block(parent_fn, &format!("comp_recurse_{}", gen_idx));
                    let combined = combined_cond.ok_or_else(|| CodegenError::MissingValue { what: "combined_cond".to_string(), context: "comp loop IntMatrix conditions".to_string(), span: None })?;
                    self.builder
                        .build_conditional_branch(combined, recurse_bb, incr_bb)
                        .map_err(|_| CodegenError::LLVMError { operation: "build_conditional_branch".to_string(), details: "failed to branch on condition in comp loop IntMatrix".to_string(), span: None })?;

                    self.builder.position_at_end(recurse_bb);
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr in comp loop IntMatrix".to_string(), span: None })?;
                } else {
                    // No conditions, just recurse
                    self.generate_comp_loop(
                        expr,
                        generators,
                        gen_idx + 1,
                        temp_array,
                        temp_type,
                        count_alloca,
                    )?;
                    self.builder.build_unconditional_branch(incr_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to branch to incr (no cond) in comp loop IntMatrix".to_string(), span: None })?;
                }

                // Increment block
                self.builder.position_at_end(incr_bb);
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, i64_type.const_int(1, false), "next_idx")
                    .map_err(|_| CodegenError::LLVMError { operation: "build_int_add".to_string(), details: "failed to increment idx in comp loop IntMatrix".to_string(), span: None })?;
                self.builder.build_store(idx_alloca, next_idx).map_err(|_| CodegenError::LLVMError { operation: "build_store".to_string(), details: "failed to store next_idx in comp loop IntMatrix".to_string(), span: None })?;
                self.builder.build_unconditional_branch(cond_bb).map_err(|_| CodegenError::LLVMError { operation: "build_unconditional_branch".to_string(), details: "failed to loop back in comp loop IntMatrix".to_string(), span: None })?;

                // After block: restore variables
                self.builder.position_at_end(after_bb);

                for (var_name, old_var_opt) in old_vars {
                    if let Some(old) = old_var_opt {
                        self.variables.insert(var_name, old);
                    } else {
                        self.variables.remove(&var_name);
                    }
                }

                Ok(())
            }

            _ => {
                eprintln!(
                    "Error: Unsupported iterable type in list comprehension: {:?}",
                    iterable_type
                );
                Err(CodegenError::InvalidOperation {
                    operation: "list comprehension".to_string(),
                    reason: format!("unsupported iterable type: {:?}", iterable_type),
                                    span: None,
                })
            }
        }
    }
}
