/// Closure Capture Analysis
///
/// Identifies which variables are captured by closures.
/// This must run AFTER parsing to fill the `captured_vars` field in Closure nodes.

use crate::ast::{Expr, ExprKind, Stmt, StmtKind, Closure, Program};
use std::collections::HashSet;

/// Analyze all closures in the program and fill their `captured_vars` fields
pub fn analyze_closures(program: &mut Program) {
    let mut program_scope = HashSet::new();

    for stmt in &mut program.statements {
        analyze_stmt_closures(stmt, &program_scope);

        // Add variables declared in this statement to the program scope
        // so subsequent closures can capture them
        if let StmtKind::VariableDecl { name, .. } = &stmt.kind {
            program_scope.insert(name.clone());
        } else if let StmtKind::DestructuringDecl { names, .. } = &stmt.kind {
            for name in names {
                program_scope.insert(name.clone());
            }
        }
    }
}

/// Recursively analyze statements for closures
fn analyze_stmt_closures(stmt: &mut Stmt, outer_scope: &HashSet<String>) {
    match &mut stmt.kind {
        StmtKind::VariableDecl { name, value, .. } => {
            // Analyze the value expression
            analyze_expr_closures(value, outer_scope);

            // Add this variable to scope for nested closures
            let mut new_scope = outer_scope.clone();
            new_scope.insert(name.clone());
            // (no nested statements in value)
        }

        StmtKind::DestructuringDecl { names, value, .. } => {
            analyze_expr_closures(value, outer_scope);

            let mut new_scope = outer_scope.clone();
            for name in names {
                new_scope.insert(name.clone());
            }
        }

        StmtKind::Assignment { target, value } => {
            analyze_expr_closures(target, outer_scope);
            analyze_expr_closures(value, outer_scope);
        }

        StmtKind::Block(stmts) => {
            let mut block_scope = outer_scope.clone();
            for stmt in stmts {
                analyze_stmt_closures(stmt, &block_scope);

                // Add variables declared in this statement to scope
                if let StmtKind::VariableDecl { name, .. } = &stmt.kind {
                    block_scope.insert(name.clone());
                } else if let StmtKind::DestructuringDecl { names, .. } = &stmt.kind {
                    for name in names {
                        block_scope.insert(name.clone());
                    }
                }
            }
        }

        StmtKind::If { condition, then_block, else_block } => {
            analyze_expr_closures(condition, outer_scope);
            analyze_stmt_closures(then_block, outer_scope);
            if let Some(else_b) = else_block {
                analyze_stmt_closures(else_b, outer_scope);
            }
        }

        StmtKind::While { condition, body } => {
            analyze_expr_closures(condition, outer_scope);
            analyze_stmt_closures(body, outer_scope);
        }

        StmtKind::For { var_names, iterable, body } => {
            analyze_expr_closures(iterable, outer_scope);

            // Loop variables are in scope inside the body
            let mut loop_scope = outer_scope.clone();
            for var_name in var_names {
                loop_scope.insert(var_name.clone());
            }
            analyze_stmt_closures(body, &loop_scope);
        }

        StmtKind::FunctionDef { params, body, .. } => {
            // Function parameters are in scope
            let mut func_scope = outer_scope.clone();
            for (param_name, _, _) in params {
                func_scope.insert(param_name.clone());
            }
            analyze_stmt_closures(body, &func_scope);
        }

        StmtKind::MethodDef(method_def) => {
            // Receiver and parameters are in scope
            let mut method_scope = outer_scope.clone();
            method_scope.insert(method_def.receiver_name.clone());
            for (param_name, _, _) in &method_def.params {
                method_scope.insert(param_name.clone());
            }
            analyze_stmt_closures(&mut method_def.body, &method_scope);
        }

        StmtKind::Return { values } => {
            for value in values {
                analyze_expr_closures(value, outer_scope);
            }
        }

        StmtKind::Expr(expr) => {
            analyze_expr_closures(expr, outer_scope);
        }

        StmtKind::Import { .. }
        | StmtKind::TypeAlias { .. }
        | StmtKind::Printf { .. }
        | StmtKind::Print { .. }
        | StmtKind::Println { .. }
        | StmtKind::StructDef(_) => {
            // These don't contain closures or affect scope
        }
    }
}

/// Recursively analyze expressions for closures
fn analyze_expr_closures(expr: &mut Expr, outer_scope: &HashSet<String>) {
    match &mut expr.kind {
        ExprKind::Closure(closure) => {
            // This is a closure! Analyze its captured variables
            analyze_closure(closure, outer_scope);
        }

        ExprKind::Binary { lhs, rhs, .. } => {
            analyze_expr_closures(lhs, outer_scope);
            analyze_expr_closures(rhs, outer_scope);
        }

        ExprKind::Unary { expr: inner, .. } => {
            analyze_expr_closures(inner, outer_scope);
        }

        ExprKind::Ternary { condition, then_expr, else_expr } => {
            analyze_expr_closures(condition, outer_scope);
            analyze_expr_closures(then_expr, outer_scope);
            analyze_expr_closures(else_expr, outer_scope);
        }

        ExprKind::Increment { expr: inner, .. }
        | ExprKind::Decrement { expr: inner, .. } => {
            analyze_expr_closures(inner, outer_scope);
        }

        ExprKind::FString { parts } => {
            for part in parts {
                if let crate::ast::FStringPart::Expr { expr: inner, .. } = part {
                    analyze_expr_closures(inner, outer_scope);
                }
            }
        }

        ExprKind::Array(exprs) => {
            for e in exprs {
                analyze_expr_closures(e, outer_scope);
            }
        }

        ExprKind::Index { array, indices } => {
            analyze_expr_closures(array, outer_scope);
            for idx in indices {
                analyze_expr_closures(idx, outer_scope);
            }
        }

        ExprKind::Call { func, args } | ExprKind::GenericCall { func, args, .. } => {
            analyze_expr_closures(func, outer_scope);
            for arg in args {
                analyze_expr_closures(arg, outer_scope);
            }
        }

        ExprKind::FieldAccess { target, .. } => {
            analyze_expr_closures(target, outer_scope);
        }

        ExprKind::Match { value, arms } => {
            analyze_expr_closures(value, outer_scope);
            for arm in arms {
                // Guard expression
                if let Some(guard) = &mut arm.guard {
                    analyze_expr_closures(guard, outer_scope);
                }
                // Body expression
                analyze_expr_closures(&mut arm.body, outer_scope);
            }
        }

        ExprKind::StructInit { fields, .. } => {
            for (_, value) in fields {
                analyze_expr_closures(value, outer_scope);
            }
        }

        ExprKind::Range { start, end, step } => {
            analyze_expr_closures(start, outer_scope);
            analyze_expr_closures(end, outer_scope);
            if let Some(s) = step {
                analyze_expr_closures(s, outer_scope);
            }
        }

        ExprKind::ListComprehension { expr: comp_expr, generators } => {
            // List comprehensions introduce their own scope
            let mut comp_scope = outer_scope.clone();

            for generator in generators {
                // Analyze the iterable with current scope
                analyze_expr_closures(&mut generator.iterable, &comp_scope);

                // Add loop variables to scope
                for var_name in &generator.var_names {
                    comp_scope.insert(var_name.clone());
                }

                // Analyze conditions
                for cond in &mut generator.conditions {
                    analyze_expr_closures(cond, &comp_scope);
                }
            }

            // Analyze the expression with accumulated scope
            analyze_expr_closures(comp_expr, &comp_scope);
        }

        ExprKind::StaticInit { dimensions, .. } => {
            for dim in dimensions {
                analyze_expr_closures(dim, outer_scope);
            }
        }

        ExprKind::Literal(_) | ExprKind::Identifier(_) => {
            // No nested closures here
        }
    }
}

/// Analyze a single closure and fill its captured_vars field
fn analyze_closure(closure: &mut Closure, outer_scope: &HashSet<String>) {
    // Build the closure's local scope (parameters ONLY, not outer scope)
    let mut closure_scope = HashSet::new();
    for (param_name, _) in &closure.params {
        closure_scope.insert(param_name.clone());
    }

    // Find all identifiers used in the closure body
    let mut used_vars = HashSet::new();
    collect_used_identifiers(&closure.body, &mut used_vars);

    // Captured variables = used variables that are NOT in closure's local scope
    let mut captured = Vec::new();
    for var in used_vars {
        if !closure_scope.contains(&var) && outer_scope.contains(&var) {
            captured.push(var);
        }
    }

    // Sort for deterministic output
    captured.sort();
    closure.captured_vars = captured;
}

/// Collect all identifiers used in a statement
fn collect_used_identifiers(stmt: &Stmt, used: &mut HashSet<String>) {
    match &stmt.kind {
        StmtKind::VariableDecl { value, .. } => {
            collect_used_identifiers_expr(value, used);
        }

        StmtKind::DestructuringDecl { value, .. } => {
            collect_used_identifiers_expr(value, used);
        }

        StmtKind::Assignment { target, value } => {
            collect_used_identifiers_expr(target, used);
            collect_used_identifiers_expr(value, used);
        }

        StmtKind::Block(stmts) => {
            for s in stmts {
                collect_used_identifiers(s, used);
            }
        }

        StmtKind::If { condition, then_block, else_block } => {
            collect_used_identifiers_expr(condition, used);
            collect_used_identifiers(then_block, used);
            if let Some(else_b) = else_block {
                collect_used_identifiers(else_b, used);
            }
        }

        StmtKind::While { condition, body } => {
            collect_used_identifiers_expr(condition, used);
            collect_used_identifiers(body, used);
        }

        StmtKind::For { iterable, body, .. } => {
            collect_used_identifiers_expr(iterable, used);
            collect_used_identifiers(body, used);
        }

        StmtKind::Return { values } => {
            for v in values {
                collect_used_identifiers_expr(v, used);
            }
        }

        StmtKind::Expr(expr) => {
            collect_used_identifiers_expr(expr, used);
        }

        StmtKind::Printf { args, .. } => {
            for arg in args {
                collect_used_identifiers_expr(arg, used);
            }
        }

        StmtKind::Print { expr } | StmtKind::Println { expr } => {
            collect_used_identifiers_expr(expr, used);
        }

        _ => {}
    }
}

/// Collect all identifiers used in an expression
fn collect_used_identifiers_expr(expr: &Expr, used: &mut HashSet<String>) {
    match &expr.kind {
        ExprKind::Identifier(name) => {
            used.insert(name.clone());
        }

        ExprKind::Binary { lhs, rhs, .. } => {
            collect_used_identifiers_expr(lhs, used);
            collect_used_identifiers_expr(rhs, used);
        }

        ExprKind::Unary { expr: inner, .. } => {
            collect_used_identifiers_expr(inner, used);
        }

        ExprKind::Ternary { condition, then_expr, else_expr } => {
            collect_used_identifiers_expr(condition, used);
            collect_used_identifiers_expr(then_expr, used);
            collect_used_identifiers_expr(else_expr, used);
        }

        ExprKind::Array(exprs) => {
            for e in exprs {
                collect_used_identifiers_expr(e, used);
            }
        }

        ExprKind::Index { array, indices } => {
            collect_used_identifiers_expr(array, used);
            for idx in indices {
                collect_used_identifiers_expr(idx, used);
            }
        }

        ExprKind::Call { func, args } | ExprKind::GenericCall { func, args, .. } => {
            collect_used_identifiers_expr(func, used);
            for arg in args {
                collect_used_identifiers_expr(arg, used);
            }
        }

        ExprKind::FieldAccess { target, .. } => {
            collect_used_identifiers_expr(target, used);
        }

        ExprKind::StructInit { fields, .. } => {
            for (_, value) in fields {
                collect_used_identifiers_expr(value, used);
            }
        }

        ExprKind::Range { start, end, step } => {
            collect_used_identifiers_expr(start, used);
            collect_used_identifiers_expr(end, used);
            if let Some(s) = step {
                collect_used_identifiers_expr(s, used);
            }
        }

        ExprKind::ListComprehension { expr: comp_expr, generators } => {
            for generator in generators {
                collect_used_identifiers_expr(&generator.iterable, used);
                for cond in &generator.conditions {
                    collect_used_identifiers_expr(cond, used);
                }
            }
            collect_used_identifiers_expr(comp_expr, used);
        }

        _ => {}
    }
}
