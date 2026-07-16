// Built-in Function Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Expr, ExprKind, Literal, Program, Stmt, StmtKind};

fn compile_program(program: Program) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(
            &context,
            &builder,
            &module,
            "test.bx".to_string(),
            "".to_string(),
        );
        compiler.compile_program(&program);
        module.print_to_string().to_string()
    });
    match result {
        Ok(ir) => Ok(ir),
        Err(_) => Err("Compilation panicked".to_string()),
    }
}

#[test]
fn test_typeof_builtin() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        })))],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("typeof") || ir.contains("call"));
}

#[test]
fn test_int_conversion() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("int".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_conversion() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("float".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_conversion() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("string".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_print_builtin() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Print {
            expr: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("printf") || ir.contains("call"));
}

#[test]
fn test_println_builtin() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Println {
            expr: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("printf") || ir.contains("call"));
}

#[test]
fn test_bool_conversion() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("bool".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TYPE CHECKING FUNCTIONS (v1.1) ====================

#[test]
fn test_is_nil() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_nil".to_string()))),
            args: vec![Expr::dummy(ExprKind::Identifier("nil".to_string()))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_atom() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_atom".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Atom(
                "ok".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_boolean() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_boolean".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_number() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_number".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_integer() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_integer".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_float() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_float".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_string() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_string".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "hello".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_list() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_list".to_string()))),
            args: vec![Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            ]))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_tuple() {
    // Note: Tuples don't have direct Expr variant, they're created via function returns
    // This tests that is_tuple compiles correctly (will return 0 for non-tuple)
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_tuple".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_function() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_function".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== STRING FUNCTIONS (v1.1) ====================

#[test]
fn test_uppercase() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("uppercase".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "hello".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_lowercase() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("lowercase".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "HELLO".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_capitalize() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("capitalize".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "hello world".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_byte_size() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("byte_size".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "Brix".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_length() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("length".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "Hello".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_replace() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("replace".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::String(
                    "hello world".to_string(),
                ))),
                Expr::dummy(ExprKind::Literal(Literal::String("world".to_string()))),
                Expr::dummy(ExprKind::Literal(Literal::String("Brix".to_string()))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_replace_all() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("replace_all".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::String("hi hi hi".to_string()))),
                Expr::dummy(ExprKind::Literal(Literal::String("hi".to_string()))),
                Expr::dummy(ExprKind::Literal(Literal::String("bye".to_string()))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATRIX CONSTRUCTORS ====================

#[test]
fn test_zeros_1d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
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
fn test_izeros_1d() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("izeros".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
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
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                Expr::dummy(ExprKind::Literal(Literal::Int(4))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATH LIBRARY FUNCTIONS (v0.7) ====================

#[test]
fn test_math_sin() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "sin".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_cos() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "cos".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_sqrt() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "sqrt".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(16.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_exp() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "exp".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_log() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "log".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(2.718)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_abs() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "abs".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(-5.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_floor() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "floor".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.7)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_ceil() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "ceil".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.2)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_pi_constant() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                field: "pi".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_e_constant() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                field: "e".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_sum() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "sum".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                ]))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_mean() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "mean".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                ]))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_import_with_alias() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: Some("m".to_string()),
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("m".to_string()))),
                    field: "sqrt".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(4.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATH LIBRARY - TRIGONOMETRIC FUNCTIONS ====================

#[test]
fn test_math_tan() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "tan".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(0.785)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_asin() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "asin".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(0.5)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_acos() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "acos".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(0.5)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_atan() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "atan".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_atan2() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "atan2".to_string(),
                })),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATH LIBRARY - HYPERBOLIC FUNCTIONS ====================

#[test]
fn test_math_sinh() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "sinh".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_cosh() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "cosh".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_tanh() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "tanh".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATH LIBRARY - LOGARITHMIC FUNCTIONS ====================

#[test]
fn test_math_log10() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "log10".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(100.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_log2() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "log2".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(8.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_cbrt() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "cbrt".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(27.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATH LIBRARY - ROUNDING FUNCTIONS ====================

#[test]
fn test_math_round() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "round".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.6)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATH LIBRARY - UTILITY FUNCTIONS ====================

#[test]
fn test_math_mod() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "mod".to_string(),
                })),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(5.5))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_min() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "min".to_string(),
                })),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(7.0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_max() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "max".to_string(),
                })),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(7.0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATH LIBRARY - CONSTANTS ====================

#[test]
fn test_math_tau_constant() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                field: "tau".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_phi_constant() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                field: "phi".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_sqrt2_constant() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                field: "sqrt2".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_ln2_constant() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                field: "ln2".to_string(),
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ---- Physical constants (v1.8 Grupo A) ----

/// Helper: compile `import math` + a field access on `math.<field>`.
fn compile_math_constant_access(field: &str) -> bool {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::FieldAccess {
                target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                field: field.to_string(),
            }))),
        ],
    };
    compile_program(program).is_ok()
}

#[test]
fn test_math_c_light_constant() {
    // Plain large value (299792458.0)
    assert!(compile_math_constant_access("c_light"));
}

#[test]
fn test_math_g_earth_constant() {
    // Small decimal value (9.80665)
    assert!(compile_math_constant_access("g_earth"));
}

#[test]
fn test_math_avogadro_constant() {
    // Large scientific notation (6.02214076e23)
    assert!(compile_math_constant_access("avogadro"));
}

#[test]
fn test_math_h_planck_constant() {
    // Small scientific notation (6.62607015e-34)
    assert!(compile_math_constant_access("h_planck"));
}

// ==================== MATH LIBRARY - STATISTICS ====================

#[test]
fn test_math_median() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "median".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(4.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(5.0))),
                ]))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_std() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "std".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                ]))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_var() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "var".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                ]))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATH LIBRARY - LINEAR ALGEBRA ====================

#[test]
fn test_math_det() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "det".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ],
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_inv() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "inv".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_tr() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "tr".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                    ],
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_eigvals() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "eigvals".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_lu() {
    // math.lu(zeros(2,2)) compiles: declares math_lu, unpacks LUResult*,
    // and assembles a Tuple(Matrix, Matrix, IntMatrix) value (v1.8 Grupo B).
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "lu".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_qr() {
    // math.qr(zeros(2,2)) compiles: declares math_qr, unpacks the QRResult*,
    // and assembles a Tuple(Matrix, Matrix) value (v1.8 Grupo B).
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "qr".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_svd() {
    // math.svd(zeros(2,2)) compiles: declares math_svd, unpacks the SVDResult*,
    // and assembles a Tuple(Matrix, Matrix, Matrix) value (v1.8 Grupo B).
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "svd".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// Helper: `import math` + a call `math.<field>(zeros(2,2)[, extra])`.
fn compile_math_linalg_call(field: &str, extra_args: Vec<Expr>) -> bool {
    let mut call_args = vec![Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
        args: vec![
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
        ],
    })];
    call_args.extend(extra_args);
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: field.to_string(),
                })),
                args: call_args,
            }))),
        ],
    };
    compile_program(program).is_ok()
}

#[test]
fn test_math_cholesky() {
    assert!(compile_math_linalg_call("cholesky", vec![]));
}

#[test]
fn test_math_solve() {
    // solve(A, b) — second matrix arg (reuse zeros(2,2) as the RHS).
    let b = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
        args: vec![
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            Expr::dummy(ExprKind::Literal(Literal::Int(2))),
        ],
    });
    assert!(compile_math_linalg_call("solve", vec![b]));
}

#[test]
fn test_math_norm() {
    assert!(compile_math_linalg_call("norm", vec![]));
}

#[test]
fn test_math_norm_mat() {
    // Default (no code) and explicit int code both compile.
    assert!(compile_math_linalg_call("norm_mat", vec![]));
    assert!(compile_math_linalg_call(
        "norm_mat",
        vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))]
    ));
}

#[test]
fn test_math_eigvecs() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "eigvecs".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ],
                })],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_eye() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "eye".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(5)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== INPUT FUNCTIONS ====================

#[test]
fn test_input_int() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("input".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "int".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_input_float() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("input".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "float".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_input_string() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("input".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "string".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== EDGE CASES - TYPE CONVERSIONS ====================

#[test]
fn test_int_conversion_from_string() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("int".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "123".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_conversion_from_string() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("float".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "3.14".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_bool_conversion_from_int_zero() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("bool".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_bool_conversion_from_int_nonzero() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("bool".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_conversion_from_float() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("string".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_typeof_on_different_types() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
            }))),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))],
            }))),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::Identifier("typeof".to_string()))),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                    "hello".to_string(),
                )))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== EDGE CASES - MATH WITH SPECIAL VALUES ====================

#[test]
fn test_math_abs_negative() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "abs".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(-10.5)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_sqrt_zero() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "sqrt".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_log_one() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "log".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(1.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ADDITIONAL MATH EDGE CASES ====================

#[test]
fn test_math_pow_zero_exponent() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "pow".to_string(),
                })),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(5.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(0.0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_pow_negative_base() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "pow".to_string(),
                })),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(-2.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_exp_zero() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "exp".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_sin_zero() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "sin".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_cos_zero() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "cos".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_tan_zero() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "tan".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(0.0)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_floor_negative() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "floor".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(-3.7)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_ceil_negative() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "ceil".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(-3.2)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_round_negative() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "round".to_string(),
                })),
                args: vec![Expr::dummy(ExprKind::Literal(Literal::Float(-3.5)))],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_min_negative_values() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "min".to_string(),
                })),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(-10.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(-5.0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_max_negative_values() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
                func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(Expr::dummy(ExprKind::Identifier("math".to_string()))),
                    field: "max".to_string(),
                })),
                args: vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(-10.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(-5.0))),
                ],
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== STRING FUNCTIONS ====================

#[test]
fn test_string_uppercase() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("uppercase".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "hello".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_lowercase() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("lowercase".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "WORLD".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_capitalize() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("capitalize".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String(
                "hello".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_replace() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("replace".to_string()))),
            args: vec![
                Expr::dummy(ExprKind::Literal(Literal::String(
                    "hello world".to_string(),
                ))),
                Expr::dummy(ExprKind::Literal(Literal::String("world".to_string()))),
                Expr::dummy(ExprKind::Literal(Literal::String("Brix".to_string()))),
            ],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TYPE CHECKING FUNCTIONS ====================

#[test]
fn test_is_nil_function() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_nil".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Nil))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_atom_function() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_atom".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Atom(
                "ok".to_string(),
            )))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_boolean_function() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_boolean".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Bool(true)))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== VECTOR<T> (v1.8 Grupo C, Phase 1) ====================

/// True iff the program compiles with no CodegenError and no panic. Unlike the
/// weaker `compile_program` helper above, this surfaces CodegenErrors so that
/// negative tests (type errors, unsupported types) are meaningful.
fn vector_compiles(program: Program) -> bool {
    std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(
            &context,
            &builder,
            &module,
            "test.bx".to_string(),
            "".to_string(),
        );
        compiler.compile_program(&program).is_ok()
    })
    .unwrap_or(false)
}

fn vector_new_expr(elem: &str) -> Expr {
    Expr::dummy(ExprKind::GenericCall {
        func: Box::new(Expr::dummy(ExprKind::Identifier("Vector".to_string()))),
        type_args: vec![elem.to_string()],
        args: vec![],
    })
}

fn vec_method_stmt(recv: &str, method: &str, args: Vec<Expr>) -> Stmt {
    Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(Expr::dummy(ExprKind::Identifier(recv.to_string()))),
            field: method.to_string(),
        })),
        args,
    })))
}

fn vec_decl(name: &str, type_hint: Option<&str>, elem: &str) -> Stmt {
    Stmt::dummy(StmtKind::VariableDecl {
        name: name.to_string(),
        type_hint: type_hint.map(|s| s.to_string()),
        value: vector_new_expr(elem),
        is_const: false,
    })
}

#[test]
fn test_vector_int_new_push_get_len() {
    // var v := Vector<int>(); v.push(10); v.get(0); v.len() — all compile.
    let program = Program {
        statements: vec![
            vec_decl("v", None, "int"),
            vec_method_stmt(
                "v",
                "push",
                vec![Expr::dummy(ExprKind::Literal(Literal::Int(10)))],
            ),
            vec_method_stmt(
                "v",
                "get",
                vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
            ),
            vec_method_stmt("v", "len", vec![]),
        ],
    };
    assert!(vector_compiles(program));
}

#[test]
fn test_vector_push_type_error() {
    // v.push("x") on Vector<int> must fail to compile.
    let program = Program {
        statements: vec![
            vec_decl("v", None, "int"),
            vec_method_stmt(
                "v",
                "push",
                vec![Expr::dummy(ExprKind::Literal(Literal::String(
                    "x".to_string(),
                )))],
            ),
        ],
    };
    assert!(!vector_compiles(program));
}

#[test]
fn test_vector_float_rejected_phase1() {
    // Vector<float>() is not enabled until Grupo C Phase 3.
    let program = Program {
        statements: vec![vec_decl("v", None, "float")],
    };
    assert!(!vector_compiles(program));
}

#[test]
fn test_vector_annotation_ok() {
    // var v: Vector<int> = Vector<int>() compiles.
    let program = Program {
        statements: vec![vec_decl("v", Some("Vector<int>"), "int")],
    };
    assert!(vector_compiles(program));
}

#[test]
fn test_vector_matrix_annotation_rejected() {
    // var v: Vector<Matrix> = Vector<int>() must fail (invalid element type).
    let program = Program {
        statements: vec![vec_decl("v", Some("Vector<Matrix>"), "int")],
    };
    assert!(!vector_compiles(program));
}
