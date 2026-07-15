// Iterator / array method compilation for Brix
//
// Hosts (since refactor Extraction 3) the iterator-method dispatch and the
// array helper methods on IntMatrix/Matrix:
//   compile_iterator_method (map/filter/reduce/any/all/find + v1.7 Grupo B
//   methods), compile_array_{sort,min,max,flatten,unique,reverse,append,
//   prepend,count}, and the call_array_{unary,scalar} runtime shims.
//
// All operate on the Compiler via an inherent impl block, so they can reach
// the sibling helpers still in lib.rs (compile_expr, compile_closure_call,
// infer_closure_return_type, etc.).

use crate::builtins::matrix::MatrixFunctions;
use crate::helpers::HelperFunctions;
use crate::{BrixType, CodegenError, CodegenResult, Compiler};
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum};
use inkwell::{AddressSpace, IntPredicate};
use parser::ast::Expr;

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    /// Dispatch iterator methods (map, filter, reduce, any, all, find) on IntMatrix/Matrix.
    /// Returns None if the method is not recognized.
    pub(crate) fn compile_iterator_method(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        method: &str,
        args: &[Expr],
        call_expr: &Expr,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        let i64_type = self.context.i64_type();
        let f64_type = self.context.f64_type();
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let span = &call_expr.span;
        let one = i64_type.const_int(1, false);
        let zero = i64_type.const_int(0, false);

        let is_int = *receiver_type == BrixType::IntMatrix;
        let matrix_ptr = receiver_val.into_pointer_value();
        let matrix_llvm_type = if is_int {
            self.get_intmatrix_type()
        } else {
            self.get_matrix_type()
        };
        let elem_llvm_type: BasicTypeEnum = if is_int {
            i64_type.into()
        } else {
            f64_type.into()
        };

        // Load rows (field 1) and cols (field 2) from the matrix struct
        let rows_ptr = self
            .builder
            .build_struct_gep(matrix_llvm_type, matrix_ptr, 1, "iter_rows")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get rows field for iterator".to_string(),
                span: Some(span.clone()),
            })?;
        let rows = self
            .builder
            .build_load(i64_type, rows_ptr, "iter_rows")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_load".to_string(),
                details: "Failed to load rows for iterator".to_string(),
                span: Some(span.clone()),
            })?
            .into_int_value();
        let cols_ptr = self
            .builder
            .build_struct_gep(matrix_llvm_type, matrix_ptr, 2, "iter_cols")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get cols field for iterator".to_string(),
                span: Some(span.clone()),
            })?;
        let len = self
            .builder
            .build_load(i64_type, cols_ptr, "iter_len")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_load".to_string(),
                details: "Failed to load len for iterator".to_string(),
                span: Some(span.clone()),
            })?
            .into_int_value();
        // total = rows * cols; flat loop bound (for 1D: rows=1, total=cols)
        let total = self
            .builder
            .build_int_mul(rows, len, "iter_total")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_int_mul".to_string(),
                details: "Failed to compute total elements for iterator".to_string(),
                span: Some(span.clone()),
            })?;

        // Load data pointer (field 3)
        let data_ptr_ptr = self
            .builder
            .build_struct_gep(matrix_llvm_type, matrix_ptr, 3, "iter_data_ptr")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get data field for iterator".to_string(),
                span: Some(span.clone()),
            })?;
        let data_ptr = self
            .builder
            .build_load(ptr_type, data_ptr_ptr, "iter_data")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_load".to_string(),
                details: "Failed to load data ptr for iterator".to_string(),
                span: Some(span.clone()),
            })?
            .into_pointer_value();

        match method {
            "map" => {
                if args.len() != 1 {
                    return Err(CodegenError::InvalidOperation {
                        operation: "map".to_string(),
                        reason: "expects exactly 1 argument (callback)".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let callback = &args[0];
                let ret_type = self.infer_closure_return_type(callback);
                let (closure_val, _) = self.compile_expr(callback)?;
                let (fn_ptr, env_ptr) = self.load_closure_fn_env(closure_val, span)?;

                // Determine result array type
                let ret_brix_type = match &ret_type {
                    BrixType::Float => BrixType::Matrix,
                    _ => BrixType::IntMatrix,
                };
                let ret_llvm_elem: BasicTypeEnum = match &ret_type {
                    BrixType::Float => f64_type.into(),
                    _ => i64_type.into(),
                };
                let result_alloc_fn_name = match &ret_brix_type {
                    BrixType::Matrix => "matrix_new",
                    _ => "intmatrix_new",
                };
                let result_matrix_type = match &ret_brix_type {
                    BrixType::Matrix => self.get_matrix_type(),
                    _ => self.get_intmatrix_type(),
                };

                // Allocate result array
                let alloc_fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let alloc_fn = self
                    .module
                    .get_function(result_alloc_fn_name)
                    .unwrap_or_else(|| {
                        self.module.add_function(
                            result_alloc_fn_name,
                            alloc_fn_type,
                            Some(Linkage::External),
                        )
                    });
                let result_ptr = self
                    .builder
                    .build_call(alloc_fn, &[rows.into(), len.into()], "map_result")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to alloc map result".to_string(),
                        span: Some(span.clone()),
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue {
                        what: "map result array".to_string(),
                        context: "map".to_string(),
                        span: Some(span.clone()),
                    })?
                    .into_pointer_value();

                // Get result data pointer
                let res_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(result_matrix_type, result_ptr, 3, "map_res_data_ptr")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed to get result data ptr".to_string(),
                        span: Some(span.clone()),
                    })?;
                let res_data_ptr = self
                    .builder
                    .build_load(ptr_type, res_data_ptr_ptr, "map_res_data")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load result data ptr".to_string(),
                        span: Some(span.clone()),
                    })?
                    .into_pointer_value();

                // Build loop
                let parent_fn = self.current_function()?;
                let cond_bb = self.context.append_basic_block(parent_fn, "map_cond");
                let body_bb = self.context.append_basic_block(parent_fn, "map_body");
                let inc_bb = self.context.append_basic_block(parent_fn, "map_inc");
                let after_bb = self.context.append_basic_block(parent_fn, "map_after");

                let idx_alloca = self.create_entry_block_alloca(i64_type.into(), "map_idx")?;
                self.builder.build_store(idx_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed to init map idx".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed branch to map cond".to_string(),
                        span: None,
                    })?;

                // Cond: idx < len
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "map_cur_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load map idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total, "map_loop_cond")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed map cond".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(cond, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed map branch".to_string(),
                        span: None,
                    })?;

                // Body: elem = src[idx]; val = fn(env, elem); res[idx] = val
                self.builder.position_at_end(body_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "map_body_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed body idx load".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let elem = unsafe {
                    let elem_ptr = self
                        .builder
                        .build_gep(elem_llvm_type, data_ptr, &[cur_idx], "map_elem_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed map elem gep".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_load(elem_llvm_type, elem_ptr, "map_elem")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed map elem load".to_string(),
                            span: None,
                        })?
                };
                // fn_type: (ptr env, elem_type) -> ret_elem_type
                let map_fn_type = match &ret_llvm_elem {
                    BasicTypeEnum::IntType(t) => {
                        t.fn_type(&[ptr_type.into(), elem_llvm_type.into()], false)
                    }
                    BasicTypeEnum::FloatType(t) => {
                        t.fn_type(&[ptr_type.into(), elem_llvm_type.into()], false)
                    }
                    _ => i64_type.fn_type(&[ptr_type.into(), elem_llvm_type.into()], false),
                };
                let map_result_val = self
                    .builder
                    .build_indirect_call(
                        map_fn_type,
                        fn_ptr,
                        &[env_ptr.into(), elem.into()],
                        "map_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_indirect_call".to_string(),
                        details: "Failed map closure call".to_string(),
                        span: Some(span.clone()),
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue {
                        what: "map callback result".to_string(),
                        context: "map".to_string(),
                        span: Some(span.clone()),
                    })?;
                unsafe {
                    let res_elem_ptr = self
                        .builder
                        .build_gep(ret_llvm_elem, res_data_ptr, &[cur_idx], "map_res_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed map result gep".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_store(res_elem_ptr, map_result_val)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "Failed map result store".to_string(),
                            span: None,
                        })?;
                }
                self.builder
                    .build_unconditional_branch(inc_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed map body branch".to_string(),
                        span: None,
                    })?;

                // Inc: idx++
                self.builder.position_at_end(inc_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "map_inc_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed map inc load".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, one, "map_next_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_add".to_string(),
                        details: "Failed map idx inc".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(idx_alloca, next_idx)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed map store idx".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed map inc branch".to_string(),
                        span: None,
                    })?;

                // After
                self.builder.position_at_end(after_bb);
                Ok(Some((result_ptr.as_basic_value_enum(), ret_brix_type)))
            }

            "filter" => {
                if args.len() != 1 {
                    return Err(CodegenError::InvalidOperation {
                        operation: "filter".to_string(),
                        reason: "expects exactly 1 argument (predicate)".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let callback = &args[0];
                let (closure_val, _) = self.compile_expr(callback)?;
                let (fn_ptr, env_ptr) = self.load_closure_fn_env(closure_val, span)?;
                let pred_fn_type =
                    i64_type.fn_type(&[ptr_type.into(), elem_llvm_type.into()], false);

                // Allocate temp array same size as source (worst case: all pass)
                let alloc_fn_name = if is_int {
                    "intmatrix_new"
                } else {
                    "matrix_new"
                };
                let result_matrix_type = if is_int {
                    self.get_intmatrix_type()
                } else {
                    self.get_matrix_type()
                };
                let alloc_fn_type = ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                let alloc_fn = self.module.get_function(alloc_fn_name).unwrap_or_else(|| {
                    self.module
                        .add_function(alloc_fn_name, alloc_fn_type, Some(Linkage::External))
                });
                let temp_ptr = self
                    .builder
                    .build_call(alloc_fn, &[one.into(), total.into()], "filter_temp")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed alloc filter temp".to_string(),
                        span: Some(span.clone()),
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue {
                        what: "filter temp".to_string(),
                        context: "filter".to_string(),
                        span: Some(span.clone()),
                    })?
                    .into_pointer_value();
                let temp_data_ptr_ptr = self
                    .builder
                    .build_struct_gep(result_matrix_type, temp_ptr, 3, "filter_temp_dp")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed filter temp data ptr".to_string(),
                        span: Some(span.clone()),
                    })?;
                let temp_data_ptr = self
                    .builder
                    .build_load(ptr_type, temp_data_ptr_ptr, "filter_temp_data")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load filter temp data".to_string(),
                        span: Some(span.clone()),
                    })?
                    .into_pointer_value();

                // count = 0
                let count_alloca =
                    self.create_entry_block_alloca(i64_type.into(), "filter_count")?;
                self.builder.build_store(count_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init filter count".to_string(),
                        span: None,
                    }
                })?;

                // Build loop
                let parent_fn = self.current_function()?;
                let cond_bb = self.context.append_basic_block(parent_fn, "filter_cond");
                let body_bb = self.context.append_basic_block(parent_fn, "filter_body");
                let store_bb = self.context.append_basic_block(parent_fn, "filter_store");
                let inc_bb = self.context.append_basic_block(parent_fn, "filter_inc");
                let after_bb = self.context.append_basic_block(parent_fn, "filter_after");

                let idx_alloca = self.create_entry_block_alloca(i64_type.into(), "filter_idx")?;
                self.builder.build_store(idx_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init filter idx".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed branch to filter cond".to_string(),
                        span: None,
                    })?;

                // Cond: idx < len
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "fi_cur_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load filter idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let cond_val = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total, "fi_cond")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed filter cond".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(cond_val, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed filter cond branch".to_string(),
                        span: None,
                    })?;

                // Body: elem = src[idx]; if pred(env, elem) { temp[count] = elem; count++ }
                self.builder.position_at_end(body_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "fi_body_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed filter body idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let elem = unsafe {
                    let ep = self
                        .builder
                        .build_gep(elem_llvm_type, data_ptr, &[cur_idx], "fi_elem_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed filter elem gep".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_load(elem_llvm_type, ep, "fi_elem")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed filter elem load".to_string(),
                            span: None,
                        })?
                };
                let pred_result = self
                    .builder
                    .build_indirect_call(
                        pred_fn_type,
                        fn_ptr,
                        &[env_ptr.into(), elem.into()],
                        "fi_pred",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_indirect_call".to_string(),
                        details: "Failed filter predicate call".to_string(),
                        span: Some(span.clone()),
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue {
                        what: "filter predicate result".to_string(),
                        context: "filter".to_string(),
                        span: Some(span.clone()),
                    })?
                    .into_int_value();
                let pred_bool = self
                    .builder
                    .build_int_compare(IntPredicate::NE, pred_result, zero, "fi_pass")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed filter pred check".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(pred_bool, store_bb, inc_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed filter pred branch".to_string(),
                        span: None,
                    })?;

                // Store: temp[count] = elem; count++
                self.builder.position_at_end(store_bb);
                let cur_count = self
                    .builder
                    .build_load(i64_type, count_alloca, "fi_count")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load filter count".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                unsafe {
                    let tp = self
                        .builder
                        .build_gep(elem_llvm_type, temp_data_ptr, &[cur_count], "fi_temp_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed filter temp gep".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_store(tp, elem)
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "Failed filter temp store".to_string(),
                            span: None,
                        })?;
                }
                let next_count = self
                    .builder
                    .build_int_add(cur_count, one, "fi_next_count")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_add".to_string(),
                        details: "Failed filter count inc".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(count_alloca, next_count)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed filter store count".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_unconditional_branch(inc_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed filter store branch".to_string(),
                        span: None,
                    })?;

                // Inc: idx++
                self.builder.position_at_end(inc_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "fi_inc_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load filter inc idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, one, "fi_next_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_add".to_string(),
                        details: "Failed filter idx inc".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(idx_alloca, next_idx)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed filter store idx".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed filter inc branch".to_string(),
                        span: None,
                    })?;

                // After: allocate result array of actual count and copy
                self.builder.position_at_end(after_bb);
                let final_count = self
                    .builder
                    .build_load(i64_type, count_alloca, "fi_final_count")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load filter final count".to_string(),
                        span: None,
                    })?
                    .into_int_value();

                // Allocate result
                let result_ptr = self
                    .builder
                    .build_call(alloc_fn, &[one.into(), final_count.into()], "filter_result")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed alloc filter result".to_string(),
                        span: Some(span.clone()),
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue {
                        what: "filter result array".to_string(),
                        context: "filter".to_string(),
                        span: Some(span.clone()),
                    })?
                    .into_pointer_value();
                let res_data_pp = self
                    .builder
                    .build_struct_gep(result_matrix_type, result_ptr, 3, "fi_res_dp")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_struct_gep".to_string(),
                        details: "Failed filter result gep".to_string(),
                        span: None,
                    })?;
                let res_data_ptr = self
                    .builder
                    .build_load(ptr_type, res_data_pp, "fi_res_data")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load filter result data".to_string(),
                        span: None,
                    })?
                    .into_pointer_value();

                // Copy loop: copy final_count elements from temp to result
                let parent_fn = self.current_function()?;
                let cp_cond_bb = self.context.append_basic_block(parent_fn, "fi_cp_cond");
                let cp_body_bb = self.context.append_basic_block(parent_fn, "fi_cp_body");
                let cp_inc_bb = self.context.append_basic_block(parent_fn, "fi_cp_inc");
                let cp_after_bb = self.context.append_basic_block(parent_fn, "fi_cp_after");

                let cp_idx_alloca = self.create_entry_block_alloca(i64_type.into(), "fi_cp_idx")?;
                self.builder.build_store(cp_idx_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init copy idx".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(cp_cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed branch to copy cond".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(cp_cond_bb);
                let cp_idx = self
                    .builder
                    .build_load(i64_type, cp_idx_alloca, "fi_cp_cur_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed cp idx load".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let cp_cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cp_idx, final_count, "fi_cp_cond")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed cp cond".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(cp_cond, cp_body_bb, cp_after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed cp cond branch".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(cp_body_bb);
                let cp_idx = self
                    .builder
                    .build_load(i64_type, cp_idx_alloca, "fi_cp_body_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed cp body idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                unsafe {
                    let src_ep = self
                        .builder
                        .build_gep(elem_llvm_type, temp_data_ptr, &[cp_idx], "fi_cp_src")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed cp src gep".to_string(),
                            span: None,
                        })?;
                    let src_val = self
                        .builder
                        .build_load(elem_llvm_type, src_ep, "fi_cp_val")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed cp src load".to_string(),
                            span: None,
                        })?;
                    let dst_ep = self
                        .builder
                        .build_gep(elem_llvm_type, res_data_ptr, &[cp_idx], "fi_cp_dst")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed cp dst gep".to_string(),
                            span: None,
                        })?;
                    self.builder.build_store(dst_ep, src_val).map_err(|_| {
                        CodegenError::LLVMError {
                            operation: "build_store".to_string(),
                            details: "Failed cp store".to_string(),
                            span: None,
                        }
                    })?;
                }
                self.builder
                    .build_unconditional_branch(cp_inc_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed cp body branch".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(cp_inc_bb);
                let cp_idx = self
                    .builder
                    .build_load(i64_type, cp_idx_alloca, "fi_cp_inc_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed cp inc idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let next = self
                    .builder
                    .build_int_add(cp_idx, one, "fi_cp_next")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_add".to_string(),
                        details: "Failed cp idx add".to_string(),
                        span: None,
                    })?;
                self.builder.build_store(cp_idx_alloca, next).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed cp store idx".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(cp_cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed cp inc branch".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(cp_after_bb);
                let ret_type = if is_int {
                    BrixType::IntMatrix
                } else {
                    BrixType::Matrix
                };
                Ok(Some((result_ptr.as_basic_value_enum(), ret_type)))
            }

            "reduce" => {
                if args.len() != 2 {
                    return Err(CodegenError::InvalidOperation {
                        operation: "reduce".to_string(),
                        reason: "expects exactly 2 arguments (init, callback)".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let (init_val, init_type) = self.compile_expr(&args[0])?;
                let callback = &args[1];
                let (closure_val, _) = self.compile_expr(callback)?;
                let (fn_ptr, env_ptr) = self.load_closure_fn_env(closure_val, span)?;

                // acc_llvm_type = llvm type of init value
                let acc_llvm_type: BasicTypeEnum = match &init_type {
                    BrixType::Float => f64_type.into(),
                    _ => i64_type.into(),
                };

                // fn_type: (ptr env, acc_type, elem_type) -> acc_type
                let red_fn_type = match &acc_llvm_type {
                    BasicTypeEnum::FloatType(t) => t.fn_type(
                        &[ptr_type.into(), acc_llvm_type.into(), elem_llvm_type.into()],
                        false,
                    ),
                    _ => i64_type.fn_type(
                        &[ptr_type.into(), acc_llvm_type.into(), elem_llvm_type.into()],
                        false,
                    ),
                };

                // Initialize accumulator
                let acc_alloca = self.create_entry_block_alloca(acc_llvm_type, "reduce_acc")?;
                self.builder
                    .build_store(acc_alloca, init_val)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init reduce acc".to_string(),
                        span: None,
                    })?;

                // Build loop
                let parent_fn = self.current_function()?;
                let cond_bb = self.context.append_basic_block(parent_fn, "reduce_cond");
                let body_bb = self.context.append_basic_block(parent_fn, "reduce_body");
                let inc_bb = self.context.append_basic_block(parent_fn, "reduce_inc");
                let after_bb = self.context.append_basic_block(parent_fn, "reduce_after");

                let idx_alloca = self.create_entry_block_alloca(i64_type.into(), "reduce_idx")?;
                self.builder.build_store(idx_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init reduce idx".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed branch to reduce cond".to_string(),
                        span: None,
                    })?;

                // Cond: idx < len
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "red_cur_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load reduce idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let cond_v = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total, "red_cond")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed reduce cond".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(cond_v, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed reduce cond branch".to_string(),
                        span: None,
                    })?;

                // Body: acc = fn(env, acc, elem)
                self.builder.position_at_end(body_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "red_body_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed reduce body idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let acc = self
                    .builder
                    .build_load(acc_llvm_type, acc_alloca, "red_acc")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load reduce acc".to_string(),
                        span: None,
                    })?;
                let elem = unsafe {
                    let ep = self
                        .builder
                        .build_gep(elem_llvm_type, data_ptr, &[cur_idx], "red_elem_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed reduce elem gep".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_load(elem_llvm_type, ep, "red_elem")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed reduce elem load".to_string(),
                            span: None,
                        })?
                };
                let new_acc = self
                    .builder
                    .build_indirect_call(
                        red_fn_type,
                        fn_ptr,
                        &[env_ptr.into(), acc.into(), elem.into()],
                        "red_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_indirect_call".to_string(),
                        details: "Failed reduce closure call".to_string(),
                        span: Some(span.clone()),
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue {
                        what: "reduce callback result".to_string(),
                        context: "reduce".to_string(),
                        span: Some(span.clone()),
                    })?;
                self.builder.build_store(acc_alloca, new_acc).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed store new acc".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(inc_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed reduce body branch".to_string(),
                        span: None,
                    })?;

                // Inc: idx++
                self.builder.position_at_end(inc_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "red_inc_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed reduce inc idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, one, "red_next_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_add".to_string(),
                        details: "Failed reduce idx add".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(idx_alloca, next_idx)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed reduce store idx".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed reduce inc branch".to_string(),
                        span: None,
                    })?;

                // After: return final acc
                self.builder.position_at_end(after_bb);
                let result = self
                    .builder
                    .build_load(acc_llvm_type, acc_alloca, "red_result")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load reduce result".to_string(),
                        span: None,
                    })?;
                Ok(Some((result, init_type)))
            }

            "any" => {
                if args.len() != 1 {
                    return Err(CodegenError::InvalidOperation {
                        operation: "any".to_string(),
                        reason: "expects exactly 1 argument (predicate)".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let callback = &args[0];
                let (closure_val, _) = self.compile_expr(callback)?;
                let (fn_ptr, env_ptr) = self.load_closure_fn_env(closure_val, span)?;
                let pred_fn_type =
                    i64_type.fn_type(&[ptr_type.into(), elem_llvm_type.into()], false);

                // any: loop, early return true on first match, false at end
                let parent_fn = self.current_function()?;
                let cond_bb = self.context.append_basic_block(parent_fn, "any_cond");
                let body_bb = self.context.append_basic_block(parent_fn, "any_body");
                let true_bb = self.context.append_basic_block(parent_fn, "any_true");
                let inc_bb = self.context.append_basic_block(parent_fn, "any_inc");
                let after_bb = self.context.append_basic_block(parent_fn, "any_after");

                // result_alloca: stores 0 (false) initially; set to 1 on match
                let result_alloca =
                    self.create_entry_block_alloca(i64_type.into(), "any_result")?;
                self.builder.build_store(result_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init any result".to_string(),
                        span: None,
                    }
                })?;
                let idx_alloca = self.create_entry_block_alloca(i64_type.into(), "any_idx")?;
                self.builder.build_store(idx_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init any idx".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed branch to any cond".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "any_cur_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load any idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let cond_v = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total, "any_cond")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed any cond".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(cond_v, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed any cond branch".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(body_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "any_body_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed any body idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let elem = unsafe {
                    let ep = self
                        .builder
                        .build_gep(elem_llvm_type, data_ptr, &[cur_idx], "any_elem_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed any elem gep".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_load(elem_llvm_type, ep, "any_elem")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed any elem load".to_string(),
                            span: None,
                        })?
                };
                let pred_result = self
                    .builder
                    .build_indirect_call(
                        pred_fn_type,
                        fn_ptr,
                        &[env_ptr.into(), elem.into()],
                        "any_pred",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_indirect_call".to_string(),
                        details: "Failed any predicate call".to_string(),
                        span: Some(span.clone()),
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue {
                        what: "any predicate result".to_string(),
                        context: "any".to_string(),
                        span: Some(span.clone()),
                    })?
                    .into_int_value();
                let pred_bool = self
                    .builder
                    .build_int_compare(IntPredicate::NE, pred_result, zero, "any_pass")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed any pred check".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(pred_bool, true_bb, inc_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed any pred branch".to_string(),
                        span: None,
                    })?;

                // True: set result = 1, jump to after
                self.builder.position_at_end(true_bb);
                self.builder.build_store(result_alloca, one).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed any set true".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed any true branch".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(inc_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "any_inc_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed any inc idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, one, "any_next_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_add".to_string(),
                        details: "Failed any idx add".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(idx_alloca, next_idx)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed any store idx".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed any inc branch".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(after_bb);
                let result = self
                    .builder
                    .build_load(i64_type, result_alloca, "any_result_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load any result".to_string(),
                        span: None,
                    })?;
                Ok(Some((result, BrixType::Int)))
            }

            "all" => {
                if args.len() != 1 {
                    return Err(CodegenError::InvalidOperation {
                        operation: "all".to_string(),
                        reason: "expects exactly 1 argument (predicate)".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let callback = &args[0];
                let (closure_val, _) = self.compile_expr(callback)?;
                let (fn_ptr, env_ptr) = self.load_closure_fn_env(closure_val, span)?;
                let pred_fn_type =
                    i64_type.fn_type(&[ptr_type.into(), elem_llvm_type.into()], false);

                // all: loop, early return false on first non-match, true at end
                let parent_fn = self.current_function()?;
                let cond_bb = self.context.append_basic_block(parent_fn, "all_cond");
                let body_bb = self.context.append_basic_block(parent_fn, "all_body");
                let false_bb = self.context.append_basic_block(parent_fn, "all_false");
                let inc_bb = self.context.append_basic_block(parent_fn, "all_inc");
                let after_bb = self.context.append_basic_block(parent_fn, "all_after");

                let result_alloca =
                    self.create_entry_block_alloca(i64_type.into(), "all_result")?;
                self.builder
                    .build_store(result_alloca, one) // default: true
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init all result".to_string(),
                        span: None,
                    })?;
                let idx_alloca = self.create_entry_block_alloca(i64_type.into(), "all_idx")?;
                self.builder.build_store(idx_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init all idx".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed branch to all cond".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "all_cur_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load all idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let cond_v = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total, "all_cond_v")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed all cond".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(cond_v, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed all cond branch".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(body_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "all_body_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed all body idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let elem = unsafe {
                    let ep = self
                        .builder
                        .build_gep(elem_llvm_type, data_ptr, &[cur_idx], "all_elem_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed all elem gep".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_load(elem_llvm_type, ep, "all_elem")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed all elem load".to_string(),
                            span: None,
                        })?
                };
                let pred_result = self
                    .builder
                    .build_indirect_call(
                        pred_fn_type,
                        fn_ptr,
                        &[env_ptr.into(), elem.into()],
                        "all_pred",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_indirect_call".to_string(),
                        details: "Failed all predicate call".to_string(),
                        span: Some(span.clone()),
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue {
                        what: "all predicate result".to_string(),
                        context: "all".to_string(),
                        span: Some(span.clone()),
                    })?
                    .into_int_value();
                let pred_bool = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, pred_result, zero, "all_fail")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed all pred check".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(pred_bool, false_bb, inc_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed all pred branch".to_string(),
                        span: None,
                    })?;

                // False: set result = 0, jump to after
                self.builder.position_at_end(false_bb);
                self.builder.build_store(result_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed all set false".to_string(),
                        span: None,
                    }
                })?;
                self.builder
                    .build_unconditional_branch(after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed all false branch".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(inc_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "all_inc_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed all inc idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, one, "all_next_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_add".to_string(),
                        details: "Failed all idx add".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(idx_alloca, next_idx)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed all store idx".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed all inc branch".to_string(),
                        span: None,
                    })?;

                self.builder.position_at_end(after_bb);
                let result = self
                    .builder
                    .build_load(i64_type, result_alloca, "all_result_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load all result".to_string(),
                        span: None,
                    })?;
                Ok(Some((result, BrixType::Int)))
            }

            "find" => {
                // find(predicate) → elem_type? = Union(elem_brix_type, Nil)
                // Returns tagged union { tag=0, value=elem } if found, { tag=1 } if not found.
                if args.len() != 1 {
                    return Err(CodegenError::InvalidOperation {
                        operation: "find".to_string(),
                        reason: "expects exactly 1 argument (predicate)".to_string(),
                        span: Some(span.clone()),
                    });
                }
                let callback = &args[0];
                let (closure_val, _) = self.compile_expr(callback)?;
                let (fn_ptr, env_ptr) = self.load_closure_fn_env(closure_val, span)?;
                let pred_fn_type =
                    i64_type.fn_type(&[ptr_type.into(), elem_llvm_type.into()], false);

                // Return type: Union(elem_brix_type, Nil)
                let elem_brix_type = if is_int {
                    BrixType::Int
                } else {
                    BrixType::Float
                };
                let ret_union_type = BrixType::Union(vec![elem_brix_type, BrixType::Nil]);
                // LLVM struct: { i64 tag, elem_llvm_type value }
                let union_struct_type = self
                    .context
                    .struct_type(&[i64_type.into(), elem_llvm_type], false);

                // Alloca result with nil default (tag=1)
                let result_alloca =
                    self.create_entry_block_alloca(union_struct_type.into(), "find_result")?;
                let nil_tag = i64_type.const_int(1, false);
                let nil_struct = {
                    let mut s = union_struct_type.get_undef();
                    s = self
                        .builder
                        .build_insert_value(s, nil_tag, 0, "find_nil_tag")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_insert_value".to_string(),
                            details: "Failed insert nil tag".to_string(),
                            span: None,
                        })?
                        .into_struct_value();
                    s
                };
                self.builder
                    .build_store(result_alloca, nil_struct)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed store nil default".to_string(),
                        span: None,
                    })?;

                let idx_alloca = self.create_entry_block_alloca(i64_type.into(), "find_idx")?;
                self.builder.build_store(idx_alloca, zero).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed init find idx".to_string(),
                        span: None,
                    }
                })?;

                let parent_fn = self.current_function()?;
                let cond_bb = self.context.append_basic_block(parent_fn, "find_cond");
                let body_bb = self.context.append_basic_block(parent_fn, "find_body");
                let found_bb = self.context.append_basic_block(parent_fn, "find_found");
                let inc_bb = self.context.append_basic_block(parent_fn, "find_inc");
                let after_bb = self.context.append_basic_block(parent_fn, "find_after");

                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed branch find cond".to_string(),
                        span: None,
                    })?;

                // find_cond
                self.builder.position_at_end(cond_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "find_cur_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load find idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let cond_v = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur_idx, total, "find_cond_v")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed find cond".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(cond_v, body_bb, after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed find cond branch".to_string(),
                        span: None,
                    })?;

                // find_body
                self.builder.position_at_end(body_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "find_body_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed find body idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let elem = unsafe {
                    let ep = self
                        .builder
                        .build_gep(elem_llvm_type, data_ptr, &[cur_idx], "find_elem_ptr")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_gep".to_string(),
                            details: "Failed find elem gep".to_string(),
                            span: None,
                        })?;
                    self.builder
                        .build_load(elem_llvm_type, ep, "find_elem")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_load".to_string(),
                            details: "Failed find elem load".to_string(),
                            span: None,
                        })?
                };
                let pred_result = self
                    .builder
                    .build_indirect_call(
                        pred_fn_type,
                        fn_ptr,
                        &[env_ptr.into(), elem.into()],
                        "find_pred",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_indirect_call".to_string(),
                        details: "Failed find predicate call".to_string(),
                        span: Some(span.clone()),
                    })?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| CodegenError::MissingValue {
                        what: "find predicate result".to_string(),
                        context: "find".to_string(),
                        span: Some(span.clone()),
                    })?
                    .into_int_value();
                let pred_match = self
                    .builder
                    .build_int_compare(IntPredicate::NE, pred_result, zero, "find_match")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed find match check".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_conditional_branch(pred_match, found_bb, inc_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed find match branch".to_string(),
                        span: None,
                    })?;

                // find_found: store { tag=0, value=elem }
                self.builder.position_at_end(found_bb);
                let int_tag = i64_type.const_int(0, false);
                let found_struct = {
                    let mut s = union_struct_type.get_undef();
                    s = self
                        .builder
                        .build_insert_value(s, int_tag, 0, "find_tag")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_insert_value".to_string(),
                            details: "Failed insert found tag".to_string(),
                            span: None,
                        })?
                        .into_struct_value();
                    s = self
                        .builder
                        .build_insert_value(s, elem, 1, "find_val")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_insert_value".to_string(),
                            details: "Failed insert found value".to_string(),
                            span: None,
                        })?
                        .into_struct_value();
                    s
                };
                self.builder
                    .build_store(result_alloca, found_struct)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed store found result".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_unconditional_branch(after_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed find found branch".to_string(),
                        span: None,
                    })?;

                // find_inc
                self.builder.position_at_end(inc_bb);
                let cur_idx = self
                    .builder
                    .build_load(i64_type, idx_alloca, "find_inc_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed find inc idx".to_string(),
                        span: None,
                    })?
                    .into_int_value();
                let next_idx = self
                    .builder
                    .build_int_add(cur_idx, one, "find_next_idx")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_add".to_string(),
                        details: "Failed find idx add".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_store(idx_alloca, next_idx)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: "Failed find store idx".to_string(),
                        span: None,
                    })?;
                self.builder
                    .build_unconditional_branch(cond_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed find inc branch".to_string(),
                        span: None,
                    })?;

                // find_after: load and return result
                self.builder.position_at_end(after_bb);
                let result = self
                    .builder
                    .build_load(union_struct_type, result_alloca, "find_result_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed load find result".to_string(),
                        span: None,
                    })?;
                Ok(Some((result, ret_union_type)))
            }

            // ===== v1.7 Group B: array methods =====
            "sort" => self.compile_array_sort(receiver_val, receiver_type, args, false, span),
            "sort_desc" => self.compile_array_sort(receiver_val, receiver_type, args, true, span),
            "min" => self.compile_array_min(receiver_val, receiver_type, args, span),
            "max" => self.compile_array_max(receiver_val, receiver_type, args, span),
            "flatten" => self.compile_array_flatten(receiver_val, receiver_type, args, span),
            "unique" => self.compile_array_unique(receiver_val, receiver_type, args, span),
            "reverse" => self.compile_array_reverse(receiver_val, receiver_type, args, span),
            "append" => self.compile_array_append(receiver_val, receiver_type, args, span),
            "prepend" => self.compile_array_prepend(receiver_val, receiver_type, args, span),
            "count" => self.compile_array_count(receiver_val, receiver_type, args, span),

            _ => Ok(None), // Not a recognized iterator method
        }
    }

    /// Compile `.sort()` / `.sort_desc()` on IntMatrix/Matrix (v1.7 Group B).
    /// Returns a new array of the same type.
    fn compile_array_sort(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        args: &[Expr],
        descending: bool,
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        let method = if descending { "sort_desc" } else { "sort" };
        if !args.is_empty() {
            return Err(CodegenError::InvalidOperation {
                operation: method.to_string(),
                reason: "expects no arguments".to_string(),
                span: Some(span.clone()),
            });
        }
        let is_int = *receiver_type == BrixType::IntMatrix;
        let func = match (is_int, descending) {
            (true, false) => self.get_intmatrix_sort_asc(),
            (true, true) => self.get_intmatrix_sort_desc(),
            (false, false) => self.get_matrix_sort_asc(),
            (false, true) => self.get_matrix_sort_desc(),
        };
        let result = self.call_array_unary(func, receiver_val, method, span)?;
        Ok(Some((result, receiver_type.clone())))
    }

    /// Compile `.min()` on IntMatrix/Matrix (v1.7 Group B). Returns scalar.
    fn compile_array_min(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        args: &[Expr],
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        if !args.is_empty() {
            return Err(CodegenError::InvalidOperation {
                operation: "min".to_string(),
                reason: "expects no arguments".to_string(),
                span: Some(span.clone()),
            });
        }
        let is_int = *receiver_type == BrixType::IntMatrix;
        let func = if is_int {
            self.get_brix_intmatrix_min()
        } else {
            self.get_brix_matrix_min()
        };
        let result = self.call_array_unary(func, receiver_val, "min", span)?;
        let ret_type = if is_int {
            BrixType::Int
        } else {
            BrixType::Float
        };
        Ok(Some((result, ret_type)))
    }

    /// Compile `.max()` on IntMatrix/Matrix (v1.7 Group B). Returns scalar.
    fn compile_array_max(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        args: &[Expr],
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        if !args.is_empty() {
            return Err(CodegenError::InvalidOperation {
                operation: "max".to_string(),
                reason: "expects no arguments".to_string(),
                span: Some(span.clone()),
            });
        }
        let is_int = *receiver_type == BrixType::IntMatrix;
        let func = if is_int {
            self.get_brix_intmatrix_max()
        } else {
            self.get_brix_matrix_max()
        };
        let result = self.call_array_unary(func, receiver_val, "max", span)?;
        let ret_type = if is_int {
            BrixType::Int
        } else {
            BrixType::Float
        };
        Ok(Some((result, ret_type)))
    }

    /// Compile `.flatten()` on IntMatrix/Matrix (v1.7 Group B). Returns same type, 1D.
    fn compile_array_flatten(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        args: &[Expr],
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        if !args.is_empty() {
            return Err(CodegenError::InvalidOperation {
                operation: "flatten".to_string(),
                reason: "expects no arguments".to_string(),
                span: Some(span.clone()),
            });
        }
        let is_int = *receiver_type == BrixType::IntMatrix;
        let func = if is_int {
            self.get_intmatrix_flatten()
        } else {
            self.get_matrix_flatten()
        };
        let result = self.call_array_unary(func, receiver_val, "flatten", span)?;
        Ok(Some((result, receiver_type.clone())))
    }

    /// Compile `.unique()` on IntMatrix/Matrix (v1.7 Group B). Returns same type.
    fn compile_array_unique(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        args: &[Expr],
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        if !args.is_empty() {
            return Err(CodegenError::InvalidOperation {
                operation: "unique".to_string(),
                reason: "expects no arguments".to_string(),
                span: Some(span.clone()),
            });
        }
        let is_int = *receiver_type == BrixType::IntMatrix;
        let func = if is_int {
            self.get_intmatrix_unique()
        } else {
            self.get_matrix_unique()
        };
        let result = self.call_array_unary(func, receiver_val, "unique", span)?;
        Ok(Some((result, receiver_type.clone())))
    }

    /// Compile `.reverse()` on IntMatrix/Matrix (v1.7 Group B). Returns same type.
    fn compile_array_reverse(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        args: &[Expr],
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        if !args.is_empty() {
            return Err(CodegenError::InvalidOperation {
                operation: "reverse".to_string(),
                reason: "expects no arguments".to_string(),
                span: Some(span.clone()),
            });
        }
        let is_int = *receiver_type == BrixType::IntMatrix;
        let func = if is_int {
            self.get_intmatrix_reverse()
        } else {
            self.get_matrix_reverse()
        };
        let result = self.call_array_unary(func, receiver_val, "reverse", span)?;
        Ok(Some((result, receiver_type.clone())))
    }

    /// Compile `.append(val)` on IntMatrix/Matrix (v1.7 Group B). Returns new array.
    fn compile_array_append(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        args: &[Expr],
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        if args.len() != 1 {
            return Err(CodegenError::InvalidOperation {
                operation: "append".to_string(),
                reason: "expects exactly 1 argument (value to append)".to_string(),
                span: Some(span.clone()),
            });
        }
        let is_int = *receiver_type == BrixType::IntMatrix;
        let (arg_val, arg_type) = self.compile_expr(&args[0])?;
        let scalar: inkwell::values::BasicMetadataValueEnum<'ctx> = if is_int {
            self.coerce_to_i64(arg_val, &arg_type, "append")?.into()
        } else {
            self.coerce_to_f64(arg_val, &arg_type)?.into()
        };
        let func = if is_int {
            self.get_intmatrix_append()
        } else {
            self.get_matrix_append()
        };
        let result = self.call_array_scalar(func, receiver_val, scalar, "append", span)?;
        Ok(Some((result, receiver_type.clone())))
    }

    /// Compile `.prepend(val)` on IntMatrix/Matrix (v1.7 Group B). Returns new array.
    fn compile_array_prepend(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        args: &[Expr],
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        if args.len() != 1 {
            return Err(CodegenError::InvalidOperation {
                operation: "prepend".to_string(),
                reason: "expects exactly 1 argument (value to prepend)".to_string(),
                span: Some(span.clone()),
            });
        }
        let is_int = *receiver_type == BrixType::IntMatrix;
        let (arg_val, arg_type) = self.compile_expr(&args[0])?;
        let scalar: inkwell::values::BasicMetadataValueEnum<'ctx> = if is_int {
            self.coerce_to_i64(arg_val, &arg_type, "prepend")?.into()
        } else {
            self.coerce_to_f64(arg_val, &arg_type)?.into()
        };
        let func = if is_int {
            self.get_intmatrix_prepend()
        } else {
            self.get_matrix_prepend()
        };
        let result = self.call_array_scalar(func, receiver_val, scalar, "prepend", span)?;
        Ok(Some((result, receiver_type.clone())))
    }

    /// Compile `.count()` on IntMatrix/Matrix (v1.7 Group B). Returns rows*cols as Int.
    fn compile_array_count(
        &mut self,
        receiver_val: BasicValueEnum<'ctx>,
        receiver_type: &BrixType,
        args: &[Expr],
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<Option<(BasicValueEnum<'ctx>, BrixType)>> {
        if !args.is_empty() {
            return Err(CodegenError::InvalidOperation {
                operation: "count".to_string(),
                reason: "expects no arguments".to_string(),
                span: Some(span.clone()),
            });
        }
        let i64_type = self.context.i64_type();
        let is_int = *receiver_type == BrixType::IntMatrix;
        let matrix_llvm_type = if is_int {
            self.get_intmatrix_type()
        } else {
            self.get_matrix_type()
        };
        let matrix_ptr = receiver_val.into_pointer_value();

        // Load rows (field 1) and cols (field 2)
        let rows_ptr = self
            .builder
            .build_struct_gep(matrix_llvm_type, matrix_ptr, 1, "count_rows_ptr")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get rows field for count".to_string(),
                span: Some(span.clone()),
            })?;
        let rows = self
            .builder
            .build_load(i64_type, rows_ptr, "count_rows")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_load".to_string(),
                details: "Failed to load rows for count".to_string(),
                span: Some(span.clone()),
            })?
            .into_int_value();
        let cols_ptr = self
            .builder
            .build_struct_gep(matrix_llvm_type, matrix_ptr, 2, "count_cols_ptr")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_struct_gep".to_string(),
                details: "Failed to get cols field for count".to_string(),
                span: Some(span.clone()),
            })?;
        let cols = self
            .builder
            .build_load(i64_type, cols_ptr, "count_cols")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_load".to_string(),
                details: "Failed to load cols for count".to_string(),
                span: Some(span.clone()),
            })?
            .into_int_value();
        let total = self
            .builder
            .build_int_mul(rows, cols, "count_total")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_int_mul".to_string(),
                details: "Failed to compute total elements for count".to_string(),
                span: Some(span.clone()),
            })?;
        Ok(Some((total.into(), BrixType::Int)))
    }

    /// Helper: call a runtime function taking a single array pointer argument.
    fn call_array_unary(
        &mut self,
        func: inkwell::values::FunctionValue<'ctx>,
        receiver_val: BasicValueEnum<'ctx>,
        op: &str,
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let call = self
            .builder
            .build_call(func, &[receiver_val.into()], &format!("{}_result", op))
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: format!("Failed to call array method '{}'", op),
                span: Some(span.clone()),
            })?;
        call.try_as_basic_value()
            .left()
            .ok_or_else(|| CodegenError::MissingValue {
                what: format!("{} result", op),
                context: op.to_string(),
                span: Some(span.clone()),
            })
    }

    /// Helper: call a runtime function taking an array pointer plus a scalar argument.
    fn call_array_scalar(
        &mut self,
        func: inkwell::values::FunctionValue<'ctx>,
        receiver_val: BasicValueEnum<'ctx>,
        scalar: inkwell::values::BasicMetadataValueEnum<'ctx>,
        op: &str,
        span: &std::ops::Range<usize>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let call = self
            .builder
            .build_call(
                func,
                &[receiver_val.into(), scalar],
                &format!("{}_result", op),
            )
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_call".to_string(),
                details: format!("Failed to call array method '{}'", op),
                span: Some(span.clone()),
            })?;
        call.try_as_basic_value()
            .left()
            .ok_or_else(|| CodegenError::MissingValue {
                what: format!("{} result", op),
                context: op.to_string(),
                span: Some(span.clone()),
            })
    }
}
