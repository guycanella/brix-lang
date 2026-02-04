// String Operations Advanced Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, FStringPart, Literal, Program, Stmt};

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

// Helper function to create binary operations
fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Expr {
    Expr::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    }
}

// ==================== FORMAT SPECIFIERS ====================

#[test]
fn test_fstring_octal_format() {
    // f"{64:o}" -> "100" (octal)
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![FStringPart::Expr {
                expr: Box::new(Expr::Literal(Literal::Int(64))),
                format: Some("o".to_string()),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_hex_lowercase() {
    // f"{255:x}" -> "ff"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![FStringPart::Expr {
                expr: Box::new(Expr::Literal(Literal::Int(255))),
                format: Some("x".to_string()),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_hex_uppercase() {
    // f"{255:X}" -> "FF"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![FStringPart::Expr {
                expr: Box::new(Expr::Literal(Literal::Int(255))),
                format: Some("X".to_string()),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_scientific_notation() {
    // f"{1234.5:e}" -> "1.2345e+03"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![FStringPart::Expr {
                expr: Box::new(Expr::Literal(Literal::Float(1234.5))),
                format: Some("e".to_string()),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_scientific_uppercase() {
    // f"{1234.5:E}" -> "1.2345E+03"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![FStringPart::Expr {
                expr: Box::new(Expr::Literal(Literal::Float(1234.5))),
                format: Some("E".to_string()),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_precision_1_decimal() {
    // f"{3.14159:.1f}" -> "3.1"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![FStringPart::Expr {
                expr: Box::new(Expr::Literal(Literal::Float(3.14159))),
                format: Some(".1f".to_string()),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_precision_3_decimals() {
    // f"{3.14159:.3f}" -> "3.142"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![FStringPart::Expr {
                expr: Box::new(Expr::Literal(Literal::Float(3.14159))),
                format: Some(".3f".to_string()),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_precision_zero_decimals() {
    // f"{3.14159:.0f}" -> "3"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![FStringPart::Expr {
                expr: Box::new(Expr::Literal(Literal::Float(3.14159))),
                format: Some(".0f".to_string()),
            }],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_multiple_formats_in_one() {
    // f"Hex: {255:x}, Oct: {64:o}, Sci: {1000.0:e}"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![
                FStringPart::Text("Hex: ".to_string()),
                FStringPart::Expr {
                    expr: Box::new(Expr::Literal(Literal::Int(255))),
                    format: Some("x".to_string()),
                },
                FStringPart::Text(", Oct: ".to_string()),
                FStringPart::Expr {
                    expr: Box::new(Expr::Literal(Literal::Int(64))),
                    format: Some("o".to_string()),
                },
                FStringPart::Text(", Sci: ".to_string()),
                FStringPart::Expr {
                    expr: Box::new(Expr::Literal(Literal::Float(1000.0))),
                    format: Some("e".to_string()),
                },
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_format_with_variable() {
    // var x := 42;
    // f"{x:x}"
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(42)),
                is_const: false,
            },
            Stmt::Expr(Expr::FString {
                parts: vec![FStringPart::Expr {
                    expr: Box::new(Expr::Identifier("x".to_string())),
                    format: Some("x".to_string()),
                }],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== ESCAPE SEQUENCES ====================

#[test]
fn test_string_with_newline() {
    // "Hello\nWorld"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            "Hello\nWorld".to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_with_tab() {
    // "Name:\tValue"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            "Name:\tValue".to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_with_carriage_return() {
    // "Line1\rLine2"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            "Line1\rLine2".to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_with_backslash() {
    // "C:\\Users\\path"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            "C:\\Users\\path".to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_with_quote() {
    // "She said \"Hello\""
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            "She said \"Hello\"".to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_with_multiple_escapes() {
    // "Line1\nLine2\tIndented\\Path"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            "Line1\nLine2\tIndented\\Path".to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_with_newline() {
    // f"Value: {42}\n"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![
                FStringPart::Text("Value: ".to_string()),
                FStringPart::Expr {
                    expr: Box::new(Expr::Literal(Literal::Int(42))),
                    format: None,
                },
                FStringPart::Text("\n".to_string()),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_with_tab() {
    // f"Name:\t{value}"
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "value".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(100)),
                is_const: false,
            },
            Stmt::Expr(Expr::FString {
                parts: vec![
                    FStringPart::Text("Name:\t".to_string()),
                    FStringPart::Expr {
                        expr: Box::new(Expr::Identifier("value".to_string())),
                        format: None,
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_with_backslash() {
    // f"Path: C:\\{folder}"
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "folder".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::String("Users".to_string())),
                is_const: false,
            },
            Stmt::Expr(Expr::FString {
                parts: vec![
                    FStringPart::Text("Path: C:\\".to_string()),
                    FStringPart::Expr {
                        expr: Box::new(Expr::Identifier("folder".to_string())),
                        format: None,
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_empty_string_literal() {
    // ""
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String("".to_string())))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== STRING OPERATIONS ====================

#[test]
fn test_string_concatenation() {
    // "Hello" + " " + "World"
    let program = Program {
        statements: vec![Stmt::Expr(binary(
            BinaryOp::Add,
            binary(
                BinaryOp::Add,
                Expr::Literal(Literal::String("Hello".to_string())),
                Expr::Literal(Literal::String(" ".to_string())),
            ),
            Expr::Literal(Literal::String("World".to_string())),
        ))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_concatenation_with_variable() {
    // var name := "Alice";
    // "Hello, " + name
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "name".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::String("Alice".to_string())),
                is_const: false,
            },
            Stmt::Expr(binary(
                BinaryOp::Add,
                Expr::Literal(Literal::String("Hello, ".to_string())),
                Expr::Identifier("name".to_string()),
            )),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_equality() {
    // "test" == "test"
    let program = Program {
        statements: vec![Stmt::Expr(binary(
            BinaryOp::Eq,
            Expr::Literal(Literal::String("test".to_string())),
            Expr::Literal(Literal::String("test".to_string())),
        ))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_inequality() {
    // "hello" != "world"
    let program = Program {
        statements: vec![Stmt::Expr(binary(
            BinaryOp::NotEq,
            Expr::Literal(Literal::String("hello".to_string())),
            Expr::Literal(Literal::String("world".to_string())),
        ))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_comparison_less_than() {
    // "a" < "b"
    let program = Program {
        statements: vec![Stmt::Expr(binary(
            BinaryOp::Lt,
            Expr::Literal(Literal::String("a".to_string())),
            Expr::Literal(Literal::String("b".to_string())),
        ))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_comparison_greater_than() {
    // "z" > "a"
    let program = Program {
        statements: vec![Stmt::Expr(binary(
            BinaryOp::Gt,
            Expr::Literal(Literal::String("z".to_string())),
            Expr::Literal(Literal::String("a".to_string())),
        ))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_empty_string_concatenation() {
    // "" + "Hello" + ""
    let program = Program {
        statements: vec![Stmt::Expr(binary(
            BinaryOp::Add,
            binary(
                BinaryOp::Add,
                Expr::Literal(Literal::String("".to_string())),
                Expr::Literal(Literal::String("Hello".to_string())),
            ),
            Expr::Literal(Literal::String("".to_string())),
        ))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_long_string_literal() {
    // Very long string (100+ chars)
    let long_str = "This is a very long string that contains more than one hundred characters to test how the compiler handles long string literals in the code generation phase.";
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            long_str.to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_with_special_chars() {
    // String with special characters: !@#$%^&*()
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            "!@#$%^&*()_+-=[]{}|;':,.<>?/~`".to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_with_numbers() {
    // "Value: 123456789"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::String(
            "Value: 123456789".to_string(),
        )))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}


// ==================== STRING EDGE CASES ====================

#[test]
fn test_fstring_with_complex_expression() {
    // f"Result: {2 + 3 * 4}"
    let program = Program {
        statements: vec![Stmt::Expr(Expr::FString {
            parts: vec![
                FStringPart::Text("Result: ".to_string()),
                FStringPart::Expr {
                    expr: Box::new(binary(
                        BinaryOp::Add,
                        Expr::Literal(Literal::Int(2)),
                        binary(
                            BinaryOp::Mul,
                            Expr::Literal(Literal::Int(3)),
                            Expr::Literal(Literal::Int(4)),
                        ),
                    )),
                    format: None,
                },
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_with_function_call() {
    // fn get_value() -> int { return 42; }
    // f"Value: {get_value()}"
    let program = Program {
        statements: vec![
            Stmt::FunctionDef {
                name: "get_value".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::Block(vec![Stmt::Return {
                    values: vec![Expr::Literal(Literal::Int(42))],
                }])),
            },
            Stmt::Expr(Expr::FString {
                parts: vec![
                    FStringPart::Text("Value: ".to_string()),
                    FStringPart::Expr {
                        expr: Box::new(Expr::Call {
                            func: Box::new(Expr::Identifier("get_value".to_string())),
                            args: vec![],
                        }),
                        format: None,
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_with_ternary() {
    // var x := 10;
    // f"Result: {x > 5 ? \"big\" : \"small\"}"
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(10)),
                is_const: false,
            },
            Stmt::Expr(Expr::FString {
                parts: vec![
                    FStringPart::Text("Result: ".to_string()),
                    FStringPart::Expr {
                        expr: Box::new(Expr::Ternary {
                            condition: Box::new(binary(
                                BinaryOp::Gt,
                                Expr::Identifier("x".to_string()),
                                Expr::Literal(Literal::Int(5)),
                            )),
                            then_expr: Box::new(Expr::Literal(Literal::String("big".to_string()))),
                            else_expr: Box::new(Expr::Literal(Literal::String("small".to_string()))),
                        }),
                        format: None,
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_many_interpolations() {
    // f"{a} + {b} = {c}"
    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "a".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(5)),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "b".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(3)),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "c".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(8)),
                is_const: false,
            },
            Stmt::Expr(Expr::FString {
                parts: vec![
                    FStringPart::Expr {
                        expr: Box::new(Expr::Identifier("a".to_string())),
                        format: None,
                    },
                    FStringPart::Text(" + ".to_string()),
                    FStringPart::Expr {
                        expr: Box::new(Expr::Identifier("b".to_string())),
                        format: None,
                    },
                    FStringPart::Text(" = ".to_string()),
                    FStringPart::Expr {
                        expr: Box::new(Expr::Identifier("c".to_string())),
                        format: None,
                    },
                ],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_concatenation_in_variable() {
    // var greeting := "Hello, " + "World!";
    let program = Program {
        statements: vec![Stmt::VariableDecl {
            name: "greeting".to_string(),
            type_hint: None,
            value: binary(
                BinaryOp::Add,
                Expr::Literal(Literal::String("Hello, ".to_string())),
                Expr::Literal(Literal::String("World!".to_string())),
            ),
            is_const: false,
        }],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

