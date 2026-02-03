// Built-in Function Codegen Tests

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
fn test_typeof_builtin() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("typeof".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("typeof") || ir.contains("call"));
}

#[test]
fn test_int_conversion() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("int".to_string())),
            args: vec![Expr::Literal(Literal::Float(3.14))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_conversion() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("float".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_string_conversion() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("string".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_print_builtin() {
    let program = Program {
        statements: vec![Stmt::Print {
            expr: Expr::Literal(Literal::String("hello".to_string())),
        }],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("printf") || ir.contains("call"));
}

#[test]
fn test_println_builtin() {
    let program = Program {
        statements: vec![Stmt::Println {
            expr: Expr::Literal(Literal::String("hello".to_string())),
        }],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("printf") || ir.contains("call"));
}

#[test]
fn test_bool_conversion() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("bool".to_string())),
            args: vec![Expr::Literal(Literal::Int(1))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TYPE CHECKING FUNCTIONS (v1.1) ====================

#[test]
fn test_is_nil() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_nil".to_string())),
            args: vec![Expr::Identifier("nil".to_string())],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_atom() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_atom".to_string())),
            args: vec![Expr::Literal(Literal::Atom("ok".to_string()))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_boolean() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_boolean".to_string())),
            args: vec![Expr::Literal(Literal::Int(1))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_number() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_number".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_integer() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_integer".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_float() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_float".to_string())),
            args: vec![Expr::Literal(Literal::Float(3.14))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_string() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_string".to_string())),
            args: vec![Expr::Literal(Literal::String("hello".to_string()))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_list() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_list".to_string())),
            args: vec![Expr::Array(vec![
                Expr::Literal(Literal::Int(1)),
                Expr::Literal(Literal::Int(2)),
            ])],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_tuple() {
    // Note: Tuples don't have direct Expr variant, they're created via function returns
    // This tests that is_tuple compiles correctly (will return 0 for non-tuple)
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_tuple".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_is_function() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("is_function".to_string())),
            args: vec![Expr::Literal(Literal::Int(42))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== STRING FUNCTIONS (v1.1) ====================

#[test]
fn test_uppercase() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("uppercase".to_string())),
            args: vec![Expr::Literal(Literal::String("hello".to_string()))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_lowercase() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("lowercase".to_string())),
            args: vec![Expr::Literal(Literal::String("HELLO".to_string()))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_capitalize() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("capitalize".to_string())),
            args: vec![Expr::Literal(Literal::String("hello world".to_string()))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_byte_size() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("byte_size".to_string())),
            args: vec![Expr::Literal(Literal::String("Brix".to_string()))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_length() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("length".to_string())),
            args: vec![Expr::Literal(Literal::String("Hello".to_string()))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_replace() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("replace".to_string())),
            args: vec![
                Expr::Literal(Literal::String("hello world".to_string())),
                Expr::Literal(Literal::String("world".to_string())),
                Expr::Literal(Literal::String("Brix".to_string())),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_replace_all() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("replace_all".to_string())),
            args: vec![
                Expr::Literal(Literal::String("hi hi hi".to_string())),
                Expr::Literal(Literal::String("hi".to_string())),
                Expr::Literal(Literal::String("bye".to_string())),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATRIX CONSTRUCTORS ====================

#[test]
fn test_zeros_1d() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("zeros".to_string())),
            args: vec![Expr::Literal(Literal::Int(5))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
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
fn test_izeros_1d() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("izeros".to_string())),
            args: vec![Expr::Literal(Literal::Int(5))],
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
                Expr::Literal(Literal::Int(3)),
                Expr::Literal(Literal::Int(4)),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MATH LIBRARY FUNCTIONS (v0.7) ====================

#[test]
fn test_math_sin() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "sin".to_string(),
            }),
            args: vec![Expr::Literal(Literal::Float(3.14))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_cos() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "cos".to_string(),
            }),
            args: vec![Expr::Literal(Literal::Float(0.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_sqrt() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "sqrt".to_string(),
            }),
            args: vec![Expr::Literal(Literal::Float(16.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_exp() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "exp".to_string(),
            }),
            args: vec![Expr::Literal(Literal::Float(1.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_log() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "log".to_string(),
            }),
            args: vec![Expr::Literal(Literal::Float(2.718))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_abs() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "abs".to_string(),
            }),
            args: vec![Expr::Literal(Literal::Float(-5.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_floor() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "floor".to_string(),
            }),
            args: vec![Expr::Literal(Literal::Float(3.7))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_ceil() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "ceil".to_string(),
            }),
            args: vec![Expr::Literal(Literal::Float(3.2))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_pi_constant() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::FieldAccess {
            target: Box::new(Expr::Identifier("math".to_string())),
            field: "pi".to_string(),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_e_constant() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::FieldAccess {
            target: Box::new(Expr::Identifier("math".to_string())),
            field: "e".to_string(),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_sum() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "sum".to_string(),
            }),
            args: vec![Expr::Array(vec![
                Expr::Literal(Literal::Float(1.0)),
                Expr::Literal(Literal::Float(2.0)),
                Expr::Literal(Literal::Float(3.0)),
            ])],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_math_mean() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: None,
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("math".to_string())),
                field: "mean".to_string(),
            }),
            args: vec![Expr::Array(vec![
                Expr::Literal(Literal::Float(1.0)),
                Expr::Literal(Literal::Float(2.0)),
                Expr::Literal(Literal::Float(3.0)),
            ])],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_import_with_alias() {
    let program = Program {
        statements: vec![Stmt::Import {
            module: "math".to_string(),
            alias: Some("m".to_string()),
        }, Stmt::Expr(Expr::Call {
            func: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Identifier("m".to_string())),
                field: "sqrt".to_string(),
            }),
            args: vec![Expr::Literal(Literal::Float(4.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
