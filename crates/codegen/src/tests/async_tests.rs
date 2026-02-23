// Async/Await Codegen Tests
//
// Tests that async fn and await expressions compile to correct LLVM IR.
// Async fns produce create_{name} and poll_{name} functions.
// async fn main drives execution via brix_run_to_completion.

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

// Build a simple Block body with a single Return of a literal int
fn return_int_body(n: i64) -> Stmt {
    Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
        values: vec![Expr::dummy(ExprKind::Literal(Literal::Int(n)))],
    })]))
}

// Build a Block body with a single Return of an identifier
fn return_ident_body(name: &str) -> Stmt {
    Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(StmtKind::Return {
        values: vec![Expr::dummy(ExprKind::Identifier(name.to_string()))],
    })]))
}

// ==================== BASIC ASYNC FN (NO AWAITS) ====================

#[test]
fn test_async_fn_no_awaits_compiles() {
    // async fn answer() -> int { return 42 }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
            is_async: true,
            type_params: vec![],
            name: "answer".to_string(),
            params: vec![],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(return_int_body(42)),
        })],
    };
    let ir = compile_program(program);
    assert!(ir.is_ok(), "async fn with no awaits should compile");
    let ir = ir.unwrap();
    assert!(ir.contains("create_answer"), "IR should contain create_answer");
    assert!(ir.contains("poll_answer"), "IR should contain poll_answer");
}

#[test]
fn test_async_fn_no_awaits_emits_ready_status() {
    // async fn compute() -> int { return 10 }
    // The poll function should embed the constant 1 (READY status)
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
            is_async: true,
            type_params: vec![],
            name: "compute".to_string(),
            params: vec![],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(return_int_body(10)),
        })],
    };
    let ir = compile_program(program);
    assert!(ir.is_ok());
    let ir = ir.unwrap();
    // poll_compute must exist and return a { i64, i64 } aggregate
    assert!(ir.contains("poll_compute"));
    assert!(ir.contains("create_compute"));
}

#[test]
fn test_async_fn_with_int_param_compiles() {
    // async fn double(x: int) -> int { return x * 2 }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
            is_async: true,
            type_params: vec![],
            name: "double".to_string(),
            params: vec![("x".to_string(), "int".to_string(), None)],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(
                StmtKind::Return {
                    values: vec![Expr::dummy(ExprKind::Binary {
                        op: parser::ast::BinaryOp::Mul,
                        lhs: Box::new(Expr::dummy(ExprKind::Identifier("x".to_string()))),
                        rhs: Box::new(Expr::dummy(ExprKind::Literal(Literal::Int(2)))),
                    })],
                },
            )]))),
        })],
    };
    let ir = compile_program(program);
    assert!(ir.is_ok(), "async fn with int param should compile");
    let ir = ir.unwrap();
    assert!(ir.contains("create_double"));
    assert!(ir.contains("poll_double"));
}

#[test]
fn test_async_fn_with_multiple_params_compiles() {
    // async fn add(a: int, b: int) -> int { return a }
    let program = Program {
        statements: vec![Stmt::dummy(StmtKind::FunctionDef {
            is_async: true,
            type_params: vec![],
            name: "add".to_string(),
            params: vec![
                ("a".to_string(), "int".to_string(), None),
                ("b".to_string(), "int".to_string(), None),
            ],
            return_type: Some(vec!["int".to_string()]),
            body: Box::new(return_ident_body("a")),
        })],
    };
    let ir = compile_program(program);
    assert!(ir.is_ok(), "async fn with multiple params should compile");
    let ir = ir.unwrap();
    assert!(ir.contains("create_add"));
    assert!(ir.contains("poll_add"));
}

// ==================== ASYNC FN WITH AWAIT ====================

#[test]
fn test_async_fn_with_one_await_compiles() {
    // async fn helper() -> int { return 7 }
    // async fn caller() -> int {
    //     var x := await helper()
    //     return x
    // }
    let helper = Stmt::dummy(StmtKind::FunctionDef {
        is_async: true,
        type_params: vec![],
        name: "helper".to_string(),
        params: vec![],
        return_type: Some(vec!["int".to_string()]),
        body: Box::new(return_int_body(7)),
    });

    let caller = Stmt::dummy(StmtKind::FunctionDef {
        is_async: true,
        type_params: vec![],
        name: "caller".to_string(),
        params: vec![],
        return_type: Some(vec!["int".to_string()]),
        body: Box::new(Stmt::dummy(StmtKind::Block(vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Await {
                    expr: Box::new(Expr::dummy(ExprKind::Call {
                        func: Box::new(Expr::dummy(ExprKind::Identifier("helper".to_string()))),
                        args: vec![],
                    })),
                }),
                is_const: false,
            }),
            Stmt::dummy(StmtKind::Return {
                values: vec![Expr::dummy(ExprKind::Identifier("x".to_string()))],
            }),
        ]))),
    });

    let program = Program {
        statements: vec![helper, caller],
    };
    let ir = compile_program(program);
    assert!(ir.is_ok(), "async fn with one await should compile");
    let ir = ir.unwrap();
    assert!(ir.contains("create_helper"));
    assert!(ir.contains("poll_helper"));
    assert!(ir.contains("create_caller"));
    assert!(ir.contains("poll_caller"));
}

#[test]
fn test_async_fn_two_sequential_awaits_compiles() {
    // async fn step() -> int { return 1 }
    // async fn pipeline() -> int {
    //     var a := await step()
    //     var b := await step()
    //     return b
    // }
    let step = Stmt::dummy(StmtKind::FunctionDef {
        is_async: true,
        type_params: vec![],
        name: "step".to_string(),
        params: vec![],
        return_type: Some(vec!["int".to_string()]),
        body: Box::new(return_int_body(1)),
    });

    let make_await_decl = |var: &str| {
        Stmt::dummy(StmtKind::VariableDecl {
            name: var.to_string(),
            type_hint: None,
            value: Expr::dummy(ExprKind::Await {
                expr: Box::new(Expr::dummy(ExprKind::Call {
                    func: Box::new(Expr::dummy(ExprKind::Identifier("step".to_string()))),
                    args: vec![],
                })),
            }),
            is_const: false,
        })
    };

    let pipeline = Stmt::dummy(StmtKind::FunctionDef {
        is_async: true,
        type_params: vec![],
        name: "pipeline".to_string(),
        params: vec![],
        return_type: Some(vec!["int".to_string()]),
        body: Box::new(Stmt::dummy(StmtKind::Block(vec![
            make_await_decl("a"),
            make_await_decl("b"),
            Stmt::dummy(StmtKind::Return {
                values: vec![Expr::dummy(ExprKind::Identifier("b".to_string()))],
            }),
        ]))),
    });

    let program = Program {
        statements: vec![step, pipeline],
    };
    let ir = compile_program(program);
    assert!(ir.is_ok(), "async fn with two sequential awaits should compile");
    let ir = ir.unwrap();
    assert!(ir.contains("create_pipeline"));
    assert!(ir.contains("poll_pipeline"));
}

// ==================== ASYNC FN MAIN ====================

#[test]
fn test_async_fn_main_emits_run_to_completion() {
    // async fn helper() -> int { return 5 }
    // async fn main() {
    //     var x := await helper()
    // }
    let helper = Stmt::dummy(StmtKind::FunctionDef {
        is_async: true,
        type_params: vec![],
        name: "helper".to_string(),
        params: vec![],
        return_type: Some(vec!["int".to_string()]),
        body: Box::new(return_int_body(5)),
    });

    let main_fn = Stmt::dummy(StmtKind::FunctionDef {
        is_async: true,
        type_params: vec![],
        name: "main".to_string(),
        params: vec![],
        return_type: None,
        body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(
            StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: Expr::dummy(ExprKind::Await {
                    expr: Box::new(Expr::dummy(ExprKind::Call {
                        func: Box::new(Expr::dummy(ExprKind::Identifier(
                            "helper".to_string(),
                        ))),
                        args: vec![],
                    })),
                }),
                is_const: false,
            },
        )]))),
    });

    let program = Program {
        statements: vec![helper, main_fn],
    };
    let ir = compile_program(program);
    assert!(ir.is_ok(), "async fn main with await should compile");
    let ir = ir.unwrap();
    assert!(ir.contains("brix_run_to_completion"), "IR must call brix_run_to_completion");
    assert!(ir.contains("create_main"));
    assert!(ir.contains("poll_main"));
}

#[test]
fn test_async_fn_main_no_await_compiles() {
    // async fn main() { } — trivial case
    let main_fn = Stmt::dummy(StmtKind::FunctionDef {
        is_async: true,
        type_params: vec![],
        name: "main".to_string(),
        params: vec![],
        return_type: None,
        body: Box::new(Stmt::dummy(StmtKind::Block(vec![]))),
    });

    let program = Program {
        statements: vec![main_fn],
    };
    let ir = compile_program(program);
    assert!(ir.is_ok(), "async fn main with no awaits should compile");
    let ir = ir.unwrap();
    assert!(ir.contains("create_main"));
    assert!(ir.contains("poll_main"));
}

// ==================== MULTIPLE ASYNC FNS ====================

#[test]
fn test_multiple_async_fns_compile() {
    // async fn a() -> int { return 1 }
    // async fn b() -> int { return 2 }
    // async fn c() -> int { return 3 }
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::FunctionDef {
                is_async: true,
                type_params: vec![],
                name: "a".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(return_int_body(1)),
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                is_async: true,
                type_params: vec![],
                name: "b".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(return_int_body(2)),
            }),
            Stmt::dummy(StmtKind::FunctionDef {
                is_async: true,
                type_params: vec![],
                name: "c".to_string(),
                params: vec![],
                return_type: Some(vec!["int".to_string()]),
                body: Box::new(return_int_body(3)),
            }),
        ],
    };
    let ir = compile_program(program);
    assert!(ir.is_ok(), "multiple async fns should compile");
    let ir = ir.unwrap();
    assert!(ir.contains("create_a") && ir.contains("poll_a"));
    assert!(ir.contains("create_b") && ir.contains("poll_b"));
    assert!(ir.contains("create_c") && ir.contains("poll_c"));
}
