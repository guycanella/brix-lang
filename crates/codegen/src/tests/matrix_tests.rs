// Matrix Operations Codegen Tests

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
fn test_zeros_function() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zeros".to_string())),
            args: vec![Expr::Literal(Literal::Int(5))],
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("zeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_izeros_function() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("izeros".to_string())),
            args: vec![Expr::Literal(Literal::Int(5))],
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("izeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_zeros_2d() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zeros".to_string())),
            args: vec![
                Expr::Literal(Literal::Int(3)),
                Expr::Literal(Literal::Int(4)),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_index_1d() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(5))],
                },
                is_const: false,
            },
            Stmt::Expr(Expr::Index {
                array: Box::new(Expr::Identifier("m".to_string())),
                indices: vec![Expr::Literal(Literal::Int(0))],
            }),
        ],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("getelementptr") || ir.contains("load"));
}

#[test]
fn test_matrix_index_2d() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![
                        Expr::Literal(Literal::Int(3)),
                        Expr::Literal(Literal::Int(4)),
                    ],
                },
                is_const: false,
            },
            Stmt::Expr(Expr::Index {
                array: Box::new(Expr::Identifier("m".to_string())),
                indices: vec![
                    Expr::Literal(Literal::Int(0)),
                    Expr::Literal(Literal::Int(1)),
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_static_init_int() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::StaticInit {
            element_type: "int".to_string(),
            dimensions: vec![Expr::Literal(Literal::Int(5))],
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("izeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_static_init_float() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::StaticInit {
            element_type: "float".to_string(),
            dimensions: vec![Expr::Literal(Literal::Int(5))],
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("zeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_static_init_2d() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::StaticInit {
            element_type: "float".to_string(),
            dimensions: vec![
                Expr::Literal(Literal::Int(3)),
                Expr::Literal(Literal::Int(4)),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_int() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Array(vec![
            Expr::Literal(Literal::Int(1)),
            Expr::Literal(Literal::Int(2)),
            Expr::Literal(Literal::Int(3)),
        ]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_float() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Array(vec![
            Expr::Literal(Literal::Float(1.0)),
            Expr::Literal(Literal::Float(2.0)),
            Expr::Literal(Literal::Float(3.0)),
        ]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
