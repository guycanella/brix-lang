// Edge Case Tests for Parser

use crate::ast::{Expr, Literal, Stmt};
use crate::parser::parser;
use chumsky::Parser;
use lexer::token::Token;

fn parse_expr(input: &str) -> Result<Expr, String> {
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).map_err(|e| format!("{:?}", e))?;
    if let Some(Stmt::Expr(expr)) = program.statements.first() {
        Ok(expr.clone())
    } else {
        Err("No expr".to_string())
    }
}

fn parse_stmt(input: &str) -> Result<Stmt, String> {
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).map_err(|e| format!("{:?}", e))?;
    program
        .statements
        .first()
        .cloned()
        .ok_or("No stmt".to_string())
}

// ==================== DEEPLY NESTED STRUCTURES ====================

#[test]
fn test_deeply_nested_parens() {
    let expr = parse_expr("((((((1))))))").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Int(1)));
}

#[test]
fn test_deeply_nested_arrays() {
    let expr = parse_expr("[[[[1]]]]").unwrap();
    // Should create nested arrays
    match expr {
        Expr::Array(_) => {}
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_deeply_nested_calls() {
    let expr = parse_expr("f(g(h(1)))").unwrap();
    match expr {
        Expr::Call { .. } => {}
        _ => panic!("Expected call"),
    }
}

// ==================== CHAINED OPERATIONS ====================

#[test]
fn test_chained_field_access() {
    let expr = parse_expr("a.b.c.d.e").unwrap();
    match expr {
        Expr::FieldAccess { .. } => {}
        _ => panic!("Expected field access"),
    }
}

#[test]
fn test_chained_index_access() {
    let expr = parse_expr("arr[0][1][2]").unwrap();
    match expr {
        Expr::Index { indices, .. } => {
            assert_eq!(indices.len(), 3);
        }
        _ => panic!("Expected index"),
    }
}

#[test]
#[ignore = "Feature not implemented: function call chaining"]
fn test_chained_function_calls() {
    let expr = parse_expr("foo()()()").unwrap();
    match expr {
        Expr::Call { func, .. } => {
            match *func {
                Expr::Call { .. } => {} // Chained
                _ => panic!("Expected chained calls"),
            }
        }
        _ => panic!("Expected call"),
    }
}

// ==================== MIXED OPERATIONS ====================

#[test]
fn test_mixed_field_and_index() {
    let expr = parse_expr("obj.field[0]").unwrap();
    match expr {
        Expr::Index { array, .. } => match *array {
            Expr::FieldAccess { .. } => {}
            _ => panic!("Expected field access"),
        },
        _ => panic!("Expected index"),
    }
}

#[test]
#[ignore = "Feature not implemented: field access on call result"]
fn test_mixed_call_and_field() {
    let expr = parse_expr("foo().field").unwrap();
    match expr {
        Expr::FieldAccess { target, .. } => match *target {
            Expr::Call { .. } => {}
            _ => panic!("Expected call"),
        },
        _ => panic!("Expected field access"),
    }
}

// ==================== EMPTY/MINIMAL STRUCTURES ====================

#[test]
fn test_empty_array_literal() {
    let expr = parse_expr("[]").unwrap();
    assert_eq!(expr, Expr::Array(vec![]));
}

#[test]
fn test_empty_function_call() {
    let expr = parse_expr("foo()").unwrap();
    match expr {
        Expr::Call { args, .. } => {
            assert_eq!(args.len(), 0);
        }
        _ => panic!("Expected call"),
    }
}

#[test]
fn test_empty_block() {
    let stmt = parse_stmt("{ }").unwrap();
    match stmt {
        Stmt::Block(stmts) => {
            assert_eq!(stmts.len(), 0);
        }
        _ => panic!("Expected block"),
    }
}

// ==================== WHITESPACE VARIATIONS ====================

#[test]
fn test_no_whitespace() {
    let expr = parse_expr("1+2*3").unwrap();
    // Should parse correctly despite no whitespace
    match expr {
        Expr::Binary { .. } => {}
        _ => panic!("Expected binary"),
    }
}

#[test]
fn test_excessive_whitespace() {
    let expr = parse_expr("1    +    2    *    3").unwrap();
    match expr {
        Expr::Binary { .. } => {}
        _ => panic!("Expected binary"),
    }
}

#[test]
fn test_newlines_in_expression() {
    let expr = parse_expr("1 +\n2 *\n3").unwrap();
    match expr {
        Expr::Binary { .. } => {}
        _ => panic!("Expected binary"),
    }
}

// ==================== SPECIAL CHARACTER SEQUENCES ====================

#[test]
fn test_escaped_string_in_expr() {
    let expr = parse_expr(r#""hello\nworld""#).unwrap();
    match expr {
        Expr::Literal(Literal::String(_)) => {}
        _ => panic!("Expected string"),
    }
}

#[test]
fn test_fstring_with_nested_braces() {
    let expr = parse_expr(r#"f"array: {[1, 2, 3]}""#).unwrap();
    match expr {
        Expr::FString { .. } => {}
        _ => panic!("Expected fstring"),
    }
}

// ==================== NUMBER EDGE CASES ====================

#[test]
fn test_zero() {
    let expr = parse_expr("0").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Int(0)));
}

#[test]
fn test_negative_number() {
    // Parses as unary negate
    let expr = parse_expr("-42").unwrap();
    match expr {
        Expr::Unary { .. } => {}
        _ => panic!("Expected unary"),
    }
}

#[test]
fn test_float_zero() {
    let expr = parse_expr("0.0").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Float(0.0)));
}

// ==================== IDENTIFIER EDGE CASES ====================

#[test]
fn test_single_char_identifier() {
    let expr = parse_expr("x").unwrap();
    assert_eq!(expr, Expr::Identifier("x".to_string()));
}

#[test]
fn test_underscore_identifier() {
    let expr = parse_expr("_").unwrap();
    assert_eq!(expr, Expr::Identifier("_".to_string()));
}

#[test]
fn test_long_identifier() {
    let expr = parse_expr("very_long_identifier_name_that_is_still_valid").unwrap();
    match expr {
        Expr::Identifier(_) => {}
        _ => panic!("Expected identifier"),
    }
}

// ==================== COMMENTS IN CODE ====================

#[test]
fn test_comment_in_expression() {
    let expr = parse_expr("1 + // comment\n2").unwrap();
    // Comment should be ignored by lexer
    match expr {
        Expr::Binary { .. } => {}
        _ => panic!("Expected binary"),
    }
}

// ==================== RANGE VARIATIONS ====================

#[test]
#[ignore = "Lexer issue: :end is tokenized as atom, not colon + identifier"]
fn test_range_variables() {
    let expr = parse_expr("start:end").unwrap();
    match expr {
        Expr::Range { .. } => {}
        _ => panic!("Expected range"),
    }
}

#[test]
fn test_range_expressions() {
    let expr = parse_expr("(x + 1):(y - 1)").unwrap();
    match expr {
        Expr::Range { .. } => {}
        _ => panic!("Expected range"),
    }
}
