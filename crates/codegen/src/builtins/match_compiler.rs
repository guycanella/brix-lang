// Pattern-match compilation for Brix
//
// Hosts (since refactor Extraction 4) the pattern-matching codegen used by
// the `match` expression/statement handler (which stays inline in lib.rs's
// ExprKind::Match, along with exhaustiveness checking):
//   compile_pattern_match (Literal/Binding/Wildcard/Or/Range/Destructure/
//   NamedField/ArrayRest), the apply_sub_pattern / apply_sub_pattern_with_prebound
//   sub-pattern dispatchers, and collect_pattern_binding_names (arm-scoped
//   binding collection).
//
// Implemented as an inherent impl block on Compiler, so it reaches the
// sibling helpers still in lib.rs (compile_expr, struct_defs lookups, etc.).

use crate::builtins::matrix::MatrixFunctions;
use crate::helpers::HelperFunctions;
use crate::{BrixType, CodegenError, CodegenResult, Compiler};
use inkwell::module::Linkage;
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;
use std::collections::{HashMap, HashSet};

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    /// Walk a pattern collecting every variable name it would bind (top-level
    /// `Binding`, and nested inside `Or`/`Destructure`/`NamedField`). Used by
    /// the `match` arm loop to scope bindings to their own arm — see the
    /// save/restore around `self.variables` in `ExprKind::Match`.
    pub(crate) fn collect_pattern_binding_names(
        &self,
        pattern: &parser::ast::Pattern,
        out: &mut Vec<String>,
    ) {
        use parser::ast::Pattern;
        match pattern {
            Pattern::Binding(name) => out.push(name.clone()),
            Pattern::Or(patterns) | Pattern::Destructure(patterns) => {
                for p in patterns {
                    self.collect_pattern_binding_names(p, out);
                }
            }
            Pattern::NamedField(fields) => {
                for (_, p) in fields {
                    self.collect_pattern_binding_names(p, out);
                }
            }
            Pattern::ArrayRest { head, rest } => {
                for p in head {
                    self.collect_pattern_binding_names(p, out);
                }
                // The rest capture is always a direct binding
                out.push(rest.clone());
            }
            Pattern::Literal(_) | Pattern::Wildcard | Pattern::Range { .. } => {}
        }
    }

    /// Compile pattern matching: returns i1 (bool) indicating if pattern matches
    pub(crate) fn compile_pattern_match(
        &mut self,
        pattern: &parser::ast::Pattern,
        value: BasicValueEnum<'ctx>,
        value_type: &BrixType,
    ) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        use parser::ast::Pattern;

        match pattern {
            Pattern::Literal(lit) => {
                // Compare value with literal
                match (lit, value_type) {
                    (parser::ast::Literal::Int(n), BrixType::Int) => {
                        let literal_val = self.context.i64_type().const_int(*n as u64, false);
                        let cmp = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                value.into_int_value(),
                                literal_val,
                                "pat_int_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to compare int pattern".to_string(),
                                span: None,
                            })?;
                        Ok(cmp)
                    }
                    (parser::ast::Literal::Float(f), BrixType::Float) => {
                        let literal_val = self.context.f64_type().const_float(*f);
                        let cmp = self
                            .builder
                            .build_float_compare(
                                inkwell::FloatPredicate::OEQ,
                                value.into_float_value(),
                                literal_val,
                                "pat_float_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_float_compare".to_string(),
                                details: "Failed to compare float pattern".to_string(),
                                span: None,
                            })?;
                        Ok(cmp)
                    }
                    (parser::ast::Literal::Bool(b), BrixType::Int) => {
                        // bool is stored as i64
                        let literal_val = self.context.i64_type().const_int(*b as u64, false);
                        let cmp = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                value.into_int_value(),
                                literal_val,
                                "pat_bool_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to compare bool pattern".to_string(),
                                span: None,
                            })?;
                        Ok(cmp)
                    }
                    (parser::ast::Literal::String(s), BrixType::String) => {
                        // String comparison via runtime str_eq
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let fn_type = self
                            .context
                            .bool_type()
                            .fn_type(&[ptr_type.into(), ptr_type.into()], false);

                        let str_eq_fn = self.module.get_function("str_eq").unwrap_or_else(|| {
                            self.module
                                .add_function("str_eq", fn_type, Some(Linkage::External))
                        });

                        // Create literal string
                        let raw_str =
                            self.builder
                                .build_global_string_ptr(s, "pat_str")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_global_string_ptr".to_string(),
                                    details: "Failed to create pattern string".to_string(),
                                    span: None,
                                })?;
                        let str_new_fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
                        let str_new_fn = self.module.get_function("str_new").unwrap_or_else(|| {
                            self.module.add_function(
                                "str_new",
                                str_new_fn_type,
                                Some(Linkage::External),
                            )
                        });

                        let call = self
                            .builder
                            .build_call(
                                str_new_fn,
                                &[raw_str.as_pointer_value().into()],
                                "pat_lit_str",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call str_new for pattern".to_string(),
                                span: None,
                            })?;
                        let literal_str = call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "str_new result".to_string(),
                                context: "pattern string literal".to_string(),
                                span: None,
                            }
                        })?;

                        // Compare strings
                        let call = self
                            .builder
                            .build_call(
                                str_eq_fn,
                                &[value.into(), literal_str.into()],
                                "pat_str_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call str_eq for pattern".to_string(),
                                span: None,
                            })?;
                        let result = call
                            .try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "str_eq result".to_string(),
                                context: "pattern string comparison".to_string(),
                                span: None,
                            })?
                            .into_int_value();

                        Ok(result)
                    }
                    (parser::ast::Literal::Atom(name), BrixType::Atom) => {
                        // Atom comparison: compare atom IDs (i64)
                        // First, intern the pattern atom to get its ID
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
                            .build_global_string_ptr(name, "pat_atom_name")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_global_string_ptr".to_string(),
                                details: "Failed to create atom name string for pattern"
                                    .to_string(),
                                span: None,
                            })?;

                        // Call atom_intern(name) to get the pattern atom ID
                        let call = self
                            .builder
                            .build_call(
                                atom_intern_fn,
                                &[name_cstr.as_pointer_value().into()],
                                "pat_atom_id",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call atom_intern for pattern".to_string(),
                                span: None,
                            })?;
                        let pattern_atom_id = call
                            .try_as_basic_value()
                            .left()
                            .ok_or_else(|| CodegenError::MissingValue {
                                what: "atom_intern result".to_string(),
                                context: "pattern atom comparison".to_string(),
                                span: None,
                            })?
                            .into_int_value();

                        // Compare atom IDs (O(1) comparison)
                        let cmp = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::EQ,
                                value.into_int_value(),
                                pattern_atom_id,
                                "pat_atom_cmp",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to compare atom IDs in pattern".to_string(),
                                span: None,
                            })?;

                        Ok(cmp)
                    }
                    _ => Err(CodegenError::TypeError {
                        expected: format!("{:?}", value_type),
                        found: format!("{:?}", lit),
                        context: "Pattern literal type mismatch".to_string(),
                        span: None,
                    }),
                }
            }

            Pattern::Wildcard => {
                // Wildcard always matches
                Ok(self.context.bool_type().const_int(1, false))
            }

            Pattern::Binding(_) => {
                // Binding always matches (variable name is bound in caller)
                Ok(self.context.bool_type().const_int(1, false))
            }

            Pattern::Or(patterns) => {
                // Or pattern: match any of the sub-patterns
                let mut result = self.context.bool_type().const_int(0, false);

                for pat in patterns {
                    let pat_match = self.compile_pattern_match(pat, value, value_type)?;
                    result = self
                        .builder
                        .build_or(result, pat_match, "or_pat")
                        .map_err(|_| CodegenError::LLVMError {
                            operation: "build_or".to_string(),
                            details: "Failed to OR pattern results".to_string(),
                            span: None,
                        })?;
                }

                Ok(result)
            }

            Pattern::Destructure(sub_patterns) => {
                let combined_init = self.context.bool_type().const_int(1, false);
                match value_type {
                    BrixType::Tuple(field_types) => {
                        let sv = value.into_struct_value();
                        let mut combined = combined_init;
                        for (i, (sub_pat, ft)) in
                            sub_patterns.iter().zip(field_types.iter()).enumerate()
                        {
                            let extracted = self
                                .builder
                                .build_extract_value(sv, i as u32, &format!("dt_{}", i))
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_extract_value".to_string(),
                                    details: format!("Failed to extract tuple field {}", i),
                                    span: None,
                                })?;
                            combined =
                                self.apply_sub_pattern(sub_pat, extracted, ft, combined, i)?;
                        }
                        Ok(combined)
                    }
                    BrixType::Struct(struct_name) => {
                        let struct_def =
                            self.struct_defs.get(struct_name).cloned().ok_or_else(|| {
                                CodegenError::UndefinedSymbol {
                                    name: struct_name.clone(),
                                    context: "Destructuring pattern".to_string(),
                                    span: None,
                                }
                            })?;
                        let sv = value.into_struct_value();
                        let mut combined = combined_init;
                        for (i, (sub_pat, (_, ft, _))) in
                            sub_patterns.iter().zip(struct_def.iter()).enumerate()
                        {
                            let extracted = self
                                .builder
                                .build_extract_value(sv, i as u32, &format!("ds_{}", i))
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_extract_value".to_string(),
                                    details: format!("Failed to extract struct field {}", i),
                                    span: None,
                                })?;
                            combined =
                                self.apply_sub_pattern(sub_pat, extracted, ft, combined, i)?;
                        }
                        Ok(combined)
                    }
                    BrixType::IntMatrix | BrixType::Matrix => {
                        let is_int = matches!(value_type, BrixType::IntMatrix);
                        let elem_brix = if is_int {
                            BrixType::Int
                        } else {
                            BrixType::Float
                        };
                        let elem_llvm = if is_int {
                            self.context.i64_type().as_basic_type_enum()
                        } else {
                            self.context.f64_type().as_basic_type_enum()
                        };
                        let matrix_struct_type = if is_int {
                            self.get_intmatrix_type()
                        } else {
                            self.get_matrix_type()
                        };
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let matrix_ptr = value.into_pointer_value();
                        let data_ptr_ptr = self
                            .builder
                            .build_struct_gep(matrix_struct_type, matrix_ptr, 3, "data_pp")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_struct_gep".to_string(),
                                details: "Failed to get matrix data pointer".to_string(),
                                span: None,
                            })?;
                        let data_ptr = self
                            .builder
                            .build_load(ptr_type, data_ptr_ptr, "data_p")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load matrix data pointer".to_string(),
                                span: None,
                            })?
                            .into_pointer_value();
                        let mut combined = combined_init;
                        for (i, sub_pat) in sub_patterns.iter().enumerate() {
                            let idx_val = self.context.i64_type().const_int(i as u64, false);
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_gep(
                                        elem_llvm,
                                        data_ptr,
                                        &[idx_val],
                                        &format!("ep_{}", i),
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_gep".to_string(),
                                        details: format!("Failed to GEP matrix element {}", i),
                                        span: None,
                                    })?
                            };
                            let extracted = self
                                .builder
                                .build_load(elem_llvm, elem_ptr, &format!("ev_{}", i))
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!("Failed to load matrix element {}", i),
                                    span: None,
                                })?;
                            combined = self
                                .apply_sub_pattern(sub_pat, extracted, &elem_brix, combined, i)?;
                        }
                        Ok(combined)
                    }
                    _ => Err(CodegenError::TypeError {
                        expected: "Tuple, Struct, or Matrix".to_string(),
                        found: format!("{:?}", value_type),
                        context: "Destructuring pattern".to_string(),
                        span: None,
                    }),
                }
            }

            Pattern::NamedField(fields) => match value_type {
                BrixType::Struct(struct_name) => {
                    let struct_def =
                        self.struct_defs.get(struct_name).cloned().ok_or_else(|| {
                            CodegenError::UndefinedSymbol {
                                name: struct_name.clone(),
                                context: "Named field pattern".to_string(),
                                span: None,
                            }
                        })?;
                    let sv = value.into_struct_value();
                    let mut combined = self.context.bool_type().const_int(1, false);
                    for (field_name, sub_pat) in fields {
                        let idx = struct_def
                            .iter()
                            .position(|(n, _, _)| n == field_name)
                            .ok_or_else(|| CodegenError::UndefinedSymbol {
                                name: field_name.clone(),
                                context: format!("Field not found in struct '{}'", struct_name),
                                span: None,
                            })?;
                        let ft = struct_def[idx].1.clone();
                        let extracted = self
                            .builder
                            .build_extract_value(sv, idx as u32, &format!("nf_{}", field_name))
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_extract_value".to_string(),
                                details: format!("Failed to extract field '{}'", field_name),
                                span: None,
                            })?;
                        combined =
                            self.apply_sub_pattern(sub_pat, extracted, &ft, combined, idx)?;
                    }
                    Ok(combined)
                }
                _ => Err(CodegenError::TypeError {
                    expected: "Struct".to_string(),
                    found: format!("{:?}", value_type),
                    context: "Named field pattern".to_string(),
                    span: None,
                }),
            },

            Pattern::Range {
                start,
                end,
                inclusive,
            } => {
                use inkwell::FloatPredicate::{OLE, OLT};
                use inkwell::IntPredicate::{SLE, SLT};
                use parser::ast::Literal as Lit;
                match (start, end, value_type) {
                    (Lit::Int(s), Lit::Int(e), BrixType::Int) => {
                        let val = value.into_int_value();
                        let sv = self.context.i64_type().const_int(*s as u64, true);
                        let ev = self.context.i64_type().const_int(*e as u64, true);
                        let lo = self
                            .builder
                            .build_int_compare(SLE, sv, val, "rng_lo")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to build range lower bound".to_string(),
                                span: None,
                            })?;
                        let hi = self
                            .builder
                            .build_int_compare(
                                if *inclusive { SLE } else { SLT },
                                val,
                                ev,
                                "rng_hi",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to build range upper bound".to_string(),
                                span: None,
                            })?;
                        Ok(self.builder.build_and(lo, hi, "rng_and").map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_and".to_string(),
                                details: "Failed to AND range bounds".to_string(),
                                span: None,
                            }
                        })?)
                    }
                    (Lit::Float(s), Lit::Float(e), BrixType::Float) => {
                        let val = value.into_float_value();
                        let sv = self.context.f64_type().const_float(*s);
                        let ev = self.context.f64_type().const_float(*e);
                        let lo = self
                            .builder
                            .build_float_compare(OLE, sv, val, "rng_lo")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_float_compare".to_string(),
                                details: "Failed to build float range lower bound".to_string(),
                                span: None,
                            })?;
                        let hi = self
                            .builder
                            .build_float_compare(
                                if *inclusive { OLE } else { OLT },
                                val,
                                ev,
                                "rng_hi",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_float_compare".to_string(),
                                details: "Failed to build float range upper bound".to_string(),
                                span: None,
                            })?;
                        Ok(self.builder.build_and(lo, hi, "rng_and").map_err(|_| {
                            CodegenError::LLVMError {
                                operation: "build_and".to_string(),
                                details: "Failed to AND float range bounds".to_string(),
                                span: None,
                            }
                        })?)
                    }
                    _ => Err(CodegenError::TypeError {
                        expected: "Int range with Int value, or Float range with Float value"
                            .to_string(),
                        found: format!("{:?} range with {:?} value", start, value_type),
                        context: "Range pattern".to_string(),
                        span: None,
                    }),
                }
            }

            Pattern::ArrayRest { head, rest } => {
                match value_type {
                    BrixType::IntMatrix | BrixType::Matrix => {
                        let is_int = matches!(value_type, BrixType::IntMatrix);
                        let elem_brix = if is_int {
                            BrixType::Int
                        } else {
                            BrixType::Float
                        };
                        let elem_llvm = if is_int {
                            self.context.i64_type().as_basic_type_enum()
                        } else {
                            self.context.f64_type().as_basic_type_enum()
                        };
                        let matrix_struct_type = if is_int {
                            self.get_intmatrix_type()
                        } else {
                            self.get_matrix_type()
                        };
                        let i64_type = self.context.i64_type();
                        let ptr_type = self.context.ptr_type(AddressSpace::default());
                        let matrix_ptr = value.into_pointer_value();

                        // Load rows (field 1) and cols (field 2), compute total = rows * cols
                        let rows_ptr = self
                            .builder
                            .build_struct_gep(matrix_struct_type, matrix_ptr, 1, "ar_rows_p")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_struct_gep".to_string(),
                                details: "Failed to get matrix rows pointer".to_string(),
                                span: None,
                            })?;
                        let rows = self
                            .builder
                            .build_load(i64_type, rows_ptr, "ar_rows")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load matrix rows".to_string(),
                                span: None,
                            })?
                            .into_int_value();
                        let cols_ptr = self
                            .builder
                            .build_struct_gep(matrix_struct_type, matrix_ptr, 2, "ar_cols_p")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_struct_gep".to_string(),
                                details: "Failed to get matrix cols pointer".to_string(),
                                span: None,
                            })?;
                        let cols = self
                            .builder
                            .build_load(i64_type, cols_ptr, "ar_cols")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load matrix cols".to_string(),
                                span: None,
                            })?
                            .into_int_value();
                        let total =
                            self.builder
                                .build_int_mul(rows, cols, "ar_total")
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_int_mul".to_string(),
                                    details: "Failed to compute matrix total elements".to_string(),
                                    span: None,
                                })?;

                        // Load data pointer (field 3)
                        let data_ptr_ptr = self
                            .builder
                            .build_struct_gep(matrix_struct_type, matrix_ptr, 3, "ar_data_pp")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_struct_gep".to_string(),
                                details: "Failed to get matrix data pointer".to_string(),
                                span: None,
                            })?;
                        let data_ptr = self
                            .builder
                            .build_load(ptr_type, data_ptr_ptr, "ar_data_p")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_load".to_string(),
                                details: "Failed to load matrix data pointer".to_string(),
                                span: None,
                            })?
                            .into_pointer_value();

                        // len check: total >= head.len()
                        let head_len_val = i64_type.const_int(head.len() as u64, false);
                        let len_check = self
                            .builder
                            .build_int_compare(
                                inkwell::IntPredicate::SGE,
                                total,
                                head_len_val,
                                "ar_len_chk",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_int_compare".to_string(),
                                details: "Failed to build array-rest length check".to_string(),
                                span: None,
                            })?;

                        // Pre-allocate all bindings before the conditional CFG below. Stores
                        // still happen only on the matched path, but the alloca values must
                        // dominate later guard/body blocks that can reference these names.
                        let mut head_binding_names = Vec::new();
                        for sub_pat in head {
                            self.collect_pattern_binding_names(sub_pat, &mut head_binding_names);
                        }
                        let mut seen_head_bindings = HashSet::new();
                        let mut prebound_head_bindings = HashMap::new();
                        for name in head_binding_names {
                            if !seen_head_bindings.insert(name.clone()) {
                                continue;
                            }
                            let alloca = self.create_entry_block_alloca(elem_llvm, &name)?;
                            prebound_head_bindings
                                .insert(name.clone(), (alloca, elem_brix.clone()));
                            self.variables.insert(name, (alloca, elem_brix.clone()));
                        }

                        let rest_alloca = self.create_entry_block_alloca(ptr_type.into(), rest)?;
                        self.variables
                            .insert(rest.clone(), (rest_alloca, value_type.clone()));

                        // The head element reads (out-of-bounds if total < head.len()) and the
                        // rest slice call (a real heap allocation) are side-effecting/unsafe, so
                        // they must be gated behind len_check actually holding — not run
                        // unconditionally and only affect the boolean *result*. Branch into real
                        // basic blocks (same PHI-merge shape used for ternary/match elsewhere)
                        // instead of a straight-line AND chain.
                        let cur_block = self.builder.get_insert_block().ok_or_else(|| {
                            CodegenError::LLVMError {
                                operation: "get_insert_block".to_string(),
                                details: "No current basic block for array-rest pattern"
                                    .to_string(),
                                span: None,
                            }
                        })?;
                        let parent_fn =
                            cur_block
                                .get_parent()
                                .ok_or_else(|| CodegenError::LLVMError {
                                    operation: "get_parent".to_string(),
                                    details: "Basic block has no parent function".to_string(),
                                    span: None,
                                })?;

                        let head_bb = self.context.append_basic_block(parent_fn, "ar_head_check");
                        let match_bb = self.context.append_basic_block(parent_fn, "ar_match");
                        let fail_bb = self.context.append_basic_block(parent_fn, "ar_fail");
                        let merge_bb = self.context.append_basic_block(parent_fn, "ar_merge");

                        self.builder
                            .build_conditional_branch(len_check, head_bb, fail_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_conditional_branch".to_string(),
                                details: "Failed to branch on array-rest length check".to_string(),
                                span: None,
                            })?;

                        // head_bb: only reached when total >= head.len(), so these reads are
                        // in-bounds. Apply each head sub-pattern, AND-ing into head_combined.
                        self.builder.position_at_end(head_bb);
                        let mut head_combined = self.context.bool_type().const_int(1, false);
                        for (i, sub_pat) in head.iter().enumerate() {
                            let idx_val = i64_type.const_int(i as u64, false);
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_gep(
                                        elem_llvm,
                                        data_ptr,
                                        &[idx_val],
                                        &format!("ar_ep_{}", i),
                                    )
                                    .map_err(|_| CodegenError::LLVMError {
                                        operation: "build_gep".to_string(),
                                        details: format!(
                                            "Failed to GEP array-rest head element {}",
                                            i
                                        ),
                                        span: None,
                                    })?
                            };
                            let extracted = self
                                .builder
                                .build_load(elem_llvm, elem_ptr, &format!("ar_ev_{}", i))
                                .map_err(|_| CodegenError::LLVMError {
                                    operation: "build_load".to_string(),
                                    details: format!(
                                        "Failed to load array-rest head element {}",
                                        i
                                    ),
                                    span: None,
                                })?;
                            head_combined = self.apply_sub_pattern_with_prebound(
                                sub_pat,
                                extracted,
                                &elem_brix,
                                head_combined,
                                i,
                                &prebound_head_bindings,
                            )?;
                        }
                        self.builder
                            .build_conditional_branch(head_combined, match_bb, fail_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_conditional_branch".to_string(),
                                details: "Failed to branch on array-rest head sub-patterns"
                                    .to_string(),
                                span: None,
                            })?;

                        // match_bb: only reached when the whole pattern matched. Build the rest
                        // sub-array here — elements [head.len() .. total) — so the allocation
                        // (matrix_slice/intmatrix_slice) never runs for a failed match attempt.
                        self.builder.position_at_end(match_bb);
                        let slice_fn = if is_int {
                            self.get_intmatrix_slice()
                        } else {
                            self.get_matrix_slice()
                        };
                        let start_const = i64_type.const_int(head.len() as u64, false);
                        let slice_call = self
                            .builder
                            .build_call(
                                slice_fn,
                                &[matrix_ptr.into(), start_const.into(), total.into()],
                                "ar_rest_slice",
                            )
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_call".to_string(),
                                details: "Failed to call slice for array-rest capture".to_string(),
                                span: None,
                            })?;
                        let rest_val = slice_call.try_as_basic_value().left().ok_or_else(|| {
                            CodegenError::MissingValue {
                                what: "slice result".to_string(),
                                context: "array-rest capture".to_string(),
                                span: None,
                            }
                        })?;

                        self.builder
                            .build_store(rest_alloca, rest_val)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_store".to_string(),
                                details: format!("Failed to store array-rest binding '{}'", rest),
                                span: None,
                            })?;
                        self.builder
                            .build_unconditional_branch(merge_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_unconditional_branch".to_string(),
                                details: "Failed to branch from array-rest match block".to_string(),
                                span: None,
                            })?;
                        let match_end_bb = self.builder.get_insert_block().ok_or_else(|| {
                            CodegenError::LLVMError {
                                operation: "get_insert_block".to_string(),
                                details: "No insert block after array-rest match".to_string(),
                                span: None,
                            }
                        })?;

                        // fail_bb: length check or head sub-patterns failed — no rest allocated.
                        self.builder.position_at_end(fail_bb);
                        self.builder
                            .build_unconditional_branch(merge_bb)
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_unconditional_branch".to_string(),
                                details: "Failed to branch from array-rest fail block".to_string(),
                                span: None,
                            })?;

                        // merge_bb: PHI the final boolean result.
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(self.context.bool_type(), "ar_result")
                            .map_err(|_| CodegenError::LLVMError {
                                operation: "build_phi".to_string(),
                                details: "Failed to build PHI for array-rest pattern result"
                                    .to_string(),
                                span: None,
                            })?;
                        let true_val = self.context.bool_type().const_int(1, false);
                        let false_val = self.context.bool_type().const_int(0, false);
                        phi.add_incoming(&[(&true_val, match_end_bb), (&false_val, fail_bb)]);

                        Ok(phi.as_basic_value().into_int_value())
                    }
                    _ => Err(CodegenError::TypeError {
                        expected: "IntMatrix or Matrix".to_string(),
                        found: format!("{:?}", value_type),
                        context: "Array-rest pattern".to_string(),
                        span: None,
                    }),
                }
            }
        }
    }

    /// Helper: Apply a single sub-pattern in a destructure context.
    /// - Wildcard: always matches, no binding
    /// - Binding: allocate variable, store value, return combined unchanged
    /// - Anything else: compile recursively and AND with combined
    fn apply_sub_pattern(
        &mut self,
        sub_pat: &parser::ast::Pattern,
        extracted: inkwell::values::BasicValueEnum<'ctx>,
        ft: &BrixType,
        combined: inkwell::values::IntValue<'ctx>,
        i: usize,
    ) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        use parser::ast::Pattern;
        match sub_pat {
            Pattern::Wildcard => Ok(combined),
            Pattern::Binding(name) => {
                let llvm_type = self.brix_type_to_llvm(ft);
                let alloca = self.builder.build_alloca(llvm_type, name).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_alloca".to_string(),
                        details: format!("Failed to allocate binding '{}'", name),
                        span: None,
                    }
                })?;
                self.builder.build_store(alloca, extracted).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: format!("Failed to store binding '{}'", name),
                        span: None,
                    }
                })?;
                self.variables.insert(name.clone(), (alloca, ft.clone()));
                Ok(combined)
            }
            _ => {
                let field_match = self.compile_pattern_match(sub_pat, extracted, ft)?;
                Ok(self
                    .builder
                    .build_and(combined, field_match, &format!("da_{}", i))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_and".to_string(),
                        details: format!("Failed to AND sub-pattern at index {}", i),
                        span: None,
                    })?)
            }
        }
    }

    /// Apply a sub-pattern while reusing binding slots allocated in a dominating
    /// block. Array-rest head bindings need this because the element reads are
    /// gated behind a length check, but guards/bodies compiled later may still
    /// reference those names.
    fn apply_sub_pattern_with_prebound(
        &mut self,
        sub_pat: &parser::ast::Pattern,
        extracted: inkwell::values::BasicValueEnum<'ctx>,
        ft: &BrixType,
        combined: inkwell::values::IntValue<'ctx>,
        i: usize,
        prebound: &HashMap<String, (PointerValue<'ctx>, BrixType)>,
    ) -> CodegenResult<inkwell::values::IntValue<'ctx>> {
        use parser::ast::Pattern;
        match sub_pat {
            Pattern::Wildcard => Ok(combined),
            Pattern::Binding(name) => {
                let (alloca, _) =
                    prebound
                        .get(name)
                        .ok_or_else(|| CodegenError::UndefinedSymbol {
                            name: name.clone(),
                            context: "prebound array-rest pattern binding".to_string(),
                            span: None,
                        })?;
                self.builder.build_store(*alloca, extracted).map_err(|_| {
                    CodegenError::LLVMError {
                        operation: "build_store".to_string(),
                        details: format!("Failed to store prebound binding '{}'", name),
                        span: None,
                    }
                })?;
                self.variables.insert(name.clone(), (*alloca, ft.clone()));
                Ok(combined)
            }
            _ => {
                let field_match = self.compile_pattern_match(sub_pat, extracted, ft)?;
                Ok(self
                    .builder
                    .build_and(combined, field_match, &format!("da_{}", i))
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_and".to_string(),
                        details: format!("Failed to AND sub-pattern at index {}", i),
                        span: None,
                    })?)
            }
        }
    }
}
