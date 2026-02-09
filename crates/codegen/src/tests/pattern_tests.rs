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
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
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
            Stmt::dummy(StmtKind::FunctionDef {
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
            Stmt::dummy(StmtKind::FunctionDef {
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

