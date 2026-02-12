// Statement Parsing Tests
//
// Comprehensive tests for all statement types.

use crate::ast::{ExprKind, Literal, MethodDef, Stmt, StmtKind};
use crate::parser::parser;
use chumsky::Parser;
use lexer::token::Token;

fn parse_stmt(input: &str) -> Result<Stmt, String> {
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).map_err(|e| format!("{:?}", e))?;
    program
        .statements
        .first()
        .cloned()
        .ok_or("No statement".to_string())
}

// ==================== VARIABLE DECLARATION TESTS ====================

#[test]
fn test_var_decl_inferred() {
    let stmt = parse_stmt("var x := 10").unwrap();
    match &stmt.kind {
        StmtKind::VariableDecl {
            name,
            type_hint,
            value,
            is_const,
        } => {
            assert_eq!(name, "x");
            assert_eq!(*type_hint, None);
            assert_eq!(value.kind, ExprKind::Literal(Literal::Int(10)));
            assert_eq!(*is_const, false);
        }
        _ => panic!("Expected var decl"),
    }
}

#[test]
fn test_var_decl_typed() {
    let stmt = parse_stmt("var x: int = 10").unwrap();
    match &stmt.kind {
        StmtKind::VariableDecl {
            name, type_hint, ..
        } => {
            assert_eq!(name, "x");
            assert_eq!(*type_hint, Some("int".to_string()));
        }
        _ => panic!("Expected var decl"),
    }
}

#[test]
fn test_const_decl() {
    let stmt = parse_stmt("const PI := 3.14").unwrap();
    match &stmt.kind {
        StmtKind::VariableDecl { is_const, .. } => {
            assert_eq!(*is_const, true);
        }
        _ => panic!("Expected const decl"),
    }
}

// ==================== DESTRUCTURING TESTS ====================

#[test]
fn test_destructuring_simple() {
    let stmt = parse_stmt("var { a, b } := foo()").unwrap();
    match &stmt.kind {
        StmtKind::DestructuringDecl {
            names, is_const, ..
        } => {
            assert_eq!(*names, vec!["a".to_string(), "b".to_string()]);
            assert_eq!(*is_const, false);
        }
        _ => panic!("Expected destructuring"),
    }
}

#[test]
fn test_destructuring_const() {
    let stmt = parse_stmt("const { x, y } := point()").unwrap();
    match &stmt.kind {
        StmtKind::DestructuringDecl { is_const, .. } => {
            assert_eq!(*is_const, true);
        }
        _ => panic!("Expected destructuring"),
    }
}

// ==================== ASSIGNMENT TESTS ====================

#[test]
fn test_assignment_simple() {
    let stmt = parse_stmt("x = 10").unwrap();
    match &stmt.kind {
        StmtKind::Assignment { target, value } => {
            assert_eq!(target.kind, ExprKind::Identifier("x".to_string()));
            assert_eq!(value.kind, ExprKind::Literal(Literal::Int(10)));
        }
        _ => panic!("Expected assignment"),
    }
}

#[test]
fn test_assignment_array_index() {
    let stmt = parse_stmt("arr[0] = 42").unwrap();
    match &stmt.kind {
        StmtKind::Assignment { target, .. } => match &target.kind {
            ExprKind::Index { .. } => {}
            _ => panic!("Expected index in target"),
        },
        _ => panic!("Expected assignment"),
    }
}

// ==================== IF/ELSE TESTS ====================

#[test]
fn test_if_no_else() {
    let stmt = parse_stmt("if x > 0 { x = x + 1 }").unwrap();
    match &stmt.kind {
        StmtKind::If {
            condition,
            then_block: _,
            else_block,
        } => {
            assert!(else_block.is_none());
            match &condition.kind {
                ExprKind::Binary { .. } => {}
                _ => panic!("Expected binary in condition"),
            }
        }
        _ => panic!("Expected if"),
    }
}

#[test]
fn test_if_with_else() {
    let stmt = parse_stmt("if x > 0 { y = 1 } else { y = 0 }").unwrap();
    match &stmt.kind {
        StmtKind::If { else_block, .. } => {
            assert!(else_block.is_some());
        }
        _ => panic!("Expected if"),
    }
}

// ==================== WHILE LOOP TESTS ====================

#[test]
fn test_while_loop() {
    let stmt = parse_stmt("while x > 0 { x = x - 1 }").unwrap();
    match &stmt.kind {
        StmtKind::While { condition, body } => {
            match &condition.kind {
                ExprKind::Binary { .. } => {}
                _ => panic!("Expected binary condition"),
            }
            match &body.kind {
                StmtKind::Block(_) => {}
                _ => panic!("Expected block body"),
            }
        }
        _ => panic!("Expected while"),
    }
}

// ==================== FOR LOOP TESTS ====================

#[test]
fn test_for_range() {
    let stmt = parse_stmt("for i in 1:10 { }").unwrap();
    match &stmt.kind {
        StmtKind::For {
            var_names,
            iterable,
            ..
        } => {
            assert_eq!(*var_names, vec!["i".to_string()]);
            match &iterable.kind {
                ExprKind::Range { .. } => {}
                _ => panic!("Expected range"),
            }
        }
        _ => panic!("Expected for"),
    }
}

#[test]
fn test_for_destructure() {
    let stmt = parse_stmt("for x, y in pairs { }").unwrap();
    match &stmt.kind {
        StmtKind::For { var_names, .. } => {
            assert_eq!(*var_names, vec!["x".to_string(), "y".to_string()]);
        }
        _ => panic!("Expected for"),
    }
}

// ==================== IMPORT TESTS ====================

#[test]
fn test_import_simple() {
    let stmt = parse_stmt("import math").unwrap();
    match &stmt.kind {
        StmtKind::Import { module, alias } => {
            assert_eq!(module, "math");
            assert_eq!(*alias, None);
        }
        _ => panic!("Expected import"),
    }
}

#[test]
fn test_import_with_alias() {
    let stmt = parse_stmt("import math as m").unwrap();
    match &stmt.kind {
        StmtKind::Import { module, alias } => {
            assert_eq!(module, "math");
            assert_eq!(*alias, Some("m".to_string()));
        }
        _ => panic!("Expected import"),
    }
}

// ==================== PRINT TESTS ====================

#[test]
fn test_printf() {
    let stmt = parse_stmt(r#"printf("x = %d", x)"#).unwrap();
    match &stmt.kind {
        StmtKind::Printf { format, args } => {
            assert_eq!(format, "x = %d");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected printf"),
    }
}

#[test]
fn test_print() {
    let stmt = parse_stmt("print(x)").unwrap();
    match &stmt.kind {
        StmtKind::Print { expr } => {
            assert_eq!(expr.kind, ExprKind::Identifier("x".to_string()));
        }
        _ => panic!("Expected print"),
    }
}

#[test]
fn test_println() {
    let stmt = parse_stmt("println(x)").unwrap();
    match &stmt.kind {
        StmtKind::Println { .. } => {}
        _ => panic!("Expected println"),
    }
}

// ==================== FUNCTION DEFINITION TESTS ====================

#[test]
fn test_function_no_params() {
    let stmt = parse_stmt("function foo() -> int { return 42 }").unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef {
            name,
            params,
            return_type,
            ..
        } => {
            assert_eq!(name, "foo");
            assert_eq!(params.len(), 0);
            assert_eq!(*return_type, Some(vec!["int".to_string()]));
        }
        _ => panic!("Expected function def"),
    }
}

#[test]
fn test_function_with_params() {
    let stmt = parse_stmt("function add(a: int, b: int) -> int { return a + b }").unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef { params, .. } => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].0, "a");
            assert_eq!(params[0].1, "int");
        }
        _ => panic!("Expected function def"),
    }
}

#[test]
fn test_function_void() {
    let stmt = parse_stmt("function greet(name: string) { println(name) }").unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef { return_type, .. } => {
            assert_eq!(*return_type, None);
        }
        _ => panic!("Expected function def"),
    }
}

#[test]
fn test_function_multiple_returns() {
    let stmt = parse_stmt("function calc(a: int, b: int) -> (int, int) { return (a + b, a - b) }")
        .unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef { return_type, .. } => {
            assert_eq!(
                *return_type,
                Some(vec!["int".to_string(), "int".to_string()])
            );
        }
        _ => panic!("Expected function def"),
    }
}

// ==================== RETURN TESTS ====================

#[test]
fn test_return_void() {
    let stmt = parse_stmt("return").unwrap();
    match &stmt.kind {
        StmtKind::Return { values } => {
            assert_eq!(values.len(), 0);
        }
        _ => panic!("Expected return"),
    }
}

#[test]
fn test_return_single() {
    let stmt = parse_stmt("return 42").unwrap();
    match &stmt.kind {
        StmtKind::Return { values } => {
            assert_eq!(values.len(), 1);
        }
        _ => panic!("Expected return"),
    }
}

#[test]
fn test_return_multiple() {
    let stmt = parse_stmt("return (a, b, c)").unwrap();
    match &stmt.kind {
        StmtKind::Return { values } => {
            assert_eq!(values.len(), 3);
        }
        _ => panic!("Expected return"),
    }
}

// ==================== METHOD DEFINITION TESTS ====================

#[test]
fn test_method_simple() {
    // fn (p: Point) get_x() -> int { return p.x }
    let stmt = parse_stmt("fn (p: Point) get_x() -> int { return 42 }").unwrap();
    match &stmt.kind {
        StmtKind::MethodDef(MethodDef {
            receiver_name,
            receiver_type,
            method_name,
            params,
            return_type,
            ..
        }) => {
            assert_eq!(receiver_name, "p");
            assert_eq!(receiver_type, "Point");
            assert_eq!(method_name, "get_x");
            assert_eq!(params.len(), 0);
            assert_eq!(*return_type, Some(vec!["int".to_string()]));
        }
        _ => panic!("Expected method def, got {:?}", stmt.kind),
    }
}

#[test]
fn test_method_with_params() {
    let stmt = parse_stmt("fn (p: Point) add(dx: int, dy: int) -> int { return dx }").unwrap();
    match &stmt.kind {
        StmtKind::MethodDef(MethodDef {
            method_name,
            params,
            ..
        }) => {
            assert_eq!(method_name, "add");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].0, "dx");
            assert_eq!(params[0].1, "int");
            assert_eq!(params[1].0, "dy");
            assert_eq!(params[1].1, "int");
        }
        _ => panic!("Expected method def"),
    }
}

#[test]
fn test_method_void() {
    let stmt = parse_stmt("fn (p: Point) reset() { p = 0 }").unwrap();
    match &stmt.kind {
        StmtKind::MethodDef(MethodDef {
            return_type,
            ..
        }) => {
            assert_eq!(*return_type, None);
        }
        _ => panic!("Expected method def"),
    }
}

#[test]
fn test_method_with_function_keyword() {
    // "function" keyword should also work for methods
    let stmt = parse_stmt("function (p: Point) get_x() -> int { return 42 }").unwrap();
    match &stmt.kind {
        StmtKind::MethodDef(MethodDef {
            receiver_name,
            receiver_type,
            method_name,
            ..
        }) => {
            assert_eq!(receiver_name, "p");
            assert_eq!(receiver_type, "Point");
            assert_eq!(method_name, "get_x");
        }
        _ => panic!("Expected method def"),
    }
}

#[test]
fn test_method_and_function_in_same_program() {
    // Critical test: both methods and functions in same program
    let input = "fn (p: Point) get_x() -> int { return 42 }\nfn foo() -> int { return 1 }";
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).expect("Should parse both fn and method");
    assert_eq!(program.statements.len(), 2);

    match &program.statements[0].kind {
        StmtKind::MethodDef(MethodDef { method_name, .. }) => {
            assert_eq!(method_name, "get_x");
        }
        _ => panic!("Expected method def as first statement"),
    }

    match &program.statements[1].kind {
        StmtKind::FunctionDef { name, .. } => {
            assert_eq!(name, "foo");
        }
        _ => panic!("Expected function def as second statement"),
    }
}

#[test]
fn test_function_then_method() {
    // Reverse order: function first, then method
    let input = "fn foo() -> int { return 1 }\nfn (p: Point) get_x() -> int { return 42 }";
    let tokens: Vec<Token> = lexer::lex(input);
    let program = parser().parse(tokens).expect("Should parse both fn and method");
    assert_eq!(program.statements.len(), 2);

    match &program.statements[0].kind {
        StmtKind::FunctionDef { name, .. } => {
            assert_eq!(name, "foo");
        }
        _ => panic!("Expected function def as first statement"),
    }

    match &program.statements[1].kind {
        StmtKind::MethodDef(MethodDef { method_name, .. }) => {
            assert_eq!(method_name, "get_x");
        }
        _ => panic!("Expected method def as second statement"),
    }
}

#[test]
fn test_generic_function_still_works() {
    // Ensure generic functions aren't broken
    let stmt = parse_stmt("fn identity<T>(x: T) -> T { return x }").unwrap();
    match &stmt.kind {
        StmtKind::FunctionDef {
            name,
            type_params,
            params,
            ..
        } => {
            assert_eq!(name, "identity");
            assert_eq!(type_params.len(), 1);
            assert_eq!(type_params[0].name, "T");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].0, "x");
            assert_eq!(params[0].1, "T");
        }
        _ => panic!("Expected function def"),
    }
}

#[test]
fn test_method_multiple_returns() {
    let stmt = parse_stmt("fn (p: Point) coords() -> (int, int) { return (1, 2) }").unwrap();
    match &stmt.kind {
        StmtKind::MethodDef(MethodDef {
            return_type,
            ..
        }) => {
            assert_eq!(*return_type, Some(vec!["int".to_string(), "int".to_string()]));
        }
        _ => panic!("Expected method def"),
    }
}

// ==================== BLOCK TESTS ====================

#[test]
fn test_block_empty() {
    let stmt = parse_stmt("{ }").unwrap();
    match &stmt.kind {
        StmtKind::Block(stmts) => {
            assert_eq!(stmts.len(), 0);
        }
        _ => panic!("Expected block"),
    }
}

#[test]
fn test_block_multiple_stmts() {
    let stmt = parse_stmt("{ var x := 10\nvar y := 20 }").unwrap();
    match &stmt.kind {
        StmtKind::Block(stmts) => {
            assert_eq!(stmts.len(), 2);
        }
        _ => panic!("Expected block"),
    }
}
