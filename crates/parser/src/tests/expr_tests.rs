// Expression Parsing Tests
//
// Comprehensive tests for all expression types in the Brix parser.
// Tests ensure correct AST construction for literals, operators, function calls,
// array access, field access, and complex nested expressions.

use crate::ast::{BinaryOp, Expr, ExprKind, FStringPart, Literal, StmtKind, UnaryOp};
use crate::parser::parser;
use chumsky::Parser;
use lexer::token::Token;

// Helper to parse expression from source and extract first statement's expression.
// NOTE: `lexer::lex()` (used below) discards span information entirely — when
// chumsky parses a bare `Vec<Token>` (no explicit `Stream`), it falls back to
// token-INDEX pseudo-spans, not real source byte offsets. That's fine for
// every test that only inspects the resulting AST shape, but it makes this
// helper unusable for anything that depends on real whitespace adjacency
// (e.g. the const-generic `<` vs. comparison `<` disambiguation) — use
// `parse_expr_with_real_spans` below for those instead.
fn parse_expr(input: &str) -> Result<Expr, String> {
    let tokens: Vec<Token> = lexer::lex(input);

    let program = parser()
        .parse(tokens)
        .map_err(|e| format!("Parse error: {:?}", e))?;

    // Extract expression from first statement
    if let Some(stmt) = program.statements.first() {
        if let StmtKind::Expr(expr) = &stmt.kind {
            Ok(expr.clone())
        } else {
            Err("First statement is not an expression".to_string())
        }
    } else {
        Err("No statements in program".to_string())
    }
}

// Same as `parse_expr`, but lexes with real source byte-offset spans (via
// `Token::lexer(...).spanned()` + `chumsky::Stream`), matching exactly how
// the production pipeline (`src/main.rs`'s `compile_and_run_isolated`,
// `codegen::lex_and_parse_program`) actually lexes. Required for tests that
// depend on real whitespace adjacency between tokens — `parse_expr`'s
// token-index pseudo-spans can't distinguish `Embedding<1536>` from
// `Embedding <1536` at all.
fn parse_expr_with_real_spans(input: &str) -> Result<Expr, String> {
    use logos::Logos;
    let tokens_with_spans: Vec<(Token, std::ops::Range<usize>)> = Token::lexer(input)
        .spanned()
        .map(|(t, span)| (t.unwrap_or(Token::Error), span))
        .collect();

    let token_stream =
        chumsky::Stream::from_iter(input.len()..input.len() + 1, tokens_with_spans.into_iter());

    let program = parser()
        .parse(token_stream)
        .map_err(|e| format!("Parse error: {:?}", e))?;

    if let Some(stmt) = program.statements.first() {
        if let StmtKind::Expr(expr) = &stmt.kind {
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
    assert_eq!(expr.kind, ExprKind::Literal(Literal::Int(42)));
}

#[test]
fn test_literal_float() {
    let expr = parse_expr("3.14").unwrap();
    assert_eq!(expr.kind, ExprKind::Literal(Literal::Float(3.14)));
}

#[test]
fn test_literal_float_scientific_integer_mantissa() {
    // "1e10" is a float literal (v1.8 scientific notation)
    let expr = parse_expr("1e10").unwrap();
    assert_eq!(expr.kind, ExprKind::Literal(Literal::Float(1e10)));
}

#[test]
fn test_literal_float_scientific_decimal_mantissa() {
    let expr = parse_expr("6.02e23").unwrap();
    assert_eq!(expr.kind, ExprKind::Literal(Literal::Float(6.02e23)));
}

#[test]
fn test_literal_float_scientific_negative_exponent() {
    let expr = parse_expr("1.5e-10").unwrap();
    assert_eq!(expr.kind, ExprKind::Literal(Literal::Float(1.5e-10)));
}

#[test]
fn test_literal_string() {
    let expr = parse_expr(r#""hello""#).unwrap();
    assert_eq!(
        expr.kind,
        ExprKind::Literal(Literal::String("hello".to_string()))
    );
}

#[test]
fn test_literal_bool_true() {
    let expr = parse_expr("true").unwrap();
    assert_eq!(expr.kind, ExprKind::Literal(Literal::Bool(true)));
}

#[test]
fn test_literal_bool_false() {
    let expr = parse_expr("false").unwrap();
    assert_eq!(expr.kind, ExprKind::Literal(Literal::Bool(false)));
}

#[test]
fn test_literal_nil() {
    let expr = parse_expr("nil").unwrap();
    assert_eq!(expr.kind, ExprKind::Literal(Literal::Nil));
}

#[test]
fn test_literal_atom() {
    let expr = parse_expr(":ok").unwrap();
    assert_eq!(
        expr.kind,
        ExprKind::Literal(Literal::Atom("ok".to_string()))
    );
}

#[test]
fn test_literal_complex() {
    let expr = parse_expr("3.0 + 4.0i").unwrap();
    // This should parse as Binary(Add, Float(3.0), ImaginaryLiteral)
    // Complex literal is constructed during codegen, not parsing
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Add, ..
        } => {} // OK
        _ => panic!("Expected binary addition for complex literal"),
    }
}

// ==================== IDENTIFIER TESTS ====================

#[test]
fn test_identifier_simple() {
    let expr = parse_expr("x").unwrap();
    assert_eq!(expr.kind, ExprKind::Identifier("x".to_string()));
}

#[test]
fn test_identifier_snake_case() {
    let expr = parse_expr("my_variable").unwrap();
    assert_eq!(expr.kind, ExprKind::Identifier("my_variable".to_string()));
}

#[test]
fn test_identifier_camel_case() {
    let expr = parse_expr("myVariable").unwrap();
    assert_eq!(expr.kind, ExprKind::Identifier("myVariable".to_string()));
}

// ==================== BINARY OPERATOR TESTS ====================

#[test]
fn test_binary_add() {
    let expr = parse_expr("1 + 2").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Add,
            lhs,
            rhs,
        } => {
            assert_eq!(lhs.kind, ExprKind::Literal(Literal::Int(1)));
            assert_eq!(rhs.kind, ExprKind::Literal(Literal::Int(2)));
        }
        _ => panic!("Expected binary add"),
    }
}

#[test]
fn test_binary_sub() {
    let expr = parse_expr("5 - 3").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Sub,
            lhs,
            rhs,
        } => {
            assert_eq!(lhs.kind, ExprKind::Literal(Literal::Int(5)));
            assert_eq!(rhs.kind, ExprKind::Literal(Literal::Int(3)));
        }
        _ => panic!("Expected binary sub"),
    }
}

#[test]
fn test_binary_mul() {
    let expr = parse_expr("2 * 3").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Mul, ..
        } => {}
        _ => panic!("Expected binary mul"),
    }
}

#[test]
fn test_binary_div() {
    let expr = parse_expr("10 / 2").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Div, ..
        } => {}
        _ => panic!("Expected binary div"),
    }
}

#[test]
fn test_binary_mod() {
    let expr = parse_expr("10 % 3").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Mod, ..
        } => {}
        _ => panic!("Expected binary mod"),
    }
}

#[test]
fn test_binary_pow() {
    let expr = parse_expr("2 ** 3").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Pow, ..
        } => {}
        _ => panic!("Expected binary pow"),
    }
}

#[test]
fn test_binary_bit_and() {
    let expr = parse_expr("5 & 3").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::BitAnd,
            ..
        } => {}
        _ => panic!("Expected binary bit and"),
    }
}

#[test]
fn test_binary_bit_or() {
    let expr = parse_expr("5 | 3").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::BitOr,
            ..
        } => {}
        _ => panic!("Expected binary bit or"),
    }
}

#[test]
fn test_binary_bit_xor() {
    let expr = parse_expr("5 ^ 3").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::BitXor,
            ..
        } => {}
        _ => panic!("Expected binary bit xor"),
    }
}

#[test]
fn test_binary_eq() {
    let expr = parse_expr("x == 10").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Eq, ..
        } => {}
        _ => panic!("Expected binary eq"),
    }
}

#[test]
fn test_binary_not_eq() {
    let expr = parse_expr("x != 10").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::NotEq,
            ..
        } => {}
        _ => panic!("Expected binary not eq"),
    }
}

#[test]
fn test_binary_lt() {
    let expr = parse_expr("x < 10").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Lt, ..
        } => {}
        _ => panic!("Expected binary lt"),
    }
}

#[test]
fn test_binary_gt() {
    let expr = parse_expr("x > 10").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Gt, ..
        } => {}
        _ => panic!("Expected binary gt"),
    }
}

#[test]
fn test_binary_lteq() {
    let expr = parse_expr("x <= 10").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::LtEq, ..
        } => {}
        _ => panic!("Expected binary lteq"),
    }
}

#[test]
fn test_binary_gteq() {
    let expr = parse_expr("x >= 10").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::GtEq, ..
        } => {}
        _ => panic!("Expected binary gteq"),
    }
}

#[test]
fn test_binary_logical_and() {
    let expr = parse_expr("x && y").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            ..
        } => {}
        _ => panic!("Expected binary logical and"),
    }
}

#[test]
fn test_binary_logical_or() {
    let expr = parse_expr("x || y").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::LogicalOr,
            ..
        } => {}
        _ => panic!("Expected binary logical or"),
    }
}

// ==================== UNARY OPERATOR TESTS ====================

#[test]
fn test_unary_not() {
    let expr = parse_expr("!x").unwrap();
    match &expr.kind {
        ExprKind::Unary {
            op: UnaryOp::Not,
            expr,
        } => {
            assert_eq!(expr.kind, ExprKind::Identifier("x".to_string()));
        }
        _ => panic!("Expected unary not"),
    }
}

#[test]
fn test_unary_not_word() {
    let expr = parse_expr("not x").unwrap();
    match &expr.kind {
        ExprKind::Unary {
            op: UnaryOp::Not, ..
        } => {}
        _ => panic!("Expected unary not"),
    }
}

#[test]
fn test_unary_negate() {
    let expr = parse_expr("-x").unwrap();
    match &expr.kind {
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => {
            assert_eq!(expr.kind, ExprKind::Identifier("x".to_string()));
        }
        _ => panic!("Expected unary negate"),
    }
}

#[test]
fn test_unary_negate_number() {
    let expr = parse_expr("-42").unwrap();
    match &expr.kind {
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => {
            assert_eq!(expr.kind, ExprKind::Literal(Literal::Int(42)));
        }
        _ => panic!("Expected unary negate"),
    }
}

// ==================== INCREMENT/DECREMENT TESTS ====================

#[test]
fn test_increment_prefix() {
    let expr = parse_expr("++x").unwrap();
    match &expr.kind {
        ExprKind::Increment { expr, is_prefix } => {
            assert_eq!(expr.kind, ExprKind::Identifier("x".to_string()));
            assert_eq!(*is_prefix, true);
        }
        _ => panic!("Expected prefix increment"),
    }
}

#[test]
fn test_increment_postfix() {
    let expr = parse_expr("x++").unwrap();
    match &expr.kind {
        ExprKind::Increment { expr, is_prefix } => {
            assert_eq!(expr.kind, ExprKind::Identifier("x".to_string()));
            assert_eq!(*is_prefix, false);
        }
        _ => panic!("Expected postfix increment"),
    }
}

#[test]
fn test_decrement_prefix() {
    let expr = parse_expr("--x").unwrap();
    match &expr.kind {
        ExprKind::Decrement { expr, is_prefix } => {
            assert_eq!(expr.kind, ExprKind::Identifier("x".to_string()));
            assert_eq!(*is_prefix, true);
        }
        _ => panic!("Expected prefix decrement"),
    }
}

#[test]
fn test_decrement_postfix() {
    let expr = parse_expr("x--").unwrap();
    match &expr.kind {
        ExprKind::Decrement { expr, is_prefix } => {
            assert_eq!(expr.kind, ExprKind::Identifier("x".to_string()));
            assert_eq!(*is_prefix, false);
        }
        _ => panic!("Expected postfix decrement"),
    }
}

// ==================== TERNARY OPERATOR TESTS ====================

#[test]
fn test_ternary_simple() {
    let expr = parse_expr("x > 0 ? 1 : 0").unwrap();
    match &expr.kind {
        ExprKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            // Condition should be binary comparison
            match &condition.kind {
                ExprKind::Binary {
                    op: BinaryOp::Gt, ..
                } => {}
                _ => panic!("Expected gt comparison in condition"),
            }
            assert_eq!(then_expr.kind, ExprKind::Literal(Literal::Int(1)));
            assert_eq!(else_expr.kind, ExprKind::Literal(Literal::Int(0)));
        }
        _ => panic!("Expected ternary"),
    }
}

// ==================== ARRAY TESTS ====================

#[test]
fn test_array_empty() {
    let expr = parse_expr("[]").unwrap();
    match &expr.kind {
        ExprKind::Array(elements) => {
            assert_eq!(elements.len(), 0);
        }
        _ => panic!("Expected empty array"),
    }
}

#[test]
fn test_array_single_element() {
    let expr = parse_expr("[1]").unwrap();
    match &expr.kind {
        ExprKind::Array(elements) => {
            assert_eq!(elements.len(), 1);
            assert_eq!(elements[0].kind, ExprKind::Literal(Literal::Int(1)));
        }
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_array_multiple_elements() {
    let expr = parse_expr("[1, 2, 3]").unwrap();
    match &expr.kind {
        ExprKind::Array(elements) => {
            assert_eq!(elements.len(), 3);
            assert_eq!(elements[0].kind, ExprKind::Literal(Literal::Int(1)));
            assert_eq!(elements[1].kind, ExprKind::Literal(Literal::Int(2)));
            assert_eq!(elements[2].kind, ExprKind::Literal(Literal::Int(3)));
        }
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_array_mixed_types() {
    let expr = parse_expr("[1, 2.5, 3]").unwrap();
    match &expr.kind {
        ExprKind::Array(elements) => {
            assert_eq!(elements.len(), 3);
        }
        _ => panic!("Expected array"),
    }
}

// ==================== INDEX ACCESS TESTS ====================

#[test]
fn test_index_1d() {
    let expr = parse_expr("arr[0]").unwrap();
    match &expr.kind {
        ExprKind::Index { array, indices } => {
            assert_eq!(array.kind, ExprKind::Identifier("arr".to_string()));
            assert_eq!(indices.len(), 1);
            assert_eq!(indices[0].kind, ExprKind::Literal(Literal::Int(0)));
        }
        _ => panic!("Expected index"),
    }
}

#[test]
fn test_index_2d() {
    let expr = parse_expr("matrix[0][1]").unwrap();
    match &expr.kind {
        ExprKind::Index { array, indices } => {
            assert_eq!(array.kind, ExprKind::Identifier("matrix".to_string()));
            assert_eq!(indices.len(), 2);
        }
        _ => panic!("Expected index"),
    }
}

#[test]
fn test_index_expression() {
    let expr = parse_expr("arr[i + 1]").unwrap();
    match &expr.kind {
        ExprKind::Index { indices, .. } => match &indices[0].kind {
            ExprKind::Binary {
                op: BinaryOp::Add, ..
            } => {}
            _ => panic!("Expected binary add in index"),
        },
        _ => panic!("Expected index"),
    }
}

// ==================== FUNCTION CALL TESTS ====================

#[test]
fn test_call_no_args() {
    let expr = parse_expr("foo()").unwrap();
    match &expr.kind {
        ExprKind::Call { func, args } => {
            assert_eq!(func.kind, ExprKind::Identifier("foo".to_string()));
            assert_eq!(args.len(), 0);
        }
        _ => panic!("Expected call"),
    }
}

#[test]
fn test_call_single_arg() {
    let expr = parse_expr("foo(42)").unwrap();
    match &expr.kind {
        ExprKind::Call { func, args } => {
            assert_eq!(func.kind, ExprKind::Identifier("foo".to_string()));
            assert_eq!(args.len(), 1);
            assert_eq!(args[0].kind, ExprKind::Literal(Literal::Int(42)));
        }
        _ => panic!("Expected call"),
    }
}

#[test]
fn test_call_multiple_args() {
    let expr = parse_expr("add(1, 2)").unwrap();
    match &expr.kind {
        ExprKind::Call { func, args } => {
            assert_eq!(func.kind, ExprKind::Identifier("add".to_string()));
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected call"),
    }
}

#[test]
fn test_call_nested() {
    let expr = parse_expr("foo(bar(1))").unwrap();
    match &expr.kind {
        ExprKind::Call { func, args } => {
            assert_eq!(func.kind, ExprKind::Identifier("foo".to_string()));
            assert_eq!(args.len(), 1);
            match &args[0].kind {
                ExprKind::Call { .. } => {} // Nested call
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
    match &expr.kind {
        ExprKind::FieldAccess { target, field } => {
            assert_eq!(target.kind, ExprKind::Identifier("obj".to_string()));
            assert_eq!(field, "field");
        }
        _ => panic!("Expected field access"),
    }
}

#[test]
fn test_field_access_chained() {
    let expr = parse_expr("obj.field.subfield").unwrap();
    match &expr.kind {
        ExprKind::FieldAccess { target, field } => {
            assert_eq!(field, "subfield");
            match &target.kind {
                ExprKind::FieldAccess { .. } => {} // Chained access
                _ => panic!("Expected chained field access"),
            }
        }
        _ => panic!("Expected field access"),
    }
}

#[test]
fn test_field_access_not_keyword_as_field_name() {
    // Regression test: `not` is lexed as Token::Not (used for the `not x`
    // prefix operator), so the postfix field-access rule must special-case
    // it to allow `.not.` chains like `test.expect(1).not.toBe(2)` — prior to
    // the fix (parser.rs, postfix_chain field-access rule), only
    // Token::Identifier was accepted after `.`, so `.not.toBe(...)` never
    // parsed at all (affected every `not.*` matcher since v1.5).
    let expr = parse_expr("test.expect(1).not.toBe(2)").unwrap();
    match &expr.kind {
        ExprKind::Call { func, args } => {
            assert_eq!(args.len(), 1);
            match &func.kind {
                ExprKind::FieldAccess { target, field } => {
                    assert_eq!(field, "toBe");
                    match &target.kind {
                        ExprKind::FieldAccess {
                            target: not_target,
                            field: not_field,
                        } => {
                            assert_eq!(not_field, "not");
                            match &not_target.kind {
                                ExprKind::Call { .. } => {} // test.expect(1)
                                _ => panic!("Expected test.expect(1) call under .not."),
                            }
                        }
                        _ => panic!("Expected `.not.` field access in the chain"),
                    }
                }
                _ => panic!("Expected `.toBe` field access as the outer call target"),
            }
        }
        _ => panic!("Expected a call expression (the .toBe(2) call)"),
    }
}

#[test]
fn test_field_access_not_chained_with_further_field() {
    // `.not.` followed by another plain field access (no call) still parses
    // as a FieldAccess chain with "not" as an intermediate field name.
    let expr = parse_expr("obj.not.field").unwrap();
    match &expr.kind {
        ExprKind::FieldAccess { target, field } => {
            assert_eq!(field, "field");
            match &target.kind {
                ExprKind::FieldAccess {
                    target: inner_target,
                    field: not_field,
                } => {
                    assert_eq!(not_field, "not");
                    assert_eq!(inner_target.kind, ExprKind::Identifier("obj".to_string()));
                }
                _ => panic!("Expected `.not` field access in the chain"),
            }
        }
        _ => panic!("Expected field access"),
    }
}

// ==================== RANGE TESTS ====================

#[test]
fn test_range_inclusive() {
    let expr = parse_expr("1..10").unwrap();
    match &expr.kind {
        ExprKind::Range {
            start,
            end,
            step,
            inclusive,
        } => {
            assert_eq!(start.kind, ExprKind::Literal(Literal::Int(1)));
            assert_eq!(end.kind, ExprKind::Literal(Literal::Int(10)));
            assert!(step.is_none());
            assert_eq!(*inclusive, true);
        }
        _ => panic!("Expected inclusive range"),
    }
}

#[test]
fn test_range_exclusive() {
    let expr = parse_expr("0..<10").unwrap();
    match &expr.kind {
        ExprKind::Range {
            start,
            end,
            step,
            inclusive,
        } => {
            assert_eq!(start.kind, ExprKind::Literal(Literal::Int(0)));
            assert_eq!(end.kind, ExprKind::Literal(Literal::Int(10)));
            assert!(step.is_none());
            assert_eq!(*inclusive, false);
        }
        _ => panic!("Expected exclusive range"),
    }
}

#[test]
fn test_range_with_step() {
    let expr = parse_expr("0..10 step 2").unwrap();
    match &expr.kind {
        ExprKind::Range {
            start,
            end,
            step,
            inclusive,
        } => {
            assert_eq!(start.kind, ExprKind::Literal(Literal::Int(0)));
            assert!(step.is_some());
            assert_eq!(end.kind, ExprKind::Literal(Literal::Int(10)));
            assert_eq!(*inclusive, true);
        }
        _ => panic!("Expected range with step"),
    }
}

#[test]
fn test_range_exclusive_with_step() {
    let expr = parse_expr("0..<10 step 2").unwrap();
    match &expr.kind {
        ExprKind::Range {
            start,
            end,
            step,
            inclusive,
        } => {
            assert_eq!(start.kind, ExprKind::Literal(Literal::Int(0)));
            assert!(step.is_some());
            assert_eq!(end.kind, ExprKind::Literal(Literal::Int(10)));
            assert_eq!(*inclusive, false);
        }
        _ => panic!("Expected exclusive range with step"),
    }
}

#[test]
fn test_range_with_variables() {
    let expr = parse_expr("start..end").unwrap();
    match &expr.kind {
        ExprKind::Range {
            start,
            end,
            step,
            inclusive,
        } => {
            assert_eq!(start.kind, ExprKind::Identifier("start".to_string()));
            assert_eq!(end.kind, ExprKind::Identifier("end".to_string()));
            assert!(step.is_none());
            assert_eq!(*inclusive, true);
        }
        _ => panic!("Expected range"),
    }
}

#[test]
fn test_range_descending() {
    let expr = parse_expr("10..0").unwrap();
    match &expr.kind {
        ExprKind::Range {
            start,
            end,
            inclusive,
            ..
        } => {
            assert_eq!(start.kind, ExprKind::Literal(Literal::Int(10)));
            assert_eq!(end.kind, ExprKind::Literal(Literal::Int(0)));
            assert_eq!(*inclusive, true);
        }
        _ => panic!("Expected descending range"),
    }
}

// ==================== STATIC INIT TESTS ====================

#[test]
fn test_static_init_int_1d() {
    let expr = parse_expr("int[5]").unwrap();
    match &expr.kind {
        ExprKind::StaticInit {
            element_type,
            dimensions,
        } => {
            assert_eq!(element_type, "int");
            assert_eq!(dimensions.len(), 1);
            assert_eq!(dimensions[0].kind, ExprKind::Literal(Literal::Int(5)));
        }
        _ => panic!("Expected static init"),
    }
}

#[test]
fn test_static_init_float_2d() {
    let expr = parse_expr("float[3, 4]").unwrap();
    match &expr.kind {
        ExprKind::StaticInit {
            element_type,
            dimensions,
        } => {
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
    match &expr.kind {
        ExprKind::FString { parts } => {
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
    match &expr.kind {
        ExprKind::FString { parts } => {
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Add,
            lhs,
            rhs,
        } => {
            assert_eq!(lhs.kind, ExprKind::Literal(Literal::Int(1)));
            match &rhs.kind {
                ExprKind::Binary {
                    op: BinaryOp::Mul, ..
                } => {} // Good
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
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Mul,
            lhs,
            rhs,
        } => {
            match &lhs.kind {
                ExprKind::Binary {
                    op: BinaryOp::Add, ..
                } => {}
                _ => panic!("Expected addition on left side"),
            }
            assert_eq!(rhs.kind, ExprKind::Literal(Literal::Int(3)));
        }
        _ => panic!("Expected multiplication"),
    }
}

#[test]
fn test_deeply_nested() {
    let expr = parse_expr("((((1))))").unwrap();
    assert_eq!(expr.kind, ExprKind::Literal(Literal::Int(1)));
}

// ==================== CONST GENERICS (v2.0 Grupo A Fase 0) ====================
//
// `Embedding<1536>(...)` needs an integer literal accepted where a generic
// call previously only accepted `Token::Identifier` type names. This is
// handled at the identifier-*atom* level (not the shared postfix generic-call
// combinator used by `Vector<int>`/`swap<int,float>`/etc.), gated to the
// literal names "Embedding"/"EmbeddingBatch" only — see `parser.rs`'s
// `identifier_atom` construct. These tests pin: the numeric-literal
// acceptance for those two names, that ordinary chained comparisons
// (including ones that happen to look like a generic call, e.g. `a<1>(b)`)
// are completely unaffected, and that a malformed dimension (negative,
// overflow) is a real parse error, not a silent reinterpretation.

#[test]
fn test_generic_call_const_int_arg() {
    let expr = parse_expr("Embedding<1536>([1, 2, 3])").unwrap();
    match &expr.kind {
        ExprKind::GenericCall {
            func, type_args, ..
        } => {
            assert_eq!(func.kind, ExprKind::Identifier("Embedding".to_string()));
            assert_eq!(type_args, &vec!["1536".to_string()]);
        }
        other => panic!("Expected GenericCall, got {:?}", other),
    }
}

#[test]
fn test_generic_call_embedding_batch_const_int_arg() {
    let expr = parse_expr("EmbeddingBatch<1536>(1000)").unwrap();
    match &expr.kind {
        ExprKind::GenericCall {
            func, type_args, ..
        } => {
            assert_eq!(
                func.kind,
                ExprKind::Identifier("EmbeddingBatch".to_string())
            );
            assert_eq!(type_args, &vec!["1536".to_string()]);
        }
        other => panic!("Expected GenericCall, got {:?}", other),
    }
}

#[test]
fn test_generic_call_identifier_arg_still_works() {
    // Non-regression: existing identifier-only generic calls (Vector<int>,
    // swap<int, float>) must keep parsing exactly as before — these go
    // through the ordinary postfix generic-call combinator, untouched by
    // the Embedding-specific atom-level rule.
    let expr = parse_expr("Vector<int>()").unwrap();
    match &expr.kind {
        ExprKind::GenericCall { type_args, .. } => {
            assert_eq!(type_args, &vec!["int".to_string()]);
        }
        other => panic!("Expected GenericCall, got {:?}", other),
    }

    let expr = parse_expr("swap<int, float>(a, b)").unwrap();
    match &expr.kind {
        ExprKind::GenericCall { type_args, .. } => {
            assert_eq!(type_args, &vec!["int".to_string(), "float".to_string()]);
        }
        other => panic!("Expected GenericCall, got {:?}", other),
    }
}

#[test]
fn test_chained_comparison_with_int_literal_and_parens_not_a_generic_call() {
    // P1 regression (found in review): a naive "just accept Token::Int in
    // the shared generic-call combinator" fix would make `a<1>(b)` — a
    // valid chained comparison, `(a < 1) > (b)`, for plain variables `a`/`b`
    // — ambiguously also parseable as a generic call on `a` with a numeric
    // type argument. Since the const-generic grammar is gated to the two
    // literal names "Embedding"/"EmbeddingBatch" (checked at the identifier
    // atom itself, not by token shape), `a` never enters that path at all —
    // this must still parse as the chained comparison it always did. Brix
    // desugars a Python-style chain `a < 1 > b` into `(a < 1) && (1 > b)`
    // (reusing the middle operand), NOT `(a < 1) > b` — the exact desugared
    // shape isn't this test's concern, only that it's a LogicalAnd of two
    // comparisons and definitely not a GenericCall.
    let expr = parse_expr("a < 1 > (b)").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            lhs,
            rhs,
        } => {
            assert!(
                matches!(
                    lhs.kind,
                    ExprKind::Binary {
                        op: BinaryOp::Lt,
                        ..
                    }
                ),
                "Expected (a < 1) as the left side, got {:?}",
                lhs
            );
            assert!(
                matches!(
                    rhs.kind,
                    ExprKind::Binary {
                        op: BinaryOp::Gt,
                        ..
                    }
                ),
                "Expected (1 > b) as the right side, got {:?}",
                rhs
            );
        }
        other => panic!(
            "Expected a chained comparison (not a GenericCall), got {:?}",
            other
        ),
    }
}

#[test]
fn test_generic_call_negative_int_arg_is_a_parse_error() {
    // P1 fix (found in review): the previous version of this test accepted
    // Embedding<-1536>(...) silently backtracking into a chained-comparison
    // reading, or simply failing — both of which mean no clear diagnostic
    // ever surfaces. The atom-level rule now peeks for `<` right after
    // "Embedding"/"EmbeddingBatch" and, once found, commits: it must resolve
    // as a valid dimension list or the whole expression fails to parse.
    // There is no comparison-chain fallback available once that commit
    // point is passed.
    let err = parse_expr("Embedding<-1536>([1])").unwrap_err();
    assert!(
        err.contains("invalid dimension") || err.contains("non-negative"),
        "Expected a clear invalid-dimension diagnostic, got: {}",
        err
    );
}

#[test]
fn test_generic_call_dimension_overflow_is_a_parse_error() {
    // A dimension that doesn't fit in u32 must also be a clear diagnostic,
    // not a silently-truncated or wrapped value.
    let err = parse_expr("Embedding<99999999999>([1])").unwrap_err();
    assert!(
        err.contains("invalid dimension") || err.contains("exceeds"),
        "Expected a clear overflow diagnostic, got: {}",
        err
    );
}

#[test]
fn test_plain_identifier_named_embedding_without_angle_bracket_still_works() {
    // Non-regression: "Embedding"/"EmbeddingBatch" are only special-cased
    // when immediately followed by `<` — used as an ordinary identifier
    // (e.g. a variable, or compared without angle brackets), they behave
    // exactly like any other name.
    let expr = parse_expr("Embedding + 1").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::Add,
            lhs,
            ..
        } => {
            assert_eq!(lhs.kind, ExprKind::Identifier("Embedding".to_string()));
        }
        other => panic!("Expected binary add, got {:?}", other),
    }
}

#[test]
fn test_variable_named_embedding_in_spaced_chained_comparison() {
    // P1 regression (found in review): a variable literally named
    // `Embedding`/`EmbeddingBatch` could no longer participate in a `<`
    // comparison at all — `Embedding < 1 > (b)` was forced down the
    // const-generic commit path (since it only peeked *whether* `<`
    // followed, not whether it was immediately adjacent) and failed there
    // instead of parsing as the chained comparison it should be. Fixed by
    // additionally requiring `<` to be adjacent (no whitespace) to the name
    // before committing — `Embedding<1536>` (no space) still commits;
    // `Embedding < 1` (spaced) is an ordinary comparison. This mirrors the
    // exact reproduction from the review, with `Embedding` as a real
    // variable name (not a real Embedding value, since Fase 1 doesn't exist
    // yet — only the parse shape is under test here).
    let expr = parse_expr_with_real_spans("Embedding < 1 > (b)").unwrap();
    match &expr.kind {
        ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            lhs,
            rhs,
        } => {
            assert!(
                matches!(
                    lhs.kind,
                    ExprKind::Binary {
                        op: BinaryOp::Lt,
                        ..
                    }
                ),
                "Expected (Embedding < 1) as the left side, got {:?}",
                lhs
            );
            assert!(
                matches!(
                    rhs.kind,
                    ExprKind::Binary {
                        op: BinaryOp::Gt,
                        ..
                    }
                ),
                "Expected (1 > b) as the right side, got {:?}",
                rhs
            );
        }
        other => panic!(
            "Expected a chained comparison (not a GenericCall), got {:?}",
            other
        ),
    }
}

#[test]
fn test_embedding_const_generic_requires_no_whitespace_before_lt() {
    // The flip side of the above: `Embedding <1536>(...)` (space before `<`,
    // none after) must NOT commit to the const-generic path either — only
    // truly adjacent `Embedding<1536>` does. This is a deliberate, narrow
    // convention (documented in ROADMAP_V2.0.md): whitespace immediately
    // before `<` always means "comparison", regardless of what follows it.
    let expr = parse_expr_with_real_spans("Embedding <1536").unwrap();
    assert!(
        !matches!(expr.kind, ExprKind::GenericCall { .. }),
        "Expected a spaced `<` to never commit to the const-generic path, got {:?}",
        expr
    );
}
