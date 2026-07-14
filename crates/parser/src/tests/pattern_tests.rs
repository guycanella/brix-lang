// Pattern Matching Tests

use crate::ast::{Expr, ExprKind, Literal, Pattern, StmtKind};
use crate::parser::parser;
use chumsky::Parser;
use lexer::token::Token;

fn parse_expr(input: &str) -> Result<Expr, String> {
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).map_err(|e| format!("{:?}", e))?;
    if let Some(stmt) = program.statements.first() {
        if let StmtKind::Expr(expr) = &stmt.kind {
            return Ok(expr.clone());
        }
    }
    Err("No expr".to_string())
}

#[test]
fn test_match_literal_int() {
    let expr = parse_expr("match x { 1 -> :one 2 -> :two _ -> :other }").unwrap();
    match &expr.kind {
        ExprKind::Match { value, arms } => {
            assert_eq!(value.kind, ExprKind::Identifier("x".to_string()));
            assert_eq!(arms.len(), 3);
            match &arms[0].pattern {
                Pattern::Literal(Literal::Int(1)) => {}
                _ => panic!("Expected literal 1"),
            }
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_wildcard() {
    let expr = parse_expr("match x { _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Wildcard => {}
            _ => panic!("Expected wildcard"),
        },
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_binding() {
    let expr = parse_expr("match x { n -> n * 2 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Binding(name) => assert_eq!(name, "n"),
            _ => panic!("Expected binding"),
        },
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_or_pattern() {
    let expr = parse_expr("match x { 1 | 2 | 3 -> :small _ -> :large }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Or(patterns) => assert_eq!(patterns.len(), 3),
            _ => panic!("Expected or pattern"),
        },
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_with_guard() {
    let expr = parse_expr("match x { n if n > 10 -> :big n -> :small }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert!(arms[0].guard.is_some());
            assert!(arms[1].guard.is_none());
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_string_literal() {
    let expr = parse_expr(r#"match status { "ok" -> 1 "error" -> 0 }"#).unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Literal(Literal::String(_)) => {}
            _ => panic!("Expected string literal"),
        },
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_match_atom() {
    let expr = parse_expr("match status { :ok -> 1 :error -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => match &arms[0].pattern {
            Pattern::Literal(Literal::Atom(atom)) => assert_eq!(atom, "ok"),
            _ => panic!("Expected atom"),
        },
        _ => panic!("Expected match"),
    }
}

// ==================== PHASE 4: PATTERN MATCHING 2.0 ====================

#[test]
fn test_destructure_pattern_bindings() {
    // { x, y } -> Destructure([Binding("x"), Binding("y")])
    let expr = parse_expr("match p { { x, y } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Destructure(vec![
                    Pattern::Binding("x".to_string()),
                    Pattern::Binding("y".to_string()),
                ])
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_destructure_pattern_literal_constraint() {
    // { 0, x } -> Destructure([Literal(Int(0)), Binding("x")])
    let expr = parse_expr("match p { { 0, x } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Destructure(vec![
                    Pattern::Literal(Literal::Int(0)),
                    Pattern::Binding("x".to_string()),
                ])
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_range_pattern_inclusive() {
    // 18..64 -> Range { start: Int(18), end: Int(64), inclusive: true }
    let expr = parse_expr("match age { 18..64 -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Range {
                    start: Literal::Int(18),
                    end: Literal::Int(64),
                    inclusive: true,
                }
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_range_pattern_exclusive() {
    // 0..<10 -> Range { start: Int(0), end: Int(10), inclusive: false }
    let expr = parse_expr("match x { 0..<10 -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Range {
                    start: Literal::Int(0),
                    end: Literal::Int(10),
                    inclusive: false,
                }
            );
        }
        _ => panic!("Expected match"),
    }
}

// ==================== v1.7 GRUPO D: NAMED FIELD PATTERNS ====================

#[test]
fn test_named_field_pattern_bindings() {
    // { x: px, y: py } -> NamedField([("x", Binding("px")), ("y", Binding("py"))])
    let expr = parse_expr("match p { { x: px, y: py } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::NamedField(vec![
                    ("x".to_string(), Pattern::Binding("px".to_string())),
                    ("y".to_string(), Pattern::Binding("py".to_string())),
                ])
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_named_field_pattern_with_literal_constraint() {
    // { x: px, y: 0 } -> NamedField([("x", Binding("px")), ("y", Literal(Int(0)))])
    let expr = parse_expr("match p { { x: px, y: 0 } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::NamedField(vec![
                    ("x".to_string(), Pattern::Binding("px".to_string())),
                    ("y".to_string(), Pattern::Literal(Literal::Int(0))),
                ])
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_named_field_pattern_with_wildcard() {
    // { x: _, y: py } -> NamedField([("x", Wildcard), ("y", Binding("py"))])
    let expr = parse_expr("match p { { x: _, y: py } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::NamedField(vec![
                    ("x".to_string(), Pattern::Wildcard),
                    ("y".to_string(), Pattern::Binding("py".to_string())),
                ])
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_positional_destructure_still_works() {
    // Non-regression: { p1, p2 } (no colons) must still parse as Pattern::Destructure
    let expr = parse_expr("match p { { p1, p2 } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Destructure(vec![
                    Pattern::Binding("p1".to_string()),
                    Pattern::Binding("p2".to_string()),
                ])
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_named_field_pattern_multiple_arms_on_separate_lines() {
    // Regression test: named-field patterns across multiple match arms, each on its own
    // line, with a bare-identifier body (e.g. `-> px`). This used to fail because the
    // "Non-generic struct init" postfix rule greedily consumed the NEXT arm's leading
    // `{ ... }` as a struct-init continuation of the previous arm's bare-identifier body
    // (`px { x: 0, y: py }`), swallowing the second arm's pattern whole. The `{` must now
    // only be treated as a struct-init continuation when it's on the same line as the
    // preceding identifier — here it's on the next line, so it correctly starts a new
    // pattern instead.
    let input = "match p {\n    { x: px, y: 0 } -> px\n    { x: 0, y: py } -> py\n    { x: px, y: py } -> 999\n}";
    let expr = parse_expr(input).unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(arms.len(), 3, "expected 3 arms, got {}", arms.len());

            assert_eq!(
                arms[0].pattern,
                Pattern::NamedField(vec![
                    ("x".to_string(), Pattern::Binding("px".to_string())),
                    ("y".to_string(), Pattern::Literal(Literal::Int(0))),
                ])
            );
            assert_eq!(arms[0].body.kind, ExprKind::Identifier("px".to_string()));

            assert_eq!(
                arms[1].pattern,
                Pattern::NamedField(vec![
                    ("x".to_string(), Pattern::Literal(Literal::Int(0))),
                    ("y".to_string(), Pattern::Binding("py".to_string())),
                ])
            );
            assert_eq!(arms[1].body.kind, ExprKind::Identifier("py".to_string()));

            assert_eq!(
                arms[2].pattern,
                Pattern::NamedField(vec![
                    ("x".to_string(), Pattern::Binding("px".to_string())),
                    ("y".to_string(), Pattern::Binding("py".to_string())),
                ])
            );
            assert_eq!(arms[2].body.kind, ExprKind::Literal(Literal::Int(999)));
        }
        _ => panic!("Expected match, got {:?}", expr.kind),
    }
}

#[test]
fn test_struct_init_same_line_still_works() {
    // Non-regression: `Point { x: 3, y: 0 }` on a single line (the legitimate struct-init
    // continuation) must still parse as ExprKind::StructInit, not be affected by the
    // "same line" restriction added to disambiguate it from named-field match patterns.
    use crate::ast::StmtKind;
    let input = "var p := Point { x: 3, y: 0 }";
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).map_err(|e| format!("{:?}", e)).unwrap();
    match &program.statements[0].kind {
        StmtKind::VariableDecl { value, .. } => match &value.kind {
            ExprKind::StructInit { struct_name, fields, type_args } => {
                assert_eq!(struct_name, "Point");
                assert!(type_args.is_empty());
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                assert_eq!(fields[0].1.kind, ExprKind::Literal(Literal::Int(3)));
                assert_eq!(fields[1].0, "y");
                assert_eq!(fields[1].1.kind, ExprKind::Literal(Literal::Int(0)));
            }
            other => panic!("Expected StructInit, got {:?}", other),
        },
        other => panic!("Expected VariableDecl, got {:?}", other),
    }
}

// ==================== GRUPO E: ARRAY REST PATTERNS ====================

#[test]
fn test_array_rest_pattern_one_head() {
    // { first, ...rest } -> ArrayRest { head: [Binding("first")], rest: "rest" }
    let expr = parse_expr("match arr { { first, ...rest } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::ArrayRest {
                    head: vec![Pattern::Binding("first".to_string())],
                    rest: "rest".to_string(),
                }
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_array_rest_pattern_two_head() {
    // { a, b, ...tail } -> ArrayRest { head: [Binding("a"), Binding("b")], rest: "tail" }
    let expr = parse_expr("match arr { { a, b, ...tail } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::ArrayRest {
                    head: vec![
                        Pattern::Binding("a".to_string()),
                        Pattern::Binding("b".to_string()),
                    ],
                    rest: "tail".to_string(),
                }
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_array_rest_pattern_no_head() {
    // { ...all } -> ArrayRest { head: [], rest: "all" }
    let expr = parse_expr("match arr { { ...all } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::ArrayRest {
                    head: vec![],
                    rest: "all".to_string(),
                }
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_destructure_without_rest_still_works() {
    // Non-regression: { a, b, c } (no `...rest`) must still parse as Pattern::Destructure,
    // not Pattern::ArrayRest — confirms the DestructureItem-based grammar doesn't change
    // behavior for the plain positional-destructure case added in Grupo/Fase 4a.
    let expr = parse_expr("match arr { { a, b, c } -> 1 _ -> 0 }").unwrap();
    match &expr.kind {
        ExprKind::Match { arms, .. } => {
            assert_eq!(
                arms[0].pattern,
                Pattern::Destructure(vec![
                    Pattern::Binding("a".to_string()),
                    Pattern::Binding("b".to_string()),
                    Pattern::Binding("c".to_string()),
                ])
            );
        }
        _ => panic!("Expected match"),
    }
}

#[test]
fn test_array_rest_pattern_must_be_last() {
    // Regression: `{ ...rest, a }` (rest not in last position) used to be silently
    // accepted with misleading semantics (`a` reinterpreted as head[0], ignoring
    // the position the user wrote it in). Must now be a parse error.
    let result = parse_expr("match arr { { ...rest, a } -> a _ -> 0 }");
    assert!(
        result.is_err(),
        "expected an array rest capture not in the last position to be a parse error"
    );
}

#[test]
fn test_array_rest_pattern_only_one_rest_allowed() {
    // Regression: `{ ...a, ...b }` used to silently drop `a` ("last rest wins").
    // Must now be a parse error instead of silently discarding a capture.
    let result = parse_expr("match arr { { ...a, ...b } -> 1 _ -> 0 }");
    assert!(
        result.is_err(),
        "expected multiple array rest captures in one pattern to be a parse error"
    );
}
