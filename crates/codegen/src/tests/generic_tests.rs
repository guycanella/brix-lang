// Generic Function Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Expr, ExprKind, Literal, Program, Stmt, StmtKind, TypeParam};

// Helper macros for creating AST nodes with dummy spans
macro_rules! lit_int {
    ($val:expr) => {
        Expr::dummy(ExprKind::Literal(Literal::Int($val)))
    };
}

macro_rules! lit_float {
    ($val:expr) => {
        Expr::dummy(ExprKind::Literal(Literal::Float($val)))
    };
}

macro_rules! lit_str {
    ($val:expr) => {
        Expr::dummy(ExprKind::Literal(Literal::String($val.to_string())))
    };
}

macro_rules! ident {
    ($name:expr) => {
        Expr::dummy(ExprKind::Identifier($name.to_string()))
    };
}

macro_rules! var_decl {
    ($name:expr, $value:expr) => {
        Stmt::dummy(StmtKind::VariableDecl {
            name: $name.to_string(),
            type_hint: None,
            value: $value,
            is_const: false,
        })
    };
}

fn compile_program(program: Program) -> Result<String, String> {
    let result = std::panic::catch_unwind(|| {
        let context = Context::create();
        let module = context.create_module("test");
        let builder = context.create_builder();
        let mut compiler = Compiler::new(&context, &builder, &module, "test.bx".to_string(), "".to_string());
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

#[test]
fn test_generic_function_definition() {
    // Test that generic function definition is stored, not compiled
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "identity".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![("x".to_string(), "T".to_string(), None)],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![ident!("x")],
                })),
            }),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok(), "Generic function definition should compile without errors");

    // Generic function should NOT appear in IR (not compiled yet)
    let ir = result.unwrap();
    assert!(!ir.contains("@identity"), "Generic function 'identity' should not be compiled yet (only main)");
}

#[test]
fn test_generic_call_explicit_single_type() {
    // Test: identity<int>(42)
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "identity".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![("x".to_string(), "T".to_string(), None)],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![ident!("x")],
                })),
            }),
            var_decl!(
                "result",
                Expr::dummy(ExprKind::GenericCall {
                    func: Box::new(ident!("identity")),
                    type_args: vec!["int".to_string()],
                    args: vec![lit_int!(42)],
                })
            ),
        ],
    };

    let result = compile_program(program);
    println!("=== GENERIC CALL EXPLICIT IR ===");
    match &result {
        Ok(ir) => println!("{}", ir),
        Err(e) => println!("ERROR: {}", e),
    }
    println!("================================");

    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have specialized function
    assert!(ir.contains("identity_int"), "Should have specialized function identity_int");
}

#[test]
fn test_generic_call_explicit_multiple_types() {
    // Test: swap<int, float>(42, 3.14)
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "swap".to_string(),
                type_params: vec![
                    TypeParam { name: "T".to_string() },
                    TypeParam { name: "U".to_string() },
                ],
                params: vec![
                    ("a".to_string(), "T".to_string(), None),
                    ("b".to_string(), "U".to_string(), None),
                ],
                return_type: Some(vec!["U".to_string(), "T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![ident!("b"), ident!("a")],
                })),
            }),
            var_decl!(
                "result",
                Expr::dummy(ExprKind::GenericCall {
                    func: Box::new(ident!("swap")),
                    type_args: vec!["int".to_string(), "float".to_string()],
                    args: vec![lit_int!(42), lit_float!(3.14)],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have specialized function with mangled name
    assert!(ir.contains("swap_int_float"), "Should have specialized function swap_int_float");
}

#[test]
fn test_generic_call_inferred_int() {
    // Test: identity(42) - should infer T = int
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "identity".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![("x".to_string(), "T".to_string(), None)],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![ident!("x")],
                })),
            }),
            var_decl!(
                "result",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_int!(42)],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have specialized function for int
    assert!(ir.contains("identity_int"), "Should infer T = int and create identity_int");
}

#[test]
fn test_generic_call_inferred_float() {
    // Test: identity(3.14) - should infer T = float
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "identity".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![("x".to_string(), "T".to_string(), None)],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![ident!("x")],
                })),
            }),
            var_decl!(
                "result",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_float!(3.14)],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have specialized function for float
    assert!(ir.contains("identity_float"), "Should infer T = float and create identity_float");
}

#[test]
fn test_generic_call_inferred_string() {
    // Test: identity("hello") - should infer T = string
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "identity".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![("x".to_string(), "T".to_string(), None)],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![ident!("x")],
                })),
            }),
            var_decl!(
                "result",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_str!("hello")],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have specialized function for string
    assert!(ir.contains("identity_string"), "Should infer T = string and create identity_string");
}

#[test]
fn test_generic_type_promotion() {
    // Test: add(1, 2.5) - should infer T = float with promotion
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "add".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![
                    ("a".to_string(), "T".to_string(), None),
                    ("b".to_string(), "T".to_string(), None),
                ],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Binary {
                        op: parser::ast::BinaryOp::Add,
                        lhs: Box::new(ident!("a")),
                        rhs: Box::new(ident!("b")),
                    })],
                })),
            }),
            var_decl!(
                "result",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("add")),
                    args: vec![lit_int!(1), lit_float!(2.5)],
                })
            ),
        ],
    };

    let result = compile_program(program);
    println!("=== TYPE PROMOTION IR ===");
    match &result {
        Ok(ir) => println!("{}", ir),
        Err(e) => println!("ERROR: {}", e),
    }
    println!("=========================");

    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should promote to float
    assert!(ir.contains("add_float"), "Should promote int to float and create add_float");

    // The call should pass both args as double (cast happens during compilation)
    assert!(ir.contains("call double @add_float(double"), "Should call add_float with double arguments");
}

#[test]
fn test_monomorphization_cache() {
    // Test: Multiple calls with same types should reuse specialized function
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "identity".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![("x".to_string(), "T".to_string(), None)],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![ident!("x")],
                })),
            }),
            var_decl!(
                "a",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_int!(1)],
                })
            ),
            var_decl!(
                "b",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_int!(2)],
                })
            ),
            var_decl!(
                "c",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_int!(3)],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should only have ONE definition of identity_int
    let count = ir.matches("define i64 @identity_int").count();
    assert_eq!(count, 1, "Should only compile identity_int once (cache should work)");
}

#[test]
fn test_multiple_specializations() {
    // Test: Calls with different types should create multiple specializations
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "identity".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![("x".to_string(), "T".to_string(), None)],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![ident!("x")],
                })),
            }),
            var_decl!(
                "a",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_int!(42)],
                })
            ),
            var_decl!(
                "b",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_float!(3.14)],
                })
            ),
            var_decl!(
                "c",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_str!("hello")],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have three different specializations
    assert!(ir.contains("identity_int"), "Should have identity_int");
    assert!(ir.contains("identity_float"), "Should have identity_float");
    assert!(ir.contains("identity_string"), "Should have identity_string");
}

#[test]
fn test_generic_add_operation() {
    // Test: Generic function with arithmetic operation
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "add".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![
                    ("a".to_string(), "T".to_string(), None),
                    ("b".to_string(), "T".to_string(), None),
                ],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Binary {
                        op: parser::ast::BinaryOp::Add,
                        lhs: Box::new(ident!("a")),
                        rhs: Box::new(ident!("b")),
                    })],
                })),
            }),
            var_decl!(
                "x",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("add")),
                    args: vec![lit_int!(10), lit_int!(20)],
                })
            ),
            var_decl!(
                "y",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("add")),
                    args: vec![lit_float!(1.5), lit_float!(2.5)],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have both int and float versions
    assert!(ir.contains("add_int"), "Should have add_int");
    assert!(ir.contains("add_float"), "Should have add_float");

    // Int version should use integer add
    assert!(ir.contains("add i64") || ir.contains("add nsw i64"), "Should use integer addition");

    // Float version should use float add
    assert!(ir.contains("fadd double"), "Should use float addition");
}

#[test]
fn test_explicit_and_inferred_same_result() {
    // Test: Explicit type args should produce same result as inferred
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                name: "identity".to_string(),
                type_params: vec![TypeParam { name: "T".to_string() }],
                params: vec![("x".to_string(), "T".to_string(), None)],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![ident!("x")],
                })),
            }),
            // Explicit
            var_decl!(
                "a",
                Expr::dummy(ExprKind::GenericCall {
                    func: Box::new(ident!("identity")),
                    type_args: vec!["int".to_string()],
                    args: vec![lit_int!(42)],
                })
            ),
            // Inferred
            var_decl!(
                "b",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(ident!("identity")),
                    args: vec![lit_int!(42)],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Both should use the same specialized function
    let count = ir.matches("define i64 @identity_int").count();
    assert_eq!(count, 1, "Both explicit and inferred should use same specialized function");
}

// ===========================
// GENERIC STRUCT TESTS
// ===========================

#[test]
fn test_generic_struct_definition() {
    // Test that generic struct definition is stored, not compiled
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(parser::ast::StructDef {
                name: "Box".to_string(),
                type_params: vec![parser::ast::TypeParam { name: "T".to_string() }],
                fields: vec![("value".to_string(), "T".to_string(), None)],
            })),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok(), "Generic struct definition should compile without errors");

    // Generic struct should NOT appear in IR (not compiled yet)
    let ir = result.unwrap();
    assert!(!ir.contains("%Box = type"), "Generic struct 'Box' should not be compiled yet");
}

#[test]
fn test_generic_struct_single_type() {
    // Test: Box<int>{ value: 42 }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(parser::ast::StructDef {
                name: "Box".to_string(),
                type_params: vec![parser::ast::TypeParam { name: "T".to_string() }],
                fields: vec![("value".to_string(), "T".to_string(), None)],
            })),
            var_decl!(
                "b",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["int".to_string()],
                    fields: vec![("value".to_string(), lit_int!(42))],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have specialized struct
    assert!(ir.contains("%Box_int = type"), "Should have specialized struct Box_int");
}

#[test]
fn test_generic_struct_multiple_types() {
    // Test: Pair<int, float>{ first: 42, second: 3.14 }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(parser::ast::StructDef {
                name: "Pair".to_string(),
                type_params: vec![
                    parser::ast::TypeParam { name: "T".to_string() },
                    parser::ast::TypeParam { name: "U".to_string() },
                ],
                fields: vec![
                    ("first".to_string(), "T".to_string(), None),
                    ("second".to_string(), "U".to_string(), None),
                ],
            })),
            var_decl!(
                "p",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Pair".to_string(),
                    type_args: vec!["int".to_string(), "float".to_string()],
                    fields: vec![
                        ("first".to_string(), lit_int!(42)),
                        ("second".to_string(), lit_float!(3.14)),
                    ],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have specialized struct with mangled name
    assert!(ir.contains("%Pair_int_float = type"), "Should have specialized struct Pair_int_float");
}

#[test]
fn test_generic_struct_field_access() {
    // Test: Box<int>{ value: 42 }.value
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(parser::ast::StructDef {
                name: "Box".to_string(),
                type_params: vec![parser::ast::TypeParam { name: "T".to_string() }],
                fields: vec![("value".to_string(), "T".to_string(), None)],
            })),
            var_decl!(
                "b",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["int".to_string()],
                    fields: vec![("value".to_string(), lit_int!(42))],
                })
            ),
            var_decl!(
                "x",
                Expr::dummy(ExprKind::FieldAccess {
                    target: Box::new(ident!("b")),
                    field: "value".to_string(),
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have field access
    assert!(ir.contains("%Box_int = type"), "Should have specialized struct");
}

#[test]
fn test_generic_struct_multiple_specializations() {
    // Test: Multiple instantiations with different types
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(parser::ast::StructDef {
                name: "Box".to_string(),
                type_params: vec![parser::ast::TypeParam { name: "T".to_string() }],
                fields: vec![("value".to_string(), "T".to_string(), None)],
            })),
            var_decl!(
                "int_box",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["int".to_string()],
                    fields: vec![("value".to_string(), lit_int!(42))],
                })
            ),
            var_decl!(
                "float_box",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["float".to_string()],
                    fields: vec![("value".to_string(), lit_float!(3.14))],
                })
            ),
            var_decl!(
                "str_box",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["string".to_string()],
                    fields: vec![("value".to_string(), lit_str!("hello"))],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have three different specializations
    assert!(ir.contains("%Box_int = type"), "Should have Box_int");
    assert!(ir.contains("%Box_float = type"), "Should have Box_float");
    assert!(ir.contains("%Box_string = type"), "Should have Box_string");
}

#[test]
fn test_generic_struct_cache() {
    // Test: Multiple instantiations with same types should reuse
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(parser::ast::StructDef {
                name: "Box".to_string(),
                type_params: vec![parser::ast::TypeParam { name: "T".to_string() }],
                fields: vec![("value".to_string(), "T".to_string(), None)],
            })),
            var_decl!(
                "a",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["int".to_string()],
                    fields: vec![("value".to_string(), lit_int!(1))],
                })
            ),
            var_decl!(
                "b",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["int".to_string()],
                    fields: vec![("value".to_string(), lit_int!(2))],
                })
            ),
            var_decl!(
                "c",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["int".to_string()],
                    fields: vec![("value".to_string(), lit_int!(3))],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should only have ONE definition of Box_int (cache should work)
    let count = ir.matches("%Box_int = type").count();
    assert_eq!(count, 1, "Should only define Box_int once (cache should work)");
}

// ===========================
// GENERIC METHOD TESTS
// ===========================

#[test]
fn test_generic_struct_with_method() {
    // Test: Box<T> with get() method
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(parser::ast::StructDef {
                name: "Box".to_string(),
                type_params: vec![parser::ast::TypeParam { name: "T".to_string() }],
                fields: vec![("value".to_string(), "T".to_string(), None)],
            })),
            Stmt::dummy(StmtKind::MethodDef(parser::ast::MethodDef {
                receiver_name: "b".to_string(),
                receiver_type: "Box".to_string(),
                method_name: "get".to_string(),
                params: vec![],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("b")),
                        field: "value".to_string(),
                    })],
                })),
            })),
            var_decl!(
                "int_box",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["int".to_string()],
                    fields: vec![("value".to_string(), lit_int!(42))],
                })
            ),
            var_decl!(
                "x",
                Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("int_box")),
                        field: "get".to_string(),
                    })),
                    args: vec![],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have specialized struct and method
    assert!(ir.contains("%Box_int = type"), "Should have Box_int struct");
    assert!(ir.contains("@Box_int_get"), "Should have Box_int_get method");
}

#[test]
fn test_generic_method_multiple_types() {
    // Test: Methods monomorphized for different type instantiations
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::StructDef(parser::ast::StructDef {
                name: "Box".to_string(),
                type_params: vec![parser::ast::TypeParam { name: "T".to_string() }],
                fields: vec![("value".to_string(), "T".to_string(), None)],
            })),
            Stmt::dummy(StmtKind::MethodDef(parser::ast::MethodDef {
                receiver_name: "b".to_string(),
                receiver_type: "Box".to_string(),
                method_name: "get".to_string(),
                params: vec![],
                return_type: Some(vec!["T".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::FieldAccess {
                        target: Box::new(ident!("b")),
                        field: "value".to_string(),
                    })],
                })),
            })),
            var_decl!(
                "int_box",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["int".to_string()],
                    fields: vec![("value".to_string(), lit_int!(42))],
                })
            ),
            var_decl!(
                "float_box",
                Expr::dummy(ExprKind::StructInit {
                    struct_name: "Box".to_string(),
                    type_args: vec!["float".to_string()],
                    fields: vec![("value".to_string(), lit_float!(3.14))],
                })
            ),
        ],
    };

    let result = compile_program(program);
    assert!(result.is_ok());
    let ir = result.unwrap();

    // Should have both specialized methods
    assert!(ir.contains("@Box_int_get"), "Should have Box_int_get");
    assert!(ir.contains("@Box_float_get"), "Should have Box_float_get");
}
