// Pattern Matching Advanced Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, Literal, MatchArm, Pattern, Program, Stmt, ExprKind, StmtKind};

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

// ==================== PATTERN MATCHING - TYPE COERCION ====================

#[test]
fn test_match_int_float_coercion() {
    // match 5 {
    //     1 -> 1,
    //     2 -> 2.5,  // int arm + float arm -> result should be float
    //     _ -> 0.0
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(1)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(2)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(2.5)))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_type_promotion_in_arms() {
    // var x := 10;
    // match x {
    //     5 -> 100,      // int
    //     10 -> 3.14,    // float - promotes result type
    //     _ -> 0.0
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::Int(5)),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(100)))),
                    },
                    MatchArm {
                        pattern: Pattern::Literal(Literal::Int(10)),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_complex_and_float_coercion() {
    // match 1 {
    //     0 -> 1.0,
    //     1 -> Complex(2.0, 3.0),  // Complex arm forces coercion
    //     _ -> Complex(0.0, 0.0)
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(0)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(1)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Complex(2.0, 3.0)))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Complex(0.0, 0.0)))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_implicit_cast_in_computation() {
    // match 5 {
    //     1 -> 10,
    //     2 -> 20 + 2.5,  // int + float = float
    //     _ -> 0.0
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(1)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(2)),
                    guard: None,
                    body: Box::new(binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                        Expr::dummy(ExprKind::Literal(Literal::Float(2.5))),
                    )),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_all_int_arms() {
    // match 5 {
    //     1 -> 10,
    //     2 -> 20,
    //     _ -> 0
    // }
    // All int arms -> result should be int
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(1)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(2)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(20)))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_all_float_arms() {
    // match 5.5 {
    //     1.0 -> 10.0,
    //     2.0 -> 20.0,
    //     _ -> 0.0
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(5.5)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Float(1.0)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(10.0)))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Float(2.0)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(20.0)))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== PATTERN MATCHING - TYPEOF() ====================

#[test]
fn test_match_typeof_int() {
    // var x := 42;
    // match typeof(x) {
    //     "int" -> 1,
    //     _ -> 0
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::String("int".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
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

#[test]
fn test_match_typeof_float() {
    // var x := 3.14;
    // match typeof(x) {
    //     "float" -> 1,
    //     _ -> 0
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::String("float".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
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

#[test]
fn test_match_typeof_string() {
    // var x := "hello";
    // match typeof(x) {
    //     "string" -> 1,
    //     _ -> 0
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::String("string".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
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

#[test]
fn test_match_typeof_atom() {
    // var x := :ok;
    // match typeof(x) {
    //     "atom" -> 1,
    //     _ -> 0
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Atom("ok".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::String("atom".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
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

#[test]
fn test_match_typeof_nil() {
    // var x := nil;
    // match typeof(x) {
    //     "nil" -> 1,
    //     _ -> 0
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Nil)),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::String("nil".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
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

#[test]
fn test_match_typeof_with_multiple_types() {
    // var x := 42;
    // match typeof(x) {
    //     "int" -> 1,
    //     "float" -> 2,
    //     "string" -> 3,
    //     _ -> 0
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::String("int".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Literal(Literal::String("float".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                    },
                    MatchArm {
                        pattern: Pattern::Literal(Literal::String("string".to_string())),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
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


// ==================== PATTERN MATCHING - PATTERN TYPES ====================

#[test]
fn test_match_with_float_patterns() {
    // match 3.14 {
    //     1.0 -> "one",
    //     2.5 -> "two and half",
    //     3.14 -> "pi",
    //     _ -> "other"
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Float(1.0)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("one".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Float(2.5)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("two and half".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Float(3.14)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("pi".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("other".to_string())))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_boolean_patterns() {
    // match true {
    //     true -> 1,
    //     false -> 0
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Bool(true)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Bool(false)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_nil_pattern() {
    // var x := nil;
    // match x {
    //     nil -> 1,
    //     _ -> 0
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Nil)),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::Nil),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
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

#[test]
fn test_match_with_complex_patterns() {
    // match Complex(1.0, 2.0) {
    //     Complex(0.0, 0.0) -> "zero",
    //     Complex(1.0, 0.0) -> "real",
    //     _ -> "other"
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Complex(1.0, 2.0)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Complex(0.0, 0.0)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("zero".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Complex(1.0, 0.0)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("real".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("other".to_string())))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_negative_numbers() {
    // match -5 {
    //     -10 -> "minus ten",
    //     -5 -> "minus five",
    //     0 -> "zero",
    //     5 -> "five",
    //     _ -> "other"
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Unary {
                op: parser::ast::UnaryOp::Negate,
                expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
            })),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(-10)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("minus ten".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(-5)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("minus five".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(0)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("zero".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(5)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("five".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("other".to_string())))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_zero() {
    // match 0 {
    //     0 -> "zero",
    //     _ -> "non-zero"
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(0)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("zero".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("non-zero".to_string())))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== PATTERN MATCHING - COMPLEX PATTERNS ====================

#[test]
fn test_match_with_complex_guard() {
    // var x := 10;
    // match x {
    //     n if n > 0 && n < 10 -> "small positive",
    //     n if n >= 10 -> "large",
    //     _ -> "other"
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Binding("n".to_string()),
                        guard: Some(Box::new(binary(
                            BinaryOp::LogicalAnd,
                            binary(
                                BinaryOp::Gt,
                                Expr::dummy(ExprKind::Identifier("n".to_string())),
                                Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                            ),
                            binary(
                                BinaryOp::Lt,
                                Expr::dummy(ExprKind::Identifier("n".to_string())),
                                Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                            ),
                        ))),
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("small positive".to_string())))),
                    },
                    MatchArm {
                        pattern: Pattern::Binding("n".to_string()),
                        guard: Some(Box::new(binary(
                            BinaryOp::GtEq,
                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                        ))),
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("large".to_string())))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("other".to_string())))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_or_pattern_many_options() {
    // match 3 {
    //     1 | 2 | 3 | 4 | 5 -> "small",
    //     6 | 7 | 8 | 9 | 10 -> "medium",
    //     _ -> "large"
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Or(vec![
                        Pattern::Literal(Literal::Int(1)),
                        Pattern::Literal(Literal::Int(2)),
                        Pattern::Literal(Literal::Int(3)),
                        Pattern::Literal(Literal::Int(4)),
                        Pattern::Literal(Literal::Int(5)),
                    ]),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("small".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Or(vec![
                        Pattern::Literal(Literal::Int(6)),
                        Pattern::Literal(Literal::Int(7)),
                        Pattern::Literal(Literal::Int(8)),
                        Pattern::Literal(Literal::Int(9)),
                        Pattern::Literal(Literal::Int(10)),
                    ]),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("medium".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("large".to_string())))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_nested_match_expression() {
    // var x := 1;
    // var y := 2;
    // match x {
    //     1 -> match y {
    //         1 -> "one-one",
    //         2 -> "one-two",
    //         _ -> "one-other"
    //     },
    //     _ -> "other"
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::Int(1)),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Match {
                            value: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
                            arms: vec![
                                MatchArm {
                                    pattern: Pattern::Literal(Literal::Int(1)),
                                    guard: None,
                                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("one-one".to_string())))),
                                },
                                MatchArm {
                                    pattern: Pattern::Literal(Literal::Int(2)),
                                    guard: None,
                                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("one-two".to_string())))),
                                },
                                MatchArm {
                                    pattern: Pattern::Wildcard,
                                    guard: None,
                                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("one-other".to_string())))),
                                },
                            ],
                        })),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("other".to_string())))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_in_if_condition() {
    // var x := 5;
    // if (match x { 5 -> true, _ -> false }) {
    //     var result := 1;
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::If {
                condition: Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Int(5)),
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(false)))),
                        },
                    ],
                }),
                then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::VariableDecl {
                    name: "result".to_string(),
                    type_hint: None,
                    value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    is_const: false,
                })]))),
                else_block: None,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_in_return() {
    // fn classify(x: int) -> string {
    //     return match x {
    //         0 -> "zero",
    //         1 -> "one",
    //         _ -> "other"
    //     };
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef { is_async: false,
                type_params: vec![],
            name: "classify".to_string(),
            params: vec![("x".to_string(), "int".to_string(), None)],
            return_type: Some(vec!["string".to_string()]),
            body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                values: vec![Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Int(0)),
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("zero".to_string())))),
                        },
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Int(1)),
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("one".to_string())))),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("other".to_string())))),
                        },
                    ],
                })],
            })]))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_in_function_argument() {
    // fn foo(x: int) -> int { return x; }
    // foo(match 5 { 5 -> 10, _ -> 0 })
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef { is_async: false,
                type_params: vec![],
                name: "foo".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("foo".to_string()))),
                args: vec![Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Int(5)),
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                        },
                    ],
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== PATTERN MATCHING - EDGE CASES ====================

#[test]
fn test_match_with_expression_value() {
    // match 2 + 3 {
    //     5 -> "five",
    //     _ -> "other"
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(binary(
                BinaryOp::Add,
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            )),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(5)),
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("five".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("other".to_string())))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_function_call_value() {
    // fn get_num() -> int { return 5; }
    // match get_num() {
    //     5 -> "five",
    //     _ -> "other"
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef { is_async: false,
                type_params: vec![],
                name: "get_num".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("get_num".to_string()))),
                    args: vec![],
                })),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Literal(Literal::Int(5)),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("five".to_string())))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("other".to_string())))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_all_wildcards() {
    // match 42 {
    //     _ -> "first",
    //     _ -> "second"  // Unreachable but should compile
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(42)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("first".to_string())))),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("second".to_string())))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_computation_in_arm() {
    // match 5 {
    //     1 -> 10 + 20,
    //     2 -> 30 * 2,
    //     _ -> 0 - 1
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(1)),
                    guard: None,
                    body: Box::new(binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                    )),
                },
                MatchArm {
                    pattern: Pattern::Literal(Literal::Int(2)),
                    guard: None,
                    body: Box::new(binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Literal(Literal::Int(30))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    )),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(binary(
                        BinaryOp::Sub,
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    )),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_binding_same_name_in_arms() {
    // match 5 {
    //     x if x > 0 -> x,
    //     x if x < 0 -> -x,
    //     _ -> 0
    // }
    // Same binding name 'x' in multiple arms
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Binding("x".to_string()),
                    guard: Some(Box::new(binary(
                        BinaryOp::Gt,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    ))),
                    body: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                },
                MatchArm {
                    pattern: Pattern::Binding("x".to_string()),
                    guard: Some(Box::new(binary(
                        BinaryOp::Lt,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    ))),
                    body: Box::new(Expr::dummy(ExprKind::Unary {
                        op: parser::ast::UnaryOp::Negate,
                        expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    })),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_match_with_ternary_in_arm() {
    // match 5 {
    //     x if x > 0 -> (x > 10 ? 10 : x),
    //     _ -> 0
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
            value: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Binding("x".to_string()),
                    guard: Some(Box::new(binary(
                        BinaryOp::Gt,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    ))),
                    body: Box::new(Expr::dummy(ExprKind::Ternary {
                        condition: Box::new(binary(
                            BinaryOp::Gt,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                        )),
                        then_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                        else_expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    })),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    guard: None,
                    body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                },
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== PHASE 4: DESTRUCTURING PATTERNS ====================

#[test]
fn test_match_struct_destructure_all_bindings() {
    use parser::ast::StructDef;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point".to_string(),
                type_params: vec![],
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "p".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point".to_string(),
                    type_args: vec![],
                    fields: vec![
                        ("x".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                        ("y".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("p".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Destructure(vec![
                            Pattern::Binding("x".to_string()),
                            Pattern::Binding("y".to_string()),
                        ]),
                        guard: None,
                        body: Box::new(binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Identifier("y".to_string())),
                        )),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_match_struct_destructure_literal_constraint() {
    use parser::ast::StructDef;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point2".to_string(),
                type_params: vec![],
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "p".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point2".to_string(),
                    type_args: vec![],
                    fields: vec![
                        ("x".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                        ("y".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("p".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Destructure(vec![
                            Pattern::Binding("x".to_string()),
                            Pattern::Literal(Literal::Int(0)),
                        ]),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_match_struct_wildcard() {
    use parser::ast::StructDef;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point3".to_string(),
                type_params: vec![],
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "p".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point3".to_string(),
                    type_args: vec![],
                    fields: vec![
                        ("x".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
                        ("y".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(9)))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("p".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Destructure(vec![
                            Pattern::Wildcard,
                            Pattern::Binding("y".to_string()),
                        ]),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_match_range_int_inclusive() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "age".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("age".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Range {
                            start: Literal::Int(18),
                            end: Literal::Int(64),
                            inclusive: true,
                        },
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_match_range_int_exclusive() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Range {
                            start: Literal::Int(0),
                            end: Literal::Int(10),
                            inclusive: false,
                        },
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_match_range_float() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "score".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Float(0.5))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("score".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::Range {
                            start: Literal::Float(0.0),
                            end: Literal::Float(1.0),
                            inclusive: true,
                        },
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_var_destructure_struct() {
    use parser::ast::StructDef;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point4".to_string(),
                type_params: vec![],
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "p".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point4".to_string(),
                    type_args: vec![],
                    fields: vec![
                        ("x".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                        ("y".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::DestructuringDecl {
                names: vec!["x".to_string(), "y".to_string()],
                value: Expr::dummy(ExprKind::Identifier("p".to_string())),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_var_destructure_array() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(30))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::DestructuringDecl {
                names: vec!["a".to_string(), "b".to_string()],
                value: Expr::dummy(ExprKind::Identifier("arr".to_string())),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

// ==================== v1.7 Grupo D — NAMED FIELD PATTERNS ====================

#[test]
fn test_match_named_field_pattern_two_bindings() {
    // struct Point5 { x: int, y: int }
    // var p := Point5 { x: 3, y: 4 }
    // match p { { x: px, y: py } -> px + py }
    use parser::ast::StructDef;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point5".to_string(),
                type_params: vec![],
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "p".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point5".to_string(),
                    type_args: vec![],
                    fields: vec![
                        ("x".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                        ("y".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("p".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::NamedField(vec![
                            ("x".to_string(), Pattern::Binding("px".to_string())),
                            ("y".to_string(), Pattern::Binding("py".to_string())),
                        ]),
                        guard: None,
                        body: Box::new(binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("px".to_string())),
                            Expr::dummy(ExprKind::Identifier("py".to_string())),
                        )),
                    },
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_match_named_field_pattern_literal_constraint() {
    // struct Point6 { x: int, y: int }
    // var on_axis := Point6 { x: 3, y: 0 }
    // var off_axis := Point6 { x: 3, y: 4 }
    // Confirms the literal sub-pattern only matches when y == 0: the IR must
    // contain a comparison against 0 gating the first arm (not an unconditional match).
    use parser::ast::StructDef;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point6".to_string(),
                type_params: vec![],
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "p".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point6".to_string(),
                    type_args: vec![],
                    fields: vec![
                        ("x".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                        ("y".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Identifier("p".to_string()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::NamedField(vec![
                                ("x".to_string(), Pattern::Binding("px".to_string())),
                                ("y".to_string(), Pattern::Literal(Literal::Int(0))),
                            ]),
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Identifier("px".to_string()))),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(-1)))),
                        },
                    ],
                }),
                is_const: false,
            }),
        ],
    };
    let ir = compile_program(program).unwrap();
    // The literal sub-pattern must be compiled as a runtime comparison
    // (icmp) rather than always matching, otherwise the wildcard arm
    // (and the y-field literal check itself) would be dead code.
    assert!(ir.contains("icmp"), "expected the literal sub-pattern to compile to a runtime comparison, got IR:\n{}", ir);
}

#[test]
fn test_match_named_field_pattern_unknown_struct_and_field_error() {
    // Named field pattern against a non-Struct value must produce a clean
    // CodegenError::TypeError, not a panic.
    let program_wrong_type = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "n".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("n".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::NamedField(vec![
                            ("x".to_string(), Pattern::Binding("px".to_string())),
                        ]),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    },
                ],
            }))),
        ],
    };
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();
    let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());
    let result = compiler.compile_program(&program_wrong_type);
    assert!(result.is_err(), "expected named field pattern on a non-Struct value to be rejected");

    // Named field pattern referencing a field that does not exist on the
    // struct must also produce a clean CodegenError, not a panic.
    use parser::ast::StructDef;
    let program_unknown_field = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(StructDef {
                name: "Point7".to_string(),
                type_params: vec![],
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "p".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::StructInit {
                    struct_name: "Point7".to_string(),
                    type_args: vec![],
                    fields: vec![
                        ("x".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                        ("y".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Match {
                value: Box::new(Expr::dummy(ExprKind::Identifier("p".to_string()))),
                arms: vec![
                    MatchArm {
                        pattern: Pattern::NamedField(vec![
                            ("z".to_string(), Pattern::Binding("pz".to_string())),
                        ]),
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    },
                ],
            }))),
        ],
    };
    let context2 = Context::create();
    let module2 = context2.create_module("test2");
    let builder2 = context2.create_builder();
    let mut compiler2 = Compiler::new(&context2, &builder2, &module2, "test2.bx".to_string(), "".to_string());
    let result2 = compiler2.compile_program(&program_unknown_field);
    assert!(result2.is_err(), "expected named field pattern referencing an unknown struct field to be rejected");
}

#[test]
fn test_match_arm_binding_does_not_leak_to_next_arm() {
    // struct Point { x: int, y: int }
    // var p := Point { x: 42, y: 1 }
    // var result := match p {
    //     { x: px, y: 0 } -> px
    //     _ -> px           // <- px must NOT be visible here
    // }
    //
    // Regression test: match arm bindings (top-level, or nested inside
    // Destructure/NamedField) used to leak into self.variables for every
    // subsequent arm's compilation, since nothing restored the symbol table
    // between arms. `_ -> px` must fail with UndefinedSymbol, not silently
    // resolve to the previous (non-matching) arm's leftover binding.
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(parser::ast::StructDef {
                name: "PointLeak".to_string(),
                type_params: vec![],
                fields: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
            })),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "p".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::StructInit {
                    struct_name: "PointLeak".to_string(),
                    type_args: vec![],
                    fields: vec![
                        ("x".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(42)))),
                        ("y".to_string(), Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Identifier("p".to_string()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::NamedField(vec![
                                ("x".to_string(), Pattern::Binding("px".to_string())),
                                ("y".to_string(), Pattern::Literal(Literal::Int(0))),
                            ]),
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Identifier("px".to_string()))),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Identifier("px".to_string()))),
                        },
                    ],
                }),
                is_const: false,
            }),
        ],
    };
    let context = Context::create();
    let module = context.create_module("test_leak");
    let builder = context.create_builder();
    let mut compiler = Compiler::new(&context, &builder, &module, "test_leak.bx".to_string(), "".to_string());
    let result = compiler.compile_program(&program);
    assert!(
        result.is_err(),
        "expected 'px' in the wildcard arm to be undefined (leaked from the previous arm's binding otherwise)"
    );
}

// ==================== v1.7 Grupo E — ARRAY REST PATTERNS ====================

#[test]
fn test_match_array_rest_one_head() {
    // var arr := [1, 2, 3, 4, 5]
    // var result := match arr {
    //     { first, ...rest } -> first
    // }
    // Confirms the IR calls intmatrix_slice to build the `rest` capture.
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    arms: vec![MatchArm {
                        pattern: Pattern::ArrayRest {
                            head: vec![Pattern::Binding("first".to_string())],
                            rest: "rest".to_string(),
                        },
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Identifier("first".to_string()))),
                    }],
                }),
                is_const: false,
            }),
        ],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("intmatrix_slice"),
        "expected a call to intmatrix_slice to build the array-rest capture, got IR:\n{}",
        ir
    );
}

#[test]
fn test_match_array_rest_multi_head() {
    // var arr := [1, 2, 3, 4, 5]
    // var result := match arr {
    //     { a, b, ...tail } -> a + b
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::ArrayRest {
                                head: vec![
                                    Pattern::Binding("a".to_string()),
                                    Pattern::Binding("b".to_string()),
                                ],
                                rest: "tail".to_string(),
                            },
                            guard: None,
                            body: Box::new(binary(
                                BinaryOp::Add,
                                Expr::dummy(ExprKind::Identifier("a".to_string())),
                                Expr::dummy(ExprKind::Identifier("b".to_string())),
                            )),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(-1)))),
                        },
                    ],
                }),
                is_const: false,
            }),
        ],
    };
    let ir = compile_program(program).unwrap();
    assert!(
        ir.contains("intmatrix_slice"),
        "expected a call to intmatrix_slice to build the array-rest capture, got IR:\n{}",
        ir
    );
}

#[test]
fn test_match_array_rest_only_rest() {
    // var arr := [1, 2, 3, 4, 5]
    // var result := match arr {
    //     { ...all } -> all.count()
    // }
    // No head elements: `head` is empty, so the length check degenerates
    // to `total >= 0` (always true) and the whole array is captured as `all`.
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    arms: vec![MatchArm {
                        pattern: Pattern::ArrayRest {
                            head: vec![],
                            rest: "all".to_string(),
                        },
                        guard: None,
                        body: Box::new(Expr::dummy(ExprKind::Call {
                            func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                                target: Box::new(Expr::dummy(ExprKind::Identifier("all".to_string()))),
                                field: "count".to_string(),
                            })),
                            args: vec![],
                        })),
                    }],
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_array_rest_head_reads_and_slice_gated_by_len_check() {
    // Regression: [1, 2] matched against { a, b, c, ...rest } used to
    // unconditionally read data[2] (out of bounds — only 2 elements exist)
    // and unconditionally call intmatrix_slice (a real heap allocation),
    // even though this arm's length check (total >= 3) fails. Both must
    // now be gated inside basic blocks only reached when the length check
    // (and, for the slice, the head sub-patterns too) actually succeed —
    // confirmed here by checking the *block structure* the calls / GEPs
    // land in, not just that the program produces the right answer.
    let arr = Expr::dummy(ExprKind::Array(vec![
        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
    ]));
    let matched = Expr::dummy(ExprKind::Match {
        value: Box::new(arr),
        arms: vec![
            MatchArm {
                pattern: Pattern::ArrayRest {
                    head: vec![
                        Pattern::Binding("a".to_string()),
                        Pattern::Binding("b".to_string()),
                        Pattern::Binding("c".to_string()),
                    ],
                    rest: "rest".to_string(),
                },
                guard: None,
                body: Box::new(Expr::dummy(ExprKind::Identifier("a".to_string()))),
            },
            MatchArm {
                pattern: Pattern::Wildcard,
                guard: None,
                body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(-1)))),
            },
        ],
    });
    let program = Program { statements: vec![Stmt::dummy(StmtKind::Expr(matched))] };
    let ir = compile_program(program).unwrap();

    // The length check must branch BEFORE any head element is read or the
    // slice function is called — i.e. those must appear textually after
    // the `br i1 %ar_len_chk` line, inside their own labeled blocks, not
    // in the same straight-line block as the check itself.
    let len_chk_pos = ir.find("br i1 %ar_len_chk").expect("expected a length-check branch");
    let head_read_pos = ir.find("ar_ep_2").expect("expected head element 2 to still be read on the matching path");
    let slice_call_pos = ir.find("call ptr @intmatrix_slice").expect("expected the rest slice call to still be emitted on the matching path");
    assert!(head_read_pos > len_chk_pos, "head element reads must come after the length-check branch, not before it");
    assert!(slice_call_pos > len_chk_pos, "the rest slice call must come after the length-check branch, not before it");

    // And the head reads / slice call must be inside a DIFFERENT block than
    // the one containing the length check — confirmed by each living after
    // its own `br i1 %ar_len_chk`/`ar_head_check:` label boundary.
    let head_check_label_pos = ir.find("ar_head_check:").expect("expected an ar_head_check block");
    let match_label_pos = ir.find("ar_match:").expect("expected an ar_match block");
    assert!(head_read_pos > head_check_label_pos, "head reads must be inside the ar_head_check block");
    assert!(slice_call_pos > match_label_pos, "the slice call must be inside the ar_match block");
}

#[test]
fn test_array_rest_guard_only_evaluated_when_pattern_matched() {
    // Regression (CRITICAL, SIGSEGV): a guard referencing a rest capture used
    // to be compiled unconditionally right after compile_pattern_match's PHI
    // merge block, so it ran even on the runtime path where the pattern's
    // length check failed and `rest` was never bound (only ar_match binds
    // it) — reading an uninitialized pointer and crashing.
    //
    // Fix: the match-arm loop now branches on the pattern's PHI result
    // BEFORE compiling the guard, so the guard's code lives in its own block
    // reached only when the pattern truly matched. Confirmed here by the
    // block structure: `ar_merge`'s PHI result must branch to a
    // `match_arm_0_guard` block (not fall straight into guard code), and the
    // guard's own instructions (which reference `rest`) must be inside that
    // block, i.e. positioned after its label.
    let arr = Expr::dummy(ExprKind::Array(vec![
        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
    ]));
    let guard = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Eq,
        lhs: Box::new(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("rest".to_string()))),
                field: "count".to_string(),
            })),
            args: vec![],
        })),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
    });
    let matched = Expr::dummy(ExprKind::Match {
        value: Box::new(arr),
        arms: vec![
            MatchArm {
                pattern: Pattern::ArrayRest {
                    head: vec![
                        Pattern::Binding("a".to_string()),
                        Pattern::Binding("b".to_string()),
                        Pattern::Binding("c".to_string()),
                    ],
                    rest: "rest".to_string(),
                },
                guard: Some(Box::new(guard)),
                body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(99)))),
            },
            MatchArm {
                pattern: Pattern::Wildcard,
                guard: None,
                body: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(-1)))),
            },
        ],
    });
    let program = Program { statements: vec![Stmt::dummy(StmtKind::Expr(matched))] };
    let ir = compile_program(program).unwrap();

    // The PHI'd pattern result must branch to a guard block, not fall
    // straight through into guard code in the same block as the PHI.
    assert!(
        ir.contains("br i1 %ar_result, label %match_arm_0_guard"),
        "expected the pattern's PHI result to branch to a dedicated guard block, got IR:\n{}",
        ir
    );

    // The guard's reference to `rest` (via .count()) must be inside that
    // guard block (after its label), not before it / in ar_merge itself.
    let guard_label_pos = ir.find("match_arm_0_guard:").expect("expected a match_arm_0_guard block");
    let rest_load_in_guard = ir[guard_label_pos..].find("load ptr, ptr %rest")
        .expect("expected the guard to load `rest` inside its own block");
    let phi_pos = ir.find("%ar_result = phi").expect("expected the pattern match PHI");
    assert!(
        guard_label_pos + rest_load_in_guard > phi_pos,
        "the guard's use of `rest` must come after the pattern PHI, inside match_arm_0_guard"
    );
}
