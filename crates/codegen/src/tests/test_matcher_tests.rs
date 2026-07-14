use crate::{CodegenError, Compiler};
use inkwell::context::Context;
use parser::ast::{Expr, ExprKind, Literal, Program, Stmt, StmtKind, StructDef};

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
