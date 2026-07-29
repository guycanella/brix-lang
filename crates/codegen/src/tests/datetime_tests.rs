// Unit tests for DateTime builtin module (v1.9 Grupo A)

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, ExprKind, Literal, Program, Stmt, StmtKind};

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

#[test]
fn test_datetime_now_today_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("datetime"),
            var_decl!(
                "now",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("datetime")),
                        field: "now".to_string(),
                    })),
                    args: vec![],
                })
            ),
        ],
    };

    let ir = compile_program(program).unwrap();
    assert!(ir.contains("declare ptr @datetime_now()"));
    assert!(ir.contains("call ptr @datetime_now()"));
    assert!(ir.contains("call void @datetime_release(ptr"));
}

#[test]
fn test_datetime_parse_union_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("datetime"),
            var_decl!(
                "dt_opt",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("datetime")),
                        field: "parse".to_string(),
                    })),
                    args: vec![lit_str!("2026-07-29"), lit_str!("YYYY-MM-DD")],
                })
            ),
        ],
    };

    let ir = compile_program(program).unwrap();
    assert!(ir.contains("declare ptr @datetime_parse(ptr, ptr)"));
    assert!(ir.contains("call ptr @datetime_parse"));
    assert!(ir.contains("parse_success"));
    assert!(ir.contains("parse_fail"));
}

#[test]
fn test_datetime_format_println_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("datetime"),
            var_decl!(
                "now",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("datetime")),
                        field: "now".to_string(),
                    })),
                    args: vec![],
                })
            ),
            Stmt::dummy(StmtKind::Println {
                expr: ident!("now"),
            }),
        ],
    };

    let ir = compile_program(program).unwrap();
    assert!(ir.contains("declare ptr @datetime_format(ptr, ptr)"));
    assert!(ir.contains("call ptr @datetime_format"));
    assert!(ir.contains("call void @string_release(ptr"));
}

#[test]
fn test_datetime_type_error_arity() {
    let program = Program {
        statements: vec![
            import_stmt!("datetime"),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(ident!("datetime")),
                    field: "now".to_string(),
                })),
                args: vec![lit_str!("extra_arg")],
            }))),
        ],
    };

    let err = compile_program(program).unwrap_err();
    assert!(err.contains("takes no arguments"));
}

#[test]
fn test_datetime_comparison_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("datetime"),
            var_decl!(
                "d1",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("datetime")),
                        field: "now".to_string(),
                    })),
                    args: vec![],
                })
            ),
            var_decl!(
                "d2",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("datetime")),
                        field: "today".to_string(),
                    })),
                    args: vec![],
                })
            ),
            var_decl!(
                "cmp",
                Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Lt,
                    lhs: Box::new(ident!("d1")),
                    rhs: Box::new(ident!("d2")),
                })
            ),
        ],
    };

    let ir = compile_program(program).unwrap();
    assert!(ir.contains("declare i32 @datetime_compare(ptr, ptr)"));
    assert!(ir.contains("call i32 @datetime_compare"));
}

#[test]
fn test_datetime_union_copy_retain_release_ir() {
    let program = Program {
        statements: vec![
            import_stmt!("datetime"),
            var_decl!(
                "dt_opt",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("datetime")),
                        field: "parse".to_string(),
                    })),
                    args: vec![lit_str!("2026-07-29"), lit_str!("YYYY-MM-DD")],
                })
            ),
            var_decl!("copy_opt", ident!("dt_opt")),
        ],
    };

    let ir = compile_program(program).unwrap();
    assert!(ir.contains("union_ret_match"));
    assert!(ir.contains("call ptr @datetime_retain"));
}

#[test]
fn test_datetime_union_assignment_copies_tagged_value() {
    let parse_call = || {
        Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(ident!("datetime")),
                field: "parse".to_string(),
            })),
            args: vec![lit_str!("2026-07-29"), lit_str!("YYYY-MM-DD")],
        })
    };
    let program = Program {
        statements: vec![
            import_stmt!("datetime"),
            var_decl!("source", parse_call()),
            var_decl!("target", parse_call()),
            Stmt::dummy(StmtKind::Assignment {
                target: ident!("target"),
                value: ident!("source"),
            }),
        ],
    };

    let ir = compile_program(program).expect("DateTime? = DateTime? should compile");
    assert!(ir.contains("union_ret_match"));
    assert!(ir.contains("store { i64, ptr }"));
}
