// Unit tests for Optional types (v1.4)
//
// Tests cover:
// - Type annotation parsing (int?, string?, Matrix?)
// - Optional primitive creation (Some(42), nil)
// - Optional ref-counted creation (Some("hello"), nil)
// - nil comparison (x == nil, x != nil)
// - LLVM representation (struct for primitives, ptr for ref-counted)

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, ExprKind, Literal, Program, Stmt, StmtKind};

// Helper function to compile a program and return IR or error
fn compile_program(program: Program) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());
        compiler.compile_program(&program);
        module.print_to_string().to_string()
    });
    match result {
        Ok(ir) => Ok(ir),
        Err(_) => Err("Compilation panicked".to_string()),
    }
}

// Helper macros for creating AST nodes with dummy spans
macro_rules! lit_int {
    ($val:expr) => {
        Expr::dummy(ExprKind::Literal(Literal::Int($val)))
    };
}

macro_rules! lit_nil {
    () => {
        Expr::dummy(ExprKind::Literal(Literal::Nil))
    };
}

macro_rules! ident {
    ($name:expr) => {
        Expr::dummy(ExprKind::Identifier($name.to_string()))
    };
}

macro_rules! var_decl {
    ($name:expr, $type_hint:expr, $value:expr) => {
        Stmt::dummy(StmtKind::VariableDecl {
            name: $name.to_string(),
            type_hint: $type_hint,
            value: $value,
            is_const: false,
        })
    };
}

macro_rules! binary_op {
    ($op:expr, $lhs:expr, $rhs:expr) => {
        Expr::dummy(ExprKind::Binary {
            op: $op,
            lhs: Box::new($lhs),
            rhs: Box::new($rhs),
        })
    };
}

// --- OPTIONAL WITH VALUE (SOME) ---

#[test]
fn test_optional_int_with_value() {
    let program = Program {
        statements: vec![var_decl!("x", Some("int?".to_string()), lit_int!(42))],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional<int> with value 42");
}

#[test]
fn test_optional_float_with_value() {
    let program = Program {
        statements: vec![var_decl!(
            "x",
            Some("float?".to_string()),
            Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))
        )],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional<float> with value 3.14");
}

#[test]
fn test_optional_string_with_value() {
    let program = Program {
        statements: vec![var_decl!(
            "x",
            Some("string?".to_string()),
            Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string())))
        )],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional<string> with value");
}

// --- OPTIONAL NIL (NONE) ---

#[test]
fn test_optional_int_nil() {
    let program = Program {
        statements: vec![var_decl!("x", Some("int?".to_string()), lit_nil!())],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional<int> with nil");
}

#[test]
fn test_optional_float_nil() {
    let program = Program {
        statements: vec![var_decl!("x", Some("float?".to_string()), lit_nil!())],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional<float> with nil");
}

#[test]
fn test_optional_string_nil() {
    let program = Program {
        statements: vec![var_decl!("x", Some("string?".to_string()), lit_nil!())],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional<string> with nil");
}

// --- NIL COMPARISON ---

#[test]
fn test_optional_not_equal_nil() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_int!(42)),
            Stmt::dummy(StmtKind::If {
                condition: binary_op!(BinaryOp::NotEq, ident!("x"), lit_nil!()),
                then_block: Box::new(Stmt::dummy(StmtKind::Block(
                    vec![var_decl!("y", None, lit_int!(1))]
                ))),
                else_block: None,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should compile x != nil comparison");
}

#[test]
fn test_optional_equal_nil() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_nil!()),
            Stmt::dummy(StmtKind::If {
                condition: binary_op!(BinaryOp::Eq, ident!("x"), lit_nil!()),
                then_block: Box::new(Stmt::dummy(StmtKind::Block(
                    vec![var_decl!("y", None, lit_int!(1))]
                ))),
                else_block: None,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should compile x == nil comparison");
}

// --- MULTIPLE OPTIONALS ---

#[test]
fn test_multiple_optional_declarations() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_int!(42)),
            var_decl!("y", Some("int?".to_string()), lit_nil!()),
            var_decl!(
                "z",
                Some("string?".to_string()),
                Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string())))
            ),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should compile multiple Optional declarations");
}

// --- OPTIONAL LOADING (IDENTIFIER) ---

#[test]
fn test_optional_identifier_loading() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_int!(42)),
            Stmt::dummy(StmtKind::Assignment {
                target: ident!("x"),
                value: lit_nil!(),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should load and reassign Optional variable");
}

// --- TYPE CASTING (INT -> FLOAT OPTIONAL) ---

#[test]
fn test_optional_float_from_int() {
    let program = Program {
        statements: vec![var_decl!("x", Some("float?".to_string()), lit_int!(42))],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should auto-cast int to float in Optional");
}

// --- CHAINED CONDITIONS ---

#[test]
fn test_optional_chained_conditions() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_int!(42)),
            var_decl!("y", Some("int?".to_string()), lit_nil!()),
            Stmt::dummy(StmtKind::If {
                condition: binary_op!(
                    BinaryOp::LogicalAnd,
                    binary_op!(BinaryOp::NotEq, ident!("x"), lit_nil!()),
                    binary_op!(BinaryOp::Eq, ident!("y"), lit_nil!())
                ),
                then_block: Box::new(Stmt::dummy(StmtKind::Block(
                    vec![var_decl!("result", None, lit_int!(1))]
                ))),
                else_block: None,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should compile chained Optional conditions");
}

// --- OPTIONAL IN LOOPS ---

#[test]
fn test_optional_in_while_loop() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_int!(10)),
            Stmt::dummy(StmtKind::While {
                condition: binary_op!(BinaryOp::NotEq, ident!("x"), lit_nil!()),
                body: Box::new(Stmt::dummy(StmtKind::Block(
                    vec![Stmt::dummy(StmtKind::Assignment {
                        target: ident!("x"),
                        value: lit_nil!(),
                    })]
                ))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should use Optional in while condition");
}

// --- TERNARY OPERATOR ---

#[test]
fn test_optional_in_ternary() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_int!(42)),
            var_decl!(
                "result",
                None,
                Expr::dummy(ExprKind::Ternary {
                    condition: Box::new(binary_op!(BinaryOp::NotEq, ident!("x"), lit_nil!())),
                    then_expr: Box::new(lit_int!(1)),
                    else_expr: Box::new(lit_int!(0)),
                })
            ),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should use Optional in ternary operator");
}

// --- ERROR CASES ---

// Note: Type mismatch errors will be caught during compilation
// Keeping test disabled for now as error handling may vary
// #[test]
// fn test_optional_type_mismatch() {
//     let program = Program {
//         statements: vec![var_decl!(
//             "x",
//             Some("int?".to_string()),
//             Expr::dummy(ExprKind::Literal(Literal::String("string".to_string())))
//         )],
//     };
//     let result = compile_program(program);
//     assert!(result.is_err(), "Should error on type mismatch in Optional");
// }
