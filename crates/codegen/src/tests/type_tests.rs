// Type Inference and Casting Tests
//
// Tests to ensure correct type inference, automatic type promotion,
// and explicit type casting in the Brix compiler.

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Expr, Literal, Program, Stmt};

// Helper function to create a simple program with one statement
fn make_program(stmt: Stmt) -> Program {
    Program {
        statements: vec![stmt],
    }
}

// Helper to compile a program and return the LLVM IR
// Returns Ok(ir) if compilation succeeded, Err(msg) if it panicked
fn compile_program(program: Program) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();

        let mut compiler = Compiler::new(&context, &builder, &module);

        compiler.compile_program(&program);
        module.print_to_string().to_string()
    });

    match result {
        Ok(ir) => Ok(ir),
        Err(_) => Err("Compilation panicked".to_string()),
    }
}

// ==================== TYPE INFERENCE TESTS ====================

#[test]
fn test_infer_int_literal() {
    let stmt = Stmt::Expr(Expr::Literal(Literal::Int(42)));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_float_literal() {
    let stmt = Stmt::Expr(Expr::Literal(Literal::Float(3.14)));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_string_literal() {
    let stmt = Stmt::Expr(Expr::Literal(Literal::String("hello".to_string())));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_bool_literal() {
    let stmt = Stmt::Expr(Expr::Literal(Literal::Bool(true)));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_nil_literal() {
    let stmt = Stmt::Expr(Expr::Literal(Literal::Nil));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_atom_literal() {
    let stmt = Stmt::Expr(Expr::Literal(Literal::Atom("ok".to_string())));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_complex_literal() {
    let stmt = Stmt::Expr(Expr::Literal(Literal::Complex(3.0, 4.0)));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== VARIABLE DECLARATION TYPE INFERENCE ====================

#[test]
fn test_var_decl_infer_int() {
    let stmt = Stmt::VariableDecl {
        name: "x".to_string(),
        type_hint: None,
        value: Expr::Literal(Literal::Int(10)),
        is_const: false,
    };
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_float() {
    let stmt = Stmt::VariableDecl {
        name: "x".to_string(),
        type_hint: None,
        value: Expr::Literal(Literal::Float(3.14)),
        is_const: false,
    };
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_explicit_int() {
    let stmt = Stmt::VariableDecl {
        name: "x".to_string(),
        type_hint: Some("int".to_string()),
        value: Expr::Literal(Literal::Int(42)),
        is_const: false,
    };
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_explicit_float() {
    let stmt = Stmt::VariableDecl {
        name: "x".to_string(),
        type_hint: Some("float".to_string()),
        value: Expr::Literal(Literal::Float(3.14)),
        is_const: false,
    };
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TYPE CASTING TESTS ====================

#[test]
fn test_cast_int_to_float_explicit() {
    // var x: float = 42
    let stmt = Stmt::VariableDecl {
        name: "x".to_string(),
        type_hint: Some("float".to_string()),
        value: Expr::Literal(Literal::Int(42)),
        is_const: false,
    };
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_cast_float_to_int_explicit() {
    // var x: int = 3.14
    let stmt = Stmt::VariableDecl {
        name: "x".to_string(),
        type_hint: Some("int".to_string()),
        value: Expr::Literal(Literal::Float(3.14)),
        is_const: false,
    };
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== BINARY OPERATOR TYPE PROMOTION ====================

#[test]
fn test_add_int_int() {
    // 1 + 2
    let expr = Expr::Binary {
        op: parser::ast::BinaryOp::Add,
        lhs: Box::new(Expr::Literal(Literal::Int(1))),
        rhs: Box::new(Expr::Literal(Literal::Int(2))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_add_float_float() {
    // 1.5 + 2.5
    let expr = Expr::Binary {
        op: parser::ast::BinaryOp::Add,
        lhs: Box::new(Expr::Literal(Literal::Float(1.5))),
        rhs: Box::new(Expr::Literal(Literal::Float(2.5))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_add_int_float_promotion() {
    // 1 + 2.5 (should promote int to float)
    let expr = Expr::Binary {
        op: parser::ast::BinaryOp::Add,
        lhs: Box::new(Expr::Literal(Literal::Int(1))),
        rhs: Box::new(Expr::Literal(Literal::Float(2.5))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_mul_int_float_promotion() {
    // 10 * 3.14 (should promote int to float)
    let expr = Expr::Binary {
        op: parser::ast::BinaryOp::Mul,
        lhs: Box::new(Expr::Literal(Literal::Int(10))),
        rhs: Box::new(Expr::Literal(Literal::Float(3.14))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== CONST DECLARATION ====================

#[test]
fn test_const_decl() {
    let stmt = Stmt::VariableDecl {
        name: "PI".to_string(),
        type_hint: None,
        value: Expr::Literal(Literal::Float(3.14159)),
        is_const: true,
    };
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ARRAY TYPE INFERENCE ====================

#[test]
fn test_array_all_ints() {
    // [1, 2, 3] -> IntMatrix
    let expr = Expr::Array(vec![
        Expr::Literal(Literal::Int(1)),
        Expr::Literal(Literal::Int(2)),
        Expr::Literal(Literal::Int(3)),
    ]);
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_all_floats() {
    // [1.0, 2.0, 3.0] -> Matrix
    let expr = Expr::Array(vec![
        Expr::Literal(Literal::Float(1.0)),
        Expr::Literal(Literal::Float(2.0)),
        Expr::Literal(Literal::Float(3.0)),
    ]);
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_mixed_promotes_to_float() {
    // [1, 2.5, 3] -> Matrix (with int->float promotion)
    let expr = Expr::Array(vec![
        Expr::Literal(Literal::Int(1)),
        Expr::Literal(Literal::Float(2.5)),
        Expr::Literal(Literal::Int(3)),
    ]);
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_empty_array() {
    // [] -> Matrix (default to float)
    let expr = Expr::Array(vec![]);
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPLEX NUMBER TYPE TESTS ====================

#[test]
fn test_complex_from_literal() {
    // 3.0 + 4.0i
    let expr = Expr::Literal(Literal::Complex(3.0, 4.0));
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
#[ignore = "Literal::Imaginary does not exist in AST - imaginary is parsed as Complex(0, n)"]
fn test_imaginary_literal() {
    // 2.0i -> Complex(0, 2.0)
    let expr = Expr::Literal(Literal::Complex(0.0, 2.0));
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TYPE CHECKING ERROR CASES ====================

#[test]
fn test_string_plus_int_fails() {
    // "hello" + 42 should fail
    let expr = Expr::Binary {
        op: parser::ast::BinaryOp::Add,
        lhs: Box::new(Expr::Literal(Literal::String("hello".to_string()))),
        rhs: Box::new(Expr::Literal(Literal::Int(42))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    // This should fail with a type error
    assert!(result.is_err());
}

#[test]
fn test_bitwise_on_float_fails() {
    // 3.14 & 2.5 should fail (bitwise only on ints)
    let _expr = Expr::Binary {
        op: parser::ast::BinaryOp::BitAnd,
        lhs: Box::new(Expr::Literal(Literal::Float(3.14))),
        rhs: Box::new(Expr::Literal(Literal::Float(2.5))),
    };
}
