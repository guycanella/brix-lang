// Operator Precedence Tests
//
// Tests to ensure correct operator precedence parsing.

use crate::ast::{BinaryOp, Expr};
use crate::parser::parser;
use chumsky::Parser;
use lexer::token::Token;

fn parse_expr(input: &str) -> Result<Expr, String> {
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).map_err(|e| format!("{:?}", e))?;
    if let Some(crate::ast::Stmt::Expr(expr)) = program.statements.first() {
        Ok(expr.clone())
    } else {
        Err("No expr".to_string())
    }
}

// ==================== ARITHMETIC PRECEDENCE ====================

#[test]
fn test_mul_over_add() {
    // 1 + 2 * 3 should be 1 + (2 * 3)
    let expr = parse_expr("1 + 2 * 3").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::Add,
            rhs,
            ..
        } => {
            match *rhs {
                Expr::Binary {
                    op: BinaryOp::Mul, ..
                } => {} // Good
                _ => panic!("Mul should bind tighter than Add"),
            }
        }
        _ => panic!("Expected Add at top"),
    }
}

#[test]
fn test_div_over_sub() {
    // 10 - 4 / 2 should be 10 - (4 / 2)
    let expr = parse_expr("10 - 4 / 2").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::Sub,
            rhs,
            ..
        } => match *rhs {
            Expr::Binary {
                op: BinaryOp::Div, ..
            } => {}
            _ => panic!("Div should bind tighter"),
        },
        _ => panic!("Expected Sub"),
    }
}

#[test]
fn test_pow_over_mul() {
    // 2 * 3 ** 4 should be 2 * (3 ** 4)
    let expr = parse_expr("2 * 3 ** 4").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::Mul,
            rhs,
            ..
        } => match *rhs {
            Expr::Binary {
                op: BinaryOp::Pow, ..
            } => {}
            _ => panic!("Pow should bind tighter"),
        },
        _ => panic!("Expected Mul"),
    }
}

// ==================== COMPARISON PRECEDENCE ====================

#[test]
fn test_comparison_over_logical() {
    // x > 0 && y < 10 should be (x > 0) && (y < 10)
    let expr = parse_expr("x > 0 && y < 10").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::LogicalAnd,
            lhs,
            rhs,
        } => match (*lhs, *rhs) {
            (
                Expr::Binary {
                    op: BinaryOp::Gt, ..
                },
                Expr::Binary {
                    op: BinaryOp::Lt, ..
                },
            ) => {}
            _ => panic!("Comparisons should bind first"),
        },
        _ => panic!("Expected LogicalAnd"),
    }
}

#[test]
fn test_add_over_comparison() {
    // x + 1 > y should be (x + 1) > y
    let expr = parse_expr("x + 1 > y").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::Gt,
            lhs,
            ..
        } => match *lhs {
            Expr::Binary {
                op: BinaryOp::Add, ..
            } => {}
            _ => panic!("Add should bind tighter"),
        },
        _ => panic!("Expected Gt"),
    }
}

// ==================== BITWISE PRECEDENCE ====================

#[test]
fn test_bitwise_over_comparison() {
    // x & 255 == 0 should be (x & 255) == 0
    // Note: using 255 instead of 0xFF since Brix doesn't support hex literals yet
    let expr = parse_expr("x & 255 == 0").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::Eq,
            lhs,
            ..
        } => match *lhs {
            Expr::Binary {
                op: BinaryOp::BitAnd,
                ..
            } => {}
            _ => panic!("BitAnd should bind first"),
        },
        _ => panic!("Expected Eq"),
    }
}

// ==================== PARENTHESES OVERRIDE ====================

#[test]
fn test_parens_override_precedence() {
    // (1 + 2) * 3 should have Add under Mul
    let expr = parse_expr("(1 + 2) * 3").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::Mul,
            lhs,
            ..
        } => match *lhs {
            Expr::Binary {
                op: BinaryOp::Add, ..
            } => {}
            _ => panic!("Parens should override"),
        },
        _ => panic!("Expected Mul"),
    }
}

// ==================== ASSOCIATIVITY TESTS ====================

#[test]
fn test_left_associative_add() {
    // 1 + 2 + 3 should be (1 + 2) + 3
    let expr = parse_expr("1 + 2 + 3").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::Add,
            lhs,
            ..
        } => {
            match *lhs {
                Expr::Binary {
                    op: BinaryOp::Add, ..
                } => {} // Left-assoc
                _ => panic!("Add should be left-associative"),
            }
        }
        _ => panic!("Expected Add"),
    }
}

#[test]
fn test_right_associative_pow() {
    // 2 ** 3 ** 2 should be 2 ** (3 ** 2) = 2 ** 9 = 512
    let expr = parse_expr("2 ** 3 ** 2").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::Pow,
            rhs,
            ..
        } => {
            match *rhs {
                Expr::Binary {
                    op: BinaryOp::Pow, ..
                } => {} // Right-assoc
                _ => panic!("Pow should be right-associative"),
            }
        }
        _ => panic!("Expected Pow"),
    }
}

// ==================== COMPLEX PRECEDENCE ====================

#[test]
fn test_complex_expression() {
    // 1 + 2 * 3 ** 4 - 5 / 6 should respect all precedence
    let expr = parse_expr("1 + 2 * 3 ** 4 - 5 / 6").unwrap();
    match expr {
        Expr::Binary {
            op: BinaryOp::Sub, ..
        } => {} // Sub at top (same precedence as Add, left-assoc)
        _ => panic!("Expected Sub at top"),
    }
}
