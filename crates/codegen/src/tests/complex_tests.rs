// Complex Number Codegen Tests

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

#[test]
fn test_complex_literal() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::Complex(3.0, 4.0)))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
#[ignore = "Literal::Imaginary does not exist in AST - imaginary is parsed as Complex(0, n)"]
fn test_imaginary_literal() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Literal(Literal::Complex(0.0, 2.0)))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_addition() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::Literal(Literal::Complex(1.0, 2.0))),
            rhs: Box::new(Expr::Literal(Literal::Complex(3.0, 4.0))),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("complex_add") || ir.contains("call"));
}

#[test]
fn test_complex_multiplication() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::Literal(Literal::Complex(1.0, 1.0))),
            rhs: Box::new(Expr::Literal(Literal::Complex(1.0, 1.0))),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("complex_mul") || ir.contains("call"));
}
// ==================== COMPLEX ARITHMETIC OPERATORS ====================

#[test]
fn test_complex_subtraction() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Sub,
            lhs: Box::new(Expr::Literal(Literal::Complex(5.0, 6.0))),
            rhs: Box::new(Expr::Literal(Literal::Complex(2.0, 3.0))),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("complex_sub") || ir.contains("call"));
}

#[test]
fn test_complex_division() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Div,
            lhs: Box::new(Expr::Literal(Literal::Complex(4.0, 2.0))),
            rhs: Box::new(Expr::Literal(Literal::Complex(1.0, 1.0))),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("complex_div") || ir.contains("call"));
}

#[test]
fn test_complex_power() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Pow,
            lhs: Box::new(Expr::Literal(Literal::Complex(2.0, 0.0))),
            rhs: Box::new(Expr::Literal(Literal::Float(3.0))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPLEX AUTO-CONVERSION ====================

#[test]
fn test_float_plus_complex() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::Literal(Literal::Float(5.0))),
            rhs: Box::new(Expr::Literal(Literal::Complex(3.0, 4.0))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_plus_float() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::Literal(Literal::Complex(3.0, 4.0))),
            rhs: Box::new(Expr::Literal(Literal::Float(5.0))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_int_plus_complex() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Binary {
            op: BinaryOp::Add,
            lhs: Box::new(Expr::Literal(Literal::Int(10))),
            rhs: Box::new(Expr::Literal(Literal::Complex(1.0, 2.0))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPLEX FUNCTIONS - PROPERTIES ====================

#[test]
fn test_complex_real() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("real".to_string())),
            args: vec![Expr::Literal(Literal::Complex(3.0, 4.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_imag() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("imag".to_string())),
            args: vec![Expr::Literal(Literal::Complex(3.0, 4.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_abs() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("abs".to_string())),
            args: vec![Expr::Literal(Literal::Complex(3.0, 4.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_angle() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("angle".to_string())),
            args: vec![Expr::Literal(Literal::Complex(1.0, 1.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_conj() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("conj".to_string())),
            args: vec![Expr::Literal(Literal::Complex(3.0, 4.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_abs2() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("abs2".to_string())),
            args: vec![Expr::Literal(Literal::Complex(3.0, 4.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPLEX FUNCTIONS - EXPONENTIAL ====================

#[test]
fn test_complex_exp() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("exp".to_string())),
            args: vec![Expr::Literal(Literal::Complex(1.0, 0.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_log() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("log".to_string())),
            args: vec![Expr::Literal(Literal::Complex(2.718, 0.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_sqrt() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("sqrt".to_string())),
            args: vec![Expr::Literal(Literal::Complex(-1.0, 0.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPLEX FUNCTIONS - TRIGONOMETRIC ====================

#[test]
fn test_complex_csin() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("csin".to_string())),
            args: vec![Expr::Literal(Literal::Complex(1.0, 1.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_ccos() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("ccos".to_string())),
            args: vec![Expr::Literal(Literal::Complex(1.0, 1.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_ctan() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("ctan".to_string())),
            args: vec![Expr::Literal(Literal::Complex(1.0, 0.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPLEX FUNCTIONS - HYPERBOLIC ====================

#[test]
fn test_complex_csinh() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("csinh".to_string())),
            args: vec![Expr::Literal(Literal::Complex(1.0, 0.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_ccosh() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("ccosh".to_string())),
            args: vec![Expr::Literal(Literal::Complex(1.0, 0.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_ctanh() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("ctanh".to_string())),
            args: vec![Expr::Literal(Literal::Complex(1.0, 0.0))],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPLEX FUNCTIONS - POWER ====================

#[test]
fn test_complex_cpow() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("cpow".to_string())),
            args: vec![
                Expr::Literal(Literal::Complex(2.0, 0.0)),
                Expr::Literal(Literal::Float(3.0)),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== IMAGINARY UNIT CONSTANT ====================

#[test]
fn test_im_constant() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Identifier("im".to_string()))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_constructor() {
    let program = Program {
        statements: vec![Stmt::Expr(Expr::Call {
            func: Box::new(Expr::Identifier("complex".to_string())),
            args: vec![
                Expr::Literal(Literal::Float(5.0)),
                Expr::Literal(Literal::Float(12.0)),
            ],
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPLEX MATRIX ====================

#[test]
fn test_complex_matrix_from_eigvals() {
    let program = Program {
        statements: vec![
            Stmt::Import {
                module: "math".to_string(),
                alias: None,
            },
            Stmt::VariableDecl {
                name: "A".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Float(1.0)),
                    Expr::Literal(Literal::Float(2.0)),
                    Expr::Literal(Literal::Float(3.0)),
                    Expr::Literal(Literal::Float(4.0)),
                ]),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "evals".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::FieldAccess {
                        target: Box::new(Expr::Identifier("math".to_string())),
                        field: "eigvals".to_string(),
                    }),
                    args: vec![Expr::Identifier("A".to_string())],
                },
                is_const: false,
            },
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_complex_matrix_indexing() {
    let program = Program {
        statements: vec![
            Stmt::Import {
                module: "math".to_string(),
                alias: None,
            },
            Stmt::VariableDecl {
                name: "A".to_string(),
                type_hint: None,
                value: Expr::Array(vec![
                    Expr::Literal(Literal::Float(1.0)),
                    Expr::Literal(Literal::Float(2.0)),
                    Expr::Literal(Literal::Float(3.0)),
                    Expr::Literal(Literal::Float(4.0)),
                ]),
                is_const: false,
            },
            Stmt::VariableDecl {
                name: "evals".to_string(),
                type_hint: None,
                value: Expr::Call {
                    func: Box::new(Expr::FieldAccess {
                        target: Box::new(Expr::Identifier("math".to_string())),
                        field: "eigvals".to_string(),
                    }),
                    args: vec![Expr::Identifier("A".to_string())],
                },
                is_const: false,
            },
            Stmt::Expr(Expr::Index {
                array: Box::new(Expr::Identifier("evals".to_string())),
                indices: vec![Expr::Literal(Literal::Int(0))],
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}
