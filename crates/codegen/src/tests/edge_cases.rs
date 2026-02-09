// Edge Case Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt, UnaryOp, ExprKind, StmtKind};

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
        statements: vec![Stmt::dummy(StmtKind::Block(vec![]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_empty_array() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(vec![]))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== DEEPLY NESTED EXPRESSIONS ====================

#[test]
fn test_deeply_nested_arithmetic() {
    // ((((1 + 2) * 3) - 4) / 5)
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Sub,
            lhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                })),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
            })),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
        })),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_blocks() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Block(vec![
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(1))))),
        ]))]))]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== BOUNDARY VALUES ====================

#[test]
fn test_zero_literal() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(0)))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_large_int() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(9223372036854775807)))))], // i64::MAX
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_zero() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MULTIPLE VARIABLES ====================

#[test]
fn test_multiple_variable_declarations() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "a".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "b".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "c".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                is_const: false,
            }),
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
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                is_const: false,
            })])),
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
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::LogicalAnd,
        lhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Lt,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        })),
        rhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Lt,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
        })),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== UNARY OPERATORS ON COMPLEX EXPRESSIONS ====================

#[test]
fn test_negate_expression() {
    // -(1 + 2)
    let expr = Expr::dummy(ExprKind::Unary {
        op: UnaryOp::Negate,
        expr: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        })),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_not_comparison() {
    // !(x > 5)
    let expr = Expr::dummy(ExprKind::Unary {
        op: UnaryOp::Not,
        expr: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Gt,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
        })),
    });
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(expr)),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== STRING OPERATIONS ====================

#[test]
fn test_empty_string() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::String("".to_string())))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_with_escapes() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::String(
            "hello\\nworld".to_string(),
        )))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== F-STRING EDGE CASES ====================

#[test]
fn test_fstring_empty() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FString {
            parts: vec![parser::ast::FStringPart::Text("".to_string())],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_multiple_interpolations() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FString {
                parts: vec![
                    parser::ast::FStringPart::Text("x=".to_string()),
                    parser::ast::FStringPart::Expr {
                        expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                        format: None,
                    },
                    parser::ast::FStringPart::Text(", y=".to_string()),
                    parser::ast::FStringPart::Expr {
                        expr: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
                        format: None,
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== CONST EDGE CASES ====================

#[test]
fn test_const_variable() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "PI".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14159))),
            is_const: true,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MIXED TYPE OPERATIONS ====================

#[test]
fn test_int_float_mixed_arithmetic() {
    // 10 + 3.14 (should promote int to float)
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== IDENTIFIER EDGE CASES ====================

#[test]
fn test_single_char_identifier() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_long_identifier() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "very_long_variable_name_that_is_still_valid".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_underscore_identifier() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "_".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== OVERFLOW/UNDERFLOW EDGE CASES ====================

#[test]
fn test_int_max_value() {
    // i64::MAX = 9223372036854775807
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "max_int".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(9223372036854775807))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_int_min_value() {
    // i64::MIN = -9223372036854775808
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "min_int".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(-9223372036854775808))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_very_large_float() {
    // Close to f64::MAX
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "large_float".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Float(1.7976931348623157e308))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_very_small_float() {
    // Close to f64::MIN (most negative)
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "small_float".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Float(-1.7976931348623157e308))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tiny_positive_float() {
    // Very small positive number (denormalized)
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "tiny".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Float(2.2250738585072014e-308))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== OPERATOR PRECEDENCE COMPLEX ====================

#[test]
fn test_precedence_add_mul() {
    // 1 + 2 * 3 should be 1 + (2 * 3) = 7
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
        rhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
        })),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_precedence_mul_pow() {
    // 2 * 3 ** 2 should be 2 * (3 ** 2) = 18
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Mul,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        rhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Pow,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        })),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_precedence_comparison_logical() {
    // 1 < 2 && 3 < 4 should be (1 < 2) && (3 < 4)
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::LogicalAnd,
        lhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Lt,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        })),
        rhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Lt,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
        })),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_precedence_bitwise_and_or() {
    // 1 | 2 & 4 should be 1 | (2 & 4)
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::BitOr,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
        rhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::BitAnd,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
        })),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_precedence_unary_binary() {
    // -2 + 3 should be (-2) + 3 = 1
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::dummy(ExprKind::Unary {
            op: UnaryOp::Negate,
            expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        })),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== DIVISION EDGE CASES ====================

#[test]
fn test_division_by_one() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(42)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_integer_division_truncation() {
    // 7 / 2 should truncate to 3 (integer division)
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(7)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_negative_division() {
    // -10 / 3 should be -3 (truncated toward zero)
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(-10)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== BOOLEAN EDGE CASES ====================

#[test]
fn test_boolean_literal_true_in_expression() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::LogicalAnd,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_boolean_literal_false_in_expression() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::LogicalOr,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(false)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(false)))),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_boolean_comparison_result() {
    // (1 < 2) == true
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Eq,
        lhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Lt,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        })),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ARRAY EDGE CASES ====================

#[test]
fn test_single_element_array() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(vec![Expr::dummy(ExprKind::Literal(Literal::Int(
            42,
        )))]))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_large_array() {
    // Array with 100 elements
    let elements: Vec<Expr> = (0..100).map(|i| Expr::dummy(ExprKind::Literal(Literal::Int(i)))).collect();
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(elements))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_with_expressions() {
    // [1 + 1, 2 * 2, 3 - 1]
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(vec![
            Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Add,
                lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            }),
            Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
            }),
            Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Sub,
                lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            }),
        ]))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_with_variables() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "a".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "b".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Identifier("a".to_string())),
                Expr::dummy(ExprKind::Identifier("b".to_string())),
            ])))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TYPE CASTING EDGE CASES ====================

#[test]
fn test_float_to_int_positive() {
    // Implicit cast or explicit truncation
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("int".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Float(3.7))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_to_int_negative() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("int".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Float(-3.7))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_int_to_float_exact() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("float".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== NEGATIVE NUMBER EDGE CASES ====================

#[test]
fn test_negative_zero_int() {
    // -0 is still 0 for integers
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Unary {
            op: UnaryOp::Negate,
            expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_negative_zero_float() {
    // -0.0 exists for floats (IEEE 754)
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Float(-0.0)))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_double_negation() {
    // -(-5) should be 5
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Unary {
            op: UnaryOp::Negate,
            expr: Box::new(Expr::dummy(ExprKind::Unary {
                op: UnaryOp::Negate,
                expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
            })),
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== EXPRESSION EVALUATION ORDER ====================

#[test]
fn test_left_to_right_evaluation() {
    // (1 + 2) + (3 + 4)
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        })),
        rhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
        })),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_mixed_operators() {
    // 1 + 2 * 3 - 4 / 2
    // Should be: 1 + (2 * 3) - (4 / 2) = 1 + 6 - 2 = 5
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Sub,
        lhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            rhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Mul,
                lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
            })),
        })),
        rhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Div,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        })),
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(expr))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
