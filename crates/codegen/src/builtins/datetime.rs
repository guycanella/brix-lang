// DateTime builtin module for Brix (v1.9 Grupo A)

use crate::{BrixType, CodegenError, CodegenResult, Compiler};
use inkwell::module::Linkage;
use inkwell::values::PointerValue;
use inkwell::AddressSpace;

pub trait DateTimeFunctions<'ctx> {
    fn declare_datetime_functions(&self);
    fn extract_datetime_ptr(
        &self,
        val: inkwell::values::BasicValueEnum<'ctx>,
        typ: &BrixType,
    ) -> CodegenResult<PointerValue<'ctx>>;
    fn compile_datetime_call(
        &self,
        fn_name: &str,
        args: &[inkwell::values::BasicValueEnum<'ctx>],
        arg_types: &[BrixType],
    ) -> CodegenResult<(inkwell::values::BasicValueEnum<'ctx>, BrixType)>;
}

impl<'a, 'ctx> DateTimeFunctions<'ctx> for Compiler<'a, 'ctx> {
    fn declare_datetime_functions(&self) {
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let i32_type = self.context.i32_type();
        let void_type = self.context.void_type();

        // datetime_now() -> ptr
        if self.module.get_function("datetime_now").is_none() {
            let fn_type = ptr_type.fn_type(&[], false);
            self.module
                .add_function("datetime_now", fn_type, Some(Linkage::External));
        }

        // datetime_today() -> ptr
        if self.module.get_function("datetime_today").is_none() {
            let fn_type = ptr_type.fn_type(&[], false);
            self.module
                .add_function("datetime_today", fn_type, Some(Linkage::External));
        }

        // datetime_timestamp(ptr) -> i64
        if self.module.get_function("datetime_timestamp").is_none() {
            let fn_type = i64_type.fn_type(&[ptr_type.into()], false);
            self.module
                .add_function("datetime_timestamp", fn_type, Some(Linkage::External));
        }

        // datetime_from_timestamp(i64) -> ptr
        if self
            .module
            .get_function("datetime_from_timestamp")
            .is_none()
        {
            let fn_type = ptr_type.fn_type(&[i64_type.into()], false);
            self.module
                .add_function("datetime_from_timestamp", fn_type, Some(Linkage::External));
        }

        // datetime_format(ptr, ptr) -> ptr
        if self.module.get_function("datetime_format").is_none() {
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
            self.module
                .add_function("datetime_format", fn_type, Some(Linkage::External));
        }

        // datetime_parse(ptr, ptr) -> ptr
        if self.module.get_function("datetime_parse").is_none() {
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
            self.module
                .add_function("datetime_parse", fn_type, Some(Linkage::External));
        }

        // datetime_add_days(ptr, i64) -> ptr
        if self.module.get_function("datetime_add_days").is_none() {
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
            self.module
                .add_function("datetime_add_days", fn_type, Some(Linkage::External));
        }

        // datetime_add_hours(ptr, i64) -> ptr
        if self.module.get_function("datetime_add_hours").is_none() {
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
            self.module
                .add_function("datetime_add_hours", fn_type, Some(Linkage::External));
        }

        // datetime_add_minutes(ptr, i64) -> ptr
        if self.module.get_function("datetime_add_minutes").is_none() {
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
            self.module
                .add_function("datetime_add_minutes", fn_type, Some(Linkage::External));
        }

        // datetime_add_seconds(ptr, i64) -> ptr
        if self.module.get_function("datetime_add_seconds").is_none() {
            let fn_type = ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false);
            self.module
                .add_function("datetime_add_seconds", fn_type, Some(Linkage::External));
        }

        // datetime_diff_seconds(ptr, ptr) -> i64
        if self.module.get_function("datetime_diff_seconds").is_none() {
            let fn_type = i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
            self.module
                .add_function("datetime_diff_seconds", fn_type, Some(Linkage::External));
        }

        // datetime_compare(ptr, ptr) -> i32
        if self.module.get_function("datetime_compare").is_none() {
            let fn_type = i32_type.fn_type(&[ptr_type.into(), ptr_type.into()], false);
            self.module
                .add_function("datetime_compare", fn_type, Some(Linkage::External));
        }

        // datetime_retain(ptr) -> ptr
        if self.module.get_function("datetime_retain").is_none() {
            let fn_type = ptr_type.fn_type(&[ptr_type.into()], false);
            self.module
                .add_function("datetime_retain", fn_type, Some(Linkage::External));
        }

        // datetime_release(ptr) -> void
        if self.module.get_function("datetime_release").is_none() {
            let fn_type = void_type.fn_type(&[ptr_type.into()], false);
            self.module
                .add_function("datetime_release", fn_type, Some(Linkage::External));
        }
    }

    fn extract_datetime_ptr(
        &self,
        val: inkwell::values::BasicValueEnum<'ctx>,
        typ: &BrixType,
    ) -> CodegenResult<PointerValue<'ctx>> {
        match typ {
            BrixType::DateTime => Ok(val.into_pointer_value()),
            BrixType::Union(types) if types.contains(&BrixType::DateTime) => {
                let s_val = val.into_struct_value();
                let ptr_val = self
                    .builder
                    .build_extract_value(s_val, 1, "union_dt_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_extract_value".to_string(),
                        details: "Failed to extract pointer from Union(DateTime, Nil)".to_string(),
                        span: None,
                    })?
                    .into_pointer_value();
                Ok(ptr_val)
            }
            _ => Err(CodegenError::TypeError {
                expected: "DateTime or Union containing DateTime".to_string(),
                found: format!("{:?}", typ),
                context: "DateTime argument extraction".to_string(),
                span: None,
            }),
        }
    }

    fn compile_datetime_call(
        &self,
        fn_name: &str,
        args: &[inkwell::values::BasicValueEnum<'ctx>],
        arg_types: &[BrixType],
    ) -> CodegenResult<(inkwell::values::BasicValueEnum<'ctx>, BrixType)> {
        self.declare_datetime_functions();

        match fn_name {
            "now" => {
                if !args.is_empty() {
                    return Err(CodegenError::InvalidOperation {
                        operation: "datetime.now()".to_string(),
                        reason: "takes no arguments".to_string(),
                        span: None,
                    });
                }
                let func = self.module.get_function("datetime_now").unwrap();
                let call = self
                    .builder
                    .build_call(func, &[], "datetime_now_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_now".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::DateTime))
            }
            "today" => {
                if !args.is_empty() {
                    return Err(CodegenError::InvalidOperation {
                        operation: "datetime.today()".to_string(),
                        reason: "takes no arguments".to_string(),
                        span: None,
                    });
                }
                let func = self.module.get_function("datetime_today").unwrap();
                let call = self
                    .builder
                    .build_call(func, &[], "datetime_today_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_today".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::DateTime))
            }
            "timestamp" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "DateTime".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "datetime.timestamp()".to_string(),
                        span: None,
                    });
                }
                let dt_ptr = self.extract_datetime_ptr(args[0], &arg_types[0])?;
                let func = self.module.get_function("datetime_timestamp").unwrap();
                let call = self
                    .builder
                    .build_call(func, &[dt_ptr.into()], "datetime_ts_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_timestamp".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::Int))
            }
            "from_timestamp" => {
                if args.len() != 1 || arg_types[0] != BrixType::Int {
                    return Err(CodegenError::TypeError {
                        expected: "Int".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "datetime.from_timestamp()".to_string(),
                        span: None,
                    });
                }
                let func = self.module.get_function("datetime_from_timestamp").unwrap();
                let call = self
                    .builder
                    .build_call(func, &[args[0].into()], "datetime_from_ts_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_from_timestamp".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::DateTime))
            }
            "format" => {
                if args.len() != 2 || arg_types[1] != BrixType::String {
                    return Err(CodegenError::TypeError {
                        expected: "(DateTime, String)".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "datetime.format()".to_string(),
                        span: None,
                    });
                }
                let dt_ptr = self.extract_datetime_ptr(args[0], &arg_types[0])?;
                let func = self.module.get_function("datetime_format").unwrap();
                let call = self
                    .builder
                    .build_call(func, &[dt_ptr.into(), args[1].into()], "datetime_fmt_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_format".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::String))
            }
            "parse" => {
                if args.len() != 2
                    || arg_types[0] != BrixType::String
                    || arg_types[1] != BrixType::String
                {
                    return Err(CodegenError::TypeError {
                        expected: "(String, String)".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "datetime.parse()".to_string(),
                        span: None,
                    });
                }
                let func = self.module.get_function("datetime_parse").unwrap();
                let call = self
                    .builder
                    .build_call(
                        func,
                        &[args[0].into(), args[1].into()],
                        "datetime_parse_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_parse".to_string(),
                        span: None,
                    })?;
                let ptr_val = call.try_as_basic_value().left().unwrap();

                // datetime.parse returns NULL on failure -> construct Tagged Struct for Union(DateTime, Nil)
                // Tag 0 = DateTime (non-null), Tag 1 = Nil (null)
                let union_type = BrixType::Union(vec![BrixType::DateTime, BrixType::Nil]);
                let is_null = self
                    .builder
                    .build_is_null(ptr_val.into_pointer_value(), "is_null_dt")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_is_null".to_string(),
                        details: "Failed in datetime.parse null check".to_string(),
                        span: None,
                    })?;

                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let non_null_bb = self.context.append_basic_block(current_fn, "parse_success");
                let null_bb = self.context.append_basic_block(current_fn, "parse_fail");
                let merge_bb = self.context.append_basic_block(current_fn, "parse_merge");

                self.builder
                    .build_conditional_branch(is_null, null_bb, non_null_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_conditional_branch".to_string(),
                        details: "Failed in datetime.parse branch".to_string(),
                        span: None,
                    })?;

                // Non-null block: tag = 0, ptr = ptr_val
                self.builder.position_at_end(non_null_bb);
                let tag_0 = self.context.i64_type().const_int(0, false);
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed in datetime.parse branch to merge".to_string(),
                        span: None,
                    })?;

                // Null block: tag = 1, ptr = 0 (null ptr)
                self.builder.position_at_end(null_bb);
                let tag_1 = self.context.i64_type().const_int(1, false);
                let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_unconditional_branch".to_string(),
                        details: "Failed in datetime.parse branch to merge".to_string(),
                        span: None,
                    })?;

                // Merge block: PHI nodes
                self.builder.position_at_end(merge_bb);
                let phi_tag = self
                    .builder
                    .build_phi(self.context.i64_type(), "union_tag")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_phi".to_string(),
                        details: "Failed in datetime.parse phi tag".to_string(),
                        span: None,
                    })?;
                phi_tag.add_incoming(&[(&tag_0, non_null_bb), (&tag_1, null_bb)]);

                let phi_val = self
                    .builder
                    .build_phi(self.context.ptr_type(AddressSpace::default()), "union_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_phi".to_string(),
                        details: "Failed in datetime.parse phi val".to_string(),
                        span: None,
                    })?;
                phi_val.add_incoming(&[(&ptr_val, non_null_bb), (&null_ptr, null_bb)]);

                // Build struct { i64 tag, ptr val }
                let struct_type = self.context.struct_type(
                    &[
                        self.context.i64_type().into(),
                        self.context.ptr_type(AddressSpace::default()).into(),
                    ],
                    false,
                );
                let mut struct_val = struct_type.get_undef();
                struct_val = self
                    .builder
                    .build_insert_value(struct_val, phi_tag.as_basic_value(), 0, "s_tag")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_insert_value".to_string(),
                        details: "Failed in datetime.parse insert tag".to_string(),
                        span: None,
                    })?
                    .into_struct_value();
                struct_val = self
                    .builder
                    .build_insert_value(struct_val, phi_val.as_basic_value(), 1, "s_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_insert_value".to_string(),
                        details: "Failed in datetime.parse insert val".to_string(),
                        span: None,
                    })?
                    .into_struct_value();

                Ok((struct_val.into(), union_type))
            }
            "add_days" => {
                if args.len() != 2 || arg_types[1] != BrixType::Int {
                    return Err(CodegenError::TypeError {
                        expected: "(DateTime, Int)".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "datetime.add_days()".to_string(),
                        span: None,
                    });
                }
                let dt_ptr = self.extract_datetime_ptr(args[0], &arg_types[0])?;
                let func = self.module.get_function("datetime_add_days").unwrap();
                let call = self
                    .builder
                    .build_call(
                        func,
                        &[dt_ptr.into(), args[1].into()],
                        "datetime_add_days_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_add_days".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::DateTime))
            }
            "add_hours" => {
                if args.len() != 2 || arg_types[1] != BrixType::Int {
                    return Err(CodegenError::TypeError {
                        expected: "(DateTime, Int)".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "datetime.add_hours()".to_string(),
                        span: None,
                    });
                }
                let dt_ptr = self.extract_datetime_ptr(args[0], &arg_types[0])?;
                let func = self.module.get_function("datetime_add_hours").unwrap();
                let call = self
                    .builder
                    .build_call(
                        func,
                        &[dt_ptr.into(), args[1].into()],
                        "datetime_add_hours_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_add_hours".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::DateTime))
            }
            "add_minutes" => {
                if args.len() != 2 || arg_types[1] != BrixType::Int {
                    return Err(CodegenError::TypeError {
                        expected: "(DateTime, Int)".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "datetime.add_minutes()".to_string(),
                        span: None,
                    });
                }
                let dt_ptr = self.extract_datetime_ptr(args[0], &arg_types[0])?;
                let func = self.module.get_function("datetime_add_minutes").unwrap();
                let call = self
                    .builder
                    .build_call(
                        func,
                        &[dt_ptr.into(), args[1].into()],
                        "datetime_add_mins_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_add_minutes".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::DateTime))
            }
            "add_seconds" => {
                if args.len() != 2 || arg_types[1] != BrixType::Int {
                    return Err(CodegenError::TypeError {
                        expected: "(DateTime, Int)".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "datetime.add_seconds()".to_string(),
                        span: None,
                    });
                }
                let dt_ptr = self.extract_datetime_ptr(args[0], &arg_types[0])?;
                let func = self.module.get_function("datetime_add_seconds").unwrap();
                let call = self
                    .builder
                    .build_call(
                        func,
                        &[dt_ptr.into(), args[1].into()],
                        "datetime_add_secs_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_add_seconds".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::DateTime))
            }
            "diff_seconds" => {
                if args.len() != 2 {
                    return Err(CodegenError::TypeError {
                        expected: "(DateTime, DateTime)".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "datetime.diff_seconds()".to_string(),
                        span: None,
                    });
                }
                let dt_ptr1 = self.extract_datetime_ptr(args[0], &arg_types[0])?;
                let dt_ptr2 = self.extract_datetime_ptr(args[1], &arg_types[1])?;
                let func = self.module.get_function("datetime_diff_seconds").unwrap();
                let call = self
                    .builder
                    .build_call(func, &[dt_ptr1.into(), dt_ptr2.into()], "datetime_diff_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call datetime_diff_seconds".to_string(),
                        span: None,
                    })?;
                let val = call.try_as_basic_value().left().unwrap();
                Ok((val, BrixType::Int))
            }
            _ => Err(CodegenError::UndefinedSymbol {
                name: fn_name.to_string(),
                context: "datetime call".to_string(),
                span: None,
            }),
        }
    }
}
