// Operator Precedence Tests
//
// Tests to ensure correct operator precedence parsing.

use crate::ast::{BinaryOp, Expr, ExprKind, StmtKind};
use crate::parser::parser;
use chumsky::Parser;
use lexer::token::Token;

fn parse_expr(input: &str) -> Result<Expr, String> {
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).map_err(|e| format!("{:?}", e))?;
    if let Some(stmt) = program.statements.first() {
        if let StmtKind::Expr(expr) = &stmt.kind {
            return Ok(expr.clone());
        }
    }
    Err("No expr".to_string())
}

// ==================== ARITHMETIC PRECEDENCE ====================

#[test]
fn test_mul_over_add() {
    // 1 + 2 * 3 should be 1 + (2 * 3)
    let expr = parse_expr("1 + 2 * 3").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Add,
            rhs,
            ..
        } => {
            match &rhs.kind {
                ExprKind::Binary {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Sub,
            rhs,
            ..
        } => match &rhs.kind {
            ExprKind::Binary {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Mul,
            rhs,
            ..
        } => match &rhs.kind {
            ExprKind::Binary {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            lhs,
            rhs,
        } => match (&lhs.kind, &rhs.kind) {
            (
                ExprKind::Binary {
                    op: BinaryOp::Gt, ..
                },
                ExprKind::Binary {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Gt,
            lhs,
            ..
        } => match &lhs.kind {
            ExprKind::Binary {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Eq,
            lhs,
            ..
        } => match &lhs.kind {
            ExprKind::Binary {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Mul,
            lhs,
            ..
        } => match &lhs.kind {
            ExprKind::Binary {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Add,
            lhs,
            ..
        } => {
            match &lhs.kind {
                ExprKind::Binary {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Pow,
            rhs,
            ..
        } => {
            match &rhs.kind {
                ExprKind::Binary {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Sub, ..
        } => {} // Sub at top (same precedence as Add, left-assoc)
        _ => panic!("Expected Sub at top"),
    }
}

// ==================== EMBEDDING OPERATORS (v2.1 Grupo 0) ====================

/// Render an expression as an s-expression: binary ops as `(Op l r)`,
/// the desugared operators as `(method receiver arg)`.
fn sexpr(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Identifier(name) => name.clone(),
        ExprKind::Literal(_) => "lit".to_string(),
        ExprKind::Binary { op, lhs, rhs } => {
            format!("({:?} {} {})", op, sexpr(lhs), sexpr(rhs))
        }
        ExprKind::Unary { expr, .. } => format!("(neg {})", sexpr(expr)),
        ExprKind::Call { func, args } => match &func.kind {
            ExprKind::FieldAccess { target, field } => {
                let args: Vec<String> = args.iter().map(sexpr).collect();
                format!("({} {} {})", field, sexpr(target), args.join(" "))
            }
            _ => format!("(call {})", sexpr(func)),
        },
        other => format!("{:?}", other),
    }
}

fn parsed(input: &str) -> String {
    sexpr(&parse_expr(input).unwrap())
}

#[test]
fn test_embedding_operators_desugar_to_method_calls() {
    assert_eq!(parsed("a @ b"), "(cosine_similarity a b)");
    assert_eq!(parsed("a <-> b"), "(euclidean_distance a b)");
    assert_eq!(parsed("a <=> b"), "(dot_product a b)");
}

#[test]
fn test_embedding_operators_bind_tighter_than_mul_and_add() {
    assert_eq!(parsed("1.0 - a @ b"), "(Sub lit (cosine_similarity a b))");
    assert_eq!(parsed("2.0 * a <=> b"), "(Mul lit (dot_product a b))");
    assert_eq!(
        parsed("a <-> b / 2.0"),
        "(Div (euclidean_distance a b) lit)"
    );
    assert_eq!(
        parsed("a @ b + c @ d"),
        "(Add (cosine_similarity a b) (cosine_similarity c d))"
    );
}

#[test]
fn test_embedding_operators_bind_tighter_than_comparison_and_logic() {
    assert_eq!(parsed("a @ b > 0.8"), "(Gt (cosine_similarity a b) lit)");
    assert_eq!(
        parsed("a @ b >= 0.5 && a <-> b < 1.0"),
        "(LogicalAnd (GtEq (cosine_similarity a b) lit) (Lt (euclidean_distance a b) lit))"
    );
}

#[test]
fn test_embedding_operators_are_left_associative_at_one_level() {
    // Same level for all three, left-to-right (type-checked later).
    assert_eq!(
        parsed("a @ b <=> c"),
        "(dot_product (cosine_similarity a b) c)"
    );
}

#[test]
fn test_embedding_operators_looser_than_postfix() {
    // Method calls / fields / indexing on the operands bind first.
    assert_eq!(
        parsed("x.e @ batch.get(0)"),
        format!(
            "(cosine_similarity {} (get batch lit))",
            sexpr(&parse_expr("x.e").unwrap())
        )
    );
}

#[test]
fn test_embedding_operators_bind_tighter_than_unary_and_pow() {
    // No operator yields an Embedding, so prefix ops and `**` apply to the
    // float result: `-a @ b` is a negated similarity, not `(-a) @ b`.
    assert_eq!(parsed("-a @ b"), "(neg (cosine_similarity a b))");
    assert_eq!(parsed("a @ b ** 2"), "(Pow (cosine_similarity a b) lit)");
    assert_eq!(parsed("2.0 ** a <=> b"), "(Pow lit (dot_product a b))");
    assert_eq!(parsed("a ** b @ c"), "(Pow a (cosine_similarity b c))");
    // `await` also applies to the whole operation (parenthesize to await
    // an operand).
    assert!(matches!(
        parse_expr("await x @ y").unwrap().kind,
        ExprKind::Await { .. }
    ));
}

#[test]
fn test_embedding_operator_span_covers_both_operands() {
    // parse_expr uses token-index spans: `a @ b` is tokens 0..3. The
    // desugared call (and its FieldAccess) must span both operands, so an
    // error on it points at the whole `a @ b`, not just `a`.
    let expr = parse_expr("a @ b").unwrap();
    assert_eq!(expr.span, 0..3);
    match &expr.kind {
        ExprKind::Call { func, .. } => assert_eq!(func.span, 0..3),
        other => panic!("expected desugared call, got {:?}", other),
    }
}
