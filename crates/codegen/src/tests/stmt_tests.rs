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
// ==================== VARIABLE DECLARATIONS - TYPE INFERENCE ====================

#[test]
fn test_var_decl_infer_float() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "pi".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Float(3.14)),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_string() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "msg".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::String("hello".to_string())),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_intmatrix() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "arr".to_string(),
            type_hint: None,
            value: Expr::Array(vec![
                Expr::Literal(Literal::Int(1)),
                Expr::Literal(Literal::Int(2)),
                Expr::Literal(Literal::Int(3)),
            ]),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_matrix() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "arr".to_string(),
            type_hint: None,
            value: Expr::Array(vec![
                Expr::Literal(Literal::Float(1.0)),
                Expr::Literal(Literal::Float(2.0)),
            ]),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_atom() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "status".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Atom("ok".to_string())),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_complex() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "z".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Complex(3.0, 4.0)),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== VARIABLE DECLARATIONS - TYPE CASTING ====================

#[test]
fn test_var_decl_cast_int_to_float() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("float".to_string()),
            value: Expr::Literal(Literal::Int(10)),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_cast_float_to_int() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("int".to_string()),
            value: Expr::Literal(Literal::Float(3.14)),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== CONST DECLARATIONS ====================

#[test]
fn test_const_declaration() {
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "MAX".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Int(100)),
            is_const: true,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ASSIGNMENTS - COMPOUND ====================

#[test]
fn test_assignment_add_compound() {
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
                value: Expr::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::Identifier("x".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Int(5))),
                },
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment_to_array_element() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                ]),
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("arr".to_string())),
                    indices: vec!(Expr::Literal(Literal::Int(0))),
                },
                value: Expr::Literal(Literal::Int(10)),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment_to_matrix_element() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "mat".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![
                        Expr::Literal(Literal::Int(2)),
                        Expr::Literal(Literal::Int(2)),
                    ],
                },
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Index {
                        array: Box::new(Expr::Identifier("mat".to_string())),
                        indices: vec!(Expr::Literal(Literal::Int(0))),
                    }),
                    indices: vec!(Expr::Literal(Literal::Int(0))),
                },
                value: Expr::Literal(Literal::Float(5.5)),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== IMPORT STATEMENTS ====================

#[test]
fn test_import_statement() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_import_with_alias_stmt() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: Some("m".to_string()),
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_imports() {
    let program = Program {
        statements: vec![
            Stmt::Import {
                module: "math".to_string(),
                alias: None,
            },
            Stmt::Import {
                module: "math".to_string(),
                alias: Some("m".to_string()),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== EXPRESSION STATEMENTS ====================

#[test]
fn test_function_call_as_statement() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("print".to_string())),
            args: vec![Expr::Literal(Literal::String("hello".to_string()))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_increment_as_statement() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(0)),
                is_const: false,
            },
            Stmt::Expr(Expr::Increment {
                expr: Box::new(Expr::Identifier("x".to_string())),
                is_prefix: true,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_decrement_as_statement() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(10)),
                is_const: false,
            },
            Stmt::Expr(Expr::Decrement {
                expr: Box::new(Expr::Identifier("x".to_string())),
                is_prefix: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== PRINT/PRINTLN STATEMENTS ====================

#[test]
fn test_print_statement() {
    let program = Program {
        statements: vec![Stmt::Print {
            expr: Expr::Literal(Literal::String("test".to_string())),
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_println_statement() {
    let program = Program {
        statements: vec![Stmt::Println {
            expr: Expr::Literal(Literal::Int(42)),
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MULTIPLE STATEMENTS ====================

#[test]
fn test_multiple_variable_declarations() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(10)),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(20)),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "z".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(30)),
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_sequential_assignments() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(1)),
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Identifier("x".to_string()),
                value: Expr::Literal(Literal::Int(2)),
            },
            Stmt::Assignment {
                target: Expr::Identifier("x".to_string()),
                value: Expr::Literal(Literal::Int(3)),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== NESTED BLOCKS ====================

#[test]
fn test_nested_blocks() {
    let program = Program {
        statements: vec![Stmt::Block(vec![
            Stmt::Block(vec![
                Stmt::Expr(Expr::Literal(Literal::Int(1))),
            ]),
            Stmt::Block(vec![
                Stmt::Expr(Expr::Literal(Literal::Int(2))),
            ]),
        ])],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== DESTRUCTURING ====================

#[test]
fn test_destructuring_declaration() {
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "get_pair".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::Return {
                    values: vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Int(2)),
                    ],
                }),
            },
            Stmt::DestructuringDecl {
                is_const: false,
                names: vec!["a".to_string(), "b".to_string()],
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("get_pair".to_string())),
                    args: vec![],
                },
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_destructuring_with_ignore() {
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "get_triple".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                ]),
                body: Box::new(Stmt::Return {
                    values: vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Int(2)),
                        Expr::Literal(Literal::Int(3)),
                    ],
                }),
            },
            Stmt::DestructuringDecl {
                is_const: false,
                names: vec!["x".to_string(), "_".to_string(), "z".to_string()],
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("get_triple".to_string())),
                    args: vec![],
                },
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
