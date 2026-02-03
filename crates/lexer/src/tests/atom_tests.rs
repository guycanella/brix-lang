// Atom Literal Tests
//
// Comprehensive tests for Elixir-style atoms (:ok, :error, :atom_name).
// Tests include priority handling (Atom vs Colon), edge cases, and pattern matching usage.

use crate::token::Token;
use logos::Logos;

// Helper function to tokenize and assert single token
fn assert_single_token(input: &str, expected: Token) {
    let mut lexer = Token::lexer(input);
    let token = lexer.next();
    assert_eq!(token, Some(Ok(expected)), "Failed to match token for input: {}", input);
    assert_eq!(lexer.next(), None, "Expected single token, found more");
}

// ==================== BASIC ATOM TESTS ====================

#[test]
fn test_atom_ok() {
    assert_single_token(":ok", Token::Atom("ok".to_string()));
}

#[test]
fn test_atom_error() {
    assert_single_token(":error", Token::Atom("error".to_string()));
}

#[test]
fn test_atom_nil() {
    // Note: :nil is an atom, different from the nil keyword
    assert_single_token(":nil", Token::Atom("nil".to_string()));
}

#[test]
fn test_atom_true() {
    assert_single_token(":true", Token::Atom("true".to_string()));
}

#[test]
fn test_atom_false() {
    assert_single_token(":false", Token::Atom("false".to_string()));
}

#[test]
fn test_atom_single_char() {
    assert_single_token(":a", Token::Atom("a".to_string()));
}

#[test]
fn test_atom_uppercase() {
    assert_single_token(":OK", Token::Atom("OK".to_string()));
}

#[test]
fn test_atom_mixed_case() {
    assert_single_token(":MyAtom", Token::Atom("MyAtom".to_string()));
}

#[test]
fn test_atom_snake_case() {
    assert_single_token(":my_atom", Token::Atom("my_atom".to_string()));
}

#[test]
fn test_atom_with_numbers() {
    assert_single_token(":atom123", Token::Atom("atom123".to_string()));
}

#[test]
fn test_atom_underscore_start() {
    assert_single_token(":_private", Token::Atom("_private".to_string()));
}

#[test]
fn test_atom_underscore_only() {
    assert_single_token(":_", Token::Atom("_".to_string()));
}

#[test]
fn test_atom_multiple_underscores() {
    assert_single_token(":__internal", Token::Atom("__internal".to_string()));
}

#[test]
fn test_atom_long_name() {
    assert_single_token(":this_is_a_very_long_atom_name", Token::Atom("this_is_a_very_long_atom_name".to_string()));
}

// ==================== ATOM NAMING CONVENTIONS ====================

#[test]
fn test_atom_http_status() {
    assert_single_token(":http_200", Token::Atom("http_200".to_string()));
}

#[test]
fn test_atom_state_pending() {
    assert_single_token(":pending", Token::Atom("pending".to_string()));
}

#[test]
fn test_atom_state_active() {
    assert_single_token(":active", Token::Atom("active".to_string()));
}

#[test]
fn test_atom_state_inactive() {
    assert_single_token(":inactive", Token::Atom("inactive".to_string()));
}

#[test]
fn test_atom_color_red() {
    assert_single_token(":red", Token::Atom("red".to_string()));
}

#[test]
fn test_atom_direction_up() {
    assert_single_token(":up", Token::Atom("up".to_string()));
}

// ==================== PRIORITY TESTS (Atom vs Colon) ====================

#[test]
fn test_priority_atom_vs_colon() {
    // ":ok" should be Atom("ok"), NOT Colon + Identifier("ok")
    let mut lexer = Token::lexer(":ok");
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), None); // Should be single token
}

#[test]
fn test_priority_standalone_colon() {
    // ":" alone should be Colon token
    assert_single_token(":", Token::Colon);
}

#[test]
fn test_priority_colon_in_range() {
    // "1:10" should be Int(1) + Colon + Int(10)
    let mut lexer = Token::lexer("1:10");
    assert_eq!(lexer.next(), Some(Ok(Token::Int(1))));
    assert_eq!(lexer.next(), Some(Ok(Token::Colon)));
    assert_eq!(lexer.next(), Some(Ok(Token::Int(10))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_priority_colon_after_identifier() {
    // "x: int" should be Identifier + Colon + Identifier
    let mut lexer = Token::lexer("x: int");
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("x".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Colon)));
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("int".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_priority_walrus_vs_atom() {
    // ":=" should be ColonEq, NOT Atom("")
    assert_single_token(":=", Token::ColonEq);
}

// ==================== ATOMS IN EXPRESSIONS ====================

#[test]
fn test_atom_in_assignment() {
    let mut lexer = Token::lexer("var status = :ok");
    assert_eq!(lexer.next(), Some(Ok(Token::Var)));
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("status".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Eq)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_atom_in_comparison() {
    let mut lexer = Token::lexer("status == :ok");
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("status".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::DoubleEq)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_atom_in_match_pattern() {
    let mut lexer = Token::lexer("match x { :ok -> 1 }");
    assert_eq!(lexer.next(), Some(Ok(Token::Match)));
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("x".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::LBrace)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Arrow)));
    assert_eq!(lexer.next(), Some(Ok(Token::Int(1))));
    assert_eq!(lexer.next(), Some(Ok(Token::RBrace)));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_multiple_atoms_in_match() {
    let mut lexer = Token::lexer(":ok | :error");
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Pipe)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("error".to_string()))));
    assert_eq!(lexer.next(), None);
}

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_edge_atom_sequence() {
    let mut lexer = Token::lexer(":ok :error :pending");
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("error".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("pending".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_edge_atom_without_space() {
    // ":ok:error" should be TWO atoms if properly separated by whitespace in parser
    // But lexer will see it as ":ok" + ":" + "error" since second : is not followed by letter immediately
    let mut lexer = Token::lexer(":ok:error");
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    // After ":ok", we have ":error" which should be an atom
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("error".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_edge_atom_in_array() {
    let mut lexer = Token::lexer("[:ok, :error]");
    assert_eq!(lexer.next(), Some(Ok(Token::LBracket)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Comma)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("error".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::RBracket)));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_edge_atom_in_tuple() {
    let mut lexer = Token::lexer("(:ok, :error)");
    assert_eq!(lexer.next(), Some(Ok(Token::LParen)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Comma)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("error".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::RParen)));
    assert_eq!(lexer.next(), None);
}

// ==================== ATOM VS KEYWORD TESTS ====================

#[test]
fn test_atom_vs_keyword_if() {
    // "if" is keyword, ":if" is atom
    assert_single_token("if", Token::If);
    assert_single_token(":if", Token::Atom("if".to_string()));
}

#[test]
fn test_atom_vs_keyword_var() {
    assert_single_token("var", Token::Var);
    assert_single_token(":var", Token::Atom("var".to_string()));
}

#[test]
fn test_atom_vs_keyword_function() {
    assert_single_token("function", Token::Function);
    assert_single_token(":function", Token::Atom("function".to_string()));
}

#[test]
fn test_atom_vs_keyword_return() {
    assert_single_token("return", Token::Return);
    assert_single_token(":return", Token::Atom("return".to_string()));
}

// ==================== COMPLEX ATOM USAGE TESTS ====================

#[test]
fn test_complex_go_style_error_handling() {
    // var { result, err } := divide(10, 0)
    // if err != :nil { ... }
    let mut lexer = Token::lexer("err != :nil");
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("err".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::NotEq)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("nil".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_complex_http_status_pattern() {
    let mut lexer = Token::lexer("match status { :http_200 -> :ok }");
    assert_eq!(lexer.next(), Some(Ok(Token::Match)));
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("status".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::LBrace)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("http_200".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Arrow)));
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::RBrace)));
    assert_eq!(lexer.next(), None);
}

// ==================== WHITESPACE WITH ATOMS ====================

#[test]
fn test_atom_with_leading_whitespace() {
    let mut lexer = Token::lexer("  :ok");
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_atom_with_trailing_whitespace() {
    let mut lexer = Token::lexer(":ok  ");
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_atom_with_newlines() {
    let mut lexer = Token::lexer("\n:ok\n");
    assert_eq!(lexer.next(), Some(Ok(Token::Atom("ok".to_string()))));
    assert_eq!(lexer.next(), None);
}
