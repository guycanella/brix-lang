// Control Flow Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt, ExprKind, StmtKind};

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

// Helper function to create binary operations
fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Expr {
    Expr::dummy(ExprKind::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    })
}

// ==================== IF STATEMENT TESTS ====================

#[test]
fn test_if_no_else() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::If {
                condition: Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Gt,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
                }),
                then_block: Box::new(Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(1)))))),
                else_block: None,
            }),
        ],
    };
    let ir = compile_program(program).unwrap();
    // Should have branch instruction
    assert!(ir.contains("br"));
}

#[test]
fn test_if_with_else() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::If {
                condition: Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Gt,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
                }),
                then_block: Box::new(Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(1)))))),
                else_block: Some(Box::new(Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(0))))))),
            }),
        ],
    };
    let ir = compile_program(program).unwrap();
    // Should have branch instruction and multiple basic blocks
    assert!(ir.contains("br") && ir.contains("label"));
}

// ==================== WHILE LOOP TESTS ====================

#[test]
fn test_while_loop() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::While {
                condition: Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Lt,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                }),
                body: Box::new(Stmt::dummy(StmtKind::Assignment {
                    target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                    value: Expr::dummy(ExprKind::Binary {
                        op: BinaryOp::Add,
                        lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    }),
                })),
            }),
        ],
    };
    let ir = compile_program(program).unwrap();
    // Should have branch and loop structure
    assert!(ir.contains("br"));
}

// ==================== FOR LOOP TESTS ====================

#[test]
fn test_for_loop_range() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::For {
            var_names: vec!["i".to_string()],
            iterable: Expr::dummy(ExprKind::Range {
                start: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                end: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                step: None,
            }),
            body: Box::new(Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Identifier("i".to_string()))))),
        })],
    };
    let ir = compile_program(program).unwrap();
    // For loop desugars to while loop
    assert!(ir.contains("br") || ir.contains("loop"));
}

#[test]
fn test_for_loop_with_step() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::For {
            var_names: vec!["i".to_string()],
            iterable: Expr::dummy(ExprKind::Range {
                start: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                end: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                step: Some(Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2))))),
            }),
            body: Box::new(Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Identifier("i".to_string()))))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATCH EXPRESSION TESTS ====================

#[test]
fn test_match_literal() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Literal(Literal::Int(1)),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("one".to_string())))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("other".to_string())))),
                    },
                ],
            }))),
        ],
    };
    let ir = compile_program(program).unwrap();
    // Match should compile to conditional branches
    assert!(ir.contains("br") || ir.contains("switch"));
}

// ==================== FUNCTION DEFINITION TESTS ====================

#[test]
fn test_function_definition() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
            name: "add".to_string(),
            params: vec![
                ("a".to_string(), "int".to_string(), None),
                ("b".to_string(), "int".to_string(), None),
            ],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(Stmt::dummy(StmtKind::Return {
                values: vec![Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("a".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Identifier("b".to_string()))),
                })],
            })),
        })],
    };
    let ir = compile_program(program).unwrap();
    // Should define a function
    assert!(ir.contains("define") && ir.contains("ret"));
}

#[test]
fn test_function_call() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "test_fn".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
                })),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("test_fn".to_string()))),
                args: vec![],
            }))),
        ],
    };
    let ir = compile_program(program).unwrap();
    // Should have call instruction
    assert!(ir.contains("call"));
}

// ==================== RETURN STATEMENT TESTS ====================

#[test]
fn test_return_void() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
            name: "void_fn".to_string(),
            params: vec![],
            return_type: None,
            body: Box::new(Stmt::dummy(StmtKind::Return { values: vec![] })),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("ret void") || ir.contains("ret"));
}

#[test]
fn test_return_single_value() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
            name: "get_int".to_string(),
            params: vec![],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(Stmt::dummy(StmtKind::Return {
                values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
            })),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("ret"));
}

#[test]
fn test_return_multiple_values() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
            name: "get_pair".to_string(),
            params: vec![],
            return_type: Some(vec!["int".to_string(), "int".to_string()]),
            body: Box::new(Stmt::dummy(StmtKind::Return {
                values: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ],
            })),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ADVANCED PATTERN MATCHING TESTS ====================

#[test]
fn test_match_with_or_pattern() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Or(vec![
                            parser::ast::Pattern::Literal(Literal::Int(1)),
                            parser::ast::Pattern::Literal(Literal::Int(2)),
                            parser::ast::Pattern::Literal(Literal::Int(3)),
                        ]),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("small".to_string())))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("large".to_string())))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_guard() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(15))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Binding("n".to_string()),
                        guard: Some(Box::new(Expr::dummy(ExprKind::Binary {
                            op: BinaryOp::Gt,
                            lhs: Box::new(Expr::dummy(ExprKind::Identifier("n".to_string()))),
                            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                        }))),
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("large".to_string())))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("small".to_string())))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_binding() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![parser::ast::MatchArm {
                    pattern: parser::ast::Pattern::Binding("val".to_string()),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Identifier("val".to_string()))),
                }],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_atoms() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "status".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Atom("ok".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("status".to_string()))),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Literal(Literal::Atom("ok".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("success".to_string())))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Literal(Literal::Atom("error".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("failed".to_string())))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("unknown".to_string())))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_multiple_guards() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(25))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Binding("n".to_string()),
                        guard: Some(Box::new(Expr::dummy(ExprKind::Binary {
                            op: BinaryOp::Lt,
                            lhs: Box::new(Expr::dummy(ExprKind::Identifier("n".to_string()))),
                            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(18)))),
                        }))),
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("child".to_string())))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Binding("n".to_string()),
                        guard: Some(Box::new(Expr::dummy(ExprKind::Binary {
                            op: BinaryOp::Lt,
                            lhs: Box::new(Expr::dummy(ExprKind::Identifier("n".to_string()))),
                            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(60)))),
                        }))),
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("adult".to_string())))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("senior".to_string())))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_strings() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "msg".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("msg".to_string()))),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Literal(Literal::String("hello".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== LIST COMPREHENSION ADVANCED ====================

#[test]
fn test_list_comp_multiple_conditions() {
    // [x for x in [1, 2, 3, 4, 5, 6] if x > 2 if x < 5]
    // Should result in [3, 4]
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "result".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(6))),
                    ]))),
                    conditions: vec![
                        binary(
                            BinaryOp::Gt,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        ),
                        binary(
                            BinaryOp::Lt,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                        ),
                    ],
                }],
            }),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comp_with_destructuring() {
    // [x + y for x, y in zip([1, 2, 3], [4, 5, 6])]
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "result".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(binary(
                    BinaryOp::Add,
                    Expr::dummy(ExprKind::Identifier("x".to_string())),
                    Expr::dummy(ExprKind::Identifier("y".to_string())),
                )),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string(), "y".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Call {
                        func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
                        args: vec![
                            Expr::dummy(ExprKind::Array(vec![
                                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                            ])),
                            Expr::dummy(ExprKind::Array(vec![
                                Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                                Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                                Expr::dummy(ExprKind::Literal(Literal::Int(6))),
                            ])),
                        ],
                    })),
                    conditions: vec![],
                }],
            }),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comp_nested_with_condition() {
    // [x * y for x in [1, 2, 3] for y in [10, 20] if x + y > 15]
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "result".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(binary(
                    BinaryOp::Mul,
                    Expr::dummy(ExprKind::Identifier("x".to_string())),
                    Expr::dummy(ExprKind::Identifier("y".to_string())),
                )),
                generators: vec![
                    parser::ast::ComprehensionGen {
                        var_names: vec!["x".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        ]))),
                        conditions: vec![],
                    },
                    parser::ast::ComprehensionGen {
                        var_names: vec!["y".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                        ]))),
                        conditions: vec![binary(
                            BinaryOp::Gt,
                            binary(
                                BinaryOp::Add,
                                Expr::dummy(ExprKind::Identifier("x".to_string())),
                                Expr::dummy(ExprKind::Identifier("y".to_string())),
                            ),
                            Expr::dummy(ExprKind::Literal(Literal::Int(15))),
                        )],
                    },
                ],
            }),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comp_three_levels() {
    // [x + y + z for x in [1, 2] for y in [10] for z in [100, 200]]
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "result".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(binary(
                    BinaryOp::Add,
                    binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Identifier("y".to_string())),
                    ),
                    Expr::dummy(ExprKind::Identifier("z".to_string())),
                )),
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
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![Expr::dummy(ExprKind::Literal(Literal::Int(10)))]))),
                        conditions: vec![],
                    },
                    parser::ast::ComprehensionGen {
                        var_names: vec!["z".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                            Expr::dummy(ExprKind::Literal(Literal::Int(100))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(200))),
                        ]))),
                        conditions: vec![],
                    },
                ],
            }),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comp_empty_result() {
    // [x for x in [1, 2, 3] if x > 10]  // No elements satisfy condition
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "result".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ]))),
                    conditions: vec![binary(
                        BinaryOp::Gt,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    )],
                }],
            }),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comp_complex_expression() {
    // [(x * 2) + 1 for x in [1, 2, 3, 4, 5] if x % 2 == 0]
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "result".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::ListComprehension {
                expr: Box::new(binary(
                    BinaryOp::Add,
                    binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ),
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                )),
                generators: vec![parser::ast::ComprehensionGen {
                    var_names: vec!["x".to_string()],
                    iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                    ]))),
                    conditions: vec![binary(
                        BinaryOp::Eq,
                        binary(
                            BinaryOp::Mod,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        ),
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    )],
                }],
            }),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comp_from_variable() {
    // var arr := [1, 2, 3];
    // var result := [x * x for x in arr];
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
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::ListComprehension {
                    expr: Box::new(binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                    )),
                    generators: vec![parser::ast::ComprehensionGen {
                        var_names: vec!["x".to_string()],
                        iterable: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                        conditions: vec![],
                    }],
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== ZIP() ADVANCED ====================

#[test]
fn test_zip_empty_with_empty() {
    // zip([], [])
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
fn test_zip_single_element() {
    // zip([1], [2])
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Array(vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))])),
                Expr::dummy(ExprKind::Array(vec![Expr::dummy(ExprKind::Literal(Literal::Int(2)))])),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_mixed_float_int() {
    // zip([1.5, 2.5], [10, 20])  // Matrix + IntMatrix
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.5))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.5))),
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
fn test_zip_with_variables() {
    // var a := [1, 2, 3];
    // var b := [4, 5, 6];
    // zip(a, b)
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "a".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "b".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(6))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
                args: vec![
                    Expr::dummy(ExprKind::Identifier("a".to_string())),
                    Expr::dummy(ExprKind::Identifier("b".to_string())),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_in_loop() {
    // for x, y in zip([1, 2], [3, 4]) { }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::For {
            var_names: vec!["x".to_string(), "y".to_string()],
            iterable: Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
                args: vec![
                    Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ])),
                    Expr::dummy(ExprKind::Array(vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    ])),
                ],
            }),
            body: Box::new(Stmt::dummy(StmtKind::Block(vec![]))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== LOOP ADVANCED ====================

#[test]
fn test_for_with_expression_in_range() {
    // var n := 5;
    // for i in 1:1:n { }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "n".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::For {
                var_names: vec!["i".to_string()],
                iterable: Expr::dummy(ExprKind::Range {
                    start: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    end: Box::new(Expr::dummy(ExprKind::Identifier("n".to_string()))),
                    step: Some(Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1))))),
                }),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![]))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_nested_for_loops() {
    // for i in [1, 2, 3] {
    //     for j in [10, 20] { }
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::For {
            var_names: vec!["i".to_string()],
            iterable: Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ])),
            body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::For {
                var_names: vec!["j".to_string()],
                iterable: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                ])),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![]))),
            })]))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_while_with_complex_condition() {
    // var x := 10;
    // var y := 5;
    // while x > 0 && y < 10 {
    //     x = x - 1;
    //     y = y + 1;
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::While {
                condition: binary(
                    BinaryOp::LogicalAnd,
                    binary(
                        BinaryOp::Gt,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    ),
                    binary(
                        BinaryOp::Lt,
                        Expr::dummy(ExprKind::Identifier("y".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    ),
                ),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::Assignment {
                        target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                        value: binary(
                            BinaryOp::Sub,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        ),
                    }),
                    Stmt::dummy(StmtKind::Assignment {
                        target: Expr::dummy(ExprKind::Identifier("y".to_string())),
                        value: binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("y".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        ),
                    }),
                ]))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_for_loop_empty_body() {
    // for i in [1, 2, 3] { }  // Empty body
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::For {
            var_names: vec!["i".to_string()],
            iterable: Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ])),
            body: Box::new(Stmt::dummy(StmtKind::Block(vec![]))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_while_immediate_false() {
    // while false { }  // Never executes
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::While {
            condition: Expr::dummy(ExprKind::Literal(Literal::Bool(false))),
            body: Box::new(Stmt::dummy(StmtKind::Block(vec![]))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== CONSTRUCTOR ADVANCED ====================

#[test]
fn test_zeros_size_zero() {
    // zeros(0)  // Empty array
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zeros_very_large() {
    // zeros(1000)  // Large array
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
fn test_izeros_size_zero() {
    // izeros(0)  // Empty int array
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_eye_matrix() {
    // eye(3)  // 3x3 identity matrix
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("eye".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_eye_size_one() {
    // eye(1)  // 1x1 identity
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("eye".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_constructor_with_variable() {
    // var n := 5;
    // zeros(n)
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "n".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                args: vec![Expr::dummy(ExprKind::Identifier("n".to_string()))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

