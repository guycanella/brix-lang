// Token Recognition Tests
//
// Comprehensive tests for all basic token types in the Brix lexer.
// Tests ensure correct recognition of keywords, operators, delimiters, and identifiers.

use crate::token::Token;
use logos::Logos;

// Helper function to tokenize input and return all tokens
fn tokenize(input: &str) -> Vec<Result<Token, ()>> {
    Token::lexer(input).collect()
}

// Helper function to tokenize and assert single token
fn assert_single_token(input: &str, expected: Token) {
    let mut lexer = Token::lexer(input);
    let token = lexer.next();
    assert_eq!(token, Some(Ok(expected)), "Failed to match token for input: {}", input);
    assert_eq!(lexer.next(), None, "Expected single token, found more");
}

// ==================== KEYWORD TESTS ====================

#[test]
fn test_keyword_function() {
    assert_single_token("function", Token::Function);
}

#[test]
fn test_keyword_var() {
    assert_single_token("var", Token::Var);
}

#[test]
fn test_keyword_const() {
    assert_single_token("const", Token::Const);
}

#[test]
fn test_keyword_type() {
    assert_single_token("type", Token::Type);
}

#[test]
fn test_keyword_return() {
    assert_single_token("return", Token::Return);
}

#[test]
fn test_keyword_if() {
    assert_single_token("if", Token::If);
}

#[test]
fn test_keyword_else() {
    assert_single_token("else", Token::Else);
}

#[test]
fn test_keyword_true() {
    assert_single_token("true", Token::True);
}

#[test]
fn test_keyword_false() {
    assert_single_token("false", Token::False);
}

#[test]
fn test_keyword_nil() {
    assert_single_token("nil", Token::Nil);
}

#[test]
fn test_keyword_and_symbol() {
    assert_single_token("&&", Token::And);
}

#[test]
fn test_keyword_and_word() {
    assert_single_token("and", Token::And);
}

#[test]
fn test_keyword_or_symbol() {
    assert_single_token("||", Token::Or);
}

#[test]
fn test_keyword_or_word() {
    assert_single_token("or", Token::Or);
}

#[test]
fn test_keyword_not_symbol() {
    assert_single_token("!", Token::Not);
}

#[test]
fn test_keyword_not_word() {
    assert_single_token("not", Token::Not);
}

#[test]
fn test_keyword_while() {
    assert_single_token("while", Token::While);
}

#[test]
fn test_keyword_for() {
    assert_single_token("for", Token::For);
}

#[test]
fn test_keyword_in() {
    assert_single_token("in", Token::In);
}

#[test]
fn test_keyword_import() {
    assert_single_token("import", Token::Import);
}

#[test]
fn test_keyword_as() {
    assert_single_token("as", Token::As);
}

#[test]
fn test_keyword_match() {
    assert_single_token("match", Token::Match);
}

#[test]
fn test_keyword_printf() {
    assert_single_token("printf", Token::Printf);
}

#[test]
fn test_keyword_print() {
    assert_single_token("print", Token::Print);
}

#[test]
fn test_keyword_println() {
    assert_single_token("println", Token::Println);
}

// ==================== IDENTIFIER TESTS ====================

#[test]
fn test_identifier_simple() {
    assert_single_token("x", Token::Identifier("x".to_string()));
}

#[test]
fn test_identifier_underscore_start() {
    assert_single_token("_var", Token::Identifier("_var".to_string()));
}

#[test]
fn test_identifier_with_numbers() {
    assert_single_token("var123", Token::Identifier("var123".to_string()));
}

#[test]
fn test_identifier_snake_case() {
    assert_single_token("my_variable", Token::Identifier("my_variable".to_string()));
}

#[test]
fn test_identifier_camel_case() {
    assert_single_token("myVariable", Token::Identifier("myVariable".to_string()));
}

#[test]
fn test_identifier_uppercase() {
    assert_single_token("CONSTANT", Token::Identifier("CONSTANT".to_string()));
}

#[test]
fn test_identifier_mixed_case() {
    assert_single_token("MyClass", Token::Identifier("MyClass".to_string()));
}

#[test]
fn test_identifier_single_char() {
    assert_single_token("a", Token::Identifier("a".to_string()));
}

#[test]
fn test_identifier_underscore_only() {
    assert_single_token("_", Token::Identifier("_".to_string()));
}

#[test]
fn test_identifier_multiple_underscores() {
    assert_single_token("__private", Token::Identifier("__private".to_string()));
}

// ==================== OPERATOR TESTS ====================

#[test]
fn test_operator_walrus() {
    assert_single_token(":=", Token::ColonEq);
}

#[test]
fn test_operator_eq() {
    assert_single_token("=", Token::Eq);
}

#[test]
fn test_operator_double_eq() {
    assert_single_token("==", Token::DoubleEq);
}

#[test]
fn test_operator_not_eq() {
    assert_single_token("!=", Token::NotEq);
}

#[test]
fn test_operator_plus() {
    assert_single_token("+", Token::Plus);
}

#[test]
fn test_operator_plus_plus() {
    assert_single_token("++", Token::PlusPlus);
}

#[test]
fn test_operator_minus() {
    assert_single_token("-", Token::Minus);
}

#[test]
fn test_operator_minus_minus() {
    assert_single_token("--", Token::MinusMinus);
}

#[test]
fn test_operator_star() {
    assert_single_token("*", Token::Star);
}

#[test]
fn test_operator_slash() {
    assert_single_token("/", Token::Slash);
}

#[test]
fn test_operator_percent() {
    assert_single_token("%", Token::Percent);
}

#[test]
fn test_operator_pow() {
    assert_single_token("**", Token::Pow);
}

#[test]
fn test_operator_plus_eq() {
    assert_single_token("+=", Token::PlusEq);
}

#[test]
fn test_operator_minus_eq() {
    assert_single_token("-=", Token::MinusEq);
}

#[test]
fn test_operator_star_eq() {
    assert_single_token("*=", Token::StarEq);
}

#[test]
fn test_operator_slash_eq() {
    assert_single_token("/=", Token::SlashEq);
}

#[test]
fn test_operator_caret() {
    assert_single_token("^", Token::Caret);
}

#[test]
fn test_operator_pipe() {
    assert_single_token("|", Token::Pipe);
}

#[test]
fn test_operator_gt() {
    assert_single_token(">", Token::Gt);
}

#[test]
fn test_operator_lt() {
    assert_single_token("<", Token::Lt);
}

#[test]
fn test_operator_gt_eq() {
    assert_single_token(">=", Token::GtEq);
}

#[test]
fn test_operator_lt_eq() {
    assert_single_token("<=", Token::LtEq);
}

#[test]
fn test_operator_arrow() {
    assert_single_token("->", Token::Arrow);
}

#[test]
fn test_operator_ampersand() {
    assert_single_token("&", Token::Ampersand);
}

#[test]
fn test_operator_question() {
    assert_single_token("?", Token::Question);
}

#[test]
fn test_operator_comma() {
    assert_single_token(",", Token::Comma);
}

#[test]
fn test_operator_colon() {
    assert_single_token(":", Token::Colon);
}

#[test]
fn test_operator_dot() {
    assert_single_token(".", Token::Dot);
}

// ==================== DELIMITER TESTS ====================

#[test]
fn test_delimiter_lparen() {
    assert_single_token("(", Token::LParen);
}

#[test]
fn test_delimiter_rparen() {
    assert_single_token(")", Token::RParen);
}

#[test]
fn test_delimiter_lbrace() {
    assert_single_token("{", Token::LBrace);
}

#[test]
fn test_delimiter_rbrace() {
    assert_single_token("}", Token::RBrace);
}

#[test]
fn test_delimiter_lbracket() {
    assert_single_token("[", Token::LBracket);
}

#[test]
fn test_delimiter_rbracket() {
    assert_single_token("]", Token::RBracket);
}

// ==================== MULTI-TOKEN TESTS ====================

#[test]
fn test_multiple_tokens_simple_expression() {
    let tokens = tokenize("var x = 10");
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0], Ok(Token::Var));
    assert_eq!(tokens[1], Ok(Token::Identifier("x".to_string())));
    assert_eq!(tokens[2], Ok(Token::Eq));
    assert_eq!(tokens[3], Ok(Token::Int(10)));
}

#[test]
fn test_multiple_tokens_arithmetic() {
    let tokens = tokenize("a + b * c");
    assert_eq!(tokens.len(), 5);
    assert_eq!(tokens[0], Ok(Token::Identifier("a".to_string())));
    assert_eq!(tokens[1], Ok(Token::Plus));
    assert_eq!(tokens[2], Ok(Token::Identifier("b".to_string())));
    assert_eq!(tokens[3], Ok(Token::Star));
    assert_eq!(tokens[4], Ok(Token::Identifier("c".to_string())));
}

#[test]
fn test_multiple_tokens_function_def() {
    let tokens = tokenize("function add(a, b)");
    assert_eq!(tokens.len(), 7);
    assert_eq!(tokens[0], Ok(Token::Function));
    assert_eq!(tokens[1], Ok(Token::Identifier("add".to_string())));
    assert_eq!(tokens[2], Ok(Token::LParen));
    assert_eq!(tokens[3], Ok(Token::Identifier("a".to_string())));
    assert_eq!(tokens[4], Ok(Token::Comma));
    assert_eq!(tokens[5], Ok(Token::Identifier("b".to_string())));
    assert_eq!(tokens[6], Ok(Token::RParen));
}

#[test]
fn test_multiple_tokens_comparison() {
    let tokens = tokenize("x >= 10 && y < 20");
    assert_eq!(tokens.len(), 7);
    assert_eq!(tokens[0], Ok(Token::Identifier("x".to_string())));
    assert_eq!(tokens[1], Ok(Token::GtEq));
    assert_eq!(tokens[2], Ok(Token::Int(10)));
    assert_eq!(tokens[3], Ok(Token::And));
    assert_eq!(tokens[4], Ok(Token::Identifier("y".to_string())));
    assert_eq!(tokens[5], Ok(Token::Lt));
    assert_eq!(tokens[6], Ok(Token::Int(20)));
}

#[test]
fn test_multiple_tokens_if_statement() {
    let tokens = tokenize("if x > 0 { return true }");
    assert_eq!(tokens.len(), 8);
    assert_eq!(tokens[0], Ok(Token::If));
    assert_eq!(tokens[1], Ok(Token::Identifier("x".to_string())));
    assert_eq!(tokens[2], Ok(Token::Gt));
    assert_eq!(tokens[3], Ok(Token::Int(0)));
    assert_eq!(tokens[4], Ok(Token::LBrace));
    assert_eq!(tokens[5], Ok(Token::Return));
    assert_eq!(tokens[6], Ok(Token::True));
    assert_eq!(tokens[7], Ok(Token::RBrace));
}

// ==================== WHITESPACE HANDLING TESTS ====================

#[test]
fn test_whitespace_ignored_spaces() {
    let tokens = tokenize("  var   x   =   10  ");
    assert_eq!(tokens.len(), 4); // Whitespace should be ignored
    assert_eq!(tokens[0], Ok(Token::Var));
    assert_eq!(tokens[1], Ok(Token::Identifier("x".to_string())));
    assert_eq!(tokens[2], Ok(Token::Eq));
    assert_eq!(tokens[3], Ok(Token::Int(10)));
}

#[test]
fn test_whitespace_ignored_tabs() {
    let tokens = tokenize("\tvar\tx\t=\t10\t");
    assert_eq!(tokens.len(), 4);
}

#[test]
fn test_whitespace_ignored_newlines() {
    let tokens = tokenize("var\nx\n=\n10");
    assert_eq!(tokens.len(), 4);
}

#[test]
fn test_whitespace_mixed() {
    let tokens = tokenize("  \t\nvar\n\t  x\t\n  =\n\t10  \t\n");
    assert_eq!(tokens.len(), 4);
}

// ==================== COMMENT TESTS ====================

#[test]
fn test_comment_single_line() {
    let tokens = tokenize("var x = 10 // This is a comment");
    assert_eq!(tokens.len(), 4); // Comment should be ignored
    assert_eq!(tokens[0], Ok(Token::Var));
}

#[test]
fn test_comment_full_line() {
    let tokens = tokenize("// This is a comment\nvar x = 10");
    assert_eq!(tokens.len(), 4);
}

#[test]
fn test_comment_multiple_lines() {
    let tokens = tokenize("// Comment 1\n// Comment 2\nvar x = 10\n// Comment 3");
    assert_eq!(tokens.len(), 4);
}

#[test]
fn test_comment_after_token() {
    let tokens = tokenize("var// comment\nx");
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], Ok(Token::Var));
    assert_eq!(tokens[1], Ok(Token::Identifier("x".to_string())));
}

// ==================== PRIORITY TESTS ====================

#[test]
fn test_priority_double_eq_vs_eq() {
    // == should be matched before =
    assert_single_token("==", Token::DoubleEq);
}

#[test]
fn test_priority_pow_vs_star() {
    // ** should be matched before *
    assert_single_token("**", Token::Pow);
}

#[test]
fn test_priority_plus_plus_vs_plus() {
    // ++ should be matched before +
    assert_single_token("++", Token::PlusPlus);
}

#[test]
fn test_priority_minus_minus_vs_minus() {
    // -- should be matched before -
    assert_single_token("--", Token::MinusMinus);
}

#[test]
fn test_priority_walrus_vs_colon() {
    // := should be matched before :
    assert_single_token(":=", Token::ColonEq);
}

#[test]
fn test_priority_arrow_vs_minus_gt() {
    // -> should be a single token, not - and >
    let tokens = tokenize("->");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], Ok(Token::Arrow));
}

#[test]
fn test_priority_gte_vs_gt_eq() {
    // >= should be a single token
    let tokens = tokenize(">=");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], Ok(Token::GtEq));
}

#[test]
fn test_priority_lte_vs_lt_eq() {
    // <= should be a single token
    let tokens = tokenize("<=");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0], Ok(Token::LtEq));
}

// ==================== KEYWORD VS IDENTIFIER TESTS ====================

#[test]
fn test_keyword_var_vs_identifier_variable() {
    assert_single_token("var", Token::Var);
    assert_single_token("variable", Token::Identifier("variable".to_string()));
}

#[test]
fn test_keyword_if_vs_identifier_iffy() {
    assert_single_token("if", Token::If);
    assert_single_token("iffy", Token::Identifier("iffy".to_string()));
}

#[test]
fn test_keyword_for_vs_identifier_format() {
    assert_single_token("for", Token::For);
    assert_single_token("format", Token::Identifier("format".to_string()));
}

#[test]
fn test_keyword_true_vs_identifier_truth() {
    assert_single_token("true", Token::True);
    assert_single_token("truth", Token::Identifier("truth".to_string()));
}

#[test]
fn test_keyword_false_vs_identifier_falsy() {
    assert_single_token("false", Token::False);
    assert_single_token("falsy", Token::Identifier("falsy".to_string()));
}

#[test]
fn test_keyword_return_vs_identifier_returned() {
    assert_single_token("return", Token::Return);
    assert_single_token("returned", Token::Identifier("returned".to_string()));
}
