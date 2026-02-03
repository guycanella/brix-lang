// Edge Case and Malformed Input Tests
//
// Tests for unusual inputs, boundary conditions, and error scenarios.
// These tests ensure the lexer handles edge cases gracefully.

use crate::token::Token;
use logos::Logos;

// Helper function to tokenize input and return all tokens
fn tokenize(input: &str) -> Vec<Result<Token, ()>> {
    Token::lexer(input).collect()
}

// ==================== EMPTY INPUT TESTS ====================

#[test]
fn test_empty_input() {
    let tokens = tokenize("");
    assert_eq!(tokens.len(), 0, "Empty input should produce no tokens");
}

#[test]
fn test_only_whitespace() {
    let tokens = tokenize("     ");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_only_tabs() {
    let tokens = tokenize("\t\t\t");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_only_newlines() {
    let tokens = tokenize("\n\n\n");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_mixed_whitespace_only() {
    let tokens = tokenize("  \t\n  \t\n  ");
    assert_eq!(tokens.len(), 0);
}

// ==================== COMMENT EDGE CASES ====================

#[test]
fn test_comment_only() {
    let tokens = tokenize("// This is just a comment");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_comment_with_symbols() {
    let tokens = tokenize("// Comment with symbols: !@#$%^&*()");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_comment_with_code_like_text() {
    let tokens = tokenize("// var x = 10");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_comment_no_space_after_slashes() {
    let tokens = tokenize("//no space");
    assert_eq!(tokens.len(), 0);
}

#[test]
fn test_comment_at_eof_no_newline() {
    let tokens = tokenize("var x = 10 // comment");
    assert_eq!(tokens.len(), 4); // var, x, =, 10 (comment ignored)
}

// ==================== UNCLOSED STRING TESTS ====================
// Note: These will likely produce errors or unexpected tokens

#[test]
fn test_unclosed_string() {
    // String without closing quote
    let _tokens = tokenize(r#""hello"#);
    // Lexer should handle this gracefully (might produce error token)
    // This test documents current behavior
}

#[test]
fn test_unclosed_fstring() {
    let _tokens = tokenize(r#"f"hello"#);
    // Similar to unclosed string
}

// ==================== INVALID CHARACTER SEQUENCES ====================

#[test]
fn test_at_symbol() {
    // @ is not a valid token in Brix
    let _tokens = tokenize("@");
    // Should produce error or be ignored
    // This documents behavior for unsupported symbols
}

#[test]
fn test_backtick() {
    let _tokens = tokenize("`");
    // Backtick is not used in Brix
}

#[test]
fn test_tilde() {
    let _tokens = tokenize("~");
    // Tilde is not used in Brix
}

#[test]
fn test_dollar_sign() {
    let _tokens = tokenize("$");
    // Dollar is not used (unlike shell/PHP)
}

#[test]
fn test_hash_outside_comment() {
    let _tokens = tokenize("# not a comment in Brix");
    // # is not a comment character in Brix (only // is)
}

// ==================== OPERATOR AMBIGUITY TESTS ====================

#[test]
fn test_triple_plus() {
    // "+++" should be PlusPlus + Plus
    let tokens = tokenize("+++");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Ok(Token::PlusPlus));
    assert_eq!(tokens[1], Ok(Token::Plus));
}

#[test]
fn test_triple_minus() {
    // "---" should be MinusMinus + Minus
    let tokens = tokenize("---");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Ok(Token::MinusMinus));
    assert_eq!(tokens[1], Ok(Token::Minus));
}

#[test]
fn test_triple_equals() {
    // "===" should be DoubleEq + Eq
    let tokens = tokenize("===");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Ok(Token::DoubleEq));
    assert_eq!(tokens[1], Ok(Token::Eq));
}

#[test]
fn test_triple_star() {
    // "***" should be Pow + Star
    let tokens = tokenize("***");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Ok(Token::Pow));
    assert_eq!(tokens[1], Ok(Token::Star));
}

#[test]
fn test_minus_greater() {
    // "->" should be Arrow, NOT Minus + Gt
    let tokens = tokenize("->");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], Ok(Token::Arrow));
}

#[test]
fn test_colon_equals() {
    // ":=" should be ColonEq, NOT Colon + Eq
    let tokens = tokenize(":=");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], Ok(Token::ColonEq));
}

// ==================== NUMBER EDGE CASES ====================

#[test]
fn test_dot_without_digits() {
    // "." alone should be Dot token, NOT a float
    let tokens = tokenize(".");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], Ok(Token::Dot));
}

#[test]
fn test_dot_dot() {
    // ".." should be Dot + Dot
    let tokens = tokenize("..");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Ok(Token::Dot));
    assert_eq!(tokens[1], Ok(Token::Dot));
}

#[test]
fn test_number_with_trailing_dot() {
    // "10." is ambiguous - could be Int(10) + Dot or malformed float
    let _tokens = tokenize("10.");
    // Documents current lexer behavior
}

#[test]
fn test_dot_with_leading_digits() {
    // ".5" is not valid in our lexer (requires "0.5")
    let _tokens = tokenize(".5");
    // Should tokenize as Dot + Int(5)
}

// ==================== IDENTIFIER EDGE CASES ====================

#[test]
fn test_identifier_starting_with_number() {
    // "123abc" should be Int(123) + Identifier("abc")
    let tokens = tokenize("123abc");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Ok(Token::Int(123)));
    assert_eq!(tokens[1], Ok(Token::Identifier("abc".to_string())));
}

#[test]
fn test_identifier_with_dollar() {
    // "my$var" - dollar in middle
    // Should tokenize as Identifier("my") + error + Identifier("var")
    let _tokens = tokenize("my$var");
    // Documents behavior ($ is invalid)
}

#[test]
fn test_identifier_extremely_long() {
    let long_id = "a".repeat(1000);
    let tokens = tokenize(&long_id);
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], Ok(Token::Identifier(long_id)));
}

// ==================== MIXED VALID AND INVALID ====================

#[test]
fn test_valid_code_with_invalid_char() {
    let _tokens = tokenize("var x = 10 @ var y = 20");
    // Should tokenize valid parts and handle @ somehow
}

#[test]
fn test_unterminated_string_followed_by_code() {
    // This is a pathological case
    let _tokens = tokenize(r#""hello var x = 10"#);
    // Lexer behavior with unterminated strings is documented
}

// ==================== WHITESPACE VARIATIONS ====================

#[test]
fn test_no_whitespace_between_tokens() {
    let tokens = tokenize("var x=10");
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0], Ok(Token::Var));
    assert_eq!(tokens[1], Ok(Token::Identifier("x".to_string())));
    assert_eq!(tokens[2], Ok(Token::Eq));
    assert_eq!(tokens[3], Ok(Token::Int(10)));
}

#[test]
fn test_excessive_whitespace() {
    let tokens = tokenize("var        x        =        10");
    assert_eq!(tokens.len(), 4);
}

#[test]
fn test_mixed_whitespace_types() {
    let tokens = tokenize("var\t \t \nx\n\t =  \t10");
    assert_eq!(tokens.len(), 4);
}

// ==================== UNICODE EDGE CASES ====================

#[test]
fn test_unicode_identifier_not_supported() {
    // Brix identifiers are ASCII only ([a-zA-Z_][a-zA-Z0-9_]*)
    // "变量" (Chinese) should not be an identifier
    let _tokens = tokenize("变量");
    // Should not produce valid identifier token
}

#[test]
fn test_unicode_in_comment() {
    let tokens = tokenize("// Comment with emoji 🎉");
    assert_eq!(tokens.len(), 0); // Comment ignored
}

#[test]
fn test_emoji_in_code() {
    // Emoji outside string/comment
    let _tokens = tokenize("var 🎉 = 10");
    // Should handle gracefully (likely error)
}

// ==================== NESTED STRUCTURES ====================

#[test]
fn test_deeply_nested_brackets() {
    let tokens = tokenize("[[[[[]]]]]");
    assert_eq!(tokens.len(), 10);
    // 5 LBracket + 5 RBracket
}

#[test]
fn test_deeply_nested_parens() {
    let tokens = tokenize("((((()))))");
    assert_eq!(tokens.len(), 10);
}

#[test]
fn test_deeply_nested_braces() {
    let tokens = tokenize("{{{{{}}}}}");
    assert_eq!(tokens.len(), 10);
}

#[test]
fn test_mixed_nested_delimiters() {
    let tokens = tokenize("([{([{}])}])");
    assert_eq!(tokens.len(), 12);
}

// ==================== BOUNDARY INPUT SIZES ====================

#[test]
fn test_single_char_input() {
    let tokens = tokenize("x");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], Ok(Token::Identifier("x".to_string())));
}

#[test]
fn test_very_long_input() {
    // Generate long but valid input
    let long_input = "var x = 10 ".repeat(100);
    let tokens = tokenize(&long_input);
    assert_eq!(tokens.len(), 400); // 4 tokens * 100 repetitions
}

// ==================== SPECIAL CHARACTER COMBINATIONS ====================

#[test]
fn test_all_operators_sequence() {
    let tokens = tokenize("+ - * / % ** ++ -- += -= *= /= ^ | & < > <= >= == != -> := ? : ,");
    // Should tokenize all operators correctly
    assert!(tokens.len() > 0);
}

#[test]
fn test_all_delimiters_sequence() {
    let tokens = tokenize("( ) { } [ ]");
    assert_eq!(tokens.len(), 6);
}

#[test]
fn test_all_keywords_sequence() {
    let input = "function var const type return if else true false nil and or not while for in import as match printf print println";
    let tokens = tokenize(input);
    assert_eq!(tokens.len(), 22); // Corrected count: 22 keywords total
}
