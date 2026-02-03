// Destructuring Tests

use crate::ast::Stmt;
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
        .ok_or("No stmt".to_string())
}

#[test]
fn test_destructure_two_vars() {
    let stmt = parse_stmt("var { a, b } := foo()").unwrap();
    match stmt {
        Stmt::DestructuringDecl { names, .. } => {
            assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
        }
        _ => panic!("Expected destructuring"),
    }
}

#[test]
fn test_destructure_three_vars() {
    let stmt = parse_stmt("var { x, y, z } := calc()").unwrap();
    match stmt {
        Stmt::DestructuringDecl { names, .. } => {
            assert_eq!(names.len(), 3);
        }
        _ => panic!("Expected destructuring"),
    }
}

#[test]
fn test_destructure_const() {
    let stmt = parse_stmt("const { a, b } := foo()").unwrap();
    match stmt {
        Stmt::DestructuringDecl { is_const, .. } => {
            assert_eq!(is_const, true);
        }
        _ => panic!("Expected destructuring"),
    }
}

#[test]
fn test_destructure_in_for_loop() {
    let stmt = parse_stmt("for x, y in pairs { }").unwrap();
    match stmt {
        Stmt::For { var_names, .. } => {
            assert_eq!(var_names, vec!["x".to_string(), "y".to_string()]);
        }
        _ => panic!("Expected for loop"),
    }
}

#[test]
fn test_destructure_with_underscore() {
    // Note: Parser might handle _ as identifier or special
    let stmt = parse_stmt("var { result, _ } := divmod(10, 3)").unwrap();
    match stmt {
        Stmt::DestructuringDecl { names, .. } => {
            assert_eq!(names[0], "result");
            assert_eq!(names[1], "_");
        }
        _ => panic!("Expected destructuring"),
    }
}
