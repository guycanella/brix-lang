// Struct Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Expr, ExprKind, Literal, MethodDef, Program, Stmt, StmtKind, StructDef};

// Helper macros for creating AST nodes with dummy spans
macro_rules! lit_int {
    ($val:expr) => {
        Expr::dummy(ExprKind::Literal(Literal::Int($val)))
    };
}

macro_rules! lit_float {
    ($val:expr) => {
        Expr::dummy(ExprKind::Literal(Literal::Float($val)))
    };
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

fn compile_program(program: Program) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());
        match compiler.compile_program(&program) {
            Ok(_) => Ok(module.print_to_string().to_string()),
            Err(e) => Err(format!("Compilation error: {}", e)),
        }
    });
    match result {
        Ok(Ok(ir)) => Ok(ir),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Compilation panicked".to_string()),
    }
}

#[test]
fn test_struct_definition_simple() {
    // Test that struct definition compiles without errors
    // Note: LLVM doesn't include unused types in IR, so we just check compilation succeeds
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::StructDef(StructDef {
            name: "Point".to_string(),
            fields: vec![
                ("x".to_string(), "int".to_string(), None),
                ("y".to_string(), "int".to_string(), None),
            ],
        }))],
    };

    let result = compile_program(program);
    assert!(result.is_ok(), "Struct definition should compile without errors");
}

#[test]
fn test_struct_definition_with_defaults() {
    // Test that struct definition with defaults compiles without errors
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::StructDef(StructDef {
            name: "Config".to_string(),
            fields: vec![
                ("timeout".to_string(), "int".to_string(), Some(lit_int!(30))),
                ("retries".to_string(), "int".to_string(), Some(lit_int!(3))),
            ],
        }))],
    };

    let result = compile_program(program);
    assert!(result.is_ok(), "Struct definition with defaults should compile without errors");
}

#[test]
fn test_struct_init_all_fields() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            var_decl!(
                "p",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point".to_string(),
                    fields: vec![
                        ("x".to_string(), lit_int!(10)),
                        ("y".to_string(), lit_int!(20)),
                    ],
                })
            ),
        ],
    };

    let result = compile_program(program);
    println!("=== STRUCT INIT IR ===");
    match &result {
        Ok(ir) => println!("{}", ir),
        Err(e) => println!("ERROR: {}", e),
    }
    println!("======================");
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Check that struct was allocated
    assert!(ir.contains("alloca %Point"));

    // Check that struct value was stored (using store instruction, not GEP for initialization)
    assert!(ir.contains("store %Point"));
}

#[test]
fn test_struct_init_with_defaults() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Config".to_string(),
                fields: vec![
                    ("timeout".to_string(), "int".to_string(), Some(lit_int!(30))),
                    ("retries".to_string(), "int".to_string(), Some(lit_int!(3))),
                ],
            })),
            var_decl!(
                "cfg",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Config".to_string(),
                    fields: vec![
                        // Only specify timeout, retries should use default
                        ("timeout".to_string(), lit_int!(60)),
                    ],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Check that struct was allocated
    assert!(ir.contains("alloca %Config"));

    // Check that struct value was stored with both fields (one provided, one default)
    assert!(ir.contains("store %Config"));
}

#[test]
fn test_field_access() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            var_decl!(
                "p",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point".to_string(),
                    fields: vec![
                        ("x".to_string(), lit_int!(10)),
                        ("y".to_string(), lit_int!(20)),
                    ],
                })
            ),
            var_decl!(
                "x_val",
                Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(ident!("p")),
                    field: "x".to_string(),
                })
            ),
        ],
    };

    let result = compile_program(program);
    println!("===== FIELD ACCESS IR =====");
    match &result {
        Ok(ir) => println!("{}", ir),
        Err(e) => println!("ERROR: {}", e),
    }
    println!("===========================");
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Check that field was accessed
    assert!(ir.contains("getelementptr inbounds %Point") || ir.contains("getelementptr"));
    assert!(ir.contains("load i64"));
}

#[test]
fn test_method_definition() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::MethodDef(MethodDef {
                receiver_name: "p".to_string(),
                receiver_type: "Point".to_string(),
                method_name: "get_x".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("p")),
                        field: "x".to_string(),
                    })],
                })),
            })),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Check that method was compiled with mangled name
    assert!(ir.contains("define i64 @Point_get_x(%Point*"));
}

#[test]
fn test_method_call() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::MethodDef(MethodDef {
                receiver_name: "p".to_string(),
                receiver_type: "Point".to_string(),
                method_name: "get_x".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("p")),
                        field: "x".to_string(),
                    })],
                })),
            })),
            var_decl!(
                "p",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point".to_string(),
                    fields: vec![
                        ("x".to_string(), lit_int!(10)),
                        ("y".to_string(), lit_int!(20)),
                    ],
                })
            ),
            var_decl!(
                "x_val",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("p")),
                        field: "get_x".to_string(),
                    })),
                    args: vec![],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Check that method was called
    assert!(ir.contains("call i64 @Point_get_x(%Point*"));
}

#[test]
fn test_method_with_parameters() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::MethodDef(MethodDef {
                receiver_name: "p".to_string(),
                receiver_type: "Point".to_string(),
                method_name: "add".to_string(),
                params: vec![("dx".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Binary {
                        op: parser::ast::BinaryOp::Add,
                        lhs: Box::new(Expr::dummy(ExprKind::FieldAccess {
                            target: Box::new(ident!("p")),
                            field: "x".to_string(),
                        })),
                        rhs: Box::new(ident!("dx")),
                    })],
                })),
            })),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Check that method accepts both receiver and parameter
    assert!(ir.contains("define i64 @Point_add(%Point*"));
    assert!(ir.contains("i64 %dx"));
}

#[test]
fn test_multiple_structs() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point".to_string(),
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Circle".to_string(),
                fields: vec![
                    ("radius".to_string(), "int".to_string(), None),
                ],
            })),
            var_decl!(
                "p",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point".to_string(),
                    fields: vec![
                        ("x".to_string(), lit_int!(10)),
                        ("y".to_string(), lit_int!(20)),
                    ],
                })
            ),
            var_decl!(
                "c",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Circle".to_string(),
                    fields: vec![
                        ("radius".to_string(), lit_int!(5)),
                    ],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Check that both struct types were created
    assert!(ir.contains("%Point = type"));
    assert!(ir.contains("%Circle = type"));
}

#[test]
fn test_struct_with_mixed_types() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Person".to_string(),
                fields: vec![
                    ("age".to_string(), "int".to_string(), None),
                    ("height".to_string(), "float".to_string(), None),
                ],
            })),
            var_decl!(
                "person",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Person".to_string(),
                    fields: vec![
                        ("age".to_string(), lit_int!(30)),
                        ("height".to_string(), lit_float!(1.75)),
                    ],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Check that struct has mixed types
    assert!(ir.contains("%Person = type"));
    // Should contain both i64 and double
    assert!(ir.contains("i64"));
    assert!(ir.contains("double"));
}
