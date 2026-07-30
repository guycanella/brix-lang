// Unit tests for JSON builtin module (v1.9 Grupo B)

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Expr, ExprKind, Literal, Program, Stmt, StmtKind};

fn compile_program(program: Program) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(
            &context,
            &builder,
            &module,
            "test.bx".to_string(),
            "".to_string(),
        );
        match compiler.compile_program(&program) {
            Ok(_) => Ok(module.print_to_string().to_string()),
            Err(e) => Err(format!("{:?}", e)),
        }
    });
    match result {
        Ok(res) => res,
        Err(_) => Err("Compilation panicked".to_string()),
    }
}

macro_rules! lit_str {
    ($val:expr) => {
        Expr::dummy(ExprKind::Literal(Literal::String($val.to_string())))
    };
}

macro_rules! lit_int {
    ($val:expr) => {
        Expr::dummy(ExprKind::Literal(Literal::Int($val)))
    };
}

macro_rules! ident {
    ($name:expr) => {
        Expr::dummy(ExprKind::Identifier($name.to_string()))
    };
}

macro_rules! import_stmt {
    ($module:expr) => {
        Stmt::dummy(StmtKind::Import {
            module: $module.to_string(),
            alias: None,
        })
    };
}

macro_rules! var_decl {
    ($name:expr, $value:expr) => {
        Stmt::dummy(StmtKind::VariableDecl {
            name: $name.to_string(),
            type_hint: None,
            value: $value,
            is_const: false,
        })
    };
}

macro_rules! json_call {
    ($fn_name:expr) => {
        Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(ident!("json")),
                field: $fn_name.to_string(),
            })),
            args: vec![],
        })
    };
    ($fn_name:expr, $($arg:expr),+) => {
        Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(ident!("json")),
                field: $fn_name.to_string(),
            })),
            args: vec![$($arg),+],
        })
    };
}

#[test]
fn test_json_null_object_array_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("json"),
            var_decl!("nil_val", json_call!("null")),
            var_decl!("obj", json_call!("object")),
            var_decl!("arr", json_call!("array")),
        ],
    };

    let ir = compile_program(program).expect("Compilation failed");
    assert!(ir.contains("declare ptr @json_null()"));
    assert!(ir.contains("declare ptr @json_object()"));
    assert!(ir.contains("declare ptr @json_array()"));
    assert!(ir.contains("call ptr @json_null()"));
    assert!(ir.contains("call ptr @json_object()"));
    assert!(ir.contains("call ptr @json_array()"));
}

#[test]
fn test_json_parse_returns_union_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("json"),
            var_decl!("parsed", json_call!("parse", lit_str!("{\"a\": 10}"))),
        ],
    };

    let ir = compile_program(program).expect("Compilation failed");
    assert!(ir.contains("declare ptr @json_parse(ptr)"));
    assert!(ir.contains("call ptr @json_parse("));
    assert!(ir.contains("json_union_cont"));
}

#[test]
fn test_json_index_get_union_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("json"),
            var_decl!("obj", json_call!("object")),
            var_decl!(
                "val",
                Expr::dummy(ExprKind::Index {
                    array: Box::new(ident!("obj")),
                    indices: vec![lit_str!("key")],
                })
            ),
        ],
    };

    let ir = compile_program(program).expect("Compilation failed");
    assert!(ir.contains("declare ptr @json_get(ptr, ptr)"));
    assert!(ir.contains("call ptr @json_get("));
}

#[test]
fn test_json_extractors_union_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("json"),
            var_decl!("obj", json_call!("int", lit_int!(42))),
            var_decl!("num", json_call!("as_int", ident!("obj"))),
        ],
    };

    let ir = compile_program(program).expect("Compilation failed");
    assert!(ir.contains("declare i64 @json_as_int(ptr, ptr)"));
    assert!(ir.contains("call i64 @json_as_int("));
    assert!(ir.contains("extractor_cont"));
}

#[test]
fn test_json_value_to_string_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("json"),
            var_decl!("obj", json_call!("object")),
            Stmt::dummy(StmtKind::Println {
                expr: ident!("obj"),
            }),
        ],
    };

    let ir = compile_program(program).expect("Compilation failed");
    assert!(ir.contains("declare ptr @json_stringify(ptr)"));
    assert!(ir.contains("call ptr @json_stringify("));
}

#[test]
fn test_json_arc_copy_assign_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("json"),
            var_decl!("obj1", json_call!("object")),
            var_decl!("obj2", ident!("obj1")),
        ],
    };

    let ir = compile_program(program).expect("Compilation failed");
    assert!(ir.contains("declare ptr @json_retain(ptr)"));
    assert!(ir.contains("declare void @json_release(ptr)"));
    assert!(ir.contains("call ptr @json_retain("));
    assert!(ir.contains("call void @json_release("));
}

#[test]
fn test_json_union_arc_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("json"),
            var_decl!("opt1", json_call!("parse", lit_str!("123"))),
            var_decl!("opt2", ident!("opt1")),
        ],
    };

    let ir = compile_program(program).expect("Compilation failed");
    assert!(ir.contains("call ptr @json_retain("));
    assert!(ir.contains("call void @json_release("));
}

#[test]
fn test_json_invalid_arity_error() {
    let program = Program {
        statements: vec![
            import_stmt!("json"),
            Stmt::dummy(StmtKind::Expr(json_call!("null", lit_int!(1)))),
        ],
    };

    let res = compile_program(program);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("0 arguments"));
}

#[test]
fn test_string_matrix_index_arc_retain_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("json"),
            var_decl!("json_obj", json_call!("object")),
            var_decl!(
                "json_val",
                Expr::dummy(ExprKind::Index {
                    array: Box::new(ident!("json_obj")),
                    indices: vec![lit_str!("k")],
                })
            ),
        ],
    };

    let ir = compile_program(program).expect("Compilation failed");
    // json_get returns an owned JsonValue* with ref_count=1, so no additional json_retain call on assignment
    assert!(ir.contains("call ptr @json_get("));
}
