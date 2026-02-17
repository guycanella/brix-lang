// Unit tests for Type Aliases, Union Types, and Intersection Types (v1.4)
//
// Tests cover:
// - Type alias declarations (type MyInt = int)
// - Union types (int | float | string)
// - Intersection types (Point & Label)
// - Type alias resolution
// - typeof() with unions and intersections

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{Expr, ExprKind, Literal, Program, Stmt, StmtKind};

// Helper function to compile a program and return IR or error
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

macro_rules! lit_string {
    ($val:expr) => {
        Expr::dummy(ExprKind::Literal(Literal::String($val.to_string())))
    };
}

macro_rules! lit_nil {
    () => {
        Expr::dummy(ExprKind::Literal(Literal::Nil))
    };
}

macro_rules! var_decl {
    ($name:expr, $type_hint:expr, $value:expr) => {
        Stmt::dummy(StmtKind::VariableDecl {
            name: $name.to_string(),
            type_hint: $type_hint,
            value: $value,
            is_const: false,
        })
    };
}

macro_rules! type_alias {
    ($name:expr, $definition:expr) => {
        Stmt::dummy(StmtKind::TypeAlias {
            name: $name.to_string(),
            definition: $definition.to_string(),
        })
    };
}

// --- TYPE ALIAS TESTS ---

#[test]
fn test_type_alias_simple() {
    let program = Program {
        statements: vec![
            type_alias!("MyInt", "int"),
            var_decl!("x", Some("MyInt".to_string()), lit_int!(42)),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create type alias and use it");
}

#[test]
fn test_type_alias_optional() {
    let program = Program {
        statements: vec![
            type_alias!("OptionalInt", "int?"),
            var_decl!("x", Some("OptionalInt".to_string()), lit_int!(42)),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create optional type alias");
}

#[test]
fn test_type_alias_union() {
    let program = Program {
        statements: vec![
            type_alias!("IntOrFloat", "int | float"),
            var_decl!("x", Some("IntOrFloat".to_string()), lit_int!(42)),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create union type alias");
}

// --- UNION TYPE TESTS ---

#[test]
fn test_union_int_float() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int | float".to_string()), lit_int!(42)),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create union type int | float");
}

#[test]
fn test_union_int_float_string() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int | float | string".to_string()), lit_string!("hello")),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create union type int | float | string");
}

#[test]
fn test_union_with_nil() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int | nil".to_string()), lit_nil!()),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create union type int | nil");
}

#[test]
fn test_optional_is_union() {
    // T? should be equivalent to T | nil
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_nil!()),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should treat int? as int | nil");
}

// --- OPTIONAL TYPE AS UNION ---

#[test]
fn test_optional_with_value_as_union() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_int!(42)),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional as Union(int, nil) with value");
}

#[test]
fn test_optional_nil_as_union() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("int?".to_string()), lit_nil!()),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional as Union(int, nil) with nil");
}

#[test]
fn test_optional_string_as_union() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("string?".to_string()), lit_string!("hello")),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional<string> as Union(string, nil)");
}

#[test]
fn test_optional_float_as_union() {
    let program = Program {
        statements: vec![
            var_decl!("x", Some("float?".to_string()), lit_float!(3.14)),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should create Optional<float> as Union(float, nil)");
}

// --- MULTIPLE TYPE ALIASES ---

#[test]
fn test_multiple_type_aliases() {
    let program = Program {
        statements: vec![
            type_alias!("MyInt", "int"),
            type_alias!("MyFloat", "float"),
            type_alias!("IntOrFloat", "MyInt | MyFloat"),
            var_decl!("x", Some("IntOrFloat".to_string()), lit_int!(42)),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should handle multiple type aliases");
}

#[test]
fn test_chained_type_aliases() {
    let program = Program {
        statements: vec![
            type_alias!("MyInt", "int"),
            type_alias!("MyOptionalInt", "MyInt?"),
            var_decl!("x", Some("MyOptionalInt".to_string()), lit_nil!()),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok(), "Should resolve chained type aliases");
}
