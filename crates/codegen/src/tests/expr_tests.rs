// Expression Codegen Tests
//
// Tests LLVM IR generation for all expression types.

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt, UnaryOp};

fn make_program(stmt: Stmt) -> Program {
    Program {
        statements: vec![stmt],
    }
}

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

// ==================== LITERAL EXPRESSIONS ====================

#[test]
fn test_compile_int_literal() {
    let expr = Expr::Literal(Literal::Int(42));
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_float_literal() {
    let expr = Expr::Literal(Literal::Float(3.14));
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_string_literal() {
    let expr = Expr::Literal(Literal::String("hello world".to_string()));
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    // Should create a global string constant
    assert!(result.is_ok());
}

#[test]
fn test_compile_bool_true() {
    let expr = Expr::Literal(Literal::Bool(true));
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_bool_false() {
    let expr = Expr::Literal(Literal::Bool(false));
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_nil_literal() {
    let expr = Expr::Literal(Literal::Nil);
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_atom_literal() {
    let expr = Expr::Literal(Literal::Atom("ok".to_string()));
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    // Should call atom_intern
    assert!(result.is_ok());
}

// ==================== BINARY OPERATORS ====================

#[test]
fn test_compile_add() {
    let expr = Expr::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::Literal(Literal::Int(1))),
        rhs: Box::new(Expr::Literal(Literal::Int(2))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_sub() {
    let expr = Expr::Binary {
        op: BinaryOp::Sub,
        lhs: Box::new(Expr::Literal(Literal::Int(10))),
        rhs: Box::new(Expr::Literal(Literal::Int(5))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_mul() {
    let expr = Expr::Binary {
        op: BinaryOp::Mul,
        lhs: Box::new(Expr::Literal(Literal::Int(3))),
        rhs: Box::new(Expr::Literal(Literal::Int(4))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_div_int() {
    let expr = Expr::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::Literal(Literal::Int(20))),
        rhs: Box::new(Expr::Literal(Literal::Int(5))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_div_float() {
    let expr = Expr::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::Literal(Literal::Float(10.0))),
        rhs: Box::new(Expr::Literal(Literal::Float(3.0))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_mod() {
    let expr = Expr::Binary {
        op: BinaryOp::Mod,
        lhs: Box::new(Expr::Literal(Literal::Int(10))),
        rhs: Box::new(Expr::Literal(Literal::Int(3))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_pow_int() {
    let expr = Expr::Binary {
        op: BinaryOp::Pow,
        lhs: Box::new(Expr::Literal(Literal::Int(2))),
        rhs: Box::new(Expr::Literal(Literal::Int(8))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    // Should call runtime pow function or use llvm.pow
    assert!(result.is_ok());
}

// ==================== BITWISE OPERATORS ====================

#[test]
fn test_compile_bit_and() {
    let expr = Expr::Binary {
        op: BinaryOp::BitAnd,
        lhs: Box::new(Expr::Literal(Literal::Int(0xFF))),
        rhs: Box::new(Expr::Literal(Literal::Int(0x0F))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_bit_or() {
    let expr = Expr::Binary {
        op: BinaryOp::BitOr,
        lhs: Box::new(Expr::Literal(Literal::Int(0xF0))),
        rhs: Box::new(Expr::Literal(Literal::Int(0x0F))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_bit_xor() {
    let expr = Expr::Binary {
        op: BinaryOp::BitXor,
        lhs: Box::new(Expr::Literal(Literal::Int(0xFF))),
        rhs: Box::new(Expr::Literal(Literal::Int(0xAA))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPARISON OPERATORS ====================

#[test]
fn test_compile_eq() {
    let expr = Expr::Binary {
        op: BinaryOp::Eq,
        lhs: Box::new(Expr::Literal(Literal::Int(5))),
        rhs: Box::new(Expr::Literal(Literal::Int(5))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_not_eq() {
    let expr = Expr::Binary {
        op: BinaryOp::NotEq,
        lhs: Box::new(Expr::Literal(Literal::Int(5))),
        rhs: Box::new(Expr::Literal(Literal::Int(10))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_lt() {
    let expr = Expr::Binary {
        op: BinaryOp::Lt,
        lhs: Box::new(Expr::Literal(Literal::Int(5))),
        rhs: Box::new(Expr::Literal(Literal::Int(10))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_gt() {
    let expr = Expr::Binary {
        op: BinaryOp::Gt,
        lhs: Box::new(Expr::Literal(Literal::Int(10))),
        rhs: Box::new(Expr::Literal(Literal::Int(5))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_lteq() {
    let expr = Expr::Binary {
        op: BinaryOp::LtEq,
        lhs: Box::new(Expr::Literal(Literal::Int(5))),
        rhs: Box::new(Expr::Literal(Literal::Int(10))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_gteq() {
    let expr = Expr::Binary {
        op: BinaryOp::GtEq,
        lhs: Box::new(Expr::Literal(Literal::Int(10))),
        rhs: Box::new(Expr::Literal(Literal::Int(5))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== LOGICAL OPERATORS ====================

#[test]
fn test_compile_logical_and() {
    let expr = Expr::Binary {
        op: BinaryOp::LogicalAnd,
        lhs: Box::new(Expr::Literal(Literal::Bool(true))),
        rhs: Box::new(Expr::Literal(Literal::Bool(false))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    // Should use short-circuit evaluation with branches
    assert!(result.is_ok());
}

#[test]
fn test_compile_logical_or() {
    let expr = Expr::Binary {
        op: BinaryOp::LogicalOr,
        lhs: Box::new(Expr::Literal(Literal::Bool(false))),
        rhs: Box::new(Expr::Literal(Literal::Bool(true))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    // Should use short-circuit evaluation with branches
    assert!(result.is_ok());
}

// ==================== UNARY OPERATORS ====================

#[test]
fn test_compile_not() {
    let expr = Expr::Unary {
        op: UnaryOp::Not,
        expr: Box::new(Expr::Literal(Literal::Bool(true))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_negate() {
    let expr = Expr::Unary {
        op: UnaryOp::Negate,
        expr: Box::new(Expr::Literal(Literal::Int(42))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== INCREMENT/DECREMENT ====================

#[test]
fn test_compile_increment_prefix() {
    // ++x
    let stmt = Stmt::Block(vec![
        Stmt::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Int(5)),
            is_const: false,
        },
        Stmt::Expr(Expr::Increment {
            expr: Box::new(Expr::Identifier("x".to_string())),
            is_prefix: true,
        }),
    ]);
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_increment_postfix() {
    // x++
    let stmt = Stmt::Block(vec![
        Stmt::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::Literal(Literal::Int(5)),
            is_const: false,
        },
        Stmt::Expr(Expr::Increment {
            expr: Box::new(Expr::Identifier("x".to_string())),
            is_prefix: false,
        }),
    ]);
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TERNARY OPERATOR ====================

#[test]
fn test_compile_ternary() {
    // x > 5 ? 10 : 20
    let expr = Expr::Ternary {
        condition: Box::new(Expr::Binary {
            op: BinaryOp::Gt,
            lhs: Box::new(Expr::Identifier("x".to_string())),
            rhs: Box::new(Expr::Literal(Literal::Int(5))),
        }),
        then_expr: Box::new(Expr::Literal(Literal::Int(10))),
        else_expr: Box::new(Expr::Literal(Literal::Int(20))),
    };

    let program = Program {
        statements: vec![
            Stmt::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::Literal(Literal::Int(7)),
                is_const: false,
            },
            Stmt::Expr(expr),
        ],
    };

    let result = compile_program(program);
    // Should have basic blocks for then/else branches
    assert!(result.is_ok());
}

// ==================== ARRAY EXPRESSIONS ====================

#[test]
fn test_compile_empty_array() {
    let expr = Expr::Array(vec![]);
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_array_int() {
    let expr = Expr::Array(vec![
        Expr::Literal(Literal::Int(1)),
        Expr::Literal(Literal::Int(2)),
        Expr::Literal(Literal::Int(3)),
    ]);
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    // Should allocate IntMatrix
    assert!(result.is_ok());
}

#[test]
fn test_compile_array_float() {
    let expr = Expr::Array(vec![
        Expr::Literal(Literal::Float(1.0)),
        Expr::Literal(Literal::Float(2.0)),
        Expr::Literal(Literal::Float(3.0)),
    ]);
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    // Should allocate Matrix
    assert!(result.is_ok());
}

// ==================== COMPLEX NESTED EXPRESSIONS ====================

#[test]
fn test_compile_complex_arithmetic() {
    // (1 + 2) * (3 - 4) / 5
    let expr = Expr::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::Binary {
                op: BinaryOp::Add,
                lhs: Box::new(Expr::Literal(Literal::Int(1))),
                rhs: Box::new(Expr::Literal(Literal::Int(2))),
            }),
            rhs: Box::new(Expr::Binary {
                op: BinaryOp::Sub,
                lhs: Box::new(Expr::Literal(Literal::Int(3))),
                rhs: Box::new(Expr::Literal(Literal::Int(4))),
            }),
        }),
        rhs: Box::new(Expr::Literal(Literal::Int(5))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ATOMS (v1.1) ====================

#[test]
fn test_atom_literal() {
    let expr = Expr::Literal(Literal::Atom("ok".to_string()));
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_atom_comparison() {
    let expr = Expr::Binary {
        op: BinaryOp::Eq,
        lhs: Box::new(Expr::Literal(Literal::Atom("ok".to_string()))),
        rhs: Box::new(Expr::Literal(Literal::Atom("ok".to_string()))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_atom_different_comparison() {
    let expr = Expr::Binary {
        op: BinaryOp::NotEq,
        lhs: Box::new(Expr::Literal(Literal::Atom("ok".to_string()))),
        rhs: Box::new(Expr::Literal(Literal::Atom("error".to_string()))),
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== NIL AND ERROR (v1.0) ====================

#[test]
fn test_nil_literal() {
    let expr = Expr::Identifier("nil".to_string());
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_error_creation() {
    let expr = Expr::Call {
        func: Box::new(Expr::Identifier("error".to_string())),
        args: vec![Expr::Literal(Literal::String("something failed".to_string()))],
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== LIST COMPREHENSIONS (v0.9) ====================

#[test]
fn test_list_comprehension_basic() {
    let expr = Expr::ListComprehension {
        expr: Box::new(Expr::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::Identifier("x".to_string())),
            rhs: Box::new(Expr::Literal(Literal::Int(2))),
        }),
        generators: vec![parser::ast::ComprehensionGen {
            var_names: vec!["x".to_string()],
            iterable: Box::new(Expr::Array(vec![
                Expr::Literal(Literal::Float(1.0)),
                Expr::Literal(Literal::Float(2.0)),
                Expr::Literal(Literal::Float(3.0)),
            ])),
            conditions: vec![],
        }],
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_with_filter() {
    let expr = Expr::ListComprehension {
        expr: Box::new(Expr::Identifier("x".to_string())),
        generators: vec![parser::ast::ComprehensionGen {
            var_names: vec!["x".to_string()],
            iterable: Box::new(Expr::Array(vec![
                Expr::Literal(Literal::Float(1.0)),
                Expr::Literal(Literal::Float(2.0)),
                Expr::Literal(Literal::Float(3.0)),
                Expr::Literal(Literal::Float(4.0)),
            ])),
            conditions: vec![Expr::Binary {
                op: BinaryOp::Gt,
                lhs: Box::new(Expr::Identifier("x".to_string())),
                rhs: Box::new(Expr::Literal(Literal::Float(2.0))),
            }],
        }],
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_nested() {
    let expr = Expr::ListComprehension {
        expr: Box::new(Expr::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::Identifier("x".to_string())),
            rhs: Box::new(Expr::Identifier("y".to_string())),
        }),
        generators: vec![
            parser::ast::ComprehensionGen {
                var_names: vec!["x".to_string()],
                iterable: Box::new(Expr::Array(vec![
                    Expr::Literal(Literal::Float(1.0)),
                    Expr::Literal(Literal::Float(2.0)),
                ])),
                conditions: vec![],
            },
            parser::ast::ComprehensionGen {
                var_names: vec!["y".to_string()],
                iterable: Box::new(Expr::Array(vec![
                    Expr::Literal(Literal::Float(10.0)),
                    Expr::Literal(Literal::Float(20.0)),
                ])),
                conditions: vec![],
            },
        ],
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_function() {
    let expr = Expr::Call {
        func: Box::new(Expr::Identifier("zip".to_string())),
        args: vec![
            Expr::Array(vec![
                Expr::Literal(Literal::Float(1.0)),
                Expr::Literal(Literal::Float(2.0)),
            ]),
            Expr::Array(vec![
                Expr::Literal(Literal::Float(10.0)),
                Expr::Literal(Literal::Float(20.0)),
            ]),
        ],
    };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== F-STRINGS WITH FORMAT SPECIFIERS ====================

#[test]
fn test_fstring_basic() {
    let expr = Expr::FString { parts: vec![
        parser::ast::FStringPart::Text("Value: ".to_string()),
        parser::ast::FStringPart::Expr {
            expr: Box::new(Expr::Literal(Literal::Int(42))),
            format: None,
        },
    ] };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_with_hex_format() {
    let expr = Expr::FString { parts: vec![
        parser::ast::FStringPart::Text("Hex: ".to_string()),
        parser::ast::FStringPart::Expr {
            expr: Box::new(Expr::Literal(Literal::Int(255))),
            format: Some("x".to_string()),
        },
    ] };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_with_precision_format() {
    let expr = Expr::FString { parts: vec![
        parser::ast::FStringPart::Text("Pi: ".to_string()),
        parser::ast::FStringPart::Expr {
            expr: Box::new(Expr::Literal(Literal::Float(3.14159))),
            format: Some(".2f".to_string()),
        },
    ] };
    let program = make_program(Stmt::Expr(expr));
    let result = compile_program(program);
    assert!(result.is_ok());
}
