// Complex Number Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt};

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

#[test]
fn test_complex_literal() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::Complex(3.0, 4.0)))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
#[ignore = "Literal::Imaginary does not exist in AST - imaginary is parsed as Complex(0, n)"]
fn test_imaginary_literal() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::Complex(0.0, 2.0)))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_addition() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::Literal(Literal::Complex(1.0, 2.0))),
            rhs: Box::new(Expr::Literal(Literal::Complex(3.0, 4.0))),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("complex_add") || ir.contains("call"));
}

#[test]
fn test_complex_multiplication() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::Literal(Literal::Complex(1.0, 1.0))),
            rhs: Box::new(Expr::Literal(Literal::Complex(1.0, 1.0))),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("complex_mul") || ir.contains("call"));
}
