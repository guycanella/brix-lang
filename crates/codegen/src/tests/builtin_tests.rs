// Built-in Function Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Expr, Literal, Program, Stmt};

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
fn test_typeof_builtin() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("typeof".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("typeof") || ir.contains("call"));
}

#[test]
fn test_int_conversion() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("int".to_string())),
            args: vec![Expr::Literal(Literal::Float(3.14))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_conversion() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("float".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_conversion() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("string".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_print_builtin() {
    let program = Program {
        statements: vec![Stmt::Print {
            expr: Expr::Literal(Literal::String("hello".to_string())),
        }],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("printf") || ir.contains("call"));
}

#[test]
fn test_println_builtin() {
    let program = Program {
        statements: vec![Stmt::Println {
            expr: Expr::Literal(Literal::String("hello".to_string())),
        }],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("printf") || ir.contains("call"));
}
