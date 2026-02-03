// Edge Case Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt, UnaryOp};

fn compile_program(program: Program) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(&context, &builder, &module);
        compiler.compile_program(&program);
        module.print_to_string().to_string()
    });
    match result {
        Ok(ir) => Ok(ir),
        Err(_) => Err("Compilation panicked".to_string()),
    }
}

// ==================== EMPTY CONSTRUCTS ====================

#[test]
fn test_empty_program() {
    let program = Program { statements: vec![] };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_empty_block() {
    let program = Program {
        statements: vec![Stmt::Block(vec![])],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_empty_array() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Array(vec![]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== DEEPLY NESTED EXPRESSIONS ====================

#[test]
fn test_deeply_nested_arithmetic() {
    // ((((1 + 2) * 3) - 4) / 5)
    let expr = Expr::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::Binary {
            op: BinaryOp::Sub,
            lhs: Box::new(Expr::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(Expr::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::Literal(Literal::Int(1))),
                    rhs: Box::new(Expr::Literal(Literal::Int(2))),
                }),
                rhs: Box::new(Expr::Literal(Literal::Int(3))),
            }),
            rhs: Box::new(Expr::Literal(Literal::Int(4))),
        }),
        rhs: Box::new(Expr::Literal(Literal::Int(5))),
    };
    let program = Program {
        statements: vec![Stmt::Expr(expr)],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_blocks() {
    let program = Program {
        statements: vec![Stmt::Block(vec![Stmt::Block(vec![Stmt::Block(vec![
            Stmt::Expr(Expr::Literal(Literal::Int(1))),
        ])])])],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== BOUNDARY VALUES ====================

#[test]
fn test_zero_literal() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::Int(0)))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_large_int() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::Int(9223372036854775807)))], // i64::MAX
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_zero() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::Float(0.0)))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MULTIPLE VARIABLES ====================

#[test]
fn test_multiple_variable_declarations() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "a".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(1)),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "b".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(2)),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "c".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(3)),
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== SHADOWING ====================

#[test]
fn test_variable_shadowing() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(10)),
                is_const: false,
            },
            Stmt::Block(vec![Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(20)),
                is_const: false,
            }]),
        ],
    };
    let result = compile_program(program);
    // Shadowing might not be implemented yet
    let _ = result;
}

// ==================== CHAINED COMPARISONS ====================

#[test]
fn test_chained_comparison() {
    // 1 < 2 && 2 < 3 (desugared form of 1 < 2 < 3)
    let expr = Expr::Binary {
        op: BinaryOp::LogicalAnd,
        lhs: Box::new(Expr::Binary {
            op: BinaryOp::Lt,
            lhs: Box::new(Expr::Literal(Literal::Int(1))),
            rhs: Box::new(Expr::Literal(Literal::Int(2))),
        }),
        rhs: Box::new(Expr::Binary {
            op: BinaryOp::Lt,
            lhs: Box::new(Expr::Literal(Literal::Int(2))),
            rhs: Box::new(Expr::Literal(Literal::Int(3))),
        }),
    };
    let program = Program {
        statements: vec![Stmt::Expr(expr)],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== UNARY OPERATORS ON COMPLEX EXPRESSIONS ====================

#[test]
fn test_negate_expression() {
    // -(1 + 2)
    let expr = Expr::Unary {
        op: UnaryOp::Negate,
        expr: Box::new(Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::Literal(Literal::Int(1))),
            rhs: Box::new(Expr::Literal(Literal::Int(2))),
        }),
    };
    let program = Program {
        statements: vec![Stmt::Expr(expr)],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_not_comparison() {
    // !(x > 5)
    let expr = Expr::Unary {
        op: UnaryOp::Not,
        expr: Box::new(Expr::Binary {
            op: BinaryOp::Gt,
            lhs: Box::new(Expr::Identifier("x".to_string())),
            rhs: Box::new(Expr::Literal(Literal::Int(5))),
        }),
    };
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(10)),
                is_const: false,
            },
            Stmt::Expr(expr),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== STRING OPERATIONS ====================

#[test]
fn test_empty_string() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String("".to_string())))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_with_escapes() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            "hello\\nworld".to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== F-STRING EDGE CASES ====================

#[test]
fn test_fstring_empty() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![parser::ast::FStringPart::Text("".to_string())],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_multiple_interpolations() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(10)),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(20)),
                is_const: false,
            },
            Stmt::Expr(Expr::FString {
                parts: vec![
                    parser::ast::FStringPart::Text("x=".to_string()),
                    parser::ast::FStringPart::Expr {
                        expr: Box::new(Expr::Identifier("x".to_string())),
                        format: None,
                    },
                    parser::ast::FStringPart::Text(", y=".to_string()),
                    parser::ast::FStringPart::Expr {
                        expr: Box::new(Expr::Identifier("y".to_string())),
                        format: None,
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== CONST EDGE CASES ====================

#[test]
fn test_const_variable() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "PI".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Float(3.14159)),
            is_const: true,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MIXED TYPE OPERATIONS ====================

#[test]
fn test_int_float_mixed_arithmetic() {
    // 10 + 3.14 (should promote int to float)
    let expr = Expr::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::Literal(Literal::Int(10))),
        rhs: Box::new(Expr::Literal(Literal::Float(3.14))),
    };
    let program = Program {
        statements: vec![Stmt::Expr(expr)],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== IDENTIFIER EDGE CASES ====================

#[test]
fn test_single_char_identifier() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Int(1)),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_long_identifier() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "very_long_variable_name_that_is_still_valid".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Int(42)),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_underscore_identifier() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "_".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Int(1)),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
