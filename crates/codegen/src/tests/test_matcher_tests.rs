use crate::{CodegenError, Compiler};
use inkwell::context::Context;
use parser::ast::{Closure, Expr, ExprKind, Literal, Program, Stmt, StmtKind, StructDef};

fn compile_program(program: Program) -> Result<String, CodegenError> {
    let context = Context::create();
    let module = context.create_module("test_matchers");
    let builder = context.create_builder();
    let mut compiler = Compiler::new(
        &context,
        &builder,
        &module,
        "test_matchers.bx".to_string(),
        "".to_string(),
    );

    compiler.compile_program(&program)?;
    Ok(module.print_to_string().to_string())
}

fn expr(kind: ExprKind) -> Expr {
    Expr::dummy(kind)
}

fn stmt_expr(expr: Expr) -> Stmt {
    Stmt::dummy(StmtKind::Expr(expr))
}

fn lit_int(value: i64) -> Expr {
    expr(ExprKind::Literal(Literal::Int(value)))
}

fn lit_str(value: &str) -> Expr {
    expr(ExprKind::Literal(Literal::String(value.to_string())))
}

fn ident(name: &str) -> Expr {
    expr(ExprKind::Identifier(name.to_string()))
}

fn field(target: Expr, name: &str) -> Expr {
    expr(ExprKind::FieldAccess {
        target: Box::new(target),
        field: name.to_string(),
    })
}

fn call(func: Expr, args: Vec<Expr>) -> Expr {
    expr(ExprKind::Call {
        func: Box::new(func),
        args,
    })
}

fn test_expect(actual: Expr) -> Expr {
    call(field(ident("test"), "expect"), vec![actual])
}

fn matcher_call(actual: Expr, negated: bool, matcher: &str, args: Vec<Expr>) -> Expr {
    let target = if negated {
        field(test_expect(actual), "not")
    } else {
        test_expect(actual)
    };
    call(field(target, matcher), args)
}

fn point_struct_def() -> Stmt {
    Stmt::dummy(StmtKind::StructDef(StructDef {
        name: "Point".to_string(),
        type_params: vec![],
        fields: vec![
            ("x".to_string(), "int".to_string(), None),
            ("y".to_string(), "int".to_string(), None),
        ],
    }))
}

fn point_init() -> Expr {
    expr(ExprKind::StructInit {
        struct_name: "Point".to_string(),
        type_args: vec![],
        fields: vec![("x".to_string(), lit_int(1)), ("y".to_string(), lit_int(2))],
    })
}

// ── Grupo H (toThrow) helpers ──────────────────────────────────────────

/// A synchronous, zero-parameter closure literal whose body calls
/// `panic("boom")` and then returns 0. This is the only shape `toThrow`
/// supports (v1.7 Grupo H's intentionally restricted scope).
fn panicking_closure_literal() -> Expr {
    expr(ExprKind::Closure(Closure {
        params: vec![],
        return_type: Some("int".to_string()),
        body: Box::new(Stmt::dummy(StmtKind::Block(vec![
            stmt_expr(call(ident("panic"), vec![lit_str("boom")])),
            Stmt::dummy(StmtKind::Return {
                values: vec![lit_int(0)],
            }),
        ]))),
        captured_vars: vec![],
        is_async: false,
    }))
}

fn non_panicking_void_closure_literal() -> Expr {
    expr(ExprKind::Closure(Closure {
        params: vec![],
        return_type: None,
        body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(
            StmtKind::VariableDecl {
                name: "x".to_string(),
                type_hint: None,
                value: expr(ExprKind::Binary {
                    op: parser::ast::BinaryOp::Add,
                    lhs: Box::new(lit_int(1)),
                    rhs: Box::new(lit_int(1)),
                }),
                is_const: false,
            },
        )]))),
        captured_vars: vec![],
        is_async: false,
    }))
}

fn closure_literal_with_params() -> Expr {
    expr(ExprKind::Closure(Closure {
        params: vec![("x".to_string(), "int".to_string())],
        return_type: Some("int".to_string()),
        body: Box::new(Stmt::dummy(StmtKind::Block(vec![Stmt::dummy(
            StmtKind::Return {
                values: vec![ident("x")],
            },
        )]))),
        captured_vars: vec![],
        is_async: false,
    }))
}

#[test]
fn test_not_to_match_emits_not_runtime_function() {
    let program = Program {
        statements: vec![stmt_expr(matcher_call(
            lit_str("hello123"),
            true,
            "toMatch",
            vec![lit_str("bye*")],
        ))],
    };

    let ir = compile_program(program).expect("not.toMatch should compile");
    assert!(
        ir.contains("test_expect_not_matches_string"),
        "expected not.toMatch to emit the negated runtime matcher, got IR:\n{}",
        ir
    );
}

#[test]
fn test_not_to_have_property_emits_not_runtime_function() {
    let program = Program {
        statements: vec![
            point_struct_def(),
            stmt_expr(matcher_call(
                point_init(),
                true,
                "toHaveProperty",
                vec![lit_str("z")],
            )),
        ],
    };

    let ir = compile_program(program).expect("not.toHaveProperty should compile");
    assert!(
        ir.contains("test_expect_not_has_property"),
        "expected not.toHaveProperty to emit the negated runtime matcher, got IR:\n{}",
        ir
    );
}

#[test]
fn test_to_have_property_rejects_non_literal_property_name() {
    let program = Program {
        statements: vec![
            point_struct_def(),
            Stmt::dummy(StmtKind::VariableDecl {
                name: "prop".to_string(),
                type_hint: None,
                value: lit_str("x"),
                is_const: false,
            }),
            stmt_expr(matcher_call(
                point_init(),
                false,
                "toHaveProperty",
                vec![ident("prop")],
            )),
        ],
    };

    let err = compile_program(program).expect_err("non-literal property name should be rejected");
    assert!(
        matches!(err, CodegenError::TypeError { ref context, .. } if context == "toHaveProperty property name"),
        "expected toHaveProperty property-name TypeError, got: {:?}",
        err
    );
}

#[test]
fn test_string_matchers_reject_non_string_receiver() {
    let program = Program {
        statements: vec![stmt_expr(matcher_call(
            lit_int(123),
            false,
            "toStartWith",
            vec![lit_str("1")],
        ))],
    };

    let err = compile_program(program).expect_err("toStartWith on int should be rejected");
    assert!(
        matches!(err, CodegenError::TypeError { ref context, .. } if context == "toStartWith"),
        "expected toStartWith TypeError, got: {:?}",
        err
    );
}

#[test]
fn test_string_matchers_reject_non_string_argument() {
    let program = Program {
        statements: vec![stmt_expr(matcher_call(
            lit_str("hello"),
            false,
            "toMatch",
            vec![lit_int(1)],
        ))],
    };

    let err = compile_program(program).expect_err("toMatch with int pattern should be rejected");
    assert!(
        matches!(err, CodegenError::TypeError { ref context, .. } if context == "toMatch"),
        "expected toMatch TypeError, got: {:?}",
        err
    );
}

#[test]
fn test_string_matchers_reject_missing_argument() {
    let program = Program {
        statements: vec![stmt_expr(matcher_call(
            lit_str("hello"),
            false,
            "toEndWith",
            vec![],
        ))],
    };

    let err = compile_program(program).expect_err("toEndWith without suffix should be rejected");
    assert!(
        matches!(err, CodegenError::InvalidOperation { ref operation, .. } if operation == "toEndWith"),
        "expected toEndWith InvalidOperation, got: {:?}",
        err
    );
}

// ==========================================
// v1.7 Grupo H: toThrow (restricted scope)
// ==========================================

#[test]
fn test_to_throw_on_sync_zero_param_closure_literal_emits_fork_dance() {
    let program = Program {
        statements: vec![stmt_expr(matcher_call(
            panicking_closure_literal(),
            false,
            "toThrow",
            vec![],
        ))],
    };

    let ir = compile_program(program)
        .expect("toThrow on a sync, zero-parameter closure literal should compile");
    assert!(
        ir.contains("@fork"),
        "expected toThrow to declare/call fork(), got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("@fflush"),
        "expected toThrow to flush stdio before forking, got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("@_exit"),
        "expected toThrow's child branch to call _exit(), got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("@brix_wait_for_child"),
        "expected toThrow's parent branch to call brix_wait_for_child(), got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("test_expect_to_throw"),
        "expected toThrow to dispatch to test_expect_to_throw, got IR:\n{}",
        ir
    );
}

#[test]
fn test_not_to_throw_dispatches_to_negated_runtime_function() {
    let program = Program {
        statements: vec![stmt_expr(matcher_call(
            non_panicking_void_closure_literal(),
            true,
            "toThrow",
            vec![],
        ))],
    };

    let ir = compile_program(program).expect("not.toThrow should compile");
    assert!(
        ir.contains("test_expect_not_to_throw"),
        "expected not.toThrow to emit the negated runtime matcher, got IR:\n{}",
        ir
    );
    assert!(
        !ir.contains("@test_expect_to_throw("),
        "expected not.toThrow NOT to also declare the non-negated matcher, got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("define void @__closure_0"),
        "expected no-return closure literal to be declared void, got IR:\n{}",
        ir
    );
    assert!(
        ir.contains("call void %iter_fn_ptr"),
        "expected toThrow to call the void closure as void, got IR:\n{}",
        ir
    );
    assert!(
        !ir.contains("call i64 %iter_fn_ptr"),
        "toThrow must not call a void closure as i64, got IR:\n{}",
        ir
    );
}

#[test]
fn test_to_throw_rejects_variable_instead_of_closure_literal() {
    // var f := () -> int { panic("boom"); return 0 }
    // test.expect(f).toThrow()   ← f is an identifier, not a closure literal
    let program = Program {
        statements: vec![
            Stmt::dummy(StmtKind::VariableDecl {
                name: "f".to_string(),
                type_hint: None,
                value: panicking_closure_literal(),
                is_const: false,
            }),
            stmt_expr(matcher_call(ident("f"), false, "toThrow", vec![])),
        ],
    };

    let result = compile_program(program);
    assert!(
        result.is_err(),
        "toThrow on a variable (not a closure literal) should be rejected, got: {:?}",
        result
    );
}

#[test]
fn test_to_throw_rejects_closure_with_parameters() {
    let program = Program {
        statements: vec![stmt_expr(matcher_call(
            closure_literal_with_params(),
            false,
            "toThrow",
            vec![],
        ))],
    };

    let result = compile_program(program);
    assert!(
        result.is_err(),
        "toThrow on a closure with parameters should be rejected, got: {:?}",
        result
    );
}

#[test]
fn test_to_throw_rejects_argument() {
    let program = Program {
        statements: vec![stmt_expr(matcher_call(
            panicking_closure_literal(),
            false,
            "toThrow",
            vec![lit_int(1)],
        ))],
    };

    let result = compile_program(program);
    assert!(
        result.is_err(),
        "toThrow(arg) should be rejected — toThrow takes no arguments, got: {:?}",
        result
    );
}

// ==========================================
// Regression: compile_closure() return-type inference
// (found while verifying Grupo H's toThrow, but affects ANY unannotated
// closure — e.g. .map() callbacks — not specific to toThrow)
// ==========================================

#[test]
fn test_closure_without_return_annotation_gets_correctly_typed_function() {
    // m.map((x: float) -> { return x * 2.0 })
    //
    // compile_closure() used to declare EVERY closure without an explicit
    // `-> type` annotation as an LLVM `void`-returning function, regardless
    // of what its body actually returned — while every call site (.map(),
    // toThrow) infers the real return type independently and builds its
    // indirect-call fn_type from THAT instead. The function's own
    // `define void` header disagreed with its `ret <value>` body and with
    // how callers called it — undefined behavior at the LLVM IR level that
    // happened to still produce correct results in practice only because
    // caller and body agreed with each other while the declaration lied.
    // compile_closure() must infer the same type its callers already assume.
    let arr = expr(ExprKind::Array(vec![expr(ExprKind::Literal(
        Literal::Float(1.0),
    ))]));
    let callback = expr(ExprKind::Closure(Closure {
        params: vec![("x".to_string(), "float".to_string())],
        return_type: None,
        body: Box::new(Stmt::dummy(StmtKind::Return {
            values: vec![expr(ExprKind::Binary {
                op: parser::ast::BinaryOp::Mul,
                lhs: Box::new(ident("x")),
                rhs: Box::new(expr(ExprKind::Literal(Literal::Float(2.0)))),
            })],
        })),
        captured_vars: vec![],
        is_async: false,
    }));
    let map_call = call(field(arr, "map"), vec![callback]);
    let program = Program {
        statements: vec![stmt_expr(map_call)],
    };

    let ir = compile_program(program)
        .expect("map with an unannotated float-returning closure should compile");
    assert!(
        ir.contains("define double @__closure_0"),
        "expected the closure to be declared returning double (matching its \
         `return x * 2.0` body and .map()'s call site), got IR:\n{}",
        ir
    );
    assert!(
        !ir.contains("define void @__closure_0"),
        "closure must not be declared void when its body returns a value, got IR:\n{}",
        ir
    );
}
