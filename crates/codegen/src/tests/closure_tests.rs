// Closure Tests - v1.3 Type System Expansion
// Tests for closures with capture by reference, heap allocation, and ARC

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Closure, Expr, ExprKind, Literal, Program, Stmt, StmtKind, BinaryOp};

/// Helper to compile a program and check for errors
fn compile_program(stmts: Vec<Stmt>) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(
            &context,
            &builder,
            &module,
            "test.bx".to_string(),
            "test source".to_string(),
        );

        let program = Program { statements: stmts };
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

// ==========================================
// SECTION 1: BASIC CLOSURE TESTS
// ==========================================

#[test]
fn test_closure_no_capture() {
    // var double := (x: int) -> int { return x * 2 }
    let closure = Closure {
        params: vec![("x".to_string(), "int".to_string())],
        return_type: Some("int".to_string()),
        body: Box::new(Stmt::dummy(StmtKind::Return {
            values: vec![Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
            })],
        })),
        captured_vars: vec![],
    };

    let stmts = vec![Stmt::dummy(StmtKind::VariableDecl {
        name: "double".to_string(),
        type_hint: None,
        value: Expr::dummy(ExprKind::Closure(closure)),
        is_const: false,
    })];

    assert!(compile_program(stmts).is_ok(), "Closure with no capture should compile");
}

#[test]
fn test_closure_single_capture() {
    // var offset := 10
    // var add_offset := (x: int) -> int { return x + offset }
    let stmts = vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "offset".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "add_offset".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Closure(Closure {
                params: vec![("x".to_string(), "int".to_string())],
                return_type: Some("int".to_string()),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Binary {
                        op: BinaryOp::Add,
                        lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                        rhs: Box::new(Expr::dummy(ExprKind::Identifier("offset".to_string()))),
                    })],
                })),
                captured_vars: vec!["offset".to_string()],
            })),
            is_const: false,
        }),
    ];

    assert!(compile_program(stmts).is_ok(), "Closure with single capture should compile");
}

#[test]
fn test_closure_call_no_args() {
    // var get_42 := () -> int { return 42 }
    // var result := get_42()
    let stmts = vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "get_42".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Closure(Closure {
                params: vec![],
                return_type: Some("int".to_string()),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
                })),
                captured_vars: vec![],
            })),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "result".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("get_42".to_string()))),
                args: vec![],
            }),
            is_const: false,
        }),
    ];

    assert!(compile_program(stmts).is_ok(), "Closure call with no args should compile");
}

#[test]
fn test_closure_assignment_arc() {
    // var fn1 := (x: int) -> int { return x * 2 }
    // var fn2 := fn1  // Should call retain
    let closure1 = Closure {
        params: vec![("x".to_string(), "int".to_string())],
        return_type: Some("int".to_string()),
        body: Box::new(Stmt::dummy(StmtKind::Return {
            values: vec![Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
            })],
        })),
        captured_vars: vec![],
    };

    let stmts = vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "fn1".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Closure(closure1)),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "fn2".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Identifier("fn1".to_string())),
            is_const: false,
        }),
    ];

    assert!(compile_program(stmts).is_ok(), "Closure assignment with ARC should compile");
}

#[test]
fn test_closure_many_captures() {
    // Test closure with 5 captured variables
    let mut stmts = vec![];
    let mut captured_vars = vec![];

    // Declare 5 variables
    for i in 0..5 {
        let var_name = format!("var{}", i);
        stmts.push(Stmt::dummy(StmtKind::VariableDecl {
            name: var_name.clone(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(i as i64))),
            is_const: false,
        }));
        captured_vars.push(var_name);
    }

    // Create closure that uses all captured variables: var0 + var1 + ... + var4
    let mut sum_expr = Expr::dummy(ExprKind::Identifier("var0".to_string()));
    for i in 1..5 {
        sum_expr = Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(sum_expr),
            rhs: Box::new(Expr::dummy(ExprKind::Identifier(format!("var{}", i)))),
        });
    }

    stmts.push(Stmt::dummy(StmtKind::VariableDecl {
        name: "closure".to_string(),
        type_hint: None,
        value: Expr::dummy(ExprKind::Closure(Closure {
            params: vec![],
            return_type: Some("int".to_string()),
            body: Box::new(Stmt::dummy(StmtKind::Return {
                values: vec![sum_expr],
            })),
            captured_vars,
        })),
        is_const: false,
    }));

    assert!(compile_program(stmts).is_ok(), "Closure with 5 captures should compile");
}
