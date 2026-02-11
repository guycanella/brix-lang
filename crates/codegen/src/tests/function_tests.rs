// Function Advanced Tests - Default Parameters, Recursion, Scoping, Multiple Returns

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

// Helper to create binary expressions
fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Expr {
    Expr::dummy(ExprKind::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    })
}

// ==================== DEFAULT PARAMETERS ====================

#[test]
fn test_default_param_int_literal() {
    // fn greet(times: int = 1) -> int { return times; }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "greet".to_string(),
                params: vec![(
                    "times".to_string(),
                    "int".to_string(),
                    Some(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                )],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Identifier("times".to_string()))],
                })]))),
            }),
            // Call with default
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("greet".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "multiply".to_string(),
                params: vec![(
                    "x".to_string(),
                    "float".to_string(),
                    Some(Expr::dummy(ExprKind::Literal(Literal::Float(2.5)))),
                )],
                return_type: Some(vec!["float".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("multiply".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "greet".to_string(),
                params: vec![(
                    "name".to_string(),
                    "string".to_string(),
                    Some(Expr::dummy(ExprKind::Literal(Literal::String("World".to_string())))),
                )],
                return_type: Some(vec!["string".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Identifier("name".to_string()))],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("greet".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "add".to_string(),
                params: vec![
                    ("a".to_string(), "int".to_string(), None),
                    (
                        "b".to_string(),
                        "int".to_string(),
                        Some(binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("a".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        )),
                    ),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Identifier("a".to_string())),
                        Expr::dummy(ExprKind::Identifier("b".to_string())),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("add".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "func".to_string(),
                params: vec![
                    (
                        "a".to_string(),
                        "int".to_string(),
                        Some(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    ),
                    (
                        "b".to_string(),
                        "int".to_string(),
                        Some(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                    ),
                    (
                        "c".to_string(),
                        "int".to_string(),
                        Some(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                    ),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("a".to_string())),
                            Expr::dummy(ExprKind::Identifier("b".to_string())),
                        ),
                        Expr::dummy(ExprKind::Identifier("c".to_string())),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("func".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "greet".to_string(),
                params: vec![(
                    "name".to_string(),
                    "string".to_string(),
                    Some(Expr::dummy(ExprKind::Literal(Literal::String("World".to_string())))),
                )],
                return_type: Some(vec!["string".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Identifier("name".to_string()))],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("greet".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::String("Alice".to_string())))],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "func".to_string(),
                params: vec![
                    ("required".to_string(), "int".to_string(), None),
                    (
                        "optional".to_string(),
                        "int".to_string(),
                        Some(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                    ),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Identifier("required".to_string())),
                        Expr::dummy(ExprKind::Identifier("optional".to_string())),
                    )],
                })]))),
            }),
            // Call with only required param
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("func".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "func".to_string(),
                params: vec![
                    (
                        "a".to_string(),
                        "int".to_string(),
                        Some(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    ),
                    (
                        "b".to_string(),
                        "int".to_string(),
                        Some(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                    ),
                    (
                        "c".to_string(),
                        "int".to_string(),
                        Some(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                    ),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("a".to_string())),
                            Expr::dummy(ExprKind::Identifier("b".to_string())),
                        ),
                        Expr::dummy(ExprKind::Identifier("c".to_string())),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("func".to_string()))),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                ],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "check".to_string(),
                params: vec![(
                    "flag".to_string(),
                    "bool".to_string(),
                    Some(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
                )],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Ternary {
                        condition: Box::new(Expr::dummy(ExprKind::Identifier("flag".to_string()))),
                        then_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                        else_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    })],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("check".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "calculate".to_string(),
                params: vec![
                    (
                        "x".to_string(),
                        "float".to_string(),
                        Some(Expr::dummy(ExprKind::Literal(Literal::Float(1.5)))),
                    ),
                    (
                        "y".to_string(),
                        "float".to_string(),
                        Some(Expr::dummy(ExprKind::Literal(Literal::Float(2.5)))),
                    ),
                ],
                return_type: Some(vec!["float".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Identifier("y".to_string())),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("calculate".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_coords".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                    ],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("get_coords".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_data".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "float".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(42))),
                        Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
                    ],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("get_data".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_rgb".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                ]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(255))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(128))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    ],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("get_rgb".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_info".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "float".to_string(),
                    "string".to_string(),
                ]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(42))),
                        Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
                        Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
                    ],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("get_info".to_string()))),
                args: vec![],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "pair".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1))), Expr::dummy(ExprKind::Literal(Literal::Int(2)))],
                })]))),
            }),
            Stmt::dummy(StmtKind::DestructuringDecl {
                names: vec!["a".to_string(), "b".to_string()],
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("pair".to_string()))),
                    args: vec![],
                }),
                is_const: false,
            }),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "triple".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                ]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ],
                })]))),
            }),
            Stmt::dummy(StmtKind::DestructuringDecl {
                names: vec!["x".to_string(), "y".to_string(), "z".to_string()],
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("triple".to_string()))),
                    args: vec![],
                }),
                is_const: false,
            }),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "pair".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1))), Expr::dummy(ExprKind::Literal(Literal::Int(2)))],
                })]))),
            }),
            Stmt::dummy(StmtKind::DestructuringDecl {
                names: vec!["a".to_string(), "_".to_string()],
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("pair".to_string()))),
                    args: vec![],
                }),
                is_const: false,
            }),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "pair".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                    ],
                })]))),
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("pair".to_string()))),
                    args: vec![],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "first".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("result".to_string()))),
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
fn test_tuple_with_computation() {
    // fn compute(x: int) -> (int, int) { return x, x * 2; }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "compute".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        binary(
                            BinaryOp::Mul,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        ),
                    ],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("compute".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
            }))),
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
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_quad".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                ]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    ],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("get_quad".to_string()))),
                args: vec![],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


#[test]
fn test_recursive_factorial() {
    // fn factorial(n: int) -> int {
    //     if n <= 1 { return 1; }
    //     return n * factorial(n - 1);
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "factorial".to_string(),
                params: vec![("n".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::LtEq,
                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![binary(
                            BinaryOp::Mul,
                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                            Expr::dummy(ExprKind::Call {
                                func: Box::new(Expr::dummy(ExprKind::Identifier("factorial".to_string()))),
                                args: vec![binary(
                                    BinaryOp::Sub,
                                    Expr::dummy(ExprKind::Identifier("n".to_string())),
                                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                )],
                            }),
                        )],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("factorial".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_recursive_fibonacci() {
    // fn fib(n: int) -> int {
    //     if n <= 1 { return n; }
    //     return fib(n - 1) + fib(n - 2);
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "fib".to_string(),
                params: vec![("n".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::LtEq,
                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Identifier("n".to_string()))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Call {
                                func: Box::new(Expr::dummy(ExprKind::Identifier("fib".to_string()))),
                                args: vec![binary(
                                    BinaryOp::Sub,
                                    Expr::dummy(ExprKind::Identifier("n".to_string())),
                                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                )],
                            }),
                            Expr::dummy(ExprKind::Call {
                                func: Box::new(Expr::dummy(ExprKind::Identifier("fib".to_string()))),
                                args: vec![binary(
                                    BinaryOp::Sub,
                                    Expr::dummy(ExprKind::Identifier("n".to_string())),
                                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                                )],
                            }),
                        )],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("fib".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(10)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_recursive_power() {
    // fn power(base: int, exp: int) -> int {
    //     if exp == 0 { return 1; }
    //     return base * power(base, exp - 1);
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "power".to_string(),
                params: vec![
                    ("base".to_string(), "int".to_string(), None),
                    ("exp".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::Eq,
                            Expr::dummy(ExprKind::Identifier("exp".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![binary(
                            BinaryOp::Mul,
                            Expr::dummy(ExprKind::Identifier("base".to_string())),
                            Expr::dummy(ExprKind::Call {
                                func: Box::new(Expr::dummy(ExprKind::Identifier("power".to_string()))),
                                args: vec![
                                    Expr::dummy(ExprKind::Identifier("base".to_string())),
                                    binary(
                                        BinaryOp::Sub,
                                        Expr::dummy(ExprKind::Identifier("exp".to_string())),
                                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                    ),
                                ],
                            }),
                        )],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("power".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(2))), Expr::dummy(ExprKind::Literal(Literal::Int(10)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_recursive_gcd() {
    // fn gcd(a: int, b: int) -> int {
    //     if b == 0 { return a; }
    //     return gcd(b, a % b);
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "gcd".to_string(),
                params: vec![
                    ("a".to_string(), "int".to_string(), None),
                    ("b".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::Eq,
                            Expr::dummy(ExprKind::Identifier("b".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Identifier("a".to_string()))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Call {
                            func: Box::new(Expr::dummy(ExprKind::Identifier("gcd".to_string()))),
                            args: vec![
                                Expr::dummy(ExprKind::Identifier("b".to_string())),
                                binary(
                                    BinaryOp::Mod,
                                    Expr::dummy(ExprKind::Identifier("a".to_string())),
                                    Expr::dummy(ExprKind::Identifier("b".to_string())),
                                ),
                            ],
                        })],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("gcd".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(48))), Expr::dummy(ExprKind::Literal(Literal::Int(18)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_tail_recursive_factorial() {
    // fn fact_helper(n: int, acc: int) -> int {
    //     if n <= 1 { return acc; }
    //     return fact_helper(n - 1, acc * n);
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "fact_helper".to_string(),
                params: vec![
                    ("n".to_string(), "int".to_string(), None),
                    ("acc".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::LtEq,
                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Identifier("acc".to_string()))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Call {
                            func: Box::new(Expr::dummy(ExprKind::Identifier("fact_helper".to_string()))),
                            args: vec![
                                binary(
                                    BinaryOp::Sub,
                                    Expr::dummy(ExprKind::Identifier("n".to_string())),
                                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                ),
                                binary(
                                    BinaryOp::Mul,
                                    Expr::dummy(ExprKind::Identifier("acc".to_string())),
                                    Expr::dummy(ExprKind::Identifier("n".to_string())),
                                ),
                            ],
                        })],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("fact_helper".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5))), Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_mutual_recursion_even_odd() {
    // fn is_even(n: int) -> int {
    //     if n == 0 { return 1; }
    //     return is_odd(n - 1);
    // }
    // fn is_odd(n: int) -> int {
    //     if n == 0 { return 0; }
    //     return is_even(n - 1);
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "is_even".to_string(),
                params: vec![("n".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::Eq,
                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Call {
                            func: Box::new(Expr::dummy(ExprKind::Identifier("is_odd".to_string()))),
                            args: vec![binary(
                                BinaryOp::Sub,
                                Expr::dummy(ExprKind::Identifier("n".to_string())),
                                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                            )],
                        })],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "is_odd".to_string(),
                params: vec![("n".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::Eq,
                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Call {
                            func: Box::new(Expr::dummy(ExprKind::Identifier("is_even".to_string()))),
                            args: vec![binary(
                                BinaryOp::Sub,
                                Expr::dummy(ExprKind::Identifier("n".to_string())),
                                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                            )],
                        })],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("is_even".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(10)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_recursive_countdown() {
    // fn countdown(n: int) -> int {
    //     if n == 0 { return 0; }
    //     return countdown(n - 1);
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "countdown".to_string(),
                params: vec![("n".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::Eq,
                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Call {
                            func: Box::new(Expr::dummy(ExprKind::Identifier("countdown".to_string()))),
                            args: vec![binary(
                                BinaryOp::Sub,
                                Expr::dummy(ExprKind::Identifier("n".to_string())),
                                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                            )],
                        })],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("countdown".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(100)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_recursive_sum_range() {
    // fn sum_range(start: int, end: int) -> int {
    //     if start > end { return 0; }
    //     return start + sum_range(start + 1, end);
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "sum_range".to_string(),
                params: vec![
                    ("start".to_string(), "int".to_string(), None),
                    ("end".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::Gt,
                            Expr::dummy(ExprKind::Identifier("start".to_string())),
                            Expr::dummy(ExprKind::Identifier("end".to_string())),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("start".to_string())),
                            Expr::dummy(ExprKind::Call {
                                func: Box::new(Expr::dummy(ExprKind::Identifier("sum_range".to_string()))),
                                args: vec![
                                    binary(
                                        BinaryOp::Add,
                                        Expr::dummy(ExprKind::Identifier("start".to_string())),
                                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                    ),
                                    Expr::dummy(ExprKind::Identifier("end".to_string())),
                                ],
                            }),
                        )],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("sum_range".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1))), Expr::dummy(ExprKind::Literal(Literal::Int(10)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_recursive_min() {
    // fn min(a: int, b: int) -> int {
    //     return a < b ? a : b;
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "min".to_string(),
                params: vec![
                    ("a".to_string(), "int".to_string(), None),
                    ("b".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Ternary {
                        condition: Box::new(binary(
                            BinaryOp::Lt,
                            Expr::dummy(ExprKind::Identifier("a".to_string())),
                            Expr::dummy(ExprKind::Identifier("b".to_string())),
                        )),
                        then_expr: Box::new(Expr::dummy(ExprKind::Identifier("a".to_string()))),
                        else_expr: Box::new(Expr::dummy(ExprKind::Identifier("b".to_string()))),
                    })],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("min".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5))), Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_recursive_nested_calls() {
    // fn ackermann(m: int, n: int) -> int {
    //     if m == 0 { return n + 1; }
    //     if n == 0 { return ackermann(m - 1, 1); }
    //     return ackermann(m - 1, ackermann(m, n - 1));
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "ackermann".to_string(),
                params: vec![
                    ("m".to_string(), "int".to_string(), None),
                    ("n".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::Eq,
                            Expr::dummy(ExprKind::Identifier("m".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![binary(
                                BinaryOp::Add,
                                Expr::dummy(ExprKind::Identifier("n".to_string())),
                                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                            )],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::Eq,
                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Call {
                                func: Box::new(Expr::dummy(ExprKind::Identifier("ackermann".to_string()))),
                                args: vec![
                                    binary(
                                        BinaryOp::Sub,
                                        Expr::dummy(ExprKind::Identifier("m".to_string())),
                                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                    ),
                                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                ],
                            })],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Call {
                            func: Box::new(Expr::dummy(ExprKind::Identifier("ackermann".to_string()))),
                            args: vec![
                                binary(
                                    BinaryOp::Sub,
                                    Expr::dummy(ExprKind::Identifier("m".to_string())),
                                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                ),
                                Expr::dummy(ExprKind::Call {
                                    func: Box::new(Expr::dummy(ExprKind::Identifier("ackermann".to_string()))),
                                    args: vec![
                                        Expr::dummy(ExprKind::Identifier("m".to_string())),
                                        binary(
                                            BinaryOp::Sub,
                                            Expr::dummy(ExprKind::Identifier("n".to_string())),
                                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                                        ),
                                    ],
                                }),
                            ],
                        })],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("ackermann".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(2))), Expr::dummy(ExprKind::Literal(Literal::Int(2)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== SCOPING ====================

#[test]
fn test_local_shadows_global() {
    // var x := 10;
    // fn test() -> int {
    //     var x := 20;  // Shadows global x
    //     return x;
    // }
    // test()  // Should return 20, not 10
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "test".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::VariableDecl {
                        name: "x".to_string(),
                        type_hint: None,
                        value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                        is_const: false,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("test".to_string()))),
                args: vec![],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_param_shadows_global() {
    // var x := 10;
    // fn test(x: int) -> int {  // Parameter x shadows global x
    //     return x;
    // }
    // test(20)  // Should return 20
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "test".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("test".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(20)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_access_global_from_function() {
    // var global := 42;
    // fn get_global() -> int {
    //     return global;
    // }
    // get_global()  // Should return 42
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "global".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_global".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Identifier("global".to_string()))],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("get_global".to_string()))),
                args: vec![],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_local_shadows_param() {
    // fn test(x: int) -> int {
    //     var x := 100;  // Shadows parameter
    //     return x;
    // }
    // test(50)  // Should return 100
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "test".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::VariableDecl {
                        name: "x".to_string(),
                        type_hint: None,
                        value: Expr::dummy(ExprKind::Literal(Literal::Int(100))),
                        is_const: false,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("test".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(50)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_functions_same_local_name() {
    // fn func1() -> int {
    //     var x := 10;
    //     return x;
    // }
    // fn func2() -> int {
    //     var x := 20;  // Different x, different scope
    //     return x;
    // }
    // func1() + func2()
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "func1".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::VariableDecl {
                        name: "x".to_string(),
                        type_hint: None,
                        value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                        is_const: false,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "func2".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::VariableDecl {
                        name: "x".to_string(),
                        type_hint: None,
                        value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                        is_const: false,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(binary(
                BinaryOp::Add,
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("func1".to_string()))),
                    args: vec![],
                }),
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("func2".to_string()))),
                    args: vec![],
                }),
            ))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_shadowing_with_different_types() {
    // var x := 10;  // int
    // fn test() -> float {
    //     var x := 3.14;  // float, shadows int x
    //     return x;
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "test".to_string(),
                params: vec![],
                return_type: Some(vec!["float".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::VariableDecl {
                        name: "x".to_string(),
                        type_hint: None,
                        value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
                        is_const: false,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("test".to_string()))),
                args: vec![],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_function_modifies_global() {
    // var counter := 0;
    // fn increment() -> int {
    //     counter = counter + 1;
    //     return counter;
    // }
    // increment()
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "counter".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "increment".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::Assignment {
                        target: Expr::dummy(ExprKind::Identifier("counter".to_string())),
                        value: binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("counter".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        ),
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Identifier("counter".to_string()))],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("increment".to_string()))),
                args: vec![],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_nested_scopes_in_blocks() {
    // fn test() -> int {
    //     var x := 1;
    //     if true {
    //         var x := 2;  // Shadows outer x in block
    //         return x;
    //     }
    //     return x;
    // }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
            name: "test".to_string(),
            params: vec![],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                Stmt::dummy(StmtKind::VariableDecl {
                    name: "x".to_string(),
                    type_hint: None,
                    value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    is_const: false,
                }),
                Stmt::dummy(StmtKind::If {
                    condition: Expr::dummy(ExprKind::Literal(Literal::Bool(true))),
                    then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![
                        Stmt::dummy(StmtKind::VariableDecl {
                            name: "x".to_string(),
                            type_hint: None,
                            value: Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                            is_const: false,
                        }),
                        Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                        }),
                    ]))),
                    else_block: None,
                }),
                Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                }),
            ]))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_param_access_in_nested_calls() {
    // fn outer(x: int) -> int {
    //     fn inner() -> int {  // Can access x from outer? (depends on implementation)
    //         return x + 1;
    //     }
    //     return inner();
    // }
    // For now, just test that params are accessible within their function
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "outer".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("outer".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_params_same_name_different_functions() {
    // fn add(x: int) -> int { return x + 1; }
    // fn mul(x: int) -> int { return x * 2; }
    // Each function has its own 'x' parameter
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "add".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "mul".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(binary(
                BinaryOp::Add,
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("add".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
                }),
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("mul".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
                }),
            ))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== FUNCTION EDGE CASES ====================

#[test]
fn test_function_no_return_void() {
    // fn do_nothing() -> void { }
    // do_nothing()
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "do_nothing".to_string(),
                params: vec![],
                return_type: None, // void function
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![]))), // empty body
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("do_nothing".to_string()))),
                args: vec![],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_function_early_return() {
    // fn early(x: int) -> int {
    //     if x > 10 { return 100; }
    //     return x;
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "early".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![
                    Stmt::dummy(StmtKind::If {
                        condition: binary(
                            BinaryOp::Gt,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                        ),
                        then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                            values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(100)))],
                        })]))),
                        else_block: None,
                    }),
                    Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                    }),
                ]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("early".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(15)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_function_return_in_if_else() {
    // fn abs(x: int) -> int {
    //     if x < 0 { return -x; }
    //     else { return x; }
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "abs".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::If {
                    condition: binary(
                        BinaryOp::Lt,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    ),
                    then_block: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Unary {
                            op: parser::ast::UnaryOp::Negate,
                            expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                        })],
                    })]))),
                    else_block: Some(Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                        values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                    })])))),
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("abs".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(-42)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_function_calling_function() {
    // fn double(x: int) -> int { return x * 2; }
    // fn quad(x: int) -> int { return double(double(x)); }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "double".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "quad".to_string(),
                params: vec![("x".to_string(), "int".to_string(), None)],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Call {
                        func: Box::new(Expr::dummy(ExprKind::Identifier("double".to_string()))),
                        args: vec![Expr::dummy(ExprKind::Call {
                            func: Box::new(Expr::dummy(ExprKind::Identifier("double".to_string()))),
                            args: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                        })],
                    })],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("quad".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_nested_function_calls() {
    // fn add(a: int, b: int) -> int { return a + b; }
    // fn mul(a: int, b: int) -> int { return a * b; }
    // add(mul(2, 3), mul(4, 5))
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "add".to_string(),
                params: vec![
                    ("a".to_string(), "int".to_string(), None),
                    ("b".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Identifier("a".to_string())),
                        Expr::dummy(ExprKind::Identifier("b".to_string())),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "mul".to_string(),
                params: vec![
                    ("a".to_string(), "int".to_string(), None),
                    ("b".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Identifier("a".to_string())),
                        Expr::dummy(ExprKind::Identifier("b".to_string())),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("add".to_string()))),
                args: vec![
                    Expr::dummy(ExprKind::Call {
                        func: Box::new(Expr::dummy(ExprKind::Identifier("mul".to_string()))),
                        args: vec![
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        ],
                    }),
                    Expr::dummy(ExprKind::Call {
                        func: Box::new(Expr::dummy(ExprKind::Identifier("mul".to_string()))),
                        args: vec![
                            Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                            Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                        ],
                    }),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_function_many_parameters() {
    // fn sum5(a: int, b: int, c: int, d: int, e: int) -> int {
    //     return a + b + c + d + e;
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "sum5".to_string(),
                params: vec![
                    ("a".to_string(), "int".to_string(), None),
                    ("b".to_string(), "int".to_string(), None),
                    ("c".to_string(), "int".to_string(), None),
                    ("d".to_string(), "int".to_string(), None),
                    ("e".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Add,
                        binary(
                            BinaryOp::Add,
                            binary(
                                BinaryOp::Add,
                                binary(
                                    BinaryOp::Add,
                                    Expr::dummy(ExprKind::Identifier("a".to_string())),
                                    Expr::dummy(ExprKind::Identifier("b".to_string())),
                                ),
                                Expr::dummy(ExprKind::Identifier("c".to_string())),
                            ),
                            Expr::dummy(ExprKind::Identifier("d".to_string())),
                        ),
                        Expr::dummy(ExprKind::Identifier("e".to_string())),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("sum5".to_string()))),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_function_no_parameters() {
    // fn get_pi() -> float { return 3.14159; }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_pi".to_string(),
                params: vec![],
                return_type: Some(vec!["float".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.14159)))],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("get_pi".to_string()))),
                args: vec![],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_function_returns_function_result() {
    // fn inner() -> int { return 42; }
    // fn outer() -> int { return inner(); }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "inner".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
                })]))),
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "outer".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Call {
                        func: Box::new(Expr::dummy(ExprKind::Identifier("inner".to_string()))),
                        args: vec![],
                    })],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("outer".to_string()))),
                args: vec![],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_function_with_complex_expression() {
    // fn complex(x: int, y: int) -> int {
    //     return (x + y) * (x - y);
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "complex".to_string(),
                params: vec![
                    ("x".to_string(), "int".to_string(), None),
                    ("y".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![binary(
                        BinaryOp::Mul,
                        binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Identifier("y".to_string())),
                        ),
                        binary(
                            BinaryOp::Sub,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Identifier("y".to_string())),
                        ),
                    )],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("complex".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5))), Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_function_with_ternary_return() {
    // fn max(a: int, b: int) -> int {
    //     return a > b ? a : b;
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "max".to_string(),
                params: vec![
                    ("a".to_string(), "int".to_string(), None),
                    ("b".to_string(), "int".to_string(), None),
                ],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Ternary {
                        condition: Box::new(binary(
                            BinaryOp::Gt,
                            Expr::dummy(ExprKind::Identifier("a".to_string())),
                            Expr::dummy(ExprKind::Identifier("b".to_string())),
                        )),
                        then_expr: Box::new(Expr::dummy(ExprKind::Identifier("a".to_string()))),
                        else_expr: Box::new(Expr::dummy(ExprKind::Identifier("b".to_string()))),
                    })],
                })]))),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("max".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(10))), Expr::dummy(ExprKind::Literal(Literal::Int(20)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
