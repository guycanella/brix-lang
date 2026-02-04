// Function Advanced Tests - Default Parameters, Recursion, Scoping, Multiple Returns

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

// Helper to create binary expressions
fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Expr {
    Expr::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    }
}

// ==================== DEFAULT PARAMETERS ====================

#[test]
fn test_default_param_int_literal() {
    // fn greet(times: int = 1) -> int { return times; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "greet".to_string(),
                params: vec![(
                    "times".to_string(),
                    "int".to_string(),
                    Some(Expr::Literal(Literal::Int(1))),
                )],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![Expr::Identifier("times".to_string())],
                }])),
            },
            // Call with default
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("greet".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_default_param_float_literal() {
    // fn multiply(x: float = 2.5) -> float { return x * 2.0; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "multiply".to_string(),
                params: vec![(
                    "x".to_string(),
                    "float".to_string(),
                    Some(Expr::Literal(Literal::Float(2.5))),
                )],
                return_type: Some(vec!["float".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![binary(
                        BinaryOp::Mul,
                        Expr::Identifier("x".to_string()),
                        Expr::Literal(Literal::Float(2.0)),
                    )],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("multiply".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_default_param_string_literal() {
    // fn greet(name: string = "World") -> string { return name; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "greet".to_string(),
                params: vec![(
                    "name".to_string(),
                    "string".to_string(),
                    Some(Expr::Literal(Literal::String("World".to_string()))),
                )],
                return_type: Some(vec!["string".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![Expr::Identifier("name".to_string())],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("greet".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_default_param_expression() {
    // fn add(a: int, b: int = a + 1) -> int { return a + b; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "add".to_string(),
                params: vec![
                    ("a".to_string(), "int".to_string(), None),
                    (
                        "b".to_string(),
                        "int".to_string(),
                        Some(binary(
                            BinaryOp::Add,
                            Expr::Identifier("a".to_string()),
                            Expr::Literal(Literal::Int(1)),
                        )),
                    ),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        Expr::Identifier("a".to_string()),
                        Expr::Identifier("b".to_string()),
                    )],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("add".to_string())),
                args: vec![Expr::Literal(Literal::Int(5))],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_defaults() {
    // fn func(a: int = 1, b: int = 2, c: int = 3) -> int { return a + b + c; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "func".to_string(),
                params: vec![
                    (
                        "a".to_string(),
                        "int".to_string(),
                        Some(Expr::Literal(Literal::Int(1))),
                    ),
                    (
                        "b".to_string(),
                        "int".to_string(),
                        Some(Expr::Literal(Literal::Int(2))),
                    ),
                    (
                        "c".to_string(),
                        "int".to_string(),
                        Some(Expr::Literal(Literal::Int(3))),
                    ),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        binary(
                            BinaryOp::Add,
                            Expr::Identifier("a".to_string()),
                            Expr::Identifier("b".to_string()),
                        ),
                        Expr::Identifier("c".to_string()),
                    )],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("func".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_override_default_value() {
    // fn greet(name: string = "World") -> string { return name; }
    // greet("Alice")
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "greet".to_string(),
                params: vec![(
                    "name".to_string(),
                    "string".to_string(),
                    Some(Expr::Literal(Literal::String("World".to_string()))),
                )],
                return_type: Some(vec!["string".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![Expr::Identifier("name".to_string())],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("greet".to_string())),
                args: vec![Expr::Literal(Literal::String("Alice".to_string()))],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_required_and_default_params() {
    // fn func(required: int, optional: int = 10) -> int { return required + optional; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "func".to_string(),
                params: vec![
                    ("required".to_string(), "int".to_string(), None),
                    (
                        "optional".to_string(),
                        "int".to_string(),
                        Some(Expr::Literal(Literal::Int(10))),
                    ),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        Expr::Identifier("required".to_string()),
                        Expr::Identifier("optional".to_string()),
                    )],
                }])),
            },
            // Call with only required param
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("func".to_string())),
                args: vec![Expr::Literal(Literal::Int(5))],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_partial_default_override() {
    // fn func(a: int = 1, b: int = 2, c: int = 3) -> int { return a + b + c; }
    // func(10, 20) - override first two, use default for c
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "func".to_string(),
                params: vec![
                    (
                        "a".to_string(),
                        "int".to_string(),
                        Some(Expr::Literal(Literal::Int(1))),
                    ),
                    (
                        "b".to_string(),
                        "int".to_string(),
                        Some(Expr::Literal(Literal::Int(2))),
                    ),
                    (
                        "c".to_string(),
                        "int".to_string(),
                        Some(Expr::Literal(Literal::Int(3))),
                    ),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        binary(
                            BinaryOp::Add,
                            Expr::Identifier("a".to_string()),
                            Expr::Identifier("b".to_string()),
                        ),
                        Expr::Identifier("c".to_string()),
                    )],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("func".to_string())),
                args: vec![
                    Expr::Literal(Literal::Int(10)),
                    Expr::Literal(Literal::Int(20)),
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_default_param_boolean() {
    // fn check(flag: bool = true) -> int { return flag ? 1 : 0; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "check".to_string(),
                params: vec![(
                    "flag".to_string(),
                    "bool".to_string(),
                    Some(Expr::Literal(Literal::Bool(true))),
                )],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![Expr::Ternary {
                        condition: Box::new(Expr::Identifier("flag".to_string())),
                        then_expr: Box::new(Expr::Literal(Literal::Int(1))),
                        else_expr: Box::new(Expr::Literal(Literal::Int(0))),
                    }],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("check".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_default_param_with_float_types() {
    // fn calculate(x: float = 1.5, y: float = 2.5) -> float { return x * y; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "calculate".to_string(),
                params: vec![
                    (
                        "x".to_string(),
                        "float".to_string(),
                        Some(Expr::Literal(Literal::Float(1.5))),
                    ),
                    (
                        "y".to_string(),
                        "float".to_string(),
                        Some(Expr::Literal(Literal::Float(2.5))),
                    ),
                ],
                return_type: Some(vec!["float".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![binary(
                        BinaryOp::Mul,
                        Expr::Identifier("x".to_string()),
                        Expr::Identifier("y".to_string()),
                    )],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("calculate".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MULTIPLE RETURNS / TUPLES ====================

#[test]
fn test_tuple_return_two_ints() {
    // fn get_coords() -> (int, int) { return 10, 20; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "get_coords".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![
                        Expr::Literal(Literal::Int(10)),
                        Expr::Literal(Literal::Int(20)),
                    ],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("get_coords".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_return_int_and_float() {
    // fn get_data() -> (int, float) { return 42, 3.14; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "get_data".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "float".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![
                        Expr::Literal(Literal::Int(42)),
                        Expr::Literal(Literal::Float(3.14)),
                    ],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("get_data".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_return_three_values() {
    // fn get_rgb() -> (int, int, int) { return 255, 128, 0; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "get_rgb".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                ]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![
                        Expr::Literal(Literal::Int(255)),
                        Expr::Literal(Literal::Int(128)),
                        Expr::Literal(Literal::Int(0)),
                    ],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("get_rgb".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_return_mixed_types() {
    // fn get_info() -> (int, float, string) { return 42, 3.14, "hello"; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "get_info".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "float".to_string(),
                    "string".to_string(),
                ]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![
                        Expr::Literal(Literal::Int(42)),
                        Expr::Literal(Literal::Float(3.14)),
                        Expr::Literal(Literal::String("hello".to_string())),
                    ],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("get_info".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_destructuring_simple() {
    // fn pair() -> (int, int) { return 1, 2; }
    // var { a, b } := pair();
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "pair".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![Expr::Literal(Literal::Int(1)), Expr::Literal(Literal::Int(2))],
                }])),
            },
            Stmt::DestructuringDecl {
                names: vec!["a".to_string(), "b".to_string()],
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("pair".to_string())),
                    args: vec![],
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_destructuring_three_values() {
    // fn triple() -> (int, int, int) { return 1, 2, 3; }
    // var { x, y, z } := triple();
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "triple".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                ]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Int(2)),
                        Expr::Literal(Literal::Int(3)),
                    ],
                }])),
            },
            Stmt::DestructuringDecl {
                names: vec!["x".to_string(), "y".to_string(), "z".to_string()],
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("triple".to_string())),
                    args: vec![],
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_destructuring_ignore_value() {
    // fn pair() -> (int, int) { return 1, 2; }
    // var { a, _ } := pair();  // Ignore second value
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "pair".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![Expr::Literal(Literal::Int(1)), Expr::Literal(Literal::Int(2))],
                }])),
            },
            Stmt::DestructuringDecl {
                names: vec!["a".to_string(), "_".to_string()],
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("pair".to_string())),
                    args: vec![],
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_indexing() {
    // fn pair() -> (int, int) { return 10, 20; }
    // var result := pair();
    // var first := result[0];
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "pair".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![
                        Expr::Literal(Literal::Int(10)),
                        Expr::Literal(Literal::Int(20)),
                    ],
                }])),
            },
            Stmt::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::Identifier("pair".to_string())),
                    args: vec![],
                },
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "first".to_string(),
                type_hint: None,
                value: Expr::Index {
                    array: Box::new(Expr::Identifier("result".to_string())),
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
fn test_tuple_with_computation() {
    // fn compute(x: int) -> (int, int) { return x, x * 2; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "compute".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![
                        Expr::Identifier("x".to_string()),
                        binary(
                            BinaryOp::Mul,
                            Expr::Identifier("x".to_string()),
                            Expr::Literal(Literal::Int(2)),
                        ),
                    ],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("compute".to_string())),
                args: vec![Expr::Literal(Literal::Int(5))],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_four_values() {
    // fn get_quad() -> (int, int, int, int) { return 1, 2, 3, 4; }
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "get_quad".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                ]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![
                        Expr::Literal(Literal::Int(1)),
                        Expr::Literal(Literal::Int(2)),
                        Expr::Literal(Literal::Int(3)),
                        Expr::Literal(Literal::Int(4)),
                    ],
                }])),
            },
            Stmt::Expr(Expr::Call {
                func: Box::new(Expr::Identifier("get_quad".to_string())),
                args: vec![],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
