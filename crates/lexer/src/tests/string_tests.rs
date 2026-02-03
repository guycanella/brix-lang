// String and F-String Tests
//
// Comprehensive tests for string literals, f-strings, and escape sequences.
// Tests include empty strings, escaped quotes, newlines, tabs, and complex interpolations.

use crate::token::Token;
use logos::Logos;

// Helper function to tokenize and assert single token
fn assert_single_token(input: &str, expected: Token) {
    let mut lexer = Token::lexer(input);
    let token = lexer.next();
    assert_eq!(token, Some(Ok(expected)), "Failed to match token for input: {}", input);
    assert_eq!(lexer.next(), None, "Expected single token, found more");
}

// ==================== BASIC STRING TESTS ====================

#[test]
fn test_string_simple() {
    assert_single_token(r#""hello""#, Token::String(r#""hello""#.to_string()));
}

#[test]
fn test_string_empty() {
    assert_single_token(r#""""#, Token::String(r#""""#.to_string()));
}

#[test]
fn test_string_single_char() {
    assert_single_token(r#""a""#, Token::String(r#""a""#.to_string()));
}

#[test]
fn test_string_with_spaces() {
    assert_single_token(r#""hello world""#, Token::String(r#""hello world""#.to_string()));
}

#[test]
fn test_string_with_numbers() {
    assert_single_token(r#""test123""#, Token::String(r#""test123""#.to_string()));
}

#[test]
fn test_string_with_symbols() {
    assert_single_token(r#""!@#$%^&*()""#, Token::String(r#""!@#$%^&*()""#.to_string()));
}

#[test]
fn test_string_with_underscores() {
    assert_single_token(r#""hello_world""#, Token::String(r#""hello_world""#.to_string()));
}

#[test]
fn test_string_with_dashes() {
    assert_single_token(r#""hello-world""#, Token::String(r#""hello-world""#.to_string()));
}

#[test]
fn test_string_long() {
    let long_str = r#""This is a very long string with many words and characters that tests the lexer's ability to handle longer text content""#;
    assert_single_token(long_str, Token::String(long_str.to_string()));
}

// ==================== ESCAPE SEQUENCE TESTS ====================

#[test]
fn test_string_escape_newline() {
    assert_single_token(r#""hello\nworld""#, Token::String(r#""hello\nworld""#.to_string()));
}

#[test]
fn test_string_escape_tab() {
    assert_single_token(r#""hello\tworld""#, Token::String(r#""hello\tworld""#.to_string()));
}

#[test]
fn test_string_escape_carriage_return() {
    assert_single_token(r#""hello\rworld""#, Token::String(r#""hello\rworld""#.to_string()));
}

#[test]
fn test_string_escape_backslash() {
    assert_single_token(r#""hello\\world""#, Token::String(r#""hello\\world""#.to_string()));
}

#[test]
fn test_string_escape_quote() {
    assert_single_token(r#""He said \"Hello\"""#, Token::String(r#""He said \"Hello\"""#.to_string()));
}

#[test]
fn test_string_escape_backspace() {
    assert_single_token(r#""hello\bworld""#, Token::String(r#""hello\bworld""#.to_string()));
}

#[test]
fn test_string_escape_formfeed() {
    assert_single_token(r#""hello\fworld""#, Token::String(r#""hello\fworld""#.to_string()));
}

#[test]
fn test_string_multiple_escapes() {
    assert_single_token(r#""Line1\nLine2\tTabbed""#, Token::String(r#""Line1\nLine2\tTabbed""#.to_string()));
}

#[test]
fn test_string_all_escapes() {
    assert_single_token(r#""Test:\n\t\r\\\"\b\f""#, Token::String(r#""Test:\n\t\r\\\"\b\f""#.to_string()));
}

#[test]
fn test_string_escape_only() {
    assert_single_token(r#""\n\t\r""#, Token::String(r#""\n\t\r""#.to_string()));
}

// ==================== F-STRING BASIC TESTS ====================

#[test]
fn test_fstring_simple() {
    assert_single_token(r#"f"hello""#, Token::FString(r#"f"hello""#.to_string()));
}

#[test]
fn test_fstring_empty() {
    assert_single_token(r#"f"""#, Token::FString(r#"f"""#.to_string()));
}

#[test]
fn test_fstring_with_simple_interpolation() {
    assert_single_token(r#"f"Value: {x}""#, Token::FString(r#"f"Value: {x}""#.to_string()));
}

#[test]
fn test_fstring_with_multiple_interpolations() {
    assert_single_token(r#"f"x={x}, y={y}""#, Token::FString(r#"f"x={x}, y={y}""#.to_string()));
}

#[test]
fn test_fstring_with_expression() {
    assert_single_token(r#"f"Result: {x + y}""#, Token::FString(r#"f"Result: {x + y}""#.to_string()));
}

#[test]
fn test_fstring_with_nested_braces() {
    assert_single_token(r#"f"Array: {[1, 2, 3]}""#, Token::FString(r#"f"Array: {[1, 2, 3]}""#.to_string()));
}

// ==================== F-STRING WITH ESCAPE SEQUENCES ====================

#[test]
fn test_fstring_escape_newline() {
    assert_single_token(r#"f"Line1\nLine2""#, Token::FString(r#"f"Line1\nLine2""#.to_string()));
}

#[test]
fn test_fstring_escape_tab() {
    assert_single_token(r#"f"Name:\t{name}""#, Token::FString(r#"f"Name:\t{name}""#.to_string()));
}

#[test]
fn test_fstring_escape_quote() {
    // This is the CRITICAL test that was fixed in v1.1
    assert_single_token(r#"f"He said \"Hello\"""#, Token::FString(r#"f"He said \"Hello\"""#.to_string()));
}

#[test]
fn test_fstring_escape_quote_with_interpolation() {
    assert_single_token(r#"f"He said \"{name}\" to me""#, Token::FString(r#"f"He said \"{name}\" to me""#.to_string()));
}

#[test]
fn test_fstring_escape_backslash() {
    assert_single_token(r#"f"Path: C:\\Users\\{user}""#, Token::FString(r#"f"Path: C:\\Users\\{user}""#.to_string()));
}

#[test]
fn test_fstring_multiple_escapes_with_interpolation() {
    assert_single_token(r#"f"Name:\t{name}\nAge:\t{age}""#, Token::FString(r#"f"Name:\t{name}\nAge:\t{age}""#.to_string()));
}

// ==================== F-STRING WITH FORMAT SPECIFIERS ====================

#[test]
fn test_fstring_format_float_precision() {
    assert_single_token(r#"f"Pi: {pi:.2f}""#, Token::FString(r#"f"Pi: {pi:.2f}""#.to_string()));
}

#[test]
fn test_fstring_format_hex() {
    assert_single_token(r#"f"Hex: {num:x}""#, Token::FString(r#"f"Hex: {num:x}""#.to_string()));
}

#[test]
fn test_fstring_format_octal() {
    assert_single_token(r#"f"Octal: {num:o}""#, Token::FString(r#"f"Octal: {num:o}""#.to_string()));
}

#[test]
fn test_fstring_format_scientific() {
    assert_single_token(r#"f"Scientific: {num:.2e}""#, Token::FString(r#"f"Scientific: {num:.2e}""#.to_string()));
}

#[test]
fn test_fstring_mixed_formats() {
    assert_single_token(r#"f"x={x:x}, y={y:.2f}""#, Token::FString(r#"f"x={x:x}, y={y:.2f}""#.to_string()));
}

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_string_only_whitespace() {
    assert_single_token(r#""   ""#, Token::String(r#""   ""#.to_string()));
}

#[test]
fn test_string_only_tabs() {
    assert_single_token(r#""\t\t\t""#, Token::String(r#""\t\t\t""#.to_string()));
}

#[test]
fn test_string_only_newlines() {
    assert_single_token(r#""\n\n\n""#, Token::String(r#""\n\n\n""#.to_string()));
}

#[test]
fn test_fstring_no_interpolation() {
    assert_single_token(r#"f"just text""#, Token::FString(r#"f"just text""#.to_string()));
}

#[test]
fn test_fstring_only_interpolation() {
    assert_single_token(r#"f"{x}""#, Token::FString(r#"f"{x}""#.to_string()));
}

#[test]
fn test_fstring_empty_interpolation() {
    // Empty braces - might be an error but lexer should accept it
    assert_single_token(r#"f"text {}""#, Token::FString(r#"f"text {}""#.to_string()));
}

// ==================== UNICODE TESTS ====================

#[test]
fn test_string_unicode_emoji() {
    assert_single_token(r#""Hello 👋""#, Token::String(r#""Hello 👋""#.to_string()));
}

#[test]
fn test_string_unicode_chinese() {
    assert_single_token(r#""你好世界""#, Token::String(r#""你好世界""#.to_string()));
}

#[test]
fn test_string_unicode_arabic() {
    assert_single_token(r#""مرحبا بالعالم""#, Token::String(r#""مرحبا بالعالم""#.to_string()));
}

#[test]
fn test_string_unicode_mixed() {
    assert_single_token(r#""Hello 世界 🌍""#, Token::String(r#""Hello 世界 🌍""#.to_string()));
}

#[test]
fn test_fstring_unicode_with_interpolation() {
    assert_single_token(r#"f"Hello {name} 👋""#, Token::FString(r#"f"Hello {name} 👋""#.to_string()));
}

// ==================== MULTI-TOKEN STRING TESTS ====================

#[test]
fn test_multiple_strings_concatenation() {
    let mut lexer = Token::lexer(r#""hello" + "world""#);
    assert_eq!(lexer.next(), Some(Ok(Token::String(r#""hello""#.to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Plus)));
    assert_eq!(lexer.next(), Some(Ok(Token::String(r#""world""#.to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_string_in_expression() {
    let mut lexer = Token::lexer(r#"var msg = "hello""#);
    assert_eq!(lexer.next(), Some(Ok(Token::Var)));
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("msg".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Eq)));
    assert_eq!(lexer.next(), Some(Ok(Token::String(r#""hello""#.to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_fstring_in_print() {
    let mut lexer = Token::lexer(r#"println(f"x = {x}")"#);
    assert_eq!(lexer.next(), Some(Ok(Token::Println)));
    assert_eq!(lexer.next(), Some(Ok(Token::LParen)));
    assert_eq!(lexer.next(), Some(Ok(Token::FString(r#"f"x = {x}""#.to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::RParen)));
    assert_eq!(lexer.next(), None);
}

// ==================== REGRESSION TESTS (v1.1 fixes) ====================

#[test]
fn test_regression_fstring_escaped_quotes() {
    // This was the bug fixed in v1.1 - escaped quotes in f-strings
    let input = r#"f"He said \"Hello\" to me""#;
    assert_single_token(input, Token::FString(input.to_string()));
}

#[test]
fn test_regression_fstring_multiple_escaped_quotes() {
    let input = r#"f"\"Start\" and \"End\"""#;
    assert_single_token(input, Token::FString(input.to_string()));
}

#[test]
fn test_regression_string_escaped_quotes() {
    let input = r#""Quote: \"text\"""#;
    assert_single_token(input, Token::String(input.to_string()));
}

#[test]
fn test_regression_fstring_backslash_before_brace() {
    // Test that \{ is handled correctly
    let input = r#"f"Literal brace: \\{x}""#;
    assert_single_token(input, Token::FString(input.to_string()));
}

// ==================== COMPLEX ESCAPE PATTERNS ====================

#[test]
fn test_complex_escape_pattern_printf_style() {
    assert_single_token(r#""Name:\t%s\nAge:\t%d\n""#, Token::String(r#""Name:\t%s\nAge:\t%d\n""#.to_string()));
}

#[test]
fn test_complex_escape_pattern_path() {
    assert_single_token(r#""C:\\Users\\Admin\\file.txt""#, Token::String(r#""C:\\Users\\Admin\\file.txt""#.to_string()));
}

#[test]
fn test_complex_escape_pattern_json_like() {
    assert_single_token(r#""{\n\t\"name\": \"value\"\n}""#, Token::String(r#""{\n\t\"name\": \"value\"\n}""#.to_string()));
}

// ==================== BOUNDARY TESTS ====================

#[test]
fn test_boundary_very_long_string() {
    let long_str = "\"".to_string() + &"a".repeat(1000) + "\"";
    assert_single_token(&long_str, Token::String(long_str.clone()));
}

#[test]
fn test_boundary_many_escapes() {
    let many_escapes = r#""\n\n\n\n\n\n\n\n\n\n\t\t\t\t\t\t""#;
    assert_single_token(many_escapes, Token::String(many_escapes.to_string()));
}

#[test]
fn test_boundary_fstring_many_interpolations() {
    let input = r#"f"{a}{b}{c}{d}{e}{f}{g}{h}""#;
    assert_single_token(input, Token::FString(input.to_string()));
}
