// Control Flow Codegen Tests

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

// ==================== IF STATEMENT TESTS ====================

#[test]
fn test_if_no_else() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(10)),
                is_const: false,
            },
            Stmt::If {
                condition: Expr::Binary {
                    op: BinaryOp::Gt,
                    lhs: Box::new(Expr::Identifier("x".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Int(5))),
                },
                then_block: Box::new(Stmt::Expr(Expr::Literal(Literal::Int(1)))),
                else_block: None,
            },
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
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(3)),
                is_const: false,
            },
            Stmt::If {
                condition: Expr::Binary {
                    op: BinaryOp::Gt,
                    lhs: Box::new(Expr::Identifier("x".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Int(5))),
                },
                then_block: Box::new(Stmt::Expr(Expr::Literal(Literal::Int(1)))),
                else_block: Some(Box::new(Stmt::Expr(Expr::Literal(Literal::Int(0))))),
            },
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
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(0)),
                is_const: false,
            },
            Stmt::While {
                condition: Expr::Binary {
                    op: BinaryOp::Lt,
                    lhs: Box::new(Expr::Identifier("x".to_string())),
                    rhs: Box::new(Expr::Literal(Literal::Int(10))),
                },
                body: Box::new(Stmt::Assignment {
                    target: Expr::Identifier("x".to_string()),
                    value: Expr::Binary {
                        op: BinaryOp::Add,
                        lhs: Box::new(Expr::Identifier("x".to_string())),
                        rhs: Box::new(Expr::Literal(Literal::Int(1))),
                    },
                }),
            },
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
        statements: vec![Stmt::For {
            var_names: vec!["i".to_string()],
            iterable: Expr::Range {
                start: Box::new(Expr::Literal(Literal::Int(0))),
                end: Box::new(Expr::Literal(Literal::Int(10))),
                step: None,
            },
            body: Box::new(Stmt::Expr(Expr::Identifier("i".to_string()))),
        }],
    };
    let ir = compile_program(program).unwrap();
    // For loop desugars to while loop
    assert!(ir.contains("br") || ir.contains("loop"));
}

#[test]
fn test_for_loop_with_step() {
    let program = Program {
        statements: vec![Stmt::For {
            var_names: vec!["i".to_string()],
            iterable: Expr::Range {
                start: Box::new(Expr::Literal(Literal::Int(0))),
                end: Box::new(Expr::Literal(Literal::Int(10))),
                step: Some(Box::new(Expr::Literal(Literal::Int(2)))),
            },
            body: Box::new(Stmt::Expr(Expr::Identifier("i".to_string()))),
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATCH EXPRESSION TESTS ====================

#[test]
fn test_match_literal() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(1)),
                is_const: false,
            },
            Stmt::Expr(Expr::Match {
                value: Box::new(Expr::Identifier("x".to_string())),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Literal(Literal::Int(1)),
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::String("one".to_string()))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::String("other".to_string()))),
                    },
                ],
            }),
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
        statements: vec![Stmt::FunctionDef {
            name: "add".to_string(),
            params: vec![
                ("a".to_string(), "int".to_string(), None),
                ("b".to_string(), "int".to_string(), None),
            ],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(Stmt::Return {
                values: vec![Expr::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(Expr::Identifier("a".to_string())),
                    rhs: Box::new(Expr::Identifier("b".to_string())),
                }],
            }),
        }],
    };
    let ir = compile_program(program).unwrap();
    // Should define a function
    assert!(ir.contains("define") && ir.contains("ret"));
}

#[test]
fn test_function_call() {
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "test_fn".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::Return {
                    values: vec![Expr::Literal(Literal::Int(42))],
                }),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("test_fn".to_string())),
                args: vec![],
            }),
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
        statements: vec![Stmt::FunctionDef {
            name: "void_fn".to_string(),
            params: vec![],
            return_type: None,
            body: Box::new(Stmt::Return { values: vec![] }),
        }],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("ret void") || ir.contains("ret"));
}

#[test]
fn test_return_single_value() {
    let program = Program {
        statements: vec![Stmt::FunctionDef {
            name: "get_int".to_string(),
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

#[test]
fn test_return_multiple_values() {
    let program = Program {
        statements: vec![Stmt::FunctionDef {
            name: "get_pair".to_string(),
            params: vec![],
            return_type: Some(vec!["int".to_string(), "int".to_string()]),
            body: Box::new(Stmt::Return {
                values: vec![
                    Expr::Literal(Literal::Int(1)),
                    Expr::Literal(Literal::Int(2)),
                ],
            }),
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ADVANCED PATTERN MATCHING TESTS ====================

#[test]
fn test_match_with_or_pattern() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(2)),
                is_const: false,
            },
            Stmt::Expr(Expr::Match {
                value: Box::new(Expr::Identifier("x".to_string())),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Or(vec![
                            parser::ast::Pattern::Literal(Literal::Int(1)),
                            parser::ast::Pattern::Literal(Literal::Int(2)),
                            parser::ast::Pattern::Literal(Literal::Int(3)),
                        ]),
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::String("small".to_string()))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::String("large".to_string()))),
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_guard() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(15)),
                is_const: false,
            },
            Stmt::Expr(Expr::Match {
                value: Box::new(Expr::Identifier("x".to_string())),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Binding("n".to_string()),
                        guard: Some(Box::new(Expr::Binary {
                            op: BinaryOp::Gt,
                            lhs: Box::new(Expr::Identifier("n".to_string())),
                            rhs: Box::new(Expr::Literal(Literal::Int(10))),
                        })),
                        body: Box::new(Expr::Literal(Literal::String("large".to_string()))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::String("small".to_string()))),
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_binding() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(42)),
                is_const: false,
            },
            Stmt::Expr(Expr::Match {
                value: Box::new(Expr::Identifier("x".to_string())),
                arms: vec![parser::ast::MatchArm {
                    pattern: parser::ast::Pattern::Binding("val".to_string()),
                    guard: None,
                    body: Box::new(Expr::Identifier("val".to_string())),
                }],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_atoms() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "status".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Atom("ok".to_string())),
                is_const: false,
            },
            Stmt::Expr(Expr::Match {
                value: Box::new(Expr::Identifier("status".to_string())),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Literal(Literal::Atom("ok".to_string())),
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::String("success".to_string()))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Literal(Literal::Atom("error".to_string())),
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::String("failed".to_string()))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::String("unknown".to_string()))),
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_multiple_guards() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(25)),
                is_const: false,
            },
            Stmt::Expr(Expr::Match {
                value: Box::new(Expr::Identifier("x".to_string())),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Binding("n".to_string()),
                        guard: Some(Box::new(Expr::Binary {
                            op: BinaryOp::Lt,
                            lhs: Box::new(Expr::Identifier("n".to_string())),
                            rhs: Box::new(Expr::Literal(Literal::Int(18))),
                        })),
                        body: Box::new(Expr::Literal(Literal::String("child".to_string()))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Binding("n".to_string()),
                        guard: Some(Box::new(Expr::Binary {
                            op: BinaryOp::Lt,
                            lhs: Box::new(Expr::Identifier("n".to_string())),
                            rhs: Box::new(Expr::Literal(Literal::Int(60))),
                        })),
                        body: Box::new(Expr::Literal(Literal::String("adult".to_string()))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::String("senior".to_string()))),
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_strings() {
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "msg".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::String("hello".to_string())),
                is_const: false,
            },
            Stmt::Expr(Expr::Match {
                value: Box::new(Expr::Identifier("msg".to_string())),
                arms: vec![
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Literal(Literal::String("hello".to_string())),
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::Int(1))),
                    },
                    parser::ast::MatchArm {
                        pattern: parser::ast::Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::Literal(Literal::Int(0))),
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
