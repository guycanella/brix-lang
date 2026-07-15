// Matrix Operations Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Closure, Expr, ExprKind, Literal, Program, Stmt, StmtKind, UnaryOp};

fn compile_program(program: Program) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(
            &context,
            &builder,
            &module,
            "test.bx".to_string(),
            "".to_string(),
        );
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
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
        })))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("zeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_izeros_function() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
        })))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("izeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_zeros_2d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                Expr::dummy(ExprKind::Literal(Literal::Int(4))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_index_1d() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Index {
                array: Box::new(Expr::dummy(ExprKind::Identifier("m".to_string()))),
                indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
            }))),
        ],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("getelementptr") || ir.contains("load"));
}

#[test]
fn test_matrix_index_2d() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Index {
                array: Box::new(Expr::dummy(ExprKind::Identifier("m".to_string()))),
                indices: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_static_init_int() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::StaticInit {
                element_type: "int".to_string(),
                dimensions: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
            },
        )))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("izeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_static_init_float() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::StaticInit {
                element_type: "float".to_string(),
                dimensions: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
            },
        )))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("zeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_static_init_2d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::StaticInit {
                element_type: "float".to_string(),
                dimensions: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                ],
            },
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_int() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(
            vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ],
        ))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_float() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(
            vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
            ],
        ))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ARRAY LITERAL EDGE CASES ====================

#[test]
fn test_array_literal_empty() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(
            vec![],
        ))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_single_element() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(
            vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        ))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_mixed_int_float() {
    // Mixed int/float should promote to Matrix (float)
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(
            vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                Expr::dummy(ExprKind::Literal(Literal::Float(2.5))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ],
        ))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_large() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(
            (0..100)
                .map(|i| Expr::dummy(ExprKind::Literal(Literal::Int(i))))
                .collect(),
        ))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_all_ints_becomes_intmatrix() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "arr".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ])),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATRIX FIELD ACCESS ====================

#[test]
fn test_matrix_field_rows() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("m".to_string()))),
                field: "rows".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_field_cols() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("m".to_string()))),
                field: "cols".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_intmatrix_field_rows() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(6))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("m".to_string()))),
                field: "rows".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_intmatrix_field_cols() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(10)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("m".to_string()))),
                field: "cols".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_field_len() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "s".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("s".to_string()))),
                field: "len".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ZEROS/IZEROS EDGE CASES ====================

#[test]
fn test_zeros_size_one() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zeros_large_size() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1000)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_izeros_size_one() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_izeros_2d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zeros_with_expression_arg() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Binary {
                op: parser::ast::BinaryOp::Add,
                lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
            })],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zeros_2d_with_expressions() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                }),
                Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                }),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATRIX ASSIGNMENT ====================

#[test]
fn test_array_element_assignment() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Int(99))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_2d_assignment() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "mat".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("mat".to_string()))),
                    indices: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Float(5.5))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_assignment_variable_index() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(10)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "i".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    indices: vec![Expr::dummy(ExprKind::Identifier("i".to_string()))],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_assignment_expression_index() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(10)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    indices: vec![Expr::dummy(ExprKind::Binary {
                        op: parser::ast::BinaryOp::Add,
                        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                    })],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_assignment_float_to_int() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "iarr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("iarr".to_string()))),
                    indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Float(3.7))), // Should truncate to 3
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_chained_assignment() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "mat".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("mat".to_string()))),
                    indices: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    ],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("mat".to_string()))),
                    indices: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    ],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== LIST COMPREHENSIONS ADVANCED ====================

#[test]
fn test_list_comprehension_empty_result() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ]))),
                    conditions: vec![Expr::dummy(ExprKind::Binary {
                        op: parser::ast::BinaryOp::Gt,
                        lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                    })],
                }],
            },
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_no_filter() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                })),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ]))),
                    conditions: vec![],
                }],
            },
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_three_loops() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Binary {
                        op: parser::ast::BinaryOp::Add,
                        lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                        rhs: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
                    })),
                    rhs: Box::new(Expr::dummy(ExprKind::Identifier("z".to_string()))),
                })),
                generators: vec![
                    parser::ast::ComprehensionGen {
                        var_names: vec!["x".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                            Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                        ]))),
                        conditions: vec![],
                    },
                    parser::ast::ComprehensionGen {
                        var_names: vec!["y".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Float(10.0))),
                            Expr::dummy(ExprKind::Literal(Literal::Float(20.0))),
                        ]))),
                        conditions: vec![],
                    },
                    parser::ast::ComprehensionGen {
                        var_names: vec!["z".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Float(100.0))),
                            Expr::dummy(ExprKind::Literal(Literal::Float(200.0))),
                        ]))),
                        conditions: vec![],
                    },
                ],
            },
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_multiple_conditions() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                    ]))),
                    conditions: vec![
                        Expr::dummy(ExprKind::Binary {
                            op: parser::ast::BinaryOp::Gt,
                            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                        }),
                        Expr::dummy(ExprKind::Binary {
                            op: parser::ast::BinaryOp::Lt,
                            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
                        }),
                    ],
                }],
            },
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_with_destructuring() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("a".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Identifier("b".to_string()))),
                })),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["a".to_string(), "b".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Call {
                        func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
                        args: vec![
                            Expr::dummy(ExprKind::Array(vec![
                                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                                Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                            ])),
                            Expr::dummy(ExprKind::Array(vec![
                                Expr::dummy(ExprKind::Literal(Literal::Float(10.0))),
                                Expr::dummy(ExprKind::Literal(Literal::Float(20.0))),
                            ])),
                        ],
                    })),
                    conditions: vec![],
                }],
            },
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_complex_expression() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::dummy(ExprKind::Binary {
                        op: parser::ast::BinaryOp::Add,
                        lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))),
                    })),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(2.0)))),
                })),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                        Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                        Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                    ]))),
                    conditions: vec![],
                }],
            },
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_nested_with_condition() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
                })),
                generators: vec![
                    parser::ast::ComprehensionGen {
                        var_names: vec!["x".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        ]))),
                        conditions: vec![Expr::dummy(ExprKind::Binary {
                            op: parser::ast::BinaryOp::Gt,
                            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                        })],
                    },
                    parser::ast::ComprehensionGen {
                        var_names: vec!["y".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                        ]))),
                        conditions: vec![],
                    },
                ],
            },
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_from_zeros() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))),
                })),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    conditions: vec![],
                }],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ZIP FUNCTION ====================

#[test]
fn test_zip_intmatrix_intmatrix() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ])),
                Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                ])),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_matrix_matrix() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                ])),
                Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(10.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(20.0))),
                ])),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_intmatrix_matrix() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ])),
                Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(10.5))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(20.5))),
                ])),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_empty_arrays() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Array(vec![])),
                Expr::dummy(ExprKind::Array(vec![])),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_different_sizes() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                ])),
                Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(10.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(20.0))),
                ])),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_with_zeros() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
                }),
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
                }),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== EYE FUNCTION ADVANCED ====================

#[test]
fn test_eye_indexing() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "identity".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("eye".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Index {
                array: Box::new(Expr::dummy(ExprKind::Identifier("identity".to_string()))),
                indices: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_eye_field_access() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "identity".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("eye".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("identity".to_string()))),
                field: "rows".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_eye_with_variable() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "n".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("eye".to_string()))),
                args: vec![Expr::dummy(ExprKind::Identifier("n".to_string()))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== INDEXING EDGE CASES ====================

#[test]
fn test_index_with_max_int() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(10)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Index {
                array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(9)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_index_zero() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Index {
                array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_2d_index_both_zero() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "mat".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Index {
                array: Box::new(Expr::dummy(ExprKind::Identifier("mat".to_string()))),
                indices: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_index_complex_expression() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(20)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "i".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Index {
                array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                indices: vec![Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("i".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATRIX/INTMATRIX INTEROPERABILITY ====================

#[test]
fn test_intmatrix_to_matrix_promotion() {
    // IntMatrix promoted to Matrix in mixed operations
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "iarr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("iarr".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(2.5)))),
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_intmatrix_index_returns_int() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "iarr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(30))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "val".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("iarr".to_string()))),
                    indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_mixed_array_types_in_comprehension() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "ints".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(1.5)))),
                })),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Identifier("ints".to_string()))),
                    conditions: vec![],
                }],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ARRAY OPERATIONS ADVANCED ====================

#[test]
fn test_array_in_variable_then_index() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "data".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.1))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.2))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.3))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "first".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("data".to_string()))),
                    indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zeros_then_multiple_assignments() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(2)))],
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_element_in_expression() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "nums".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(15))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "sum".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Index {
                        array: Box::new(Expr::dummy(ExprKind::Identifier("nums".to_string()))),
                        indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                    })),
                    rhs: Box::new(Expr::dummy(ExprKind::Index {
                        array: Box::new(Expr::dummy(ExprKind::Identifier("nums".to_string()))),
                        indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
                    })),
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_element_in_ternary() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(5.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(10.0))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "val".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Ternary {
                    condition: Box::new(Expr::dummy(ExprKind::Binary {
                        op: parser::ast::BinaryOp::Gt,
                        lhs: Box::new(Expr::dummy(ExprKind::Index {
                            array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                            indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                        })),
                        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.0)))),
                    })),
                    then_expr: Box::new(Expr::dummy(ExprKind::Index {
                        array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                        indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                    })),
                    else_expr: Box::new(Expr::dummy(ExprKind::Index {
                        array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                        indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
                    })),
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_from_function_call_then_index() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "z".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Index {
                array: Box::new(Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
                })),
                indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(2)))],
            }),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== CONSTRUCTOR EDGE CASES ====================

#[test]
fn test_zeros_from_variable_expression() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "size".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                args: vec![Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Div,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("size".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_eye_size_from_expression() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("eye".to_string()))),
            args: vec![Expr::dummy(ExprKind::Binary {
                op: parser::ast::BinaryOp::Add,
                lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            })],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_nested_zeros_calls() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "a".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "b".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ===== Phase 2a: Matrix/Array Constructors =====

#[test]
fn test_ones_1d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("ones".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_ones_2d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("ones".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                Expr::dummy(ExprKind::Literal(Literal::Int(4))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_linspace_float_args() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("linspace".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(0.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_linspace_int_args_coercion() {
    // int args for start/stop should be coerced to float
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("linspace".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                Expr::dummy(ExprKind::Literal(Literal::Int(6))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_arange_float_args() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("arange".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(0.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(0.25))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_rand_1d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("rand".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_rand_2d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("rand".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_irand() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("irand".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(6))),
                Expr::dummy(ExprKind::Literal(Literal::Int(100))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// =========================================================
// SECTION: 2D Matrix Iterator Tests (Phase 2b)
// =========================================================

/// Helper: build closure expr `(x: T) -> R { return body }`
fn make_unary_closure(param: &str, param_ty: &str, ret_ty: &str, body: Expr) -> Expr {
    Expr::dummy(ExprKind::Closure(Closure {
        params: vec![(param.to_string(), param_ty.to_string())],
        return_type: Some(ret_ty.to_string()),
        body: Box::new(Stmt::dummy(StmtKind::Return { values: vec![body] })),
        captured_vars: vec![],
        is_async: false,
    }))
}

/// Helper: build closure expr with NO return type annotation `(p: T) -> { return body }`
fn make_unary_closure_no_return(param: &str, param_ty: &str, body: Expr) -> Expr {
    Expr::dummy(ExprKind::Closure(Closure {
        params: vec![(param.to_string(), param_ty.to_string())],
        return_type: None,
        body: Box::new(Stmt::dummy(StmtKind::Return { values: vec![body] })),
        captured_vars: vec![],
        is_async: false,
    }))
}

/// Helper: build closure expr `(a: T, b: U) -> R { return body }`
fn make_binary_closure(p0: &str, t0: &str, p1: &str, t1: &str, ret_ty: &str, body: Expr) -> Expr {
    Expr::dummy(ExprKind::Closure(Closure {
        params: vec![
            (p0.to_string(), t0.to_string()),
            (p1.to_string(), t1.to_string()),
        ],
        return_type: Some(ret_ty.to_string()),
        body: Box::new(Stmt::dummy(StmtKind::Return { values: vec![body] })),
        captured_vars: vec![],
        is_async: false,
    }))
}

#[test]
fn test_map_2d_intmatrix() {
    // var m := izeros(2, 3)
    // m.map((x: int) -> int { return x + 1 })
    let zeros_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
        args: vec![
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
        ],
    });
    let callback = make_unary_closure(
        "x",
        "int",
        "int",
        Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
        }),
    );
    let map_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(zeros_call),
            field: "map".to_string(),
        })),
        args: vec![callback],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(map_call))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_map_2d_matrix() {
    // var m := zeros(2, 3)
    // m.map((x: float) -> float { return x + 1.0 })
    let zeros_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
        args: vec![
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
        ],
    });
    let callback = make_unary_closure(
        "x",
        "float",
        "float",
        Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))),
        }),
    );
    let map_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(zeros_call),
            field: "map".to_string(),
        })),
        args: vec![callback],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(map_call))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_filter_2d() {
    // ones(2, 3).filter((x: float) -> int { return 1 })
    let ones_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("ones".to_string()))),
        args: vec![
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
        ],
    });
    let callback = make_unary_closure(
        "x",
        "float",
        "int",
        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
    );
    let filter_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(ones_call),
            field: "filter".to_string(),
        })),
        args: vec![callback],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(filter_call))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_reduce_2d() {
    // ones(2, 3).reduce(0.0, (acc: float, x: float) -> float { return acc + x })
    let ones_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("ones".to_string()))),
        args: vec![
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
        ],
    });
    let callback = make_binary_closure(
        "acc",
        "float",
        "x",
        "float",
        "float",
        Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("acc".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
        }),
    );
    let reduce_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(ones_call),
            field: "reduce".to_string(),
        })),
        args: vec![
            Expr::dummy(ExprKind::Literal(Literal::Float(0.0))),
            callback,
        ],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(reduce_call))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== PHASE 2C: CLOSURE RETURN TYPE INFERENCE ====================

#[test]
fn test_infer_float_from_literal() {
    // zeros(3).map((x: float) -> { return x * 2.0 })  — no return type annotation
    let zeros_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
        args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
    });
    let callback = make_unary_closure_no_return(
        "x",
        "float",
        Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(2.0)))),
        }),
    );
    let map_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(zeros_call),
            field: "map".to_string(),
        })),
        args: vec![callback],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(map_call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("matrix_new"));
}

#[test]
fn test_infer_float_from_param() {
    // zeros(3).map((x: float) -> { return x })  — identity, infers float from param
    let zeros_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
        args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
    });
    let callback = make_unary_closure_no_return(
        "x",
        "float",
        Expr::dummy(ExprKind::Identifier("x".to_string())),
    );
    let map_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(zeros_call),
            field: "map".to_string(),
        })),
        args: vec![callback],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(map_call))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_int_from_literal() {
    // izeros(3).map((x: int) -> { return x + 1 })  — no return type annotation
    let izeros_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
        args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
    });
    let callback = make_unary_closure_no_return(
        "x",
        "int",
        Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
        }),
    );
    let map_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(izeros_call),
            field: "map".to_string(),
        })),
        args: vec![callback],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(map_call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("intmatrix_new"));
}

#[test]
fn test_infer_float_binary_mixed() {
    // zeros(3).map((x: float) -> { return x + 1 })  — float param + int literal promotes to float
    let zeros_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
        args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
    });
    let callback = make_unary_closure_no_return(
        "x",
        "float",
        Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
        }),
    );
    let map_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(zeros_call),
            field: "map".to_string(),
        })),
        args: vec![callback],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(map_call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("matrix_new"));
}

// =========================================================
// SECTION: v1.7 Group B — New array methods
// (.sort, .sort_desc, .min, .max, .flatten, .unique, .reverse,
//  .append, .prepend, .count)
// =========================================================

/// Helper: build an IntMatrix literal `[a, b, c, ...]`
fn int_array_literal(values: &[i64]) -> Expr {
    Expr::dummy(ExprKind::Array(
        values
            .iter()
            .map(|v| Expr::dummy(ExprKind::Literal(Literal::Int(*v))))
            .collect(),
    ))
}

/// Helper: build a Matrix (float) literal `[a.0, b.0, ...]`
fn float_array_literal(values: &[f64]) -> Expr {
    Expr::dummy(ExprKind::Array(
        values
            .iter()
            .map(|v| Expr::dummy(ExprKind::Literal(Literal::Float(*v))))
            .collect(),
    ))
}

/// Helper: build a no-arg method call `target.method()`
fn method_call(target: Expr, method: &str) -> Expr {
    Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(target),
            field: method.to_string(),
        })),
        args: vec![],
    })
}

#[test]
fn test_array_sort_intmatrix() {
    // [3, 1, 4, 1, 5].sort()
    let arr = int_array_literal(&[3, 1, 4, 1, 5]);
    let call = method_call(arr, "sort");
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("intmatrix_sort_asc"));
}

#[test]
fn test_array_sort_desc_matrix() {
    // [3.0, 1.0, 4.0].sort_desc()
    let arr = float_array_literal(&[3.0, 1.0, 4.0]);
    let call = method_call(arr, "sort_desc");
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("matrix_sort_desc"));
}

#[test]
fn test_array_min_intmatrix() {
    // [3, 1, 4, 1, 5].min()
    let arr = int_array_literal(&[3, 1, 4, 1, 5]);
    let call = method_call(arr, "min");
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("brix_intmatrix_min"));
}

#[test]
fn test_array_max_matrix() {
    // [3.0, 1.0, 4.0].max()
    let arr = float_array_literal(&[3.0, 1.0, 4.0]);
    let call = method_call(arr, "max");
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("brix_matrix_max"));
}

#[test]
fn test_array_flatten_intmatrix() {
    // [1, 2, 3, 4].flatten()
    let arr = int_array_literal(&[1, 2, 3, 4]);
    let call = method_call(arr, "flatten");
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("intmatrix_flatten"));
}

#[test]
fn test_array_unique_matrix() {
    // [1.0, 1.0, 2.0, 3.0].unique()
    let arr = float_array_literal(&[1.0, 1.0, 2.0, 3.0]);
    let call = method_call(arr, "unique");
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("matrix_unique"));
}

#[test]
fn test_array_reverse_intmatrix() {
    // [1, 2, 3].reverse()
    let arr = int_array_literal(&[1, 2, 3]);
    let call = method_call(arr, "reverse");
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("intmatrix_reverse"));
}

#[test]
fn test_array_append_matrix() {
    // [1.0, 2.0].append(3.0)
    let arr = float_array_literal(&[1.0, 2.0]);
    let call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(arr),
            field: "append".to_string(),
        })),
        args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.0)))],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("matrix_append"));
}

#[test]
fn test_array_prepend_intmatrix() {
    // [1, 2, 3].prepend(0)
    let arr = int_array_literal(&[1, 2, 3]);
    let call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(arr),
            field: "prepend".to_string(),
        })),
        args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("intmatrix_prepend"));
}

#[test]
fn test_array_count() {
    // [1, 2, 3, 4].count()
    let arr = int_array_literal(&[1, 2, 3, 4]);
    let call = method_call(arr, "count");
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("count_total"));
}

// =========================================================
// SECTION: v1.7 Group B — infer_expr_type_static() regression coverage
//
// Post-implementation review found that infer_expr_type_static() only
// special-cased .split() and fell through to None for the new Group B
// methods, silently misinferring closure/async return types (e.g. a
// callback returning [1.0, 2.0].max() with no explicit annotation was
// defaulting to Int instead of Float). These tests pin the fix.
// =========================================================

#[test]
fn test_infer_expr_type_static_max_on_matrix_is_float() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();
    let compiler = Compiler::new(
        &context,
        &builder,
        &module,
        "test.bx".to_string(),
        "".to_string(),
    );
    let expr = method_call(float_array_literal(&[1.0, 2.0]), "max");
    assert_eq!(
        compiler.infer_expr_type_static(&expr, &[]),
        Some(crate::BrixType::Float)
    );
}

#[test]
fn test_infer_expr_type_static_min_on_intmatrix_is_int() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();
    let compiler = Compiler::new(
        &context,
        &builder,
        &module,
        "test.bx".to_string(),
        "".to_string(),
    );
    let expr = method_call(int_array_literal(&[3, 1, 4]), "min");
    assert_eq!(
        compiler.infer_expr_type_static(&expr, &[]),
        Some(crate::BrixType::Int)
    );
}

#[test]
fn test_infer_expr_type_static_sort_preserves_matrix() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();
    let compiler = Compiler::new(
        &context,
        &builder,
        &module,
        "test.bx".to_string(),
        "".to_string(),
    );
    let expr = method_call(float_array_literal(&[3.0, 1.0]), "sort");
    assert_eq!(
        compiler.infer_expr_type_static(&expr, &[]),
        Some(crate::BrixType::Matrix)
    );
}

#[test]
fn test_infer_expr_type_static_append_preserves_intmatrix() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();
    let compiler = Compiler::new(
        &context,
        &builder,
        &module,
        "test.bx".to_string(),
        "".to_string(),
    );
    let expr = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(int_array_literal(&[1, 2])),
            field: "append".to_string(),
        })),
        args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
    });
    assert_eq!(
        compiler.infer_expr_type_static(&expr, &[]),
        Some(crate::BrixType::IntMatrix)
    );
}

#[test]
fn test_infer_expr_type_static_count_is_int() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();
    let compiler = Compiler::new(
        &context,
        &builder,
        &module,
        "test.bx".to_string(),
        "".to_string(),
    );
    let expr = method_call(int_array_literal(&[1, 2, 3]), "count");
    assert_eq!(
        compiler.infer_expr_type_static(&expr, &[]),
        Some(crate::BrixType::Int)
    );
}

#[test]
fn test_map_callback_infers_matrix_via_max() {
    // izeros(3).map((x: int) -> { return [1.0, 2.0].max() })
    // No explicit closure return type — must infer Float (via .max() on a
    // Matrix) and therefore allocate the *map result* as a Matrix, not
    // IntMatrix. Note the callback body's own [1.0, 2.0] literal also
    // allocates a Matrix, so we must pin the `%map_result = ...` call site
    // specifically rather than just check for "matrix_new" anywhere in the
    // IR (that substring is present regardless of the inferred type).
    let zeros_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
        args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
    });
    let callback = make_unary_closure_no_return(
        "x",
        "int",
        method_call(float_array_literal(&[1.0, 2.0]), "max"),
    );
    let map_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(zeros_call),
            field: "map".to_string(),
        })),
        args: vec![callback],
    });
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(map_call))],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("%map_result = call ptr @matrix_new"),
        "expected map result to be allocated via matrix_new (Float inferred from .max()), got IR:\n{}",
        ir
    );
}

// =========================================================
// SECTION: v1.7 Group C — Array slicing + negative index
// (closed-range slicing only: `nums[1..4]` / `nums[1..<4]`;
//  open-ended slicing `nums[..<3]` / `nums[2..]` is out of scope)
// =========================================================

/// Helper: build an inclusive/exclusive range expr `start..end` / `start..<end`
fn range_expr(start: i64, end: i64, inclusive: bool) -> Expr {
    Expr::dummy(ExprKind::Range {
        start: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(start)))),
        end: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(end)))),
        step: None,
        inclusive,
    })
}

/// Helper: build `target[index_expr]`
fn index_expr(target: Expr, index: Expr) -> Expr {
    Expr::dummy(ExprKind::Index {
        array: Box::new(target),
        indices: vec![index],
    })
}

/// Helper: build unary negation `-expr`
fn negate(expr: Expr) -> Expr {
    Expr::dummy(ExprKind::Unary {
        op: UnaryOp::Negate,
        expr: Box::new(expr),
    })
}

/// Helper: build a stepped range expr `start..end step step_val`
fn stepped_range_expr(start: i64, end: i64, step_val: i64, inclusive: bool) -> Expr {
    Expr::dummy(ExprKind::Range {
        start: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(start)))),
        end: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(end)))),
        step: Some(Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(
            step_val,
        ))))),
        inclusive,
    })
}

#[test]
fn test_slice_inclusive_intmatrix() {
    // [10, 20, 30, 40, 50][1..3]
    let arr = int_array_literal(&[10, 20, 30, 40, 50]);
    let sliced = index_expr(arr, range_expr(1, 3, true));
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(sliced))],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("call ptr @intmatrix_slice"),
        "expected call to intmatrix_slice, got IR:\n{}",
        ir
    );
    // Inclusive `1..3` must reach the runtime call as exclusive `end=4` (3 + 1).
    // LLVM constant-folds the `end + 1` for literal operands, so we assert on
    // the call-site argument rather than a named `slice_end_incl` instruction.
    assert!(
        ir.contains("call ptr @intmatrix_slice(ptr %alloc_intarr, i64 1, i64 4)"),
        "expected inclusive-end adjustment (3 -> 4) at the slice call site, got IR:\n{}",
        ir
    );
}

#[test]
fn test_slice_exclusive_matrix() {
    // [1.0, 2.0, 3.0, 4.0][1..<3]
    let arr = float_array_literal(&[1.0, 2.0, 3.0, 4.0]);
    let sliced = index_expr(arr, range_expr(1, 3, false));
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(sliced))],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("call ptr @matrix_slice"),
        "expected call to matrix_slice, got IR:\n{}",
        ir
    );
    // Exclusive range must NOT go through the +1 adjustment
    assert!(
        !ir.contains("slice_end_incl"),
        "exclusive slice should not adjust end, got IR:\n{}",
        ir
    );
}

#[test]
fn test_negative_index_literal() {
    // [10, 20, 30][-1]
    let arr = int_array_literal(&[10, 20, 30]);
    let idx = negate(Expr::dummy(ExprKind::Literal(Literal::Int(1))));
    let indexed = index_expr(arr, idx);
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(indexed))],
    };
    let ir = compile_program(program).unwrap();
    // Negative index adjustment: idx < 0 ? idx + len : idx
    // For a *literal* negative index (`-1`), inkwell/LLVM constant-folds the
    // `icmp slt` comparison down to `i1 true` at build time, so we can't
    // assert on "idx_is_neg" / "icmp slt" text directly — assert on the
    // surviving `idx_adjusted` (idx + len) and `select` instructions instead.
    assert!(
        ir.contains("idx_adjusted"),
        "expected idx + len adjustment, got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("select"),
        "expected select to pick adjusted vs raw index, got IR:\n{}",
        ir
    );
    // Length is rows*cols (not bare cols), so flat single-index access stays
    // correct for 2D matrices too (see review finding: negative index on a
    // rows>1 matrix must use total element count, not just cols).
    assert!(
        ir.contains("neg_idx_total") && ir.contains("mul i64 %rows"),
        "expected adjusted index to add rows*cols (total) to the negative index, got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("add i64 -1, %neg_idx_total") || ir.contains("add i64 %neg_idx_total"),
        "expected the index adjustment to add the computed total, got IR:\n{}",
        ir
    );
}

#[test]
fn test_negative_index_assignment() {
    // var nums := [10, 20, 30]
    // nums[-1] := 99
    let decl = Stmt::dummy(StmtKind::VariableDecl {
        name: "nums".to_string(),
        type_hint: None,
        value: int_array_literal(&[10, 20, 30]),
        is_const: false,
    });
    let idx = negate(Expr::dummy(ExprKind::Literal(Literal::Int(1))));
    let target = index_expr(Expr::dummy(ExprKind::Identifier("nums".to_string())), idx);
    let assign = Stmt::dummy(StmtKind::Assignment {
        target,
        value: Expr::dummy(ExprKind::Literal(Literal::Int(99))),
    });
    let program = Program {
        statements: vec![decl, assign],
    };
    let ir = compile_program(program).unwrap();
    // Same constant-folding caveat as test_negative_index_literal applies here.
    assert!(
        ir.contains("idx_adjusted"),
        "expected idx + len adjustment in lvalue path, got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("select"),
        "expected select in lvalue negative-index path, got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("store i64 99, ptr %addr_ptr"),
        "expected the assignment to store through the adjusted address, got IR:\n{}",
        ir
    );
}

// =========================================================
// SECTION: v1.7 Group C review fixes — regression coverage
// (stepped-range rejection, float-range type error, negative
// index on a genuine 2D matrix using rows*cols)
// =========================================================

#[test]
fn test_slice_with_step_is_rejected() {
    // [10, 20, 30, 40, 50][0..4 step 2] — stepped slicing was never
    // implemented; silently compiling a contiguous slice while dropping
    // `step` would be wrong, not just unsupported, so this must error.
    let arr = int_array_literal(&[10, 20, 30, 40, 50]);
    let sliced = index_expr(arr, stepped_range_expr(0, 4, 2, false));
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(sliced))],
    };
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();
    let mut compiler = Compiler::new(
        &context,
        &builder,
        &module,
        "test.bx".to_string(),
        "".to_string(),
    );
    let result = compiler.compile_program(&program);
    assert!(
        result.is_err(),
        "expected stepped range as a slice index to be rejected"
    );
}

#[test]
fn test_slice_with_float_bounds_is_rejected() {
    // [10, 20, 30][1.0..3.0] — start/end must be Int; a Float range should
    // produce a clean CodegenError, not panic on into_int_value().
    let arr = int_array_literal(&[10, 20, 30]);
    let range = Expr::dummy(ExprKind::Range {
        start: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))),
        end: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.0)))),
        step: None,
        inclusive: false,
    });
    let sliced = index_expr(arr, range);
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(sliced))],
    };
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();
    let mut compiler = Compiler::new(
        &context,
        &builder,
        &module,
        "test.bx".to_string(),
        "".to_string(),
    );
    let result = compiler.compile_program(&program);
    assert!(
        result.is_err(),
        "expected Float slice bounds to be rejected with a CodegenError"
    );
}

#[test]
fn test_negative_index_on_2d_matrix_uses_total_elements() {
    // izeros(2, 3); m[-1] must equal m[5] (the last of rows*cols=6 elements),
    // not m[2] (which is what bare `cols` would have given before the fix).
    let zeros_call = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
        args: vec![
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
        ],
    });
    let idx = negate(Expr::dummy(ExprKind::Literal(Literal::Int(1))));
    let indexed = index_expr(zeros_call, idx);
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(indexed))],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("neg_idx_total") && ir.contains("mul i64 %rows"),
        "expected negative index on a 2D matrix to compute rows*cols as the total, got IR:\n{}",
        ir
    );
}

// =========================================================
// SECTION: Union max_type sizing bugfix (discovered during v1.7
// Grupo D scoping, unrelated to Grupo D itself — pre-existing since
// Union types were introduced). `size_of()` on an LLVM aggregate type
// isn't always constant-foldable; the old code silently treated any
// non-foldable size as 8 bytes, undersizing the union's storage field
// for any variant wider than 8 bytes (e.g. Complex's `{ f64, f64 }`,
// 16 bytes) and overflowing it on write. Fixed by computing size
// structurally (`llvm_type_byte_size`) instead of via `size_of()`.
// =========================================================

#[test]
fn test_union_with_complex_variant_sizes_correctly() {
    // var c: int | complex = complex(1.0, 2.0)
    // The union's LLVM type must size its value field to fit the
    // *largest* variant (Complex, 16 bytes), not silently default to 8.
    let decl = Stmt::dummy(StmtKind::VariableDecl {
        name: "c".to_string(),
        type_hint: Some("int | complex".to_string()),
        value: Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("complex".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
            ],
        }),
        is_const: false,
    });
    let program = Program {
        statements: vec![decl],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("alloca { i64, { double, double } }"),
        "expected the union's value field to be sized for Complex (16 bytes), not truncated to i64, got IR:\n{}",
        ir
    );
}

// =========================================================
// SECTION: Grupo I — List comprehension result type inference (v1.7)
// `compile_list_comprehension()` used to hardcode `BrixType::Float` for
// the result element type; it now calls `infer_expr_type_static()` using
// each generator's iterable type to bind the comprehension expression's
// params before inferring. These tests assert on the LLVM IR: the
// *result* array allocation (`%result_array = call ptr @...`) must use
// `intmatrix_new` for an inferred `IntMatrix` result and `matrix_new`
// for `Matrix` — not just "the function name appears somewhere in the
// IR" (the source array literal `[1,2,3]` also allocates via
// `intmatrix_new`, so that alone would be a false positive).
// =========================================================

/// Like the module-level `compile_program`, but propagates the actual
/// `CodegenResult` error instead of silently discarding it — needed to
/// assert on `is_err()` for a case that must be rejected at compile time.
fn compile_program_checked(program: Program) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(
            &context,
            &builder,
            &module,
            "test.bx".to_string(),
            "".to_string(),
        );
        match compiler.compile_program(&program) {
            Ok(_) => Ok(module.print_to_string().to_string()),
            Err(e) => Err(format!("Compilation error: {:?}", e)),
        }
    });
    match result {
        Ok(inner) => inner,
        Err(_) => Err("Compilation panicked".to_string()),
    }
}

#[test]
fn test_comprehension_int_type() {
    // var evens := [x * 2 for x in [1, 2, 3]]
    // Result element type must infer to Int -> result array allocated via
    // intmatrix_new (checked specifically for the *result_array* SSA
    // value, not just "intmatrix_new somewhere in the IR" — the source
    // array literal [1,2,3] also allocates via intmatrix_new).
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "evens".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Mul,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                })),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ]))),
                    conditions: vec![],
                }],
            }),
            is_const: false,
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("%result_array = call ptr @intmatrix_new"),
        "expected the comprehension result array to be allocated via intmatrix_new, got IR:\n{}",
        ir
    );
}

#[test]
fn test_comprehension_float_type() {
    // var scaled := [x * 2.5 for x in [1, 2, 3]]
    // The float literal in the expression forces the result to Matrix,
    // even though the iterable itself is IntMatrix.
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "scaled".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Mul,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(2.5)))),
                })),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ]))),
                    conditions: vec![],
                }],
            }),
            is_const: false,
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("%result_array = call ptr @matrix_new"),
        "expected the comprehension result array to be allocated via matrix_new, got IR:\n{}",
        ir
    );
}

#[test]
fn test_comprehension_multi_generator_mixed_types() {
    // [x + y for x in [1, 2] for y in [1.5, 2.5]]
    // Mixed int/float generators promote the result to Matrix.
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "mixed".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
                })),
                generators: vec![
                    parser::ast::ComprehensionGen {
                        var_names: vec!["x".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        ]))),
                        conditions: vec![],
                    },
                    parser::ast::ComprehensionGen {
                        var_names: vec!["y".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Float(1.5))),
                            Expr::dummy(ExprKind::Literal(Literal::Float(2.5))),
                        ]))),
                        conditions: vec![],
                    },
                ],
            }),
            is_const: false,
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("%result_array = call ptr @matrix_new"),
        "expected mixed int/float generators to promote the comprehension result to Matrix, got IR:\n{}",
        ir
    );
}

#[test]
fn test_comprehension_destructuring_int_type() {
    // var m := izeros(2, 2); m[0,0] = 1; m[0,1] = 2; m[1,0] = 3; m[1,1] = 4
    // var pairs := [a + b for a, b in m]
    // Destructuring generator over an IntMatrix must still infer Int for
    // the per-row bound vars (a, b), yielding an IntMatrix result.
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "pairs".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::ListComprehension {
                    expr: Box::new(Expr::dummy(ExprKind::Binary {
                        op: BinaryOp::Add,
                        lhs: Box::new(Expr::dummy(ExprKind::Identifier("a".to_string()))),
                        rhs: Box::new(Expr::dummy(ExprKind::Identifier("b".to_string()))),
                    })),
                    generators: vec![parser::ast::ComprehensionGen {
                        var_names: vec!["a".to_string(), "b".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Identifier("m".to_string()))),
                        conditions: vec![],
                    }],
                }),
                is_const: false,
            }),
        ],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("%result_array = call ptr @intmatrix_new"),
        "expected destructuring over an IntMatrix to infer an IntMatrix comprehension result, got IR:\n{}",
        ir
    );
}

#[test]
fn test_comprehension_stringmatrix_iterable_rejected() {
    // [x for x in "a,b,c".split(",")]
    // A StringMatrix iterable is still explicitly rejected — this is
    // pre-existing behavior (unrelated to the Grupo I type-inference
    // fix), asserted here as a regression guard.
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(
            ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Call {
                        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                            target: Box::new(Expr::dummy(ExprKind::Literal(Literal::String(
                                "a,b,c".to_string(),
                            )))),
                            field: "split".to_string(),
                        })),
                        args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                            ",".to_string(),
                        )))],
                    })),
                    conditions: vec![],
                }],
            },
        )))],
    };
    let result = compile_program_checked(program);
    assert!(
        result.is_err(),
        "expected a StringMatrix iterable to be rejected, got IR:\n{:?}",
        result
    );
}
