// Pattern Matching Tests

use crate::ast::{Expr, Literal, Pattern};
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

#[test]
fn test_match_literal_int() {
    let expr = parse_expr("match x { 1 -> :one 2 -> :two _ -> :other }").unwrap();
    match expr {
        Expr::Match { value, arms } => {
            assert_eq!(*value, Expr::Identifier("x".to_string()));
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
    match expr {
        Expr::Match { arms, .. } => {
            match &arms[0].pattern {
                Pattern::Wildcard => {}
                _ => panic!("Expected wildcard"),
            }
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_binding() {
    let expr = parse_expr("match x { n -> n * 2 }").unwrap();
    match expr {
        Expr::Match { arms, .. } => {
            match &arms[0].pattern {
                Pattern::Binding(name) => assert_eq!(name, "n"),
                _ => panic!("Expected binding"),
            }
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_or_pattern() {
    let expr = parse_expr("match x { 1 | 2 | 3 -> :small _ -> :large }").unwrap();
    match expr {
        Expr::Match { arms, .. } => {
            match &arms[0].pattern {
                Pattern::Or(patterns) => assert_eq!(patterns.len(), 3),
                _ => panic!("Expected or pattern"),
            }
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_with_guard() {
    let expr = parse_expr("match x { n if n > 10 -> :big n -> :small }").unwrap();
    match expr {
        Expr::Match { arms, .. } => {
            assert!(arms[0].guard.is_some());
            assert!(arms[1].guard.is_none());
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_string_literal() {
    let expr = parse_expr(r#"match status { "ok" -> 1 "error" -> 0 }"#).unwrap();
    match expr {
        Expr::Match { arms, .. } => {
            match &arms[0].pattern {
                Pattern::Literal(Literal::String(_)) => {}
                _ => panic!("Expected string literal"),
            }
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_atom() {
    let expr = parse_expr("match status { :ok -> 1 :error -> 0 }").unwrap();
    match expr {
        Expr::Match { arms, .. } => {
            match &arms[0].pattern {
                Pattern::Literal(Literal::Atom(atom)) => assert_eq!(atom, "ok"),
                _ => panic!("Expected atom"),
            }
        }
        _ => panic!("Expected match"),
    }
}
