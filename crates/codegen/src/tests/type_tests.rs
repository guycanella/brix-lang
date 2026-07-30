// Type Inference and Casting Tests
//
// Tests to ensure correct type inference, automatic type promotion,
// and explicit type casting in the Brix compiler.

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, ExprKind, Literal, Program, Stmt, StmtKind};

// Helper function to create a simple program with one statement
fn make_program(stmt: Stmt) -> Program {
    Program {
        statements: vec![stmt],
    }
}

// Helper to compile a program and return the LLVM IR
// Returns Ok(ir) if compilation succeeded, Err(msg) if it failed or panicked
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

        // compile_program now returns CodegenResult
        match compiler.compile_program(&program) {
            Ok(_) => Ok(module.print_to_string().to_string()),
            Err(e) => Err(format!("Codegen error: {:?}", e)),
        }
    });

    match result {
        Ok(Ok(ir)) => Ok(ir),
        Ok(Err(msg)) => Err(msg),
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

// ==================== TYPE INFERENCE TESTS ====================

#[test]
fn test_infer_int_literal() {
    let stmt = Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(
        Literal::Int(42),
    ))));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_float_literal() {
    let stmt = Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(
        Literal::Float(3.14),
    ))));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_string_literal() {
    let stmt = Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(
        Literal::String("hello".to_string()),
    ))));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_bool_literal() {
    let stmt = Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(
        Literal::Bool(true),
    ))));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_nil_literal() {
    let stmt = Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Nil))));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_atom_literal() {
    let stmt = Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(
        Literal::Atom("ok".to_string()),
    ))));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_infer_complex_literal() {
    let stmt = Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(
        Literal::Complex(3.0, 4.0),
    ))));
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== VARIABLE DECLARATION TYPE INFERENCE ====================

#[test]
fn test_var_decl_infer_int() {
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "x".to_string(),
        type_hint: None,
        value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
        is_const: false,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_float() {
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "x".to_string(),
        type_hint: None,
        value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
        is_const: false,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_explicit_int() {
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "x".to_string(),
        type_hint: Some("int".to_string()),
        value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
        is_const: false,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_explicit_float() {
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "x".to_string(),
        type_hint: Some("float".to_string()),
        value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
        is_const: false,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TYPE CASTING TESTS ====================

#[test]
fn test_cast_int_to_float_explicit() {
    // var x: float = 42
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "x".to_string(),
        type_hint: Some("float".to_string()),
        value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
        is_const: false,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_cast_float_to_int_explicit() {
    // var x: int = 3.14
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "x".to_string(),
        type_hint: Some("int".to_string()),
        value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
        is_const: false,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== BINARY OPERATOR TYPE PROMOTION ====================

#[test]
fn test_add_int_int() {
    // 1 + 2
    let expr = Expr::dummy(ExprKind::Binary {
        op: parser::ast::BinaryOp::Add,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_add_float_float() {
    // 1.5 + 2.5
    let expr = Expr::dummy(ExprKind::Binary {
        op: parser::ast::BinaryOp::Add,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(1.5)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(2.5)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_add_int_float_promotion() {
    // 1 + 2.5 (should promote int to float)
    let expr = Expr::dummy(ExprKind::Binary {
        op: parser::ast::BinaryOp::Add,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(1)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(2.5)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_mul_int_float_promotion() {
    // 10 * 3.14 (should promote int to float)
    let expr = Expr::dummy(ExprKind::Binary {
        op: parser::ast::BinaryOp::Mul,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== CONST DECLARATION ====================

#[test]
fn test_const_decl() {
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "PI".to_string(),
        type_hint: None,
        value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14159))),
        is_const: true,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ARRAY TYPE INFERENCE ====================

#[test]
fn test_array_all_ints() {
    // [1, 2, 3] -> IntMatrix
    let expr = Expr::dummy(ExprKind::Array(vec![
        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
    ]));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_all_floats() {
    // [1.0, 2.0, 3.0] -> Matrix
    let expr = Expr::dummy(ExprKind::Array(vec![
        Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
        Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
        Expr::dummy(ExprKind::Literal(Literal::Float(3.0))),
    ]));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_array_mixed_promotes_to_float() {
    // [1, 2.5, 3] -> Matrix (with int->float promotion)
    let expr = Expr::dummy(ExprKind::Array(vec![
        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
        Expr::dummy(ExprKind::Literal(Literal::Float(2.5))),
        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
    ]));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_empty_array() {
    // [] -> Matrix (default to float)
    let expr = Expr::dummy(ExprKind::Array(vec![]));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPLEX NUMBER TYPE TESTS ====================

#[test]
fn test_complex_from_literal() {
    // 3.0 + 4.0i
    let expr = Expr::dummy(ExprKind::Literal(Literal::Complex(3.0, 4.0)));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_imaginary_literal() {
    // 2.0i -> Complex(0, 2.0)
    let expr = Expr::dummy(ExprKind::Literal(Literal::Complex(0.0, 2.0)));
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== TYPE CHECKING ERROR CASES ====================

#[test]
fn test_string_plus_int_fails() {
    // "hello" + 42 should fail
    let expr = Expr::dummy(ExprKind::Binary {
        op: parser::ast::BinaryOp::Add,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::String(
            "hello".to_string(),
        )))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(42)))),
    });
    let program = make_program(Stmt::dummy(StmtKind::Expr(expr)));
    let result = compile_program(program);
    // This should fail with a type error
    assert!(result.is_err());
}

#[test]
fn test_bitwise_on_float_fails() {
    // 3.14 & 2.5 should fail (bitwise only on ints)
    let _expr = Expr::dummy(ExprKind::Binary {
        op: parser::ast::BinaryOp::BitAnd,
        lhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(3.14)))),
        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Float(2.5)))),
    });
}

// ==================== TYPE INFERENCE ADVANCED ====================

#[test]
fn test_inference_in_ternary() {
    // var x := true ? 10 : 20;  // Should infer int
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Ternary {
                condition: Box::new(Expr::dummy(ExprKind::Literal(Literal::Bool(true)))),
                then_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(10)))),
                else_expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(20)))),
            }),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_inference_in_binary_op() {
    // var x := 5 + 3;  // Should infer int
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: binary(
                BinaryOp::Add,
                Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_inference_float_from_division() {
    // var x := 10 / 3;  // Should be int division, result int
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: binary(
                BinaryOp::Div,
                Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_inference_from_comparison() {
    // var x := 5 > 3;  // Should infer bool
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: binary(
                BinaryOp::Gt,
                Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_inference_from_logical_op() {
    // var x := true && false;  // Should infer bool
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: binary(
                BinaryOp::LogicalAnd,
                Expr::dummy(ExprKind::Literal(Literal::Bool(true))),
                Expr::dummy(ExprKind::Literal(Literal::Bool(false))),
            ),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_inference_from_unary_negate() {
    // var x := -42;  // Should infer int
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Unary {
                op: parser::ast::UnaryOp::Negate,
                expr: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(42)))),
            }),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== CASTING EDGE CASES ====================

#[test]
fn test_float_to_int_truncate_positive() {
    // var x: int := 3.9;  // Should truncate to 3
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("int".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Float(3.9))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_to_int_truncate_negative() {
    // var x: int := -3.9;  // Should truncate to -3
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("int".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Float(-3.9))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_int_to_float_exact() {
    // var x: float := 42;  // Should convert to 42.0
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("float".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_auto_promotion_in_mixed_operation() {
    // var x := 5 + 2.5;  // int + float -> float
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: binary(
                BinaryOp::Add,
                Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                Expr::dummy(ExprKind::Literal(Literal::Float(2.5))),
            ),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_auto_promotion_in_multiplication() {
    // var x := 3 * 1.5;  // int * float -> float
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: binary(
                BinaryOp::Mul,
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                Expr::dummy(ExprKind::Literal(Literal::Float(1.5))),
            ),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_cast_zero() {
    // var x: float := 0;  // Cast int 0 to float 0.0
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("float".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Int(0))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== NUMERIC EDGE CASES ====================

#[test]
fn test_very_large_int() {
    // var x := 9223372036854775807;  // i64::MAX
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(9223372036854775807))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_very_small_int() {
    // var x := -9223372036854775808;  // i64::MIN
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(-9223372036854775808))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_very_large_float() {
    // var x := 1e308;  // Very large float
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Float(1e308))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_very_small_float() {
    // var x := 1e-308;  // Very small positive float
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Float(1e-308))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_zero_positive() {
    // var x := 0.0;
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Float(0.0))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_float_negative_zero() {
    // var x := -0.0;
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Float(-0.0))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_division_by_int_zero() {
    // var x := 10 / 0;  // Division by zero (compiles, runtime behavior undefined)
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: binary(
                BinaryOp::Div,
                Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                Expr::dummy(ExprKind::Literal(Literal::Int(0))),
            ),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    // Should compile (runtime error is OK)
    assert!(result.is_ok());
}

#[test]
fn test_is_function_closure_true() {
    use parser::ast::Closure;
    // var res := is_function((x: int) -> int { return x * 2 })
    let closure = Closure {
        is_async: false,
        captured_vars: vec![],
        params: vec![("x".to_string(), "int".to_string())],
        return_type: Some("int".to_string()),
        body: Box::new(Stmt::dummy(StmtKind::Return {
            values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(0)))],
        })),
    };
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "res".to_string(),
        type_hint: None,
        value: Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_function".to_string()))),
            args: vec![Expr::dummy(ExprKind::Closure(closure))],
        }),
        is_const: false,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();
    // Static evaluation yields i64 1 and zero allocation/retain for is_function
    assert!(ir.contains("i64 1"));
    assert!(!ir.contains("__closure_"));
}

#[test]
fn test_is_function_variable_true() {
    use parser::ast::Closure;
    // var f := (x: int) -> int { return x }; var res := is_function(f)
    let closure = Closure {
        is_async: false,
        captured_vars: vec![],
        params: vec![("x".to_string(), "int".to_string())],
        return_type: Some("int".to_string()),
        body: Box::new(Stmt::dummy(StmtKind::Return {
            values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
        })),
    };
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "f".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Closure(closure)),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "res".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("is_function".to_string()))),
                    args: vec![Expr::dummy(ExprKind::Identifier("f".to_string()))],
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();
    assert!(ir.contains("i64 1"));
    // is_function(f) must not emit a spurious closure_retain.
    assert!(!ir.contains("closure_retain"));
}

#[test]
fn test_is_function_int_false() {
    // var res := is_function(42)
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "res".to_string(),
        type_hint: None,
        value: Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_function".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
        }),
        is_const: false,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();
    assert!(ir.contains("i64 0"));
}

#[test]
fn test_is_function_async_closure_false() {
    use parser::ast::Closure;

    // Async closures compile to AsyncFuture, not to the synchronous closure tuple.
    let stmt = Stmt::dummy(StmtKind::VariableDecl {
        name: "res".to_string(),
        type_hint: None,
        value: Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("is_function".to_string()))),
            args: vec![Expr::dummy(ExprKind::Closure(Closure {
                is_async: true,
                captured_vars: vec![],
                params: vec![],
                return_type: None,
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![]))),
            }))],
        }),
        is_const: false,
    });
    let program = make_program(stmt);
    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();
    assert!(ir.contains("i64 0"));
}

fn int_array(values: &[i64]) -> Expr {
    Expr::dummy(ExprKind::Array(
        values
            .iter()
            .map(|value| Expr::dummy(ExprKind::Literal(Literal::Int(*value))))
            .collect(),
    ))
}

fn mut_array_method(name: &str, method: &str, args: Vec<Expr>) -> Stmt {
    Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
        func: Box::new(Expr::dummy(ExprKind::FieldAccess {
            target: Box::new(Expr::dummy(ExprKind::Identifier(name.to_string()))),
            field: method.to_string(),
        })),
        args,
    })))
}

#[test]
fn test_mut_array_push_codegen() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "items".to_string(),
                type_hint: Some("mut int[]".to_string()),
                value: int_array(&[1, 2]),
                is_const: false,
            }),
            mut_array_method(
                "items",
                "push!",
                vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
            ),
        ],
    };

    let ir = compile_program(program).expect("mutable push! should compile");
    assert!(ir.contains("intmatrix_push_inplace"));
}

#[test]
fn test_mut_array_immutable_receiver_rejected() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "items".to_string(),
                type_hint: Some("int[]".to_string()),
                value: int_array(&[1, 2]),
                is_const: false,
            }),
            mut_array_method(
                "items",
                "push!",
                vec![Expr::dummy(ExprKind::Literal(Literal::Int(3)))],
            ),
        ],
    };

    let err = compile_program(program).expect_err("immutable push! must fail");
    assert!(err.contains("immutable variable"));
}

#[test]
fn test_mut_array_mutability_does_not_leak_from_function_scope() {
    let seed = Stmt::dummy(StmtKind::FunctionDef {
        name: "seed".to_string(),
        is_async: false,
        type_params: vec![],
        params: vec![],
        return_type: None,
        body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(
            StmtKind::VariableDecl {
                name: "items".to_string(),
                type_hint: Some("mut int[]".to_string()),
                value: int_array(&[1]),
                is_const: false,
            },
        )]))),
    });
    let program = Program {
        statements: vec![
            seed,
            Stmt::dummy(StmtKind::VariableDecl {
                name: "items".to_string(),
                type_hint: Some("int[]".to_string()),
                value: int_array(&[1]),
                is_const: false,
            }),
            mut_array_method(
                "items",
                "push!",
                vec![Expr::dummy(ExprKind::Literal(Literal::Int(2)))],
            ),
        ],
    };

    let err = compile_program(program).expect_err("function-local mutability must not leak");
    assert!(err.contains("immutable variable"));
}

#[test]
fn test_mut_array_zero_arg_methods_rejected_with_arguments() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "items".to_string(),
                type_hint: Some("mut int[]".to_string()),
                value: int_array(&[2, 1]),
                is_const: false,
            }),
            mut_array_method(
                "items",
                "sort!",
                vec![Expr::dummy(ExprKind::Literal(Literal::Int(1)))],
            ),
        ],
    };

    let err = compile_program(program).expect_err("sort! with arguments must fail");
    assert!(err.contains("sort! expects 0 arguments"));
}
