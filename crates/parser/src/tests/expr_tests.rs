// Expression Parsing Tests
//
// Comprehensive tests for all expression types in the Brix parser.
// Tests ensure correct AST construction for literals, operators, function calls,
// array access, field access, and complex nested expressions.

use crate::ast::{BinaryOp, Expr, FStringPart, Literal, UnaryOp};
use crate::parser::parser;
use chumsky::Parser;
use lexer::token::Token;

// Helper to parse expression from source and extract first statement's expression
fn parse_expr(input: &str) -> Result<Expr, String> {
    let tokens: Vec<Token> = lexer::lex(input);

    let program = parser()
        .parse(tokens)
        .map_err(|e| format!("Parse error: {:?}", e))?;

    // Extract expression from first statement
    if let Some(stmt) = program.statements.first() {
        if let crate::ast::Stmt::Expr(expr) = stmt {
            Ok(expr.clone())
        } else {
            Err("First statement is not an expression".to_string())
        }
    } else {
        Err("No statements in program".to_string())
    }
}

// ==================== LITERAL TESTS ====================

#[test]
fn test_literal_int() {
    let expr = parse_expr("42").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Int(42)));
}

#[test]
fn test_literal_float() {
    let expr = parse_expr("3.14").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Float(3.14)));
}

#[test]
fn test_literal_string() {
    let expr = parse_expr(r#""hello""#).unwrap();
    assert_eq!(expr, Expr::Literal(Literal::String("hello".to_string())));
}

#[test]
fn test_literal_bool_true() {
    let expr = parse_expr("true").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Bool(true)));
}

#[test]
fn test_literal_bool_false() {
    let expr = parse_expr("false").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Bool(false)));
}

#[test]
fn test_literal_nil() {
    let expr = parse_expr("nil").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Nil));
}

#[test]
fn test_literal_atom() {
    let expr = parse_expr(":ok").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Atom("ok".to_string())));
}

#[test]
fn test_literal_complex() {
    let expr = parse_expr("3.0 + 4.0i").unwrap();
    // This should parse as Binary(Add, Float(3.0), ImaginaryLiteral)
    // Complex literal is constructed during codegen, not parsing
    match expr {
        Expr::Binary { op: BinaryOp::Add, .. } => {}, // OK
        _ => panic!("Expected binary addition for complex literal"),
    }
}

// ==================== IDENTIFIER TESTS ====================

#[test]
fn test_identifier_simple() {
    let expr = parse_expr("x").unwrap();
    assert_eq!(expr, Expr::Identifier("x".to_string()));
}

#[test]
fn test_identifier_snake_case() {
    let expr = parse_expr("my_variable").unwrap();
    assert_eq!(expr, Expr::Identifier("my_variable".to_string()));
}

#[test]
fn test_identifier_camel_case() {
    let expr = parse_expr("myVariable").unwrap();
    assert_eq!(expr, Expr::Identifier("myVariable".to_string()));
}

// ==================== BINARY OPERATOR TESTS ====================

#[test]
fn test_binary_add() {
    let expr = parse_expr("1 + 2").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::Add, lhs, rhs } => {
            assert_eq!(*lhs, Expr::Literal(Literal::Int(1)));
            assert_eq!(*rhs, Expr::Literal(Literal::Int(2)));
        }
        _ => panic!("Expected binary add"),
    }
}

#[test]
fn test_binary_sub() {
    let expr = parse_expr("5 - 3").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::Sub, lhs, rhs } => {
            assert_eq!(*lhs, Expr::Literal(Literal::Int(5)));
            assert_eq!(*rhs, Expr::Literal(Literal::Int(3)));
        }
        _ => panic!("Expected binary sub"),
    }
}

#[test]
fn test_binary_mul() {
    let expr = parse_expr("2 * 3").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::Mul, .. } => {}
        _ => panic!("Expected binary mul"),
    }
}

#[test]
fn test_binary_div() {
    let expr = parse_expr("10 / 2").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::Div, .. } => {}
        _ => panic!("Expected binary div"),
    }
}

#[test]
fn test_binary_mod() {
    let expr = parse_expr("10 % 3").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::Mod, .. } => {}
        _ => panic!("Expected binary mod"),
    }
}

#[test]
fn test_binary_pow() {
    let expr = parse_expr("2 ** 3").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::Pow, .. } => {}
        _ => panic!("Expected binary pow"),
    }
}

#[test]
fn test_binary_bit_and() {
    let expr = parse_expr("5 & 3").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::BitAnd, .. } => {}
        _ => panic!("Expected binary bit and"),
    }
}

#[test]
fn test_binary_bit_or() {
    let expr = parse_expr("5 | 3").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::BitOr, .. } => {}
        _ => panic!("Expected binary bit or"),
    }
}

#[test]
fn test_binary_bit_xor() {
    let expr = parse_expr("5 ^ 3").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::BitXor, .. } => {}
        _ => panic!("Expected binary bit xor"),
    }
}

#[test]
fn test_binary_eq() {
    let expr = parse_expr("x == 10").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::Eq, .. } => {}
        _ => panic!("Expected binary eq"),
    }
}

#[test]
fn test_binary_not_eq() {
    let expr = parse_expr("x != 10").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::NotEq, .. } => {}
        _ => panic!("Expected binary not eq"),
    }
}

#[test]
fn test_binary_lt() {
    let expr = parse_expr("x < 10").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::Lt, .. } => {}
        _ => panic!("Expected binary lt"),
    }
}

#[test]
fn test_binary_gt() {
    let expr = parse_expr("x > 10").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::Gt, .. } => {}
        _ => panic!("Expected binary gt"),
    }
}

#[test]
fn test_binary_lteq() {
    let expr = parse_expr("x <= 10").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::LtEq, .. } => {}
        _ => panic!("Expected binary lteq"),
    }
}

#[test]
fn test_binary_gteq() {
    let expr = parse_expr("x >= 10").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::GtEq, .. } => {}
        _ => panic!("Expected binary gteq"),
    }
}

#[test]
fn test_binary_logical_and() {
    let expr = parse_expr("x && y").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::LogicalAnd, .. } => {}
        _ => panic!("Expected binary logical and"),
    }
}

#[test]
fn test_binary_logical_or() {
    let expr = parse_expr("x || y").unwrap();
    match expr {
        Expr::Binary { op: BinaryOp::LogicalOr, .. } => {}
        _ => panic!("Expected binary logical or"),
    }
}

// ==================== UNARY OPERATOR TESTS ====================

#[test]
fn test_unary_not() {
    let expr = parse_expr("!x").unwrap();
    match expr {
        Expr::Unary { op: UnaryOp::Not, expr } => {
            assert_eq!(*expr, Expr::Identifier("x".to_string()));
        }
        _ => panic!("Expected unary not"),
    }
}

#[test]
fn test_unary_not_word() {
    let expr = parse_expr("not x").unwrap();
    match expr {
        Expr::Unary { op: UnaryOp::Not, .. } => {}
        _ => panic!("Expected unary not"),
    }
}

#[test]
fn test_unary_negate() {
    let expr = parse_expr("-x").unwrap();
    match expr {
        Expr::Unary { op: UnaryOp::Negate, expr } => {
            assert_eq!(*expr, Expr::Identifier("x".to_string()));
        }
        _ => panic!("Expected unary negate"),
    }
}

#[test]
fn test_unary_negate_number() {
    let expr = parse_expr("-42").unwrap();
    match expr {
        Expr::Unary { op: UnaryOp::Negate, expr } => {
            assert_eq!(*expr, Expr::Literal(Literal::Int(42)));
        }
        _ => panic!("Expected unary negate"),
    }
}

// ==================== INCREMENT/DECREMENT TESTS ====================

#[test]
fn test_increment_prefix() {
    let expr = parse_expr("++x").unwrap();
    match expr {
        Expr::Increment { expr, is_prefix } => {
            assert_eq!(*expr, Expr::Identifier("x".to_string()));
            assert_eq!(is_prefix, true);
        }
        _ => panic!("Expected prefix increment"),
    }
}

#[test]
fn test_increment_postfix() {
    let expr = parse_expr("x++").unwrap();
    match expr {
        Expr::Increment { expr, is_prefix } => {
            assert_eq!(*expr, Expr::Identifier("x".to_string()));
            assert_eq!(is_prefix, false);
        }
        _ => panic!("Expected postfix increment"),
    }
}

#[test]
fn test_decrement_prefix() {
    let expr = parse_expr("--x").unwrap();
    match expr {
        Expr::Decrement { expr, is_prefix } => {
            assert_eq!(*expr, Expr::Identifier("x".to_string()));
            assert_eq!(is_prefix, true);
        }
        _ => panic!("Expected prefix decrement"),
    }
}

#[test]
fn test_decrement_postfix() {
    let expr = parse_expr("x--").unwrap();
    match expr {
        Expr::Decrement { expr, is_prefix } => {
            assert_eq!(*expr, Expr::Identifier("x".to_string()));
            assert_eq!(is_prefix, false);
        }
        _ => panic!("Expected postfix decrement"),
    }
}

// ==================== TERNARY OPERATOR TESTS ====================

#[test]
fn test_ternary_simple() {
    let expr = parse_expr("x > 0 ? 1 : 0").unwrap();
    match expr {
        Expr::Ternary { condition, then_expr, else_expr } => {
            // Condition should be binary comparison
            match *condition {
                Expr::Binary { op: BinaryOp::Gt, .. } => {}
                _ => panic!("Expected gt comparison in condition"),
            }
            assert_eq!(*then_expr, Expr::Literal(Literal::Int(1)));
            assert_eq!(*else_expr, Expr::Literal(Literal::Int(0)));
        }
        _ => panic!("Expected ternary"),
    }
}

#[test]
#[ignore = "Feature not implemented: nested ternary operators"]
fn test_ternary_nested() {
    let expr = parse_expr("x > 10 ? 1 : x > 5 ? 2 : 3").unwrap();
    match expr {
        Expr::Ternary { .. } => {} // Just verify it parses
        _ => panic!("Expected ternary"),
    }
}

// ==================== ARRAY TESTS ====================

#[test]
fn test_array_empty() {
    let expr = parse_expr("[]").unwrap();
    assert_eq!(expr, Expr::Array(vec![]));
}

#[test]
fn test_array_single_element() {
    let expr = parse_expr("[1]").unwrap();
    assert_eq!(expr, Expr::Array(vec![Expr::Literal(Literal::Int(1))]));
}

#[test]
fn test_array_multiple_elements() {
    let expr = parse_expr("[1, 2, 3]").unwrap();
    match expr {
        Expr::Array(elements) => {
            assert_eq!(elements.len(), 3);
            assert_eq!(elements[0], Expr::Literal(Literal::Int(1)));
            assert_eq!(elements[1], Expr::Literal(Literal::Int(2)));
            assert_eq!(elements[2], Expr::Literal(Literal::Int(3)));
        }
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_array_mixed_types() {
    let expr = parse_expr("[1, 2.5, 3]").unwrap();
    match expr {
        Expr::Array(elements) => {
            assert_eq!(elements.len(), 3);
        }
        _ => panic!("Expected array"),
    }
}

// ==================== INDEX ACCESS TESTS ====================

#[test]
fn test_index_1d() {
    let expr = parse_expr("arr[0]").unwrap();
    match expr {
        Expr::Index { array, indices } => {
            assert_eq!(*array, Expr::Identifier("arr".to_string()));
            assert_eq!(indices.len(), 1);
            assert_eq!(indices[0], Expr::Literal(Literal::Int(0)));
        }
        _ => panic!("Expected index"),
    }
}

#[test]
fn test_index_2d() {
    let expr = parse_expr("matrix[0][1]").unwrap();
    match expr {
        Expr::Index { array, indices } => {
            assert_eq!(*array, Expr::Identifier("matrix".to_string()));
            assert_eq!(indices.len(), 2);
        }
        _ => panic!("Expected index"),
    }
}

#[test]
fn test_index_expression() {
    let expr = parse_expr("arr[i + 1]").unwrap();
    match expr {
        Expr::Index { indices, .. } => {
            match &indices[0] {
                Expr::Binary { op: BinaryOp::Add, .. } => {}
                _ => panic!("Expected binary add in index"),
            }
        }
        _ => panic!("Expected index"),
    }
}

// ==================== FUNCTION CALL TESTS ====================

#[test]
fn test_call_no_args() {
    let expr = parse_expr("foo()").unwrap();
    match expr {
        Expr::Call { func, args } => {
            assert_eq!(*func, Expr::Identifier("foo".to_string()));
            assert_eq!(args.len(), 0);
        }
        _ => panic!("Expected call"),
    }
}

#[test]
fn test_call_single_arg() {
    let expr = parse_expr("foo(42)").unwrap();
    match expr {
        Expr::Call { func, args } => {
            assert_eq!(*func, Expr::Identifier("foo".to_string()));
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Expr::Literal(Literal::Int(42)));
        }
        _ => panic!("Expected call"),
    }
}

#[test]
fn test_call_multiple_args() {
    let expr = parse_expr("add(1, 2)").unwrap();
    match expr {
        Expr::Call { func, args } => {
            assert_eq!(*func, Expr::Identifier("add".to_string()));
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected call"),
    }
}

#[test]
fn test_call_nested() {
    let expr = parse_expr("foo(bar(1))").unwrap();
    match expr {
        Expr::Call { func, args } => {
            assert_eq!(*func, Expr::Identifier("foo".to_string()));
            assert_eq!(args.len(), 1);
            match &args[0] {
                Expr::Call { .. } => {} // Nested call
                _ => panic!("Expected nested call"),
            }
        }
        _ => panic!("Expected call"),
    }
}

// ==================== FIELD ACCESS TESTS ====================

#[test]
fn test_field_access_simple() {
    let expr = parse_expr("obj.field").unwrap();
    match expr {
        Expr::FieldAccess { target, field } => {
            assert_eq!(*target, Expr::Identifier("obj".to_string()));
            assert_eq!(field, "field");
        }
        _ => panic!("Expected field access"),
    }
}

#[test]
fn test_field_access_chained() {
    let expr = parse_expr("obj.field.subfield").unwrap();
    match expr {
        Expr::FieldAccess { target, field } => {
            assert_eq!(field, "subfield");
            match *target {
                Expr::FieldAccess { .. } => {} // Chained access
                _ => panic!("Expected chained field access"),
            }
        }
        _ => panic!("Expected field access"),
    }
}

// ==================== RANGE TESTS ====================

#[test]
fn test_range_simple() {
    let expr = parse_expr("1:10").unwrap();
    match expr {
        Expr::Range { start, end, step } => {
            assert_eq!(*start, Expr::Literal(Literal::Int(1)));
            assert_eq!(*end, Expr::Literal(Literal::Int(10)));
            assert!(step.is_none());
        }
        _ => panic!("Expected range"),
    }
}

#[test]
fn test_range_with_step() {
    let expr = parse_expr("0:2:10").unwrap();
    match expr {
        Expr::Range { start, end, step } => {
            assert_eq!(*start, Expr::Literal(Literal::Int(0)));
            assert!(step.is_some());
            assert_eq!(*end, Expr::Literal(Literal::Int(10)));
        }
        _ => panic!("Expected range with step"),
    }
}

#[test]
#[ignore = "Lexer issue: :end is tokenized as atom, not colon + identifier"]
fn test_range_with_variables() {
    let expr = parse_expr("start:end").unwrap();
    match expr {
        Expr::Range { start, end, step } => {
            assert_eq!(*start, Expr::Identifier("start".to_string()));
            assert_eq!(*end, Expr::Identifier("end".to_string()));
            assert!(step.is_none());
        }
        _ => panic!("Expected range"),
    }
}

// ==================== STATIC INIT TESTS ====================

#[test]
fn test_static_init_int_1d() {
    let expr = parse_expr("int[5]").unwrap();
    match expr {
        Expr::StaticInit { element_type, dimensions } => {
            assert_eq!(element_type, "int");
            assert_eq!(dimensions.len(), 1);
            assert_eq!(dimensions[0], Expr::Literal(Literal::Int(5)));
        }
        _ => panic!("Expected static init"),
    }
}

#[test]
fn test_static_init_float_2d() {
    let expr = parse_expr("float[3, 4]").unwrap();
    match expr {
        Expr::StaticInit { element_type, dimensions } => {
            assert_eq!(element_type, "float");
            assert_eq!(dimensions.len(), 2);
        }
        _ => panic!("Expected static init"),
    }
}

// ==================== F-STRING TESTS ====================

#[test]
fn test_fstring_text_only() {
    let expr = parse_expr(r#"f"hello""#).unwrap();
    match expr {
        Expr::FString { parts } => {
            assert_eq!(parts.len(), 1);
            match &parts[0] {
                FStringPart::Text(text) => assert_eq!(text, "hello"),
                _ => panic!("Expected text part"),
            }
        }
        _ => panic!("Expected fstring"),
    }
}

#[test]
fn test_fstring_with_interpolation() {
    let expr = parse_expr(r#"f"x = {x}""#).unwrap();
    match expr {
        Expr::FString { parts } => {
            assert!(parts.len() >= 2); // Should have text and expr parts
        }
        _ => panic!("Expected fstring"),
    }
}

// ==================== COMPLEX NESTED EXPRESSIONS ====================

#[test]
fn test_complex_arithmetic() {
    let expr = parse_expr("1 + 2 * 3").unwrap();
    // Should parse as 1 + (2 * 3) due to precedence
    match expr {
        Expr::Binary { op: BinaryOp::Add, lhs, rhs } => {
            assert_eq!(*lhs, Expr::Literal(Literal::Int(1)));
            match *rhs {
                Expr::Binary { op: BinaryOp::Mul, .. } => {} // Good
                _ => panic!("Expected multiplication on right side"),
            }
        }
        _ => panic!("Expected addition"),
    }
}

#[test]
fn test_complex_with_parens() {
    let expr = parse_expr("(1 + 2) * 3").unwrap();
    // Parentheses should change precedence
    match expr {
        Expr::Binary { op: BinaryOp::Mul, lhs, rhs } => {
            match *lhs {
                Expr::Binary { op: BinaryOp::Add, .. } => {}
                _ => panic!("Expected addition on left side"),
            }
            assert_eq!(*rhs, Expr::Literal(Literal::Int(3)));
        }
        _ => panic!("Expected multiplication"),
    }
}

#[test]
fn test_deeply_nested() {
    let expr = parse_expr("((((1))))").unwrap();
    assert_eq!(expr, Expr::Literal(Literal::Int(1)));
}
