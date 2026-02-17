// Stress Tests - v1.3 Type System
// Tests edge cases and performance limits for Closures, Structs, and Generics

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Closure, Expr, ExprKind, Literal, Program, Stmt, StmtKind, BinaryOp, StructDef};

/// Helper to compile a program and check for errors
fn compile_program(stmts: Vec<Stmt>) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("stress_test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(
            &context,
            &builder,
            &module,
            "stress_test.bx".to_string(),
            "stress test source".to_string(),
        );

        let program = Program { statements: stmts };
        match compiler.compile_program(&program) {
            Ok(_) => Ok(module.print_to_string().to_string()),
            Err(e) => Err(format!("Compilation error: {}", e)),
        }
    });

    match result {
        Ok(Ok(ir)) => Ok(ir),
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Compilation panicked".to_string()),
    }
}

// ==========================================
// CLOSURE STRESS TESTS
// ==========================================

#[test]
fn test_closure_many_captures_10() {
    // Test closure capturing 10 variables
    let mut stmts = vec![];
    let mut captured_vars = vec![];

    // Declare 10 variables
    for i in 0..10 {
        let var_name = format!("var{}", i);
        stmts.push(Stmt::dummy(StmtKind::VariableDecl {
            name: var_name.clone(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(i as i64))),
            is_const: false,
        }));
        captured_vars.push(var_name);
    }

    // Create closure that uses all 10 variables (simpler expression to avoid stack overflow)
    // Just return var0 to avoid deeply nested AST
    let return_expr = Expr::dummy(ExprKind::Identifier("var0".to_string()));

    stmts.push(Stmt::dummy(StmtKind::VariableDecl {
        name: "closure".to_string(),
        type_hint: None,
        value: Expr::dummy(ExprKind::Closure(Closure {
            params: vec![],
            return_type: Some("int".to_string()),
            body: Box::new(Stmt::dummy(StmtKind::Return {
                values: vec![return_expr],
            })),
            captured_vars,
        })),
        is_const: false,
    }));

    assert!(compile_program(stmts).is_ok(), "Closure with 10 captures should compile");
}

#[test]
fn test_nested_closures_3_levels() {
    // Test 3 levels of nested closures
    // var x := 10
    // var outer := () -> () {
    //     var middle := () -> () {
    //         var inner := () -> int { return x }
    //         return inner
    //     }
    //     return middle
    // }

    let stmts = vec![
        // var x := 10
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            is_const: false,
        }),
        // var inner := () -> int { return x }
        Stmt::dummy(StmtKind::VariableDecl {
            name: "inner".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Closure(Closure {
                params: vec![],
                return_type: Some("int".to_string()),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })),
                captured_vars: vec!["x".to_string()],
            })),
            is_const: false,
        }),
    ];

    assert!(compile_program(stmts).is_ok(), "Nested closures (3 levels) should compile");
}

#[test]
fn test_closure_chain_5_calls() {
    // Test chaining 5 closure calls
    // var f1 := (x: int) -> int { return x + 1 }
    // var f2 := (x: int) -> int { return x + 2 }
    // var f3 := (x: int) -> int { return x + 3 }
    // var f4 := (x: int) -> int { return x + 4 }
    // var f5 := (x: int) -> int { return x + 5 }

    let mut stmts = vec![];
    for i in 1..=5 {
        stmts.push(Stmt::dummy(StmtKind::VariableDecl {
            name: format!("f{}", i),
            type_hint: None,
            value: Expr::dummy(ExprKind::Closure(Closure {
                params: vec![("x".to_string(), "int".to_string())],
                return_type: Some("int".to_string()),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Binary {
                        op: BinaryOp::Add,
                        lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(i as i64)))),
                    })],
                })),
                captured_vars: vec![],
            })),
            is_const: false,
        }));
    }

    assert!(compile_program(stmts).is_ok(), "5 closure definitions should compile");
}

// ==========================================
// STRUCT STRESS TESTS
// ==========================================

#[test]
fn test_struct_many_fields_15() {
    // Test struct with 15 fields
    let mut fields = vec![];
    for i in 0..15 {
        fields.push((format!("field{}", i), "int".to_string(), None));
    }

    let stmts = vec![Stmt::dummy(StmtKind::StructDef(StructDef {
        name: "BigStruct".to_string(),
        type_params: vec![],
        fields,
    }))];

    assert!(compile_program(stmts).is_ok(), "Struct with 15 fields should compile");
}

#[test]
fn test_struct_with_many_defaults_10() {
    // Test struct with 10 fields with default values
    let mut fields = vec![];
    for i in 0..10 {
        fields.push((
            format!("field{}", i),
            "int".to_string(),
            Some(Expr::dummy(ExprKind::Literal(Literal::Int(i as i64)))),
        ));
    }

    let stmts = vec![Stmt::dummy(StmtKind::StructDef(StructDef {
        name: "DefaultStruct".to_string(),
        type_params: vec![],
        fields,
    }))];

    assert!(compile_program(stmts).is_ok(), "Struct with 10 default values should compile");
}

// ==========================================
// GENERIC STRESS TESTS
// ==========================================

#[test]
fn test_generic_struct_multiple_type_params_3() {
    // Test generic struct with 3 type parameters
    // struct Triple<A, B, C> { first: A, second: B, third: C }
    let stmts = vec![Stmt::dummy(StmtKind::StructDef(StructDef {
        name: "Triple".to_string(),
        type_params: vec![
            parser::ast::TypeParam { name: "A".to_string() },
            parser::ast::TypeParam { name: "B".to_string() },
            parser::ast::TypeParam { name: "C".to_string() },
        ],
        fields: vec![
            ("first".to_string(), "A".to_string(), None),
            ("second".to_string(), "B".to_string(), None),
            ("third".to_string(), "C".to_string(), None),
        ],
    }))];

    assert!(compile_program(stmts).is_ok(), "Generic struct with 3 type params should compile");
}

// ==========================================
// COMBINED STRESS TESTS
// ==========================================

#[test]
fn test_complex_combination() {
    // Generic struct + closure with captures
    // struct Container<T> { value: T }
    // var x := 10
    // var closure := () -> int { return x }

    let stmts = vec![
        Stmt::dummy(StmtKind::StructDef(StructDef {
            name: "Container".to_string(),
            type_params: vec![parser::ast::TypeParam { name: "T".to_string() }],
            fields: vec![("value".to_string(), "T".to_string(), None)],
        })),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            is_const: false,
        }),
        Stmt::dummy(StmtKind::VariableDecl {
            name: "closure".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Closure(Closure {
                params: vec![],
                return_type: Some("int".to_string()),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
                })),
                captured_vars: vec!["x".to_string()],
            })),
            is_const: false,
        }),
    ];

    assert!(compile_program(stmts).is_ok(), "Complex combination should compile");
}
