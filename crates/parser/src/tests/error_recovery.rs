// Error Recovery Tests
//
// Tests for parser error handling and recovery.

use crate::parser::parser;
use crate::error;
use chumsky::Parser;
use lexer::token::Token;
use logos::Logos;

fn parse(input: &str) -> bool {
    // Lex with spans
    let tokens_with_spans: Vec<(Token, std::ops::Range<usize>)> = Token::lexer(input)
        .spanned()
        .map(|(t, span)| (t.unwrap_or(Token::Error), span))
        .collect();

    // Check for invalid operator sequences (this catches cases like "1 ++ 2")
    if error::check_and_report_invalid_sequences("test", input, &tokens_with_spans) {
        return false;
    }

    // Extract tokens for parsing
    let tokens: Vec<Token> = tokens_with_spans
        .iter()
        .map(|(t, _)| t.clone())
        .collect();

    parser().parse(tokens).is_ok()
}

// ==================== SYNTAX ERROR TESTS ====================

#[test]
fn test_missing_semicolon_recovers() {
    // Brix doesn't require semicolons, so this should pass
    assert!(parse("var x := 10\nvar y := 20"));
}

#[test]
fn test_unclosed_paren() {
    // Should fail
    assert!(!parse("foo(1, 2"));
}

#[test]
fn test_unclosed_bracket() {
    assert!(!parse("[1, 2, 3"));
}

#[test]
fn test_unclosed_brace() {
    assert!(!parse("if x > 0 { var y := 10"));
}

#[test]
fn test_mismatched_parens() {
    assert!(!parse("(1 + 2]"));
}

#[test]
fn test_invalid_operator_sequence() {
    // Now correctly detected by error::check_and_report_invalid_sequences()
    // This test verifies that the parser rejects invalid operator sequences
    // Note: The actual error checking happens in main.rs before parsing
    assert!(!parse("1 ++ 2")); // ++ is not a binary operator
}

#[test]
fn test_missing_rhs() {
    assert!(!parse("1 +"));
}

#[test]
fn test_missing_lhs() {
    assert!(!parse("+ 1")); // Unary plus not supported
}

// ==================== TYPE ERROR DETECTION ====================

#[test]
fn test_missing_type_annotation() {
    // Should pass - type hint is optional
    assert!(parse("var x := 10"));
}

#[test]
fn test_invalid_type_syntax() {
    // Depends on parser implementation
    // This documents expected behavior
}

// ==================== STATEMENT ERROR TESTS ====================

#[test]
fn test_if_without_condition() {
    assert!(!parse("if { }"));
}

#[test]
fn test_while_without_condition() {
    assert!(!parse("while { }"));
}

#[test]
fn test_for_without_in() {
    assert!(!parse("for i 1:10 { }"));
}

#[test]
fn test_function_missing_return_type() {
    // Should pass - void functions don't need return type
    assert!(parse("function foo() { }"));
}

// ==================== EXPRESSION ERROR TESTS ====================

#[test]
fn test_empty_array() {
    // Should pass
    assert!(parse("[]"));
}

#[test]
fn test_trailing_comma_in_array() {
    // Parser might allow or reject this
    let _result = parse("[1, 2, 3,]");
    // Documents behavior
}

#[test]
fn test_double_operator() {
    assert!(!parse("1 + * 2"));
}
