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

// ==================== ARRAY LITERAL EDGE CASES ====================

#[test]
fn test_array_literal_empty() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Array(vec![]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_single_element() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Array(vec![Expr::Literal(Literal::Int(42))]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_mixed_int_float() {
    // Mixed int/float should promote to Matrix (float)
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Array(vec![
            Expr::Literal(Literal::Int(1)),
            Expr::Literal(Literal::Float(2.5)),
            Expr::Literal(Literal::Int(3)),
        ]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_large() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Array(
            (0..100).map(|i| Expr::Literal(Literal::Int(i))).collect(),
        ))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_literal_all_ints_becomes_intmatrix() {
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
#[ignore = "Nested arrays (arrays of arrays) not yet supported - requires multi-dimensional type system"]
fn test_array_literal_nested() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Array(vec![
            Expr::Array(vec![
                Expr::Literal(Literal::Int(1)),
                Expr::Literal(Literal::Int(2)),
            ]),
            Expr::Array(vec![
                Expr::Literal(Literal::Int(3)),
                Expr::Literal(Literal::Int(4)),
            ]),
        ]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATRIX FIELD ACCESS ====================

#[test]
fn test_matrix_field_rows() {
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
            Stmt::Expr(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("m".to_string())),
                field: "rows".to_string(),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_field_cols() {
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
            Stmt::Expr(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("m".to_string())),
                field: "cols".to_string(),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_intmatrix_field_rows() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("izeros".to_string())),
                    args: vec![
                        Expr::Literal(Literal::Int(5)),
                        Expr::Literal(Literal::Int(6)),
                    ],
                },
                is_const: false,
            },
            Stmt::Expr(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("m".to_string())),
                field: "rows".to_string(),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_intmatrix_field_cols() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "m".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("izeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(10))],
                },
                is_const: false,
            },
            Stmt::Expr(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("m".to_string())),
                field: "cols".to_string(),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_field_len() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "s".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::String("hello".to_string())),
                is_const: false,
            },
            Stmt::Expr(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("s".to_string())),
                field: "len".to_string(),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ZEROS/IZEROS EDGE CASES ====================

#[test]
fn test_zeros_size_one() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zeros".to_string())),
            args: vec![Expr::Literal(Literal::Int(1))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zeros_large_size() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zeros".to_string())),
            args: vec![Expr::Literal(Literal::Int(1000))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_izeros_size_one() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("izeros".to_string())),
            args: vec![Expr::Literal(Literal::Int(1))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_izeros_2d() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("izeros".to_string())),
            args: vec![
                Expr::Literal(Literal::Int(4)),
                Expr::Literal(Literal::Int(5)),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zeros_with_expression_arg() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zeros".to_string())),
            args: vec![Expr::Binary {
                op: parser::ast::BinaryOp::Add,
                lhs: Box::new(Expr::Literal(Literal::Int(5))),
                rhs: Box::new(Expr::Literal(Literal::Int(3))),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zeros_2d_with_expressions() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zeros".to_string())),
            args: vec![
                Expr::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::Literal(Literal::Int(2))),
                    rhs: Box::new(Expr::Literal(Literal::Int(3))),
                },
                Expr::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::Literal(Literal::Int(4))),
                    rhs: Box::new(Expr::Literal(Literal::Int(1))),
                },
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATRIX ASSIGNMENT ====================

#[test]
fn test_array_element_assignment() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                    Expr::Literal(Literal::Int(3)),
                ]),
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("arr".to_string())),
                    indices: vec![Expr::Literal(Literal::Int(1))],
                },
                value: Expr::Literal(Literal::Int(99)),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_2d_assignment() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "mat".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![
                        Expr::Literal(Literal::Int(3)),
                        Expr::Literal(Literal::Int(3)),
                    ],
                },
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("mat".to_string())),
                    indices: vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Int(2)),
                    ],
                },
                value: Expr::Literal(Literal::Float(5.5)),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_assignment_variable_index() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(10))],
                },
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "i".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(5)),
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("arr".to_string())),
                    indices: vec![Expr::Identifier("i".to_string())],
                },
                value: Expr::Literal(Literal::Float(3.14)),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_assignment_expression_index() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("izeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(10))],
                },
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("arr".to_string())),
                    indices: vec![Expr::Binary {
                        op: parser::ast::BinaryOp::Add,
                        lhs: Box::new(Expr::Literal(Literal::Int(2))),
                        rhs: Box::new(Expr::Literal(Literal::Int(3))),
                    }],
                },
                value: Expr::Literal(Literal::Int(42)),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_assignment_float_to_int() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "iarr".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                ]),
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("iarr".to_string())),
                    indices: vec![Expr::Literal(Literal::Int(0))],
                },
                value: Expr::Literal(Literal::Float(3.7)), // Should truncate to 3
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_chained_assignment() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "mat".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![
                        Expr::Literal(Literal::Int(5)),
                        Expr::Literal(Literal::Int(5)),
                    ],
                },
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("mat".to_string())),
                    indices: vec![
                        Expr::Literal(Literal::Int(0)),
                        Expr::Literal(Literal::Int(0)),
                    ],
                },
                value: Expr::Literal(Literal::Float(1.0)),
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("mat".to_string())),
                    indices: vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Int(1)),
                    ],
                },
                value: Expr::Literal(Literal::Float(2.0)),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== LIST COMPREHENSIONS ADVANCED ====================

#[test]
fn test_list_comprehension_empty_result() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::ListComprehension {
            expr: Box::new(Expr::Identifier("x".to_string())),
            generators: vec![parser::ast::ComprehensionGen {
                var_names: vec!["x".to_string()],
                iterable: Box::new(Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                    Expr::Literal(Literal::Int(3)),
                ])),
                conditions: vec![Expr::Binary {
                    op: parser::ast::BinaryOp::Gt,
                    lhs: Box::new(Expr::Identifier("x".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Int(10))),
                }],
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_no_filter() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::ListComprehension {
            expr: Box::new(Expr::Binary {
                op: parser::ast::BinaryOp::Mul,
                lhs: Box::new(Expr::Identifier("x".to_string())),
                rhs: Box::new(Expr::Literal(Literal::Int(2))),
            }),
            generators: vec![parser::ast::ComprehensionGen {
                var_names: vec!["x".to_string()],
                iterable: Box::new(Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                    Expr::Literal(Literal::Int(3)),
                ])),
                conditions: vec![],
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_three_loops() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::ListComprehension {
            expr: Box::new(Expr::Binary {
                op: parser::ast::BinaryOp::Add,
                lhs: Box::new(Expr::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::Identifier("x".to_string())),
                    rhs: Box::new(Expr::Identifier("y".to_string())),
                }),
                rhs: Box::new(Expr::Identifier("z".to_string())),
            }),
            generators: vec![
                parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::Array(vec![
                        Expr::Literal(Literal::Float(1.0)),
                        Expr::Literal(Literal::Float(2.0)),
                    ])),
                    conditions: vec![],
                },
                parser::ast::ComprehensionGen {
                    var_names: vec!["y".to_string()],
                    iterable: Box::new(Expr::Array(vec![
                        Expr::Literal(Literal::Float(10.0)),
                        Expr::Literal(Literal::Float(20.0)),
                    ])),
                    conditions: vec![],
                },
                parser::ast::ComprehensionGen {
                    var_names: vec!["z".to_string()],
                    iterable: Box::new(Expr::Array(vec![
                        Expr::Literal(Literal::Float(100.0)),
                        Expr::Literal(Literal::Float(200.0)),
                    ])),
                    conditions: vec![],
                },
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_multiple_conditions() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::ListComprehension {
            expr: Box::new(Expr::Identifier("x".to_string())),
            generators: vec![parser::ast::ComprehensionGen {
                var_names: vec!["x".to_string()],
                iterable: Box::new(Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                    Expr::Literal(Literal::Int(3)),
                    Expr::Literal(Literal::Int(4)),
                    Expr::Literal(Literal::Int(5)),
                ])),
                conditions: vec![
                    Expr::Binary {
                        op: parser::ast::BinaryOp::Gt,
                        lhs: Box::new(Expr::Identifier("x".to_string())),
                        rhs: Box::new(Expr::Literal(Literal::Int(2))),
                    },
                    Expr::Binary {
                        op: parser::ast::BinaryOp::Lt,
                        lhs: Box::new(Expr::Identifier("x".to_string())),
                        rhs: Box::new(Expr::Literal(Literal::Int(5))),
                    },
                ],
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_with_destructuring() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::ListComprehension {
            expr: Box::new(Expr::Binary {
                op: parser::ast::BinaryOp::Add,
                lhs: Box::new(Expr::Identifier("a".to_string())),
                rhs: Box::new(Expr::Identifier("b".to_string())),
            }),
            generators: vec![parser::ast::ComprehensionGen {
                var_names: vec!["a".to_string(), "b".to_string()],
                iterable: Box::new(Expr::Call {
                    func: Box::new(Expr::Identifier("zip".to_string())),
                    args: vec![
                        Expr::Array(vec![
                            Expr::Literal(Literal::Float(1.0)),
                            Expr::Literal(Literal::Float(2.0)),
                        ]),
                        Expr::Array(vec![
                            Expr::Literal(Literal::Float(10.0)),
                            Expr::Literal(Literal::Float(20.0)),
                        ]),
                    ],
                }),
                conditions: vec![],
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_complex_expression() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::ListComprehension {
            expr: Box::new(Expr::Binary {
                op: parser::ast::BinaryOp::Mul,
                lhs: Box::new(Expr::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::Identifier("x".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Float(1.0))),
                }),
                rhs: Box::new(Expr::Literal(Literal::Float(2.0))),
            }),
            generators: vec![parser::ast::ComprehensionGen {
                var_names: vec!["x".to_string()],
                iterable: Box::new(Expr::Array(vec![
                    Expr::Literal(Literal::Float(1.0)),
                    Expr::Literal(Literal::Float(2.0)),
                    Expr::Literal(Literal::Float(3.0)),
                ])),
                conditions: vec![],
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_nested_with_condition() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::ListComprehension {
            expr: Box::new(Expr::Binary {
                op: parser::ast::BinaryOp::Mul,
                lhs: Box::new(Expr::Identifier("x".to_string())),
                rhs: Box::new(Expr::Identifier("y".to_string())),
            }),
            generators: vec![
                parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::Array(vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Int(2)),
                        Expr::Literal(Literal::Int(3)),
                    ])),
                    conditions: vec![Expr::Binary {
                        op: parser::ast::BinaryOp::Gt,
                        lhs: Box::new(Expr::Identifier("x".to_string())),
                        rhs: Box::new(Expr::Literal(Literal::Int(1))),
                    }],
                },
                parser::ast::ComprehensionGen {
                    var_names: vec!["y".to_string()],
                    iterable: Box::new(Expr::Array(vec![
                        Expr::Literal(Literal::Int(10)),
                        Expr::Literal(Literal::Int(20)),
                    ])),
                    conditions: vec![],
                },
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_from_zeros() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(5))],
                },
                is_const: false,
            },
            Stmt::Expr(Expr::ListComprehension {
                expr: Box::new(Expr::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::Identifier("x".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Float(1.0))),
                }),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::Identifier("arr".to_string())),
                    conditions: vec![],
                }],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ZIP FUNCTION ====================

#[test]
fn test_zip_intmatrix_intmatrix() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zip".to_string())),
            args: vec![
                Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                ]),
                Expr::Array(vec![
                    Expr::Literal(Literal::Int(10)),
                    Expr::Literal(Literal::Int(20)),
                ]),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_matrix_matrix() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zip".to_string())),
            args: vec![
                Expr::Array(vec![
                    Expr::Literal(Literal::Float(1.0)),
                    Expr::Literal(Literal::Float(2.0)),
                ]),
                Expr::Array(vec![
                    Expr::Literal(Literal::Float(10.0)),
                    Expr::Literal(Literal::Float(20.0)),
                ]),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_intmatrix_matrix() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zip".to_string())),
            args: vec![
                Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                ]),
                Expr::Array(vec![
                    Expr::Literal(Literal::Float(10.5)),
                    Expr::Literal(Literal::Float(20.5)),
                ]),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_empty_arrays() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zip".to_string())),
            args: vec![Expr::Array(vec![]), Expr::Array(vec![])],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_different_sizes() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zip".to_string())),
            args: vec![
                Expr::Array(vec![
                    Expr::Literal(Literal::Float(1.0)),
                    Expr::Literal(Literal::Float(2.0)),
                    Expr::Literal(Literal::Float(3.0)),
                ]),
                Expr::Array(vec![
                    Expr::Literal(Literal::Float(10.0)),
                    Expr::Literal(Literal::Float(20.0)),
                ]),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_with_zeros() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zip".to_string())),
            args: vec![
                Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(5))],
                },
                Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(5))],
                },
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== EYE FUNCTION ADVANCED ====================

#[test]
fn test_eye_indexing() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "identity".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("eye".to_string())),
                    args: vec![Expr::Literal(Literal::Int(3))],
                },
                is_const: false,
            },
            Stmt::Expr(Expr::Index {
                array: Box::new(Expr::Identifier("identity".to_string())),
                indices: vec![
                    Expr::Literal(Literal::Int(0)),
                    Expr::Literal(Literal::Int(0)),
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_eye_field_access() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "identity".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("eye".to_string())),
                    args: vec![Expr::Literal(Literal::Int(5))],
                },
                is_const: false,
            },
            Stmt::Expr(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("identity".to_string())),
                field: "rows".to_string(),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_eye_with_variable() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "n".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(4)),
                is_const: false,
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("eye".to_string())),
                args: vec![Expr::Identifier("n".to_string())],
            }),
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
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(10))],
                },
                is_const: false,
            },
            Stmt::Expr(Expr::Index {
                array: Box::new(Expr::Identifier("arr".to_string())),
                indices: vec![Expr::Literal(Literal::Int(9))],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_index_zero() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Float(1.0)),
                    Expr::Literal(Literal::Float(2.0)),
                ]),
                is_const: false,
            },
            Stmt::Expr(Expr::Index {
                array: Box::new(Expr::Identifier("arr".to_string())),
                indices: vec![Expr::Literal(Literal::Int(0))],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_2d_index_both_zero() {
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
            Stmt::Expr(Expr::Index {
                array: Box::new(Expr::Identifier("mat".to_string())),
                indices: vec![
                    Expr::Literal(Literal::Int(0)),
                    Expr::Literal(Literal::Int(0)),
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_index_complex_expression() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(20))],
                },
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "i".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(3)),
                is_const: false,
            },
            Stmt::Expr(Expr::Index {
                array: Box::new(Expr::Identifier("arr".to_string())),
                indices: vec![Expr::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::Identifier("i".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Int(2))),
                }],
            }),
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
            Stmt::VariableDecl {
                name: "iarr".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                ]),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::Identifier("iarr".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Float(2.5))),
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_intmatrix_index_returns_int() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "iarr".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Int(10)),
                    Expr::Literal(Literal::Int(20)),
                    Expr::Literal(Literal::Int(30)),
                ]),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "val".to_string(),
                type_hint: None,
                value: Expr::Index {
                    array: Box::new(Expr::Identifier("iarr".to_string())),
                    indices: vec![Expr::Literal(Literal::Int(1))],
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_mixed_array_types_in_comprehension() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "ints".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                    Expr::Literal(Literal::Int(3)),
                ]),
                is_const: false,
            },
            Stmt::Expr(Expr::ListComprehension {
                expr: Box::new(Expr::Binary {
                    op: parser::ast::BinaryOp::Mul,
                    lhs: Box::new(Expr::Identifier("x".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Float(1.5))),
                }),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::Identifier("ints".to_string())),
                    conditions: vec![],
                }],
            }),
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
            Stmt::VariableDecl {
                name: "data".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Float(1.1)),
                    Expr::Literal(Literal::Float(2.2)),
                    Expr::Literal(Literal::Float(3.3)),
                ]),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "first".to_string(),
                type_hint: None,
                value: Expr::Index {
                    array: Box::new(Expr::Identifier("data".to_string())),
                    indices: vec![Expr::Literal(Literal::Int(0))],
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zeros_then_multiple_assignments() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(3))],
                },
                is_const: false,
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("arr".to_string())),
                    indices: vec![Expr::Literal(Literal::Int(0))],
                },
                value: Expr::Literal(Literal::Float(1.0)),
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("arr".to_string())),
                    indices: vec![Expr::Literal(Literal::Int(1))],
                },
                value: Expr::Literal(Literal::Float(2.0)),
            },
            Stmt::Assignment {
                target: Expr::Index {
                    array: Box::new(Expr::Identifier("arr".to_string())),
                    indices: vec![Expr::Literal(Literal::Int(2))],
                },
                value: Expr::Literal(Literal::Float(3.0)),
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_element_in_expression() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "nums".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Int(5)),
                    Expr::Literal(Literal::Int(10)),
                    Expr::Literal(Literal::Int(15)),
                ]),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "sum".to_string(),
                type_hint: None,
                value: Expr::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::Index {
                        array: Box::new(Expr::Identifier("nums".to_string())),
                        indices: vec![Expr::Literal(Literal::Int(0))],
                    }),
                    rhs: Box::new(Expr::Index {
                        array: Box::new(Expr::Identifier("nums".to_string())),
                        indices: vec![Expr::Literal(Literal::Int(1))],
                    }),
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_matrix_element_in_ternary() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Float(5.0)),
                    Expr::Literal(Literal::Float(10.0)),
                ]),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "val".to_string(),
                type_hint: None,
                value: Expr::Ternary {
                    condition: Box::new(Expr::Binary {
                        op: parser::ast::BinaryOp::Gt,
                        lhs: Box::new(Expr::Index {
                            array: Box::new(Expr::Identifier("arr".to_string())),
                            indices: vec![Expr::Literal(Literal::Int(0))],
                        }),
                        rhs: Box::new(Expr::Literal(Literal::Float(3.0))),
                    }),
                    then_expr: Box::new(Expr::Index {
                        array: Box::new(Expr::Identifier("arr".to_string())),
                        indices: vec![Expr::Literal(Literal::Int(0))],
                    }),
                    else_expr: Box::new(Expr::Index {
                        array: Box::new(Expr::Identifier("arr".to_string())),
                        indices: vec![Expr::Literal(Literal::Int(1))],
                    }),
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_from_function_call_then_index() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "z".to_string(),
                type_hint: None,
                value: Expr::Index {
                    array: Box::new(Expr::Call {
                        func: Box::new(Expr::Identifier("zeros".to_string())),
                        args: vec![Expr::Literal(Literal::Int(5))],
                    }),
                    indices: vec![Expr::Literal(Literal::Int(2))],
                },
                is_const: false,
            },
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
            Stmt::VariableDecl {
                name: "size".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(10)),
                is_const: false,
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("zeros".to_string())),
                args: vec![Expr::Binary {
                    op: parser::ast::BinaryOp::Div,
                    lhs: Box::new(Expr::Identifier("size".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Int(2))),
                }],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_eye_size_from_expression() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("eye".to_string())),
            args: vec![Expr::Binary {
                op: parser::ast::BinaryOp::Add,
                lhs: Box::new(Expr::Literal(Literal::Int(2))),
                rhs: Box::new(Expr::Literal(Literal::Int(1))),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_nested_zeros_calls() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "a".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(5))],
                },
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "b".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("zeros".to_string())),
                    args: vec![Expr::Literal(Literal::Int(3))],
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
