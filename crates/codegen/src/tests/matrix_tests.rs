// Matrix Operations Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Closure, Expr, Literal, Program, Stmt, ExprKind, StmtKind};

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
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::StaticInit {
            element_type: "int".to_string(),
            dimensions: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
        })))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("izeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_static_init_float() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::StaticInit {
            element_type: "float".to_string(),
            dimensions: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
        })))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("zeros") || ir.contains("calloc") || ir.contains("call"));
}

#[test]
fn test_static_init_2d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::StaticInit {
            element_type: "float".to_string(),
            dimensions: vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                Expr::dummy(ExprKind::Literal(Literal::Int(4))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_int() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(vec![
            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
        ]))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_float() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(vec![
            Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
            Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
            Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
        ]))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ARRAY LITERAL EDGE CASES ====================

#[test]
fn test_array_literal_empty() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(vec![]))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_single_element() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))]))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_mixed_int_float() {
    // Mixed int/float should promote to Matrix (float)
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(vec![
            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
            Expr::dummy(ExprKind::Literal(Literal::Float(2.5))),
            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
        ]))))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_large() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Array(
            (0..100).map(|i| Expr::dummy(ExprKind::Literal(Literal::Int(i)))).collect(),
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
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::ListComprehension {
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
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_no_filter() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::ListComprehension {
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
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_three_loops() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::ListComprehension {
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
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_multiple_conditions() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::ListComprehension {
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
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_with_destructuring() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::ListComprehension {
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
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_complex_expression() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::ListComprehension {
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
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_nested_with_condition() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::ListComprehension {
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
        })))],
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
            args: vec![Expr::dummy(ExprKind::Array(vec![])), Expr::dummy(ExprKind::Array(vec![]))],
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
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
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
            }),
        ],
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
    }))
}

/// Helper: build closure expr `(a: T, b: U) -> R { return body }`
fn make_binary_closure(p0: &str, t0: &str, p1: &str, t1: &str, ret_ty: &str, body: Expr) -> Expr {
    Expr::dummy(ExprKind::Closure(Closure {
        params: vec![(p0.to_string(), t0.to_string()), (p1.to_string(), t1.to_string())],
        return_type: Some(ret_ty.to_string()),
        body: Box::new(Stmt::dummy(StmtKind::Return { values: vec![body] })),
        captured_vars: vec![],
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
        "x", "int", "int",
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
        "x", "float", "float",
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
        "x", "float", "int",
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
        "acc", "float", "x", "float", "float",
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
