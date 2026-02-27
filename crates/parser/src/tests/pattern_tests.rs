// Pattern Matching Tests

use crate::ast::{Expr, ExprKind, Literal, Pattern, StmtKind};
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

#[test]
fn test_match_literal_int() {
    let expr = parse_expr("match x { 1 -> :one 2 -> :two _ -> :other }").unwrap();
    match &expr.kind {
        ExprKind::Match { value, arms } => {
            assert_eq!(value.kind, ExprKind::Identifier("x".to_string()));
            assert_eq!(arms.len(), 3);
            match &arms[0].pattern {
                Pattern::Literal(Literal::Int(1)) => {}
                _ => panic!("Expected literal 1"),
            }
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_wildcard() {
    let expr = parse_expr("match x { _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Wildcard => {}
            _ => panic!("Expected wildcard"),
        },
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_binding() {
    let expr = parse_expr("match x { n -> n * 2 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Binding(name) => assert_eq!(name, "n"),
            _ => panic!("Expected binding"),
        },
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_or_pattern() {
    let expr = parse_expr("match x { 1 | 2 | 3 -> :small _ -> :large }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Or(patterns) => assert_eq!(patterns.len(), 3),
            _ => panic!("Expected or pattern"),
        },
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_with_guard() {
    let expr = parse_expr("match x { n if n > 10 -> :big n -> :small }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert!(arms[0].guard.is_some());
            assert!(arms[1].guard.is_none());
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_string_literal() {
    let expr = parse_expr(r#"match status { "ok" -> 1 "error" -> 0 }"#).unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Literal(Literal::String(_)) => {}
            _ => panic!("Expected string literal"),
        },
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_atom() {
    let expr = parse_expr("match status { :ok -> 1 :error -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Literal(Literal::Atom(atom)) => assert_eq!(atom, "ok"),
            _ => panic!("Expected atom"),
        },
        _ => panic!("Expected match"),
    }
}

// ==================== PHASE 4: PATTERN MATCHING 2.0 ====================

#[test]
fn test_destructure_pattern_bindings() {
    // { x, y } -> Destructure([Binding("x"), Binding("y")])
    let expr = parse_expr("match p { { x, y } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Destructure(vec![
                    Pattern::Binding("x".to_string()),
                    Pattern::Binding("y".to_string()),
                ])
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_destructure_pattern_literal_constraint() {
    // { 0, x } -> Destructure([Literal(Int(0)), Binding("x")])
    let expr = parse_expr("match p { { 0, x } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Destructure(vec![
                    Pattern::Literal(Literal::Int(0)),
                    Pattern::Binding("x".to_string()),
                ])
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_range_pattern_inclusive() {
    // 18..64 -> Range { start: Int(18), end: Int(64), inclusive: true }
    let expr = parse_expr("match age { 18..64 -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Range {
                    start: Literal::Int(18),
                    end: Literal::Int(64),
                    inclusive: true,
                }
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_range_pattern_exclusive() {
    // 0..<10 -> Range { start: Int(0), end: Int(10), inclusive: false }
    let expr = parse_expr("match x { 0..<10 -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Range {
                    start: Literal::Int(0),
                    end: Literal::Int(10),
                    inclusive: false,
                }
            );
        }
        _ => panic!("Expected match"),
    }
}
