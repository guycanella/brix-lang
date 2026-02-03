// Number Literal Tests
//
// Comprehensive tests for integer, float, and imaginary number recognition.
// Tests include edge cases, precision limits, and priority handling.

use crate::token::Token;
use logos::Logos;

// Helper function to tokenize and assert single token
fn assert_single_token(input: &str, expected: Token) {
    let mut lexer = Token::lexer(input);
    let token = lexer.next();
    assert_eq!(
        token,
        Some(Ok(expected)),
        "Failed to match token for input: {}",
        input
    );
    assert_eq!(lexer.next(), None, "Expected single token, found more");
}

// ==================== INTEGER TESTS ====================

#[test]
fn test_int_zero() {
    assert_single_token("0", Token::Int(0));
}

#[test]
fn test_int_positive_small() {
    assert_single_token("42", Token::Int(42));
}

#[test]
fn test_int_positive_large() {
    assert_single_token("123456789", Token::Int(123456789));
}

#[test]
fn test_int_single_digit() {
    assert_single_token("1", Token::Int(1));
    assert_single_token("5", Token::Int(5));
    assert_single_token("9", Token::Int(9));
}

#[test]
fn test_int_two_digits() {
    assert_single_token("10", Token::Int(10));
    assert_single_token("99", Token::Int(99));
}

#[test]
fn test_int_three_digits() {
    assert_single_token("100", Token::Int(100));
    assert_single_token("999", Token::Int(999));
}

#[test]
fn test_int_leading_zeros_not_supported() {
    // In Rust/Brix, 007 would be parsed as Int(7)
    // Logos treats leading zeros as part of the number
    assert_single_token("007", Token::Int(7));
}

#[test]
fn test_int_max_i64() {
    // Maximum i64 value: 9223372036854775807
    // Note: lexer parses as i64, so this should work
    assert_single_token("9223372036854775807", Token::Int(9223372036854775807));
}

// ==================== FLOAT TESTS ====================

#[test]
fn test_float_simple() {
    assert_single_token("3.14", Token::Float("3.14".to_string()));
}

#[test]
fn test_float_zero_point_zero() {
    assert_single_token("0.0", Token::Float("0.0".to_string()));
}

#[test]
fn test_float_zero_point_one() {
    assert_single_token("0.1", Token::Float("0.1".to_string()));
}

#[test]
fn test_float_one_point_zero() {
    assert_single_token("1.0", Token::Float("1.0".to_string()));
}

#[test]
fn test_float_multiple_decimal_places() {
    assert_single_token("3.14159265", Token::Float("3.14159265".to_string()));
}

#[test]
fn test_float_small_decimal() {
    assert_single_token("0.001", Token::Float("0.001".to_string()));
}

#[test]
fn test_float_large_number() {
    assert_single_token("123456.789", Token::Float("123456.789".to_string()));
}

#[test]
fn test_float_many_decimals() {
    assert_single_token(
        "1.23456789012345",
        Token::Float("1.23456789012345".to_string()),
    );
}

#[test]
fn test_float_trailing_zeros() {
    assert_single_token("1.500", Token::Float("1.500".to_string()));
}

#[test]
fn test_float_leading_decimal_zeros() {
    assert_single_token("10.001", Token::Float("10.001".to_string()));
}

// ==================== IMAGINARY LITERAL TESTS ====================

#[test]
fn test_imaginary_integer() {
    assert_single_token("2i", Token::ImaginaryLiteral("2i".to_string()));
}

#[test]
fn test_imaginary_float() {
    assert_single_token("3.14i", Token::ImaginaryLiteral("3.14i".to_string()));
}

#[test]
fn test_imaginary_zero_int() {
    assert_single_token("0i", Token::ImaginaryLiteral("0i".to_string()));
}

#[test]
fn test_imaginary_zero_float() {
    assert_single_token("0.0i", Token::ImaginaryLiteral("0.0i".to_string()));
}

#[test]
fn test_imaginary_one_int() {
    assert_single_token("1i", Token::ImaginaryLiteral("1i".to_string()));
}

#[test]
fn test_imaginary_one_float() {
    assert_single_token("1.0i", Token::ImaginaryLiteral("1.0i".to_string()));
}

#[test]
fn test_imaginary_large() {
    assert_single_token("12345i", Token::ImaginaryLiteral("12345i".to_string()));
}

#[test]
fn test_imaginary_decimal_complex() {
    assert_single_token("2.718i", Token::ImaginaryLiteral("2.718i".to_string()));
}

#[test]
fn test_imaginary_many_decimals() {
    assert_single_token(
        "3.14159265359i",
        Token::ImaginaryLiteral("3.14159265359i".to_string()),
    );
}

// ==================== PRIORITY TESTS ====================
// These tests ensure ImaginaryLiteral has higher priority than Float/Int + Identifier

#[test]
fn test_priority_imaginary_int_vs_int_identifier() {
    // "2i" should be ImaginaryLiteral, NOT Int(2) + Identifier("i")
    let mut lexer = Token::lexer("2i");
    let token = lexer.next();
    assert_eq!(token, Some(Ok(Token::ImaginaryLiteral("2i".to_string()))));
    assert_eq!(lexer.next(), None); // Should be single token
}

#[test]
fn test_priority_imaginary_float_vs_float_identifier() {
    // "3.14i" should be ImaginaryLiteral, NOT Float("3.14") + Identifier("i")
    let mut lexer = Token::lexer("3.14i");
    let token = lexer.next();
    assert_eq!(
        token,
        Some(Ok(Token::ImaginaryLiteral("3.14i".to_string())))
    );
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_priority_regular_int_with_identifier() {
    // "2 i" (with space) should be Int(2) + Identifier("i")
    let mut lexer = Token::lexer("2 i");
    assert_eq!(lexer.next(), Some(Ok(Token::Int(2))));
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("i".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_priority_regular_float_with_identifier() {
    // "3.14 i" (with space) should be Float + Identifier
    let mut lexer = Token::lexer("3.14 i");
    assert_eq!(lexer.next(), Some(Ok(Token::Float("3.14".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("i".to_string()))));
    assert_eq!(lexer.next(), None);
}

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_edge_multiple_zeros() {
    assert_single_token("0000", Token::Int(0));
}

#[test]
fn test_edge_float_zero_many_decimals() {
    assert_single_token("0.000000", Token::Float("0.000000".to_string()));
}

#[test]
fn test_edge_int_sequence() {
    let mut lexer = Token::lexer("1 2 3 4 5");
    assert_eq!(lexer.next(), Some(Ok(Token::Int(1))));
    assert_eq!(lexer.next(), Some(Ok(Token::Int(2))));
    assert_eq!(lexer.next(), Some(Ok(Token::Int(3))));
    assert_eq!(lexer.next(), Some(Ok(Token::Int(4))));
    assert_eq!(lexer.next(), Some(Ok(Token::Int(5))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_edge_float_sequence() {
    let mut lexer = Token::lexer("1.0 2.5 3.14");
    assert_eq!(lexer.next(), Some(Ok(Token::Float("1.0".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Float("2.5".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Float("3.14".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_edge_mixed_number_types() {
    let mut lexer = Token::lexer("42 3.14 2i");
    assert_eq!(lexer.next(), Some(Ok(Token::Int(42))));
    assert_eq!(lexer.next(), Some(Ok(Token::Float("3.14".to_string()))));
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::ImaginaryLiteral("2i".to_string())))
    );
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_edge_numbers_in_arithmetic() {
    let mut lexer = Token::lexer("10 + 3.5 - 2i");
    assert_eq!(lexer.next(), Some(Ok(Token::Int(10))));
    assert_eq!(lexer.next(), Some(Ok(Token::Plus)));
    assert_eq!(lexer.next(), Some(Ok(Token::Float("3.5".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Minus)));
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::ImaginaryLiteral("2i".to_string())))
    );
    assert_eq!(lexer.next(), None);
}

// ==================== NEGATIVE NUMBER TESTS ====================
// Note: In most lexers, negative sign is a separate token

#[test]
fn test_negative_int_tokenizes_as_minus_plus_int() {
    // "-42" should be Minus + Int(42)
    let mut lexer = Token::lexer("-42");
    assert_eq!(lexer.next(), Some(Ok(Token::Minus)));
    assert_eq!(lexer.next(), Some(Ok(Token::Int(42))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_negative_float_tokenizes_as_minus_plus_float() {
    // "-3.14" should be Minus + Float("3.14")
    let mut lexer = Token::lexer("-3.14");
    assert_eq!(lexer.next(), Some(Ok(Token::Minus)));
    assert_eq!(lexer.next(), Some(Ok(Token::Float("3.14".to_string()))));
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_negative_imaginary_tokenizes_as_minus_plus_imaginary() {
    // "-2i" should be Minus + ImaginaryLiteral("2i")
    let mut lexer = Token::lexer("-2i");
    assert_eq!(lexer.next(), Some(Ok(Token::Minus)));
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::ImaginaryLiteral("2i".to_string())))
    );
    assert_eq!(lexer.next(), None);
}

// ==================== SCIENTIFIC NOTATION TESTS ====================
// Note: Current lexer might not support scientific notation (e.g., 1e10)
// These tests document expected behavior if/when implemented

#[test]
fn test_scientific_notation_not_yet_supported() {
    // "1e10" would currently be tokenized as Int(1) + Identifier("e10")
    // This is a documentation test for future implementation
    let mut lexer = Token::lexer("1e10");
    assert_eq!(lexer.next(), Some(Ok(Token::Int(1))));
    assert_eq!(lexer.next(), Some(Ok(Token::Identifier("e10".to_string()))));
}

// ==================== BOUNDARY VALUE TESTS ====================

#[test]
fn test_int_boundary_i64_max() {
    // i64::MAX = 9223372036854775807
    assert_single_token("9223372036854775807", Token::Int(9223372036854775807));
}

#[test]
fn test_int_boundary_i64_min_absolute() {
    // i64::MIN absolute value = 9223372036854775808 (will overflow during parsing)
    // This documents the overflow behavior
    // Note: The parser will handle the minus sign separately
}

#[test]
fn test_float_boundary_very_small() {
    assert_single_token("0.00000001", Token::Float("0.00000001".to_string()));
}

#[test]
fn test_float_boundary_very_large() {
    assert_single_token(
        "999999999.999999999",
        Token::Float("999999999.999999999".to_string()),
    );
}

// ==================== COMPLEX NUMBER EXPRESSION TESTS ====================

#[test]
fn test_complex_literal_expression() {
    // "3.0 + 4.0i" should tokenize as Float + Plus + ImaginaryLiteral
    let mut lexer = Token::lexer("3.0 + 4.0i");
    assert_eq!(lexer.next(), Some(Ok(Token::Float("3.0".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Plus)));
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::ImaginaryLiteral("4.0i".to_string())))
    );
    assert_eq!(lexer.next(), None);
}

#[test]
fn test_complex_literal_subtraction() {
    // "1.0 - 2.0i" should tokenize as Float + Minus + ImaginaryLiteral
    let mut lexer = Token::lexer("1.0 - 2.0i");
    assert_eq!(lexer.next(), Some(Ok(Token::Float("1.0".to_string()))));
    assert_eq!(lexer.next(), Some(Ok(Token::Minus)));
    assert_eq!(
        lexer.next(),
        Some(Ok(Token::ImaginaryLiteral("2.0i".to_string())))
    );
    assert_eq!(lexer.next(), None);
}
