// Integration Tests - Polish Final
// Testes que combinam múltiplas features e casos de integração complexos

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, Literal, MatchArm, Pattern, Program, Stmt, UnaryOp, ExprKind, StmtKind};

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

// 1. Array field access combined with arithmetic
#[test]
fn test_array_field_access_with_arithmetic() {
    // var arr := [1, 2, 3]
    // var len := arr.rows * 2
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
                name: "len".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::Mul,
                    Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                        field: "rows".to_string(),
                    }),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 2. Ternary with array indexing
#[test]
fn test_ternary_with_array_indexing() {
    // var arr := [10, 20, 30]
    // var idx := 1
    // var val := idx > 0 ? arr[idx] : arr[0]
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
            Stmt::dummy(StmtKind::VariableDecl {
                name: "idx".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "val".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Ternary {
                    condition: Box::new(binary(
                        BinaryOp::Gt,
                        Expr::dummy(ExprKind::Identifier("idx".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                    )),
                    then_expr: Box::new(Expr::dummy(ExprKind::Index {
                        array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                        indices: vec![Expr::dummy(ExprKind::Identifier("idx".to_string()))],
                    })),
                    else_expr: Box::new(Expr::dummy(ExprKind::Index {
                        array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                        indices: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
                    })),
                }),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 3. Match expression with arithmetic in arms
#[test]
fn test_match_with_arithmetic_in_arms() {
    // var x := 5
    // var result := match x {
    //     1 => 10 + 5,
    //     2 => 20 * 2,
    //     _ => 0 - 1
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Int(1)),
                            guard: None,
                            body: Box::new(binary(
                                BinaryOp::Add,
                                Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                                Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                            )),
                        },
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Int(2)),
                            guard: None,
                            body: Box::new(binary(
                                BinaryOp::Mul,
                                Expr::dummy(ExprKind::Literal(Literal::Int(20))),
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
                }),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 4. Complex number operations with variable
#[test]
fn test_complex_operations_with_variable() {
    // var z := 3.0 + 4.0im
    // var z2 := z * z
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "z".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::Add,
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                    binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Literal(Literal::Float(4.0))),
                        Expr::dummy(ExprKind::Identifier("im".to_string())),
                    ),
                ),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "z2".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::Mul,
                    Expr::dummy(ExprKind::Identifier("z".to_string())),
                    Expr::dummy(ExprKind::Identifier("z".to_string())),
                ),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 5. Bitwise combined with comparison
#[test]
fn test_bitwise_combined_with_comparison() {
    // var x := 0xFF
    // var y := 0x0F
    // var result := (x & y) > 10
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(0xFF))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(0x0F))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::Gt,
                    binary(
                        BinaryOp::BitAnd,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Identifier("y".to_string())),
                    ),
                    Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                ),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 6. Logical operators combined with arithmetic
#[test]
fn test_logical_with_arithmetic() {
    // var x := 10
    // var y := 20
    // var result := (x + 5 > 10) && (y - 5 < 20)
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
                value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::LogicalAnd,
                    binary(
                        BinaryOp::Gt,
                        binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                        ),
                        Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    ),
                    binary(
                        BinaryOp::Lt,
                        binary(
                            BinaryOp::Sub,
                            Expr::dummy(ExprKind::Identifier("y".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                        ),
                        Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                    ),
                ),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 7. Array indexing with arithmetic expression as index
#[test]
fn test_array_index_with_arithmetic_expression() {
    // var arr := [10, 20, 30, 40, 50]
    // var i := 2
    // var val := arr[i * 2 - 1]
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(30))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(40))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(50))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "i".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "val".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    indices: vec![binary(
                        BinaryOp::Sub,
                        binary(
                            BinaryOp::Mul,
                            Expr::dummy(ExprKind::Identifier("i".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        ),
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    )],
                }),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 8. Match with atoms and string results
#[test]
fn test_match_atoms_with_string_results() {
    // var code := :ok
    // var msg := match code {
    //     :ok => "Success",
    //     :error => "Failed",
    //     _ => "Unknown"
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "code".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Atom("ok".to_string()))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "msg".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Match {
                    value: Box::new(Expr::dummy(ExprKind::Identifier("code".to_string()))),
                    arms: vec![
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Atom("ok".to_string())),
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("Success".to_string())))),
                        },
                        MatchArm {
                            pattern: Pattern::Literal(Literal::Atom("error".to_string())),
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("Failed".to_string())))),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: Box::new(Expr::dummy(ExprKind::Literal(Literal::String("Unknown".to_string())))),
                        },
                    ],
                }),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 9. Unary operators combined with binary operations
#[test]
fn test_unary_with_binary_operations() {
    // var x := 10
    // var result := -(x + 5) * 2
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::Mul,
                    Expr::dummy(ExprKind::Unary {
                        op: UnaryOp::Negate,
                        expr: Box::new(binary(
                            BinaryOp::Add,
                            Expr::dummy(ExprKind::Identifier("x".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                        )),
                    }),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 10. Multiple variable declarations with dependencies
#[test]
fn test_multiple_var_decls_with_dependencies() {
    // var a := 10
    // var b := a * 2
    // var c := b + a
    // var d := c - b + a
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "a".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "b".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::Mul,
                    Expr::dummy(ExprKind::Identifier("a".to_string())),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "c".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::Add,
                    Expr::dummy(ExprKind::Identifier("b".to_string())),
                    Expr::dummy(ExprKind::Identifier("a".to_string())),
                ),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "d".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::Add,
                    binary(
                        BinaryOp::Sub,
                        Expr::dummy(ExprKind::Identifier("c".to_string())),
                        Expr::dummy(ExprKind::Identifier("b".to_string())),
                    ),
                    Expr::dummy(ExprKind::Identifier("a".to_string())),
                ),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 11. Compound assignment with complex right side
#[test]
fn test_compound_assignment_complex_rhs() {
    // var x := 10
    // var y := 5
    // x += y * 2 + 3
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
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: binary(
                    BinaryOp::Add,
                    Expr::dummy(ExprKind::Identifier("x".to_string())),
                    binary(
                        BinaryOp::Add,
                        binary(
                            BinaryOp::Mul,
                            Expr::dummy(ExprKind::Identifier("y".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        ),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ),
                ),
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 12. If-else with complex conditions and nested blocks
#[test]
fn test_if_else_complex_conditions_nested() {
    // var x := 15
    // var y := 20
    // if x > 10 && y < 30 {
    //     var z := x + y
    // } else {
    //     var z := x - y
    // }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(15))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::If {
                condition: binary(
                    BinaryOp::LogicalAnd,
                    binary(
                        BinaryOp::Gt,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                    ),
                    binary(
                        BinaryOp::Lt,
                        Expr::dummy(ExprKind::Identifier("y".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(30))),
                    ),
                ),
                then_block: Box::new(Stmt::dummy(StmtKind::VariableDecl {
                    name: "z".to_string(),
                    type_hint: None,
                    value: binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Identifier("y".to_string())),
                    ),
                    is_const: false,
                })),
                else_block: Some(Box::new(Stmt::dummy(StmtKind::VariableDecl {
                    name: "z".to_string(),
                    type_hint: None,
                    value: binary(
                        BinaryOp::Sub,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Identifier("y".to_string())),
                    ),
                    is_const: false,
                }))),
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 13. While loop with compound condition and multiple statements
#[test]
fn test_while_compound_condition_multiple_stmts() {
    // var x := 10
    // var y := 0
    // while x > 0 && y < 20 {
    //     x -= 1
    //     y += 2
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
                value: Expr::dummy(ExprKind::Literal(Literal::Int(0))),
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
                        Expr::dummy(ExprKind::Literal(Literal::Int(20))),
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
                            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        ),
                    }),
                ]))),
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 14. Array assignment with complex index expression
#[test]
fn test_array_assignment_complex_index() {
    // var arr := [1, 2, 3, 4, 5]
    // var i := 2
    // arr[i + 1] = arr[i - 1] * 2
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
                name: "i".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    indices: vec![binary(
                        BinaryOp::Add,
                        Expr::dummy(ExprKind::Identifier("i".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    )],
                }),
                value: binary(
                    BinaryOp::Mul,
                    Expr::dummy(ExprKind::Index {
                        array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                        indices: vec![binary(
                            BinaryOp::Sub,
                            Expr::dummy(ExprKind::Identifier("i".to_string())),
                            Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        )],
                    }),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ),
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}

// 15. Power operator combined with other operations
#[test]
fn test_power_combined_with_operations() {
    // var x := 2
    // var y := 3
    // var result := (x ** y) + (y ** x)
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "result".to_string(),
                type_hint: None,
                value: binary(
                    BinaryOp::Add,
                    binary(
                        BinaryOp::Pow,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Identifier("y".to_string())),
                    ),
                    binary(
                        BinaryOp::Pow,
                        Expr::dummy(ExprKind::Identifier("y".to_string())),
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                    ),
                ),
                is_const: false,
            }),
        ],
    };
    assert!(compile_program(program).is_ok());
}
