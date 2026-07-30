// JSON builtin module for Brix (v1.9 Grupo B)

use crate::{BrixType, CodegenError, CodegenResult, Compiler};
use inkwell::AddressSpace;
use inkwell::module::Linkage;
use inkwell::values::{BasicValueEnum, PointerValue};

pub trait JsonFunctions<'ctx> {
    fn declare_json_functions(&self);
    fn extract_json_ptr(
        &self,
        val: BasicValueEnum<'ctx>,
        typ: &BrixType,
    ) -> CodegenResult<PointerValue<'ctx>>;
    fn compile_json_call(
        &self,
        fn_name: &str,
        args: &[BasicValueEnum<'ctx>],
        arg_types: &[BrixType],
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)>;
}

impl<'a, 'ctx> JsonFunctions<'ctx> for Compiler<'a, 'ctx> {
    fn declare_json_functions(&self) {
        let ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let f64_type = self.context.f64_type();
        let void_type = self.context.void_type();

        let decls = [
            ("json_null", ptr_type.fn_type(&[], false)),
            ("json_bool", ptr_type.fn_type(&[i64_type.into()], false)),
            ("json_int", ptr_type.fn_type(&[i64_type.into()], false)),
            ("json_float", ptr_type.fn_type(&[f64_type.into()], false)),
            ("json_string", ptr_type.fn_type(&[ptr_type.into()], false)),
            ("json_array", ptr_type.fn_type(&[], false)),
            ("json_object", ptr_type.fn_type(&[], false)),
            (
                "json_get",
                ptr_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
            ),
            (
                "json_index",
                ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false),
            ),
            (
                "json_set",
                void_type.fn_type(&[ptr_type.into(), ptr_type.into(), ptr_type.into()], false),
            ),
            (
                "json_array_push",
                void_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
            ),
            (
                "json_array_len",
                i64_type.fn_type(&[ptr_type.into()], false),
            ),
            (
                "json_as_string",
                ptr_type.fn_type(&[ptr_type.into()], false),
            ),
            (
                "json_as_int",
                i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
            ),
            (
                "json_as_float",
                f64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
            ),
            (
                "json_as_bool",
                i64_type.fn_type(&[ptr_type.into(), ptr_type.into()], false),
            ),
            ("json_tag", i64_type.fn_type(&[ptr_type.into()], false)),
            ("json_parse", ptr_type.fn_type(&[ptr_type.into()], false)),
            (
                "json_stringify",
                ptr_type.fn_type(&[ptr_type.into()], false),
            ),
            (
                "json_stringify_pretty",
                ptr_type.fn_type(&[ptr_type.into(), i64_type.into()], false),
            ),
            ("json_retain", ptr_type.fn_type(&[ptr_type.into()], false)),
            ("json_release", void_type.fn_type(&[ptr_type.into()], false)),
        ];

        for (name, fn_type) in decls {
            if self.module.get_function(name).is_none() {
                self.module
                    .add_function(name, fn_type, Some(Linkage::External));
            }
        }
    }

    fn extract_json_ptr(
        &self,
        val: BasicValueEnum<'ctx>,
        typ: &BrixType,
    ) -> CodegenResult<PointerValue<'ctx>> {
        match typ {
            BrixType::Json => Ok(val.into_pointer_value()),
            BrixType::Union(types) if types.contains(&BrixType::Json) => {
                let sv = val.into_struct_value();
                let extracted = self
                    .builder
                    .build_extract_value(sv, 1, "extracted_json_ptr")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_extract_value".to_string(),
                        details: "Failed to extract pointer from Union(Json, Nil)".to_string(),
                        span: None,
                    })?;
                Ok(extracted.into_pointer_value())
            }
            _ => Err(CodegenError::TypeError {
                expected: "Json or Json?".to_string(),
                found: format!("{:?}", typ),
                context: "Extract Json pointer".to_string(),
                span: None,
            }),
        }
    }

    fn compile_json_call(
        &self,
        fn_name: &str,
        args: &[BasicValueEnum<'ctx>],
        arg_types: &[BrixType],
    ) -> CodegenResult<(BasicValueEnum<'ctx>, BrixType)> {
        self.declare_json_functions();
        let i64_type = self.context.i64_type();

        match fn_name {
            "null" => {
                if !args.is_empty() {
                    return Err(CodegenError::TypeError {
                        expected: "0 arguments".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.null() call".to_string(),
                        span: None,
                    });
                }
                let f = self.module.get_function("json_null").unwrap();
                let call = self
                    .builder
                    .build_call(f, &[], "json_null_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_null".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::Json))
            }

            "bool" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.bool() call".to_string(),
                        span: None,
                    });
                }
                let bool_val = args[0].into_int_value();
                let f = self.module.get_function("json_bool").unwrap();
                let call = self
                    .builder
                    .build_call(f, &[bool_val.into()], "json_bool_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_bool".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::Json))
            }

            "int" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.int() call".to_string(),
                        span: None,
                    });
                }
                let int_val = args[0].into_int_value();
                let f = self.module.get_function("json_int").unwrap();
                let call = self
                    .builder
                    .build_call(f, &[int_val.into()], "json_int_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_int".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::Json))
            }

            "float" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.float() call".to_string(),
                        span: None,
                    });
                }
                let float_val = args[0].into_float_value();
                let f = self.module.get_function("json_float").unwrap();
                let call = self
                    .builder
                    .build_call(f, &[float_val.into()], "json_float_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_float".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::Json))
            }

            "string" => {
                if args.len() != 1 || arg_types[0] != BrixType::String {
                    return Err(CodegenError::TypeError {
                        expected: "1 string argument".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "json.string() call".to_string(),
                        span: None,
                    });
                }
                let str_val = args[0].into_pointer_value();
                let f = self.module.get_function("json_string").unwrap();
                let call = self
                    .builder
                    .build_call(f, &[str_val.into()], "json_string_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_string".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::Json))
            }

            "array" => {
                let f = self.module.get_function("json_array").unwrap();
                let call = self
                    .builder
                    .build_call(f, &[], "json_array_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_array".to_string(),
                        span: None,
                    })?;
                let arr_val = call.try_as_basic_value().left().unwrap();
                Ok((arr_val, BrixType::Json))
            }

            "object" => {
                if !args.is_empty() {
                    return Err(CodegenError::TypeError {
                        expected: "0 arguments".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.object() call".to_string(),
                        span: None,
                    });
                }
                let f = self.module.get_function("json_object").unwrap();
                let call = self
                    .builder
                    .build_call(f, &[], "json_obj_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_object".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::Json))
            }

            "get" => {
                if args.len() != 2 || arg_types[1] != BrixType::String {
                    return Err(CodegenError::TypeError {
                        expected: "(Json, string) arguments".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "json.get() call".to_string(),
                        span: None,
                    });
                }
                let obj_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let key_ptr = args[1].into_pointer_value();

                let get_fn = self.module.get_function("json_get").unwrap();
                let res_ptr = self
                    .builder
                    .build_call(get_fn, &[obj_ptr.into(), key_ptr.into()], "json_get_ptr")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_get".to_string(),
                        span: None,
                    })?
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let ret_union_type = BrixType::Union(vec![BrixType::Json, BrixType::Nil]);
                let union_val = self.wrap_ptr_in_json_union(res_ptr)?;
                Ok((union_val, ret_union_type))
            }

            "index" => {
                if args.len() != 2 || arg_types[1] != BrixType::Int {
                    return Err(CodegenError::TypeError {
                        expected: "(Json, int) arguments".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "json.index() call".to_string(),
                        span: None,
                    });
                }
                let arr_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let idx_val = args[1].into_int_value();

                let idx_fn = self.module.get_function("json_index").unwrap();
                let res_ptr = self
                    .builder
                    .build_call(idx_fn, &[arr_ptr.into(), idx_val.into()], "json_idx_ptr")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_index".to_string(),
                        span: None,
                    })?
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let ret_union_type = BrixType::Union(vec![BrixType::Json, BrixType::Nil]);
                let union_val = self.wrap_ptr_in_json_union(res_ptr)?;
                Ok((union_val, ret_union_type))
            }

            "set" => {
                if args.len() != 3 || arg_types[1] != BrixType::String {
                    return Err(CodegenError::TypeError {
                        expected: "(Json, string, Json) arguments".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "json.set() call".to_string(),
                        span: None,
                    });
                }
                let obj_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let key_ptr = args[1].into_pointer_value();
                let val_ptr = self.extract_json_ptr(args[2], &arg_types[2])?;

                let set_fn = self.module.get_function("json_set").unwrap();
                self.builder
                    .build_call(
                        set_fn,
                        &[obj_ptr.into(), key_ptr.into(), val_ptr.into()],
                        "",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_set".to_string(),
                        span: None,
                    })?;
                Ok((i64_type.const_int(0, false).into(), BrixType::Void))
            }

            "array_push" => {
                if args.len() != 2 {
                    return Err(CodegenError::TypeError {
                        expected: "(Json, Json) arguments".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "json.array_push() call".to_string(),
                        span: None,
                    });
                }
                let arr_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let val_ptr = self.extract_json_ptr(args[1], &arg_types[1])?;

                let push_fn = self.module.get_function("json_array_push").unwrap();
                self.builder
                    .build_call(push_fn, &[arr_ptr.into(), val_ptr.into()], "")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_array_push".to_string(),
                        span: None,
                    })?;
                Ok((i64_type.const_int(0, false).into(), BrixType::Void))
            }

            "array_len" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.array_len() call".to_string(),
                        span: None,
                    });
                }
                let arr_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let len_fn = self.module.get_function("json_array_len").unwrap();
                let call = self
                    .builder
                    .build_call(len_fn, &[arr_ptr.into()], "json_len_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_array_len".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::Int))
            }

            "as_string" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.as_string() call".to_string(),
                        span: None,
                    });
                }
                let json_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let as_str_fn = self.module.get_function("json_as_string").unwrap();
                let res_ptr = self
                    .builder
                    .build_call(as_str_fn, &[json_ptr.into()], "json_str_ptr")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_as_string".to_string(),
                        span: None,
                    })?
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let ret_union_type = BrixType::Union(vec![BrixType::String, BrixType::Nil]);
                let union_val = self.wrap_ptr_in_json_union(res_ptr)?;
                Ok((union_val, ret_union_type))
            }

            "as_int" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.as_int() call".to_string(),
                        span: None,
                    });
                }
                let json_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let i32_type = self.context.i32_type();
                let ok_alloca = self
                    .builder
                    .build_alloca(i32_type, "json_as_int_ok")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_alloca".to_string(),
                        details: "Failed to allocate ok indicator for json.as_int".to_string(),
                        span: None,
                    })?;

                let as_int_fn = self.module.get_function("json_as_int").unwrap();
                let raw_val = self
                    .builder
                    .build_call(
                        as_int_fn,
                        &[json_ptr.into(), ok_alloca.into()],
                        "json_int_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_as_int".to_string(),
                        span: None,
                    })?
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let ok_val = self
                    .builder
                    .build_load(i32_type, ok_alloca, "json_int_ok_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load ok status".to_string(),
                        span: None,
                    })?
                    .into_int_value();

                let is_ok = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        ok_val,
                        i32_type.const_int(0, false),
                        "json_int_is_ok",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed ok status comparison".to_string(),
                        span: None,
                    })?;

                let union_val = self.wrap_value_in_union(raw_val, is_ok, BrixType::Int)?;
                let ret_union_type = BrixType::Union(vec![BrixType::Int, BrixType::Nil]);
                Ok((union_val, ret_union_type))
            }

            "as_float" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.as_float() call".to_string(),
                        span: None,
                    });
                }
                let json_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let i32_type = self.context.i32_type();
                let ok_alloca = self
                    .builder
                    .build_alloca(i32_type, "json_as_float_ok")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_alloca".to_string(),
                        details: "Failed to allocate ok indicator for json.as_float".to_string(),
                        span: None,
                    })?;

                let as_float_fn = self.module.get_function("json_as_float").unwrap();
                let raw_val = self
                    .builder
                    .build_call(
                        as_float_fn,
                        &[json_ptr.into(), ok_alloca.into()],
                        "json_float_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_as_float".to_string(),
                        span: None,
                    })?
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let ok_val = self
                    .builder
                    .build_load(i32_type, ok_alloca, "json_float_ok_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load ok status".to_string(),
                        span: None,
                    })?
                    .into_int_value();

                let is_ok = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        ok_val,
                        i32_type.const_int(0, false),
                        "json_float_is_ok",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed ok status comparison".to_string(),
                        span: None,
                    })?;

                let union_val = self.wrap_value_in_union(raw_val, is_ok, BrixType::Float)?;
                let ret_union_type = BrixType::Union(vec![BrixType::Float, BrixType::Nil]);
                Ok((union_val, ret_union_type))
            }

            "as_bool" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.as_bool() call".to_string(),
                        span: None,
                    });
                }
                let json_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let i32_type = self.context.i32_type();
                let ok_alloca = self
                    .builder
                    .build_alloca(i32_type, "json_as_bool_ok")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_alloca".to_string(),
                        details: "Failed to allocate ok indicator for json.as_bool".to_string(),
                        span: None,
                    })?;

                let as_bool_fn = self.module.get_function("json_as_bool").unwrap();
                let raw_val = self
                    .builder
                    .build_call(
                        as_bool_fn,
                        &[json_ptr.into(), ok_alloca.into()],
                        "json_bool_val",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_as_bool".to_string(),
                        span: None,
                    })?
                    .try_as_basic_value()
                    .left()
                    .unwrap();

                let ok_val = self
                    .builder
                    .build_load(i32_type, ok_alloca, "json_bool_ok_val")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_load".to_string(),
                        details: "Failed to load ok status".to_string(),
                        span: None,
                    })?
                    .into_int_value();

                let is_ok = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::NE,
                        ok_val,
                        i32_type.const_int(0, false),
                        "json_bool_is_ok",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_int_compare".to_string(),
                        details: "Failed ok status comparison".to_string(),
                        span: None,
                    })?;

                let union_val = self.wrap_value_in_union(raw_val, is_ok, BrixType::Int)?;
                let ret_union_type = BrixType::Union(vec![BrixType::Int, BrixType::Nil]);
                Ok((union_val, ret_union_type))
            }

            "tag" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.tag() call".to_string(),
                        span: None,
                    });
                }
                let json_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let tag_fn = self.module.get_function("json_tag").unwrap();
                let call = self
                    .builder
                    .build_call(tag_fn, &[json_ptr.into()], "json_tag_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_tag".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::Int))
            }

            "parse" => {
                if args.len() != 1 || arg_types[0] != BrixType::String {
                    return Err(CodegenError::TypeError {
                        expected: "1 string argument".to_string(),
                        found: format!("{:?}", arg_types),
                        context: "json.parse() call".to_string(),
                        span: None,
                    });
                }
                let str_ptr = args[0].into_pointer_value();
                let parse_fn = self.module.get_function("json_parse").unwrap();
                let res_ptr = self
                    .builder
                    .build_call(parse_fn, &[str_ptr.into()], "json_parse_ptr")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_parse".to_string(),
                        span: None,
                    })?
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_pointer_value();

                let ret_union_type = BrixType::Union(vec![BrixType::Json, BrixType::Nil]);
                let union_val = self.wrap_ptr_in_json_union(res_ptr)?;
                Ok((union_val, ret_union_type))
            }

            "stringify" => {
                if args.len() != 1 {
                    return Err(CodegenError::TypeError {
                        expected: "1 argument".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.stringify() call".to_string(),
                        span: None,
                    });
                }
                let json_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let str_fn = self.module.get_function("json_stringify").unwrap();
                let call = self
                    .builder
                    .build_call(str_fn, &[json_ptr.into()], "json_str_res")
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_stringify".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::String))
            }

            "stringify_pretty" => {
                if args.len() < 1 || args.len() > 2 {
                    return Err(CodegenError::TypeError {
                        expected: "1 or 2 arguments".to_string(),
                        found: format!("{} arguments", args.len()),
                        context: "json.stringify_pretty() call".to_string(),
                        span: None,
                    });
                }
                let json_ptr = self.extract_json_ptr(args[0], &arg_types[0])?;
                let indent_val = if args.len() == 2 {
                    args[1].into_int_value()
                } else {
                    i64_type.const_int(2, false)
                };

                let str_fn = self.module.get_function("json_stringify_pretty").unwrap();
                let call = self
                    .builder
                    .build_call(
                        str_fn,
                        &[json_ptr.into(), indent_val.into()],
                        "json_pretty_res",
                    )
                    .map_err(|_| CodegenError::LLVMError {
                        operation: "build_call".to_string(),
                        details: "Failed to call json_stringify_pretty".to_string(),
                        span: None,
                    })?;
                Ok((call.try_as_basic_value().left().unwrap(), BrixType::String))
            }

            _ => Err(CodegenError::UndefinedSymbol {
                name: format!("json.{}", fn_name),
                context: "json module call".to_string(),
                span: None,
            }),
        }
    }
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {
    fn wrap_ptr_in_json_union(
        &self,
        ptr_val: PointerValue<'ctx>,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let ret_union_type = BrixType::Union(vec![BrixType::Json, BrixType::Nil]);
        let union_llvm_type = self.brix_type_to_llvm(&ret_union_type);
        let struct_type = union_llvm_type.into_struct_type();
        let i64_type = self.context.i64_type();

        let is_null = self
            .builder
            .build_is_null(ptr_val, "json_is_null")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_is_null".to_string(),
                details: "Failed to check pointer for null in json union wrap".to_string(),
                span: None,
            })?;

        let parent_fn = self.current_function()?;
        let null_bb = self
            .context
            .append_basic_block(parent_fn, "json_union_null");
        let valid_bb = self
            .context
            .append_basic_block(parent_fn, "json_union_valid");
        let cont_bb = self
            .context
            .append_basic_block(parent_fn, "json_union_cont");

        self.builder
            .build_conditional_branch(is_null, null_bb, valid_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_conditional_branch".to_string(),
                details: "Failed json union branch".to_string(),
                span: None,
            })?;

        // Valid block (Tag 0 = Json)
        self.builder.position_at_end(valid_bb);
        let mut valid_union = struct_type.get_undef();
        valid_union = self
            .builder
            .build_insert_value(valid_union, i64_type.const_int(0, false), 0, "tag_json")
            .unwrap()
            .into_struct_value();
        valid_union = self
            .builder
            .build_insert_value(valid_union, ptr_val, 1, "val_json")
            .unwrap()
            .into_struct_value();
        self.builder.build_unconditional_branch(cont_bb).unwrap();

        // Null block (Tag 1 = Nil)
        self.builder.position_at_end(null_bb);
        let mut null_union = struct_type.get_undef();
        null_union = self
            .builder
            .build_insert_value(null_union, i64_type.const_int(1, false), 0, "tag_nil")
            .unwrap()
            .into_struct_value();
        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
        null_union = self
            .builder
            .build_insert_value(null_union, null_ptr, 1, "val_nil")
            .unwrap()
            .into_struct_value();
        self.builder.build_unconditional_branch(cont_bb).unwrap();

        // Cont block PHI
        self.builder.position_at_end(cont_bb);
        let phi = self
            .builder
            .build_phi(struct_type, "json_union_res")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_phi".to_string(),
                details: "Failed PHI for json union".to_string(),
                span: None,
            })?;
        phi.add_incoming(&[(&valid_union, valid_bb), (&null_union, null_bb)]);
        Ok(phi.as_basic_value())
    }

    fn wrap_value_in_union(
        &self,
        val: BasicValueEnum<'ctx>,
        is_ok: inkwell::values::IntValue<'ctx>,
        val_type: BrixType,
    ) -> CodegenResult<BasicValueEnum<'ctx>> {
        let ret_union_type = BrixType::Union(vec![val_type.clone(), BrixType::Nil]);
        let union_llvm_type = self.brix_type_to_llvm(&ret_union_type);
        let struct_type = union_llvm_type.into_struct_type();
        let i64_type = self.context.i64_type();

        let parent_fn = self.current_function()?;
        let ok_bb = self.context.append_basic_block(parent_fn, "extractor_ok");
        let err_bb = self.context.append_basic_block(parent_fn, "extractor_err");
        let cont_bb = self.context.append_basic_block(parent_fn, "extractor_cont");

        self.builder
            .build_conditional_branch(is_ok, ok_bb, err_bb)
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_conditional_branch".to_string(),
                details: "Failed extractor branch".to_string(),
                span: None,
            })?;

        // OK block (Tag 0)
        self.builder.position_at_end(ok_bb);
        let mut ok_union = struct_type.get_undef();
        ok_union = self
            .builder
            .build_insert_value(ok_union, i64_type.const_int(0, false), 0, "tag_val")
            .unwrap()
            .into_struct_value();

        let val_in_union: BasicValueEnum = if val.is_int_value() {
            self.builder
                .build_int_s_extend_or_bit_cast(val.into_int_value(), i64_type, "i64_ext")
                .unwrap()
                .into()
        } else {
            val
        };

        ok_union = self
            .builder
            .build_insert_value(ok_union, val_in_union, 1, "val_ok")
            .unwrap()
            .into_struct_value();
        self.builder.build_unconditional_branch(cont_bb).unwrap();

        // Err block (Tag 1 = Nil)
        self.builder.position_at_end(err_bb);
        let mut err_union = struct_type.get_undef();
        err_union = self
            .builder
            .build_insert_value(err_union, i64_type.const_int(1, false), 0, "tag_err")
            .unwrap()
            .into_struct_value();

        let nil_val: BasicValueEnum = match val_type {
            BrixType::Int => i64_type.const_int(0, false).into(),
            BrixType::Float => self.context.f64_type().const_float(0.0).into(),
            _ => self
                .context
                .ptr_type(AddressSpace::default())
                .const_null()
                .into(),
        };

        err_union = self
            .builder
            .build_insert_value(err_union, nil_val, 1, "val_err")
            .unwrap()
            .into_struct_value();
        self.builder.build_unconditional_branch(cont_bb).unwrap();

        // Cont block PHI
        self.builder.position_at_end(cont_bb);
        let phi = self
            .builder
            .build_phi(struct_type, "extractor_union_res")
            .map_err(|_| CodegenError::LLVMError {
                operation: "build_phi".to_string(),
                details: "Failed PHI for extractor union".to_string(),
                span: None,
            })?;
        phi.add_incoming(&[(&ok_union, ok_bb), (&err_union, err_bb)]);
        Ok(phi.as_basic_value())
    }
}
