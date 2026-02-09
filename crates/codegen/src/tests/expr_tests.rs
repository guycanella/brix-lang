// Expression Codegen Tests
//
// Tests LLVM IR generation for all expression types.

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, Literal, Program, Stmt, UnaryOp, ExprKind, StmtKind};

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

        let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());

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
    let expr = Expr::dummy(ExprKind::Literal(Literal::Int(42)));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_float_literal() {
    let expr = Expr::dummy(ExprKind::Literal(Literal::Float(3.14)));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_string_literal() {
    let expr = Expr::dummy(ExprKind::Literal(Literal::String("hello world".to_string())));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    // Should create a global string constant
    assert!(result.is_ok());
}

#[test]
fn test_compile_bool_true() {
    let expr = Expr::dummy(ExprKind::Literal(Literal::Bool(true)));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_bool_false() {
    let expr = Expr::dummy(ExprKind::Literal(Literal::Bool(false)));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_nil_literal() {
    let expr = Expr::dummy(ExprKind::Literal(Literal::Nil));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_atom_literal() {
    let expr = Expr::dummy(ExprKind::Literal(Literal::Atom("ok".to_string())));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    // Should call atom_intern
    assert!(result.is_ok());
}

// ==================== BINARY OPERATORS ====================

#[test]
fn test_compile_add() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Add,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_sub() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Sub,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_mul() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Mul,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_div_int() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(20)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_div_float() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(10.0)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.0)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_mod() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Mod,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_pow_int() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Pow,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(8)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    // Should call runtime pow function or use llvm.pow
    assert!(result.is_ok());
}

// ==================== BITWISE OPERATORS ====================

#[test]
fn test_compile_bit_and() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::BitAnd,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0xFF)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0x0F)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_bit_or() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::BitOr,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0xF0)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0x0F)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_bit_xor() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::BitXor,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0xFF)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0xAA)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPARISON OPERATORS ====================

#[test]
fn test_compile_eq() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Eq,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_not_eq() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::NotEq,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_lt() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Lt,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_gt() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Gt,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_lteq() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::LtEq,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_gteq() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::GtEq,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== LOGICAL OPERATORS ====================

#[test]
fn test_compile_logical_and() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::LogicalAnd,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(false)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    // Should use short-circuit evaluation with branches
    assert!(result.is_ok());
}

#[test]
fn test_compile_logical_or() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::LogicalOr,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(false)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    // Should use short-circuit evaluation with branches
    assert!(result.is_ok());
}

// ==================== UNARY OPERATORS ====================

#[test]
fn test_compile_not() {
    let expr = Expr::dummy(ExprKind::Unary {
        op: UnaryOp::Not,
        expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_negate() {
    let expr = Expr::dummy(ExprKind::Unary {
        op: UnaryOp::Negate,
        expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(42)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== INCREMENT/DECREMENT ====================

#[test]
fn test_compile_increment_prefix() {
    // ++x
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Increment {
            expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            is_prefix: true,
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_increment_postfix() {
    // x++
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Increment {
            expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            is_prefix: false,
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TERNARY OPERATOR ====================

#[test]
fn test_compile_ternary() {
    // x > 5 ? 10 : 20
    let expr = Expr::dummy(ExprKind::Ternary {
        condition: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Gt,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
        })),
        then_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
        else_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(20)))),
    });

    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(7))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(expr)),
        ],
    };

    let result = compile_program(program);
    // Should have basic blocks for then/else branches
    assert!(result.is_ok());
}

// ==================== ARRAY EXPRESSIONS ====================

#[test]
fn test_compile_empty_array() {
    let expr = Expr::dummy(ExprKind::Array(vec![]));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_compile_array_int() {
    let expr = Expr::dummy(ExprKind::Array(vec![
        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
    ]));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    // Should allocate IntMatrix
    assert!(result.is_ok());
}

#[test]
fn test_compile_array_float() {
    let expr = Expr::dummy(ExprKind::Array(vec![
        Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
        Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
        Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
    ]));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    // Should allocate Matrix
    assert!(result.is_ok());
}

// ==================== COMPLEX NESTED EXPRESSIONS ====================

#[test]
fn test_compile_complex_arithmetic() {
    // (1 + 2) * (3 - 4) / 5
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Div,
        lhs: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Add,
                lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
            })),
            rhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Sub,
                lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(3)))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(4)))),
            })),
        })),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ATOMS (v1.1) ====================

#[test]
fn test_atom_literal() {
    let expr = Expr::dummy(ExprKind::Literal(Literal::Atom("ok".to_string())));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_atom_comparison() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::Eq,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Atom("ok".to_string())))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Atom("ok".to_string())))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_atom_different_comparison() {
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::NotEq,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Atom("ok".to_string())))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Atom("error".to_string())))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== NIL AND ERROR (v1.0) ====================

#[test]
fn test_nil_literal() {
    let expr = Expr::dummy(ExprKind::Identifier("nil".to_string()));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_error_creation() {
    let expr = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("error".to_string()))),
        args: vec![Expr::dummy(ExprKind::Literal(Literal::String("something failed".to_string())))],
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== LIST COMPREHENSIONS (v0.9) ====================

#[test]
fn test_list_comprehension_basic() {
    let expr = Expr::dummy(ExprKind::ListComprehension {
        expr: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
        })),
        generators: vec![parser::ast::ComprehensionGen {
            var_names: vec!["x".to_string()],
            iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
            ]))),
            conditions: vec![],
        }],
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_with_filter() {
    let expr = Expr::dummy(ExprKind::ListComprehension {
        expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
        generators: vec![parser::ast::ComprehensionGen {
            var_names: vec!["x".to_string()],
            iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(4.0))),
            ]))),
            conditions: vec![Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Gt,
                lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(2.0)))),
            })],
        }],
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_list_comprehension_nested() {
    let expr = Expr::dummy(ExprKind::ListComprehension {
        expr: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Mul,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
        })),
        generators: vec![
            parser::ast::ComprehensionGen {
                var_names: vec!["x".to_string()],
                iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
                ]))),
                conditions: vec![],
            },
            parser::ast::ComprehensionGen {
                var_names: vec!["y".to_string()],
                iterable: Box::new(Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Float(10.0))),
                    Expr::dummy(ExprKind::Literal(Literal::Float(20.0))),
                ]))),
                conditions: vec![],
            },
        ],
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_zip_function() {
    let expr = Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::Identifier("zip".to_string()))),
        args: vec![
            Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
            ])),
            Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(10.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(20.0))),
            ])),
        ],
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== F-STRINGS WITH FORMAT SPECIFIERS ====================

#[test]
fn test_fstring_basic() {
    let expr = Expr::dummy(ExprKind::FString { parts: vec![
        parser::ast::FStringPart::Text("Value: ".to_string()),
        parser::ast::FStringPart::Expr {
            expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(42)))),
            format: None,
        },
    ] });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_with_hex_format() {
    let expr = Expr::dummy(ExprKind::FString { parts: vec![
        parser::ast::FStringPart::Text("Hex: ".to_string()),
        parser::ast::FStringPart::Expr {
            expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(255)))),
            format: Some("x".to_string()),
        },
    ] });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_fstring_with_precision_format() {
    let expr = Expr::dummy(ExprKind::FString { parts: vec![
        parser::ast::FStringPart::Text("Pi: ".to_string()),
        parser::ast::FStringPart::Expr {
            expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.14159)))),
            format: Some(".2f".to_string()),
        },
    ] });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== INCREMENT/DECREMENT ADVANCED ====================

#[test]
fn test_decrement_prefix() {
    // --x
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Decrement {
            expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            is_prefix: true,
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_decrement_postfix() {
    // x--
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Decrement {
            expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            is_prefix: false,
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_increment_in_expression() {
    // y := ++x + 5
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "y".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Add,
                lhs: Box::new(Expr::dummy(ExprKind::Increment {
                    expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    is_prefix: true,
                })),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
            }),
            is_const: false,
        }),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== BITWISE ADVANCED ====================

#[test]
fn test_bitwise_with_negative() {
    // -5 & 7 (bitwise AND with negative number)
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::BitAnd,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(-5)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(7)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_bitwise_with_zero() {
    // 0xFF | 0 (bitwise OR with zero should return 0xFF)
    let expr = Expr::dummy(ExprKind::Binary {
        op: BinaryOp::BitOr,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0xFF)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_bitwise_xor_same_value() {
    // x ^ x = 0 (XOR with same value)
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::BitXor,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TERNARY ADVANCED ====================

#[test]
fn test_nested_ternary() {
    // x > 10 ? (x > 20 ? 30 : 20) : 10
    let expr = Expr::dummy(ExprKind::Ternary {
        condition: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Gt,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
        })),
        then_expr: Box::new(Expr::dummy(ExprKind::Ternary {
            condition: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Gt,
                lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(20)))),
            })),
            then_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(30)))),
            else_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(20)))),
        })),
        else_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
    });

    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(25))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(expr)),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_ternary_type_coercion() {
    // x > 5 ? 10 : 3.5 (should coerce to float)
    let expr = Expr::dummy(ExprKind::Ternary {
        condition: Box::new(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::Gt,
            lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
        })),
        then_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
        else_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.5)))),
    });

    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(7))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(expr)),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_ternary_in_assignment() {
    // result := x > 0 ? x : -x (absolute value)
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(-5))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "result".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Ternary {
                condition: Box::new(Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Gt,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                })),
                then_expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                else_expr: Box::new(Expr::dummy(ExprKind::Unary {
                    op: UnaryOp::Negate,
                    expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                })),
            }),
            is_const: false,
        }),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== SHORT-CIRCUIT EVALUATION ====================

#[test]
fn test_short_circuit_and_with_side_effect() {
    // false && (++x > 0) - second part should not execute
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(false)))),
            rhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Gt,
                lhs: Box::new(Expr::dummy(ExprKind::Increment {
                    expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    is_prefix: true,
                })),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
            })),
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_short_circuit_or_with_side_effect() {
    // true || (++x > 0) - second part should not execute
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::LogicalOr,
            lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
            rhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Gt,
                lhs: Box::new(Expr::dummy(ExprKind::Increment {
                    expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    is_prefix: true,
                })),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
            })),
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_chained_logical_operators() {
    // x > 0 && y > 0 && z > 0
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "y".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "z".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(15))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            lhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::LogicalAnd,
                lhs: Box::new(Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Gt,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                })),
                rhs: Box::new(Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Gt,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                })),
            })),
            rhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Gt,
                lhs: Box::new(Expr::dummy(ExprKind::Identifier("z".to_string()))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
            })),
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== CHAINED COMPARISONS ====================

#[test]
fn test_chained_comparison_basic() {
    // 1 < x < 10
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            lhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Lt,
                lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                rhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
            })),
            rhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Lt,
                lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
            })),
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_chained_comparison_multiple() {
    // 1 < x < y < 100
    let stmt = Stmt::dummy(StmtKind::Block(vec![
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "y".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(50))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Binary {
            op: BinaryOp::LogicalAnd,
            lhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::LogicalAnd,
                lhs: Box::new(Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Lt,
                    lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
                    rhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                })),
                rhs: Box::new(Expr::dummy(ExprKind::Binary {
                    op: BinaryOp::Lt,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
                })),
            })),
            rhs: Box::new(Expr::dummy(ExprKind::Binary {
                op: BinaryOp::Lt,
                lhs: Box::new(Expr::dummy(ExprKind::Identifier("y".to_string()))),
                rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(100)))),
            })),
        }))),
    ]));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}
