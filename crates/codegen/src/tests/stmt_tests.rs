// Statement Codegen Tests

use crate::Compiler;
use inkwell::context::Context;
use parser::ast::{BinaryOp, Expr, ExprKind, Literal, Program, Stmt, StmtKind};

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
    ($name:expr, $type_hint:expr, $value:expr) => {
        Stmt::dummy(StmtKind::VariableDecl {
            name: $name.to_string(),
            type_hint: Some($type_hint.to_string()),
            value: $value,
            is_const: false,
        })
    };
}

macro_rules! assignment {
    ($target:expr, $value:expr) => {
        Stmt::dummy(StmtKind::Assignment {
            target: $target,
            value: $value,
        })
    };
}

macro_rules! expr_stmt {
    ($expr:expr) => {
        Stmt::dummy(StmtKind::Expr($expr))
    };
}

macro_rules! block_stmt {
    ($stmts:expr) => {
        Stmt::dummy(StmtKind::Block($stmts))
    };
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

// Helper function to create binary operations
fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Expr {
    Expr::dummy(ExprKind::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    })
}

#[test]
fn test_variable_decl_inferred() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            is_const: false,
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("alloca") || ir.contains("store"));
}

#[test]
fn test_variable_decl_explicit_type() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("int".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
            }),
        ],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("store"));
}

#[test]
fn test_block_statement() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Block(vec![
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(1))))),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(2))))),
        ]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_return_statement() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
            name: "test_fn".to_string(),
            params: vec![],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(Stmt::dummy(StmtKind::Return {
                values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
            })),
        })],
    };
    let ir = compile_program(program).unwrap();
    assert!(ir.contains("ret"));
}
// ==================== VARIABLE DECLARATIONS - TYPE INFERENCE ====================

#[test]
fn test_var_decl_infer_float() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "pi".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_string() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "msg".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string()))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_intmatrix() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "arr".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            ])),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_matrix() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "arr".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Array(vec![
                Expr::dummy(ExprKind::Literal(Literal::Float(1.0))),
                Expr::dummy(ExprKind::Literal(Literal::Float(2.0))),
            ])),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_atom() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "status".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Atom("ok".to_string()))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_infer_complex() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "z".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Complex(3.0, 4.0))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== VARIABLE DECLARATIONS - TYPE CASTING ====================

#[test]
fn test_var_decl_cast_int_to_float() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("float".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_var_decl_cast_float_to_int() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: Some("int".to_string()),
            value: Expr::dummy(ExprKind::Literal(Literal::Float(3.14))),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== CONST DECLARATIONS ====================

#[test]
fn test_const_declaration() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "MAX".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Int(100))),
            is_const: true,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== ASSIGNMENTS - COMPOUND ====================

#[test]
fn test_assignment_add_compound() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: Expr::dummy(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                    rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(5)))),
                }),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment_to_array_element() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "arr".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Array(vec![
                    Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ])),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Identifier("arr".to_string()))),
                    indices: vec!(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment_to_matrix_element() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "mat".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("zeros".to_string()))),
                    args: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Index {
                    array: Box::new(Expr::dummy(ExprKind::Index {
                        array: Box::new(Expr::dummy(ExprKind::Identifier("mat".to_string()))),
                        indices: vec!(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                    })),
                    indices: vec!(Expr::dummy(ExprKind::Literal(Literal::Int(0)))),
                }),
                value: Expr::dummy(ExprKind::Literal(Literal::Float(5.5))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== IMPORT STATEMENTS ====================

#[test]
fn test_import_statement() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Import {
            module: "math".to_string(),
            alias: None,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_import_with_alias_stmt() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Import {
            module: "math".to_string(),
            alias: Some("m".to_string()),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_imports() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: None,
            }),
            Stmt::dummy(StmtKind::Import {
                module: "math".to_string(),
                alias: Some("m".to_string()),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== EXPRESSION STATEMENTS ====================

#[test]
fn test_function_call_as_statement() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Call {
            func: Box::new(Expr::dummy(ExprKind::Identifier("print".to_string()))),
            args: vec![Expr::dummy(ExprKind::Literal(Literal::String("hello".to_string())))],
        })))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_increment_as_statement() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(0))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Increment {
                expr: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                is_prefix: true,
            }))),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_decrement_as_statement() {
    let program = Program {
        statements: vec![
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
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== PRINT/PRINTLN STATEMENTS ====================

#[test]
fn test_print_statement() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Print {
            expr: Expr::dummy(ExprKind::Literal(Literal::String("test".to_string()))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_println_statement() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Println {
            expr: Expr::dummy(ExprKind::Literal(Literal::Int(42))),
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== MULTIPLE STATEMENTS ====================

#[test]
fn test_multiple_variable_declarations() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "z".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(30))),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_sequential_assignments() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: Expr::dummy(ExprKind::Literal(Literal::Int(2))),
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: Expr::dummy(ExprKind::Literal(Literal::Int(3))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== NESTED BLOCKS ====================

#[test]
fn test_nested_blocks() {
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::Block(vec![
            Stmt::dummy(StmtKind::Block(vec![
                Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(1))))),
            ])),
            Stmt::dummy(StmtKind::Block(vec![
                Stmt::dummy(StmtKind::Expr(Expr::dummy(ExprKind::Literal(Literal::Int(2))))),
            ])),
        ]))],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== DESTRUCTURING ====================

#[test]
fn test_destructuring_declaration() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_pair".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string(), "int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ],
                })),
            }),
            Stmt::dummy(StmtKind::DestructuringDecl {
                is_const: false,
                names: vec!["a".to_string(), "b".to_string()],
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("get_pair".to_string()))),
                    args: vec![],
                }),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_destructuring_with_ignore() {
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_triple".to_string(),
                params: vec![],
                return_type: Some(vec![
                    "int".to_string(),
                    "int".to_string(),
                    "int".to_string(),
                ]),
                body: Box::new(Stmt::dummy(StmtKind::Return {
                    values: vec![
                        Expr::dummy(ExprKind::Literal(Literal::Int(1))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                        Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    ],
                })),
            }),
            Stmt::dummy(StmtKind::DestructuringDecl {
                is_const: false,
                names: vec!["x".to_string(), "_".to_string(), "z".to_string()],
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("get_triple".to_string()))),
                    args: vec![],
                }),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== NIL INFERENCE ====================

#[test]
fn test_var_decl_infer_nil() {
    // var x := nil;
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "x".to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Literal(Literal::Nil)),
            is_const: false,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== COMPOUND ASSIGNMENTS ====================

#[test]
fn test_assignment_sub_compound() {
    // var x := 10;
    // x -= 3;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: binary(
                    BinaryOp::Sub,
                    Expr::dummy(ExprKind::Identifier("x".to_string())),
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                ),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment_mul_compound() {
    // var x := 5;
    // x *= 2;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: binary(
                    BinaryOp::Mul,
                    Expr::dummy(ExprKind::Identifier("x".to_string())),
                    Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                ),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment_div_compound() {
    // var x := 20;
    // x /= 4;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: binary(
                    BinaryOp::Div,
                    Expr::dummy(ExprKind::Identifier("x".to_string())),
                    Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                ),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment_mod_compound() {
    // var x := 17;
    // x %= 5;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(17))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: binary(
                    BinaryOp::Mod,
                    Expr::dummy(ExprKind::Identifier("x".to_string())),
                    Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                ),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

// ==================== EDGE CASES ====================

#[test]
fn test_multiple_assignments_same_line() {
    // var x := 10;
    // var y := 20;
    // x = y = 5;  // Simulated as: y = 5; x = y;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "y".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("y".to_string())),
                value: Expr::dummy(ExprKind::Literal(Literal::Int(5))),
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: Expr::dummy(ExprKind::Identifier("y".to_string())),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment_with_expression() {
    // var x := 10;
    // x = x * 2 + 5;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: binary(
                    BinaryOp::Add,
                    binary(
                        BinaryOp::Mul,
                        Expr::dummy(ExprKind::Identifier("x".to_string())),
                        Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                    ),
                    Expr::dummy(ExprKind::Literal(Literal::Int(5))),
                ),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_const_with_expression() {
    // const result := 2 + 3 * 4;
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::VariableDecl {
            name: "result".to_string(),
            type_hint: None,
            value: binary(
                BinaryOp::Add,
                Expr::dummy(ExprKind::Literal(Literal::Int(2))),
                binary(
                    BinaryOp::Mul,
                    Expr::dummy(ExprKind::Literal(Literal::Int(3))),
                    Expr::dummy(ExprKind::Literal(Literal::Int(4))),
                ),
            ),
            is_const: true,
        })],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_variable_reassignment() {
    // var x := 10;
    // x = 20;
    // x = 30;
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Literal(Literal::Int(10))),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: Expr::dummy(ExprKind::Literal(Literal::Int(20))),
            }),
            Stmt::dummy(StmtKind::Assignment {
                target: Expr::dummy(ExprKind::Identifier("x".to_string())),
                value: Expr::dummy(ExprKind::Literal(Literal::Int(30))),
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

#[test]
fn test_assignment_from_function_result() {
    // fn get_value() -> int { return 42; }
    // var x := get_value();
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                type_params: vec![],
                name: "get_value".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(42)))],
                })]))),
            }),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("get_value".to_string()))),
                    args: vec![],
                }),
                is_const: false,
            }),
        ],
    };
    let result = compile_program(program);
    assert!(result.is_ok());
}

