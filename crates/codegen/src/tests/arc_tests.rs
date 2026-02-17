// ARC (Automatic Reference Counting) tests for heap types
// Tests Swift-like ARC for String, Matrix, IntMatrix, ComplexMatrix

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Expr, ExprKind, Literal, Program, Stmt, StmtKind, BinaryOp};

#[test]
fn test_string_arc_basic() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program: var s := "hello"
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "s".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
            is_const: false,
        })],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "String ARC basic test failed");

    // Note: str_new() already returns with ref_count=1, so no retain needed on var decl
    // The string is created and ownership is transferred to the variable
}

#[test]
fn test_string_arc_reassignment() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program:
    // var s := "hello"
    // s := "world"
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "s".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("s".to_string())),
                value: Expr::dummy(ExprKind::Literal(Literal::String("world".to_string()))),
            }),
        ],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "String ARC reassignment test failed");

    // Verify that both retain and release are declared
    assert!(module.get_function("string_retain").is_some(), "string_retain not declared");
    assert!(module.get_function("string_release").is_some(), "string_release not declared");
}

#[test]
fn test_string_arc_copy() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program:
    // var s1 := "hello"
    // var s2 := s1
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "s1".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "s2".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Identifier("s1".to_string())),
                is_const: false,
            }),
        ],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "String ARC copy test failed");

    // Note: Both s1 and s2 own their reference (ownership transfer from constructor)
}

#[test]
fn test_matrix_arc_basic() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program: var m := [1.0, 2.0, 3.0]
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "m".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
            ])),
            is_const: false,
        })],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "Matrix ARC basic test failed");

    // matrix_new() returns with ref_count=1, ownership transferred
}

#[test]
fn test_matrix_arc_reassignment() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program:
    // var m := [1.0, 2.0]
    // m := [3.0, 4.0]
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("m".to_string())),
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(4.0))),
                ])),
            }),
        ],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "Matrix ARC reassignment test failed");

    // Verify that both retain and release are declared
    assert!(module.get_function("matrix_retain").is_some(), "matrix_retain not declared");
    assert!(module.get_function("matrix_release").is_some(), "matrix_release not declared");
}

#[test]
fn test_intmatrix_arc_basic() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program: var m := [1, 2, 3]
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "m".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ])),
            is_const: false,
        })],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "IntMatrix ARC basic test failed");

    // intmatrix_new() returns with ref_count=1, ownership transferred
}

#[test]
fn test_intmatrix_arc_reassignment() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program:
    // var m := [1, 2]
    // m := [3, 4]
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("m".to_string())),
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                ])),
            }),
        ],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "IntMatrix ARC reassignment test failed");

    // Verify that both retain and release are declared
    assert!(module.get_function("intmatrix_retain").is_some(), "intmatrix_retain not declared");
    assert!(module.get_function("intmatrix_release").is_some(), "intmatrix_release not declared");
}

#[test]
fn test_mixed_arc_types() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program:
    // var s := "hello"
    // var m := [1.0, 2.0]
    // var im := [1, 2]
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "s".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "im".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ])),
                is_const: false,
            }),
        ],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "Mixed ARC types test failed");

    // All types created with ref_count=1 via constructors
}

#[test]
fn test_no_arc_for_primitives() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program:
    // var x := 42
    // var y := 3.14
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
                is_const: false,
            }),
        ],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "No ARC for primitives test failed");

    // Verify NO retain functions are declared (primitives don't need ARC)
    assert!(module.get_function("string_retain").is_none(), "string_retain should not be declared for primitives");
    assert!(module.get_function("matrix_retain").is_none(), "matrix_retain should not be declared for primitives");
}

#[test]
fn test_string_concat_arc() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

    // Program:
    // var s1 := "hello"
    // var s2 := "world"
    // var s3 := s1 + s2
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "s1".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "s2".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::String("world".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "s3".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("s1".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Identifier("s2".to_string()))),
                }),
                is_const: false,
            }),
        ],
    };

    let result = compiler.compile_program(&program);
    assert!(result.is_ok(), "String concat ARC test failed");

    // str_concat creates a new string with ref_count=1
    // s3 owns it (ownership transfer)
}
