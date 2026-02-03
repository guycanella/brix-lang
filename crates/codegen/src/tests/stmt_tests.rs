// Statement Codegen Tests

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
fn test_variable_decl_inferred() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Int(10)),
            is_const: false,
        }],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("alloca") || ir.contains("store"));
}

#[test]
fn test_variable_decl_explicit_type() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("int".to_string()),
            value: Expr::Literal(Literal::Int(42)),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(10)),
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Identifier("x".to_string()),
                value: Expr::Literal(Literal::Int(20)),
            },
        ],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("store"));
}

#[test]
fn test_block_statement() {
    let program = Program {
        statements: vec![Stmt::Block(vec![
            Stmt::Expr(Expr::Literal(Literal::Int(1))),
            Stmt::Expr(Expr::Literal(Literal::Int(2))),
        ])],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_return_statement() {
    let program = Program {
        statements: vec![Stmt::FunctionDef {
            name: "test_fn".to_string(),
            params: vec![],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(Stmt::Return {
                values: vec![Expr::Literal(Literal::Int(42))],
            }),
        }],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("ret"));
}
