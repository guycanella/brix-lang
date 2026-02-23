// Async/Await Parser Tests
//
// Tests for async fn, async { } blocks, and await expressions.

use crate::ast::{ExprKind, Stmt, StmtKind, MethodDef};
use crate::parser::parser;
use chumsky::Parser;
use lexer::token::Token;

fn parse_stmt(input: &str) -> Result<Stmt, String> {
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).map_err(|e| format!("{:?}", e))?;
    program
        .statements
        .first()
        .cloned()
        .ok_or("No statement".to_string())
}

fn parse_expr(input: &str) -> Result<crate::ast::Expr, String> {
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).map_err(|e| format!("{:?}", e))?;
    let stmt = program
        .statements
        .first()
        .cloned()
        .ok_or("No statement".to_string())?;
    match stmt.kind {
        StmtKind::Expr(e) => Ok(e),
        _ => Err("Expected expression statement".to_string()),
    }
}

// ==================== async fn TESTS ====================

#[test]
fn test_async_fn_simple() {
    let stmt = parse_stmt("async fn fetch() { }").unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef { name, is_async, params, return_type, .. } => {
            assert_eq!(name, "fetch");
            assert_eq!(*is_async, true);
            assert!(params.is_empty());
            assert!(return_type.is_none());
        }
        _ => panic!("Expected FunctionDef, got {:?}", stmt.kind),
    }
}

#[test]
fn test_sync_fn_is_not_async() {
    let stmt = parse_stmt("fn add(x: int, y: int) -> int { return x + y }").unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef { name, is_async, .. } => {
            assert_eq!(name, "add");
            assert_eq!(*is_async, false, "Regular fn should have is_async=false");
        }
        _ => panic!("Expected FunctionDef"),
    }
}

#[test]
fn test_async_fn_with_return_type() {
    let stmt = parse_stmt("async fn get_user(id: int) -> int { return id }").unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef { name, is_async, params, return_type, .. } => {
            assert_eq!(name, "get_user");
            assert_eq!(*is_async, true);
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].0, "id");
            assert_eq!(params[0].1, "int");
            assert!(return_type.is_some());
            assert_eq!(return_type.as_ref().unwrap()[0], "int");
        }
        _ => panic!("Expected FunctionDef"),
    }
}

#[test]
fn test_async_fn_multiple_params() {
    let stmt = parse_stmt("async fn send(url: string, body: string) -> int { return 200 }").unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef { name, is_async, params, .. } => {
            assert_eq!(name, "send");
            assert_eq!(*is_async, true);
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].0, "url");
            assert_eq!(params[1].0, "body");
        }
        _ => panic!("Expected FunctionDef"),
    }
}

#[test]
fn test_async_fn_void_no_params() {
    let stmt = parse_stmt("async fn main() { }").unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef { name, is_async, params, return_type, .. } => {
            assert_eq!(name, "main");
            assert_eq!(*is_async, true);
            assert!(params.is_empty());
            assert!(return_type.is_none());
        }
        _ => panic!("Expected FunctionDef"),
    }
}

// ==================== async METHOD TESTS ====================

#[test]
fn test_async_method() {
    let stmt = parse_stmt("async fn (c: Client) fetch(url: string) -> int { return 200 }").unwrap();
    match &stmt.kind {
        StmtKind::MethodDef(MethodDef { is_async, receiver_name, receiver_type, method_name, .. }) => {
            assert_eq!(*is_async, true);
            assert_eq!(receiver_name, "c");
            assert_eq!(receiver_type, "Client");
            assert_eq!(method_name, "fetch");
        }
        _ => panic!("Expected MethodDef, got {:?}", stmt.kind),
    }
}

#[test]
fn test_sync_method_is_not_async() {
    let stmt = parse_stmt("fn (p: Point) distance() -> float { return 0.0 }").unwrap();
    match &stmt.kind {
        StmtKind::MethodDef(MethodDef { is_async, .. }) => {
            assert_eq!(*is_async, false);
        }
        _ => panic!("Expected MethodDef"),
    }
}

// ==================== await EXPRESSION TESTS ====================

#[test]
fn test_await_simple_identifier() {
    let expr = parse_expr("await future_val").unwrap();
    match &expr.kind {
        ExprKind::Await { expr: inner } => {
            assert!(matches!(inner.kind, ExprKind::Identifier(_)));
        }
        _ => panic!("Expected Await, got {:?}", expr.kind),
    }
}

#[test]
fn test_await_function_call() {
    let expr = parse_expr("await fetch_user(123)").unwrap();
    match &expr.kind {
        ExprKind::Await { expr: inner } => {
            assert!(matches!(inner.kind, ExprKind::Call { .. }));
        }
        _ => panic!("Expected Await wrapping a Call"),
    }
}

// ==================== async BLOCK TESTS ====================

#[test]
fn test_async_block_empty() {
    let expr = parse_expr("async { }").unwrap();
    match &expr.kind {
        ExprKind::AsyncBlock { body } => {
            // body should be a Block
            assert!(matches!(body.kind, StmtKind::Block(_)));
        }
        _ => panic!("Expected AsyncBlock, got {:?}", expr.kind),
    }
}
