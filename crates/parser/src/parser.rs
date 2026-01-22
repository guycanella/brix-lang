use crate::ast::{BinaryOp, Expr, Literal, Program, Stmt, UnaryOp};
use chumsky::prelude::*;
use lexer::token::Token;

pub fn parser() -> impl Parser<Token, Program, Error = Simple<Token>> {
    let stmt = stmt_parser();

    stmt.repeated()
        .map(|statements| Program { statements })
        .then_ignore(end())
}

fn stmt_parser() -> impl Parser<Token, Stmt, Error = Simple<Token>> {
    recursive(|stmt| {
        let decl = just(Token::Var)
            .to(false)
            .or(just(Token::Const).to(true))
            .then(select! { Token::Identifier(name) => name })
            .then(
                // Path 1: Explicit (: int =)
                just(Token::Colon)
                    .ignore_then(select! { Token::Identifier(t) => t })
                    .then_ignore(just(Token::Eq))
                    .map(Some)
                    // Path 2: Inference (:=)
                    .or(just(Token::ColonEq).to(None)),
            )
            .then(expr_parser())
            .map(
                |(((is_const, name), type_hint), value)| Stmt::VariableDecl {
                    name,
                    type_hint,
                    value,
                    is_const,
                },
            );

        let lvalue = select! { Token::Identifier(name) => Expr::Identifier(name) }
            .then(
                expr_parser()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .map(|idx| (true, idx, String::new()))
                    .or(just(Token::Dot)
                        .ignore_then(select! { Token::Identifier(name) => name })
                        .map(|name| (false, Expr::Identifier("dummy".to_string()), name)))
                    .repeated(),
            )
            .foldl(|lhs, (is_index, index_expr, field_name)| {
                if is_index {
                    match lhs {
                        Expr::Index { array, mut indices } => {
                            indices.push(index_expr);
                            Expr::Index { array, indices }
                        }
                        _ => Expr::Index {
                            array: Box::new(lhs),
                            indices: vec![index_expr],
                        },
                    }
                } else {
                    Expr::FieldAccess {
                        target: Box::new(lhs),
                        field: field_name,
                    }
                }
            });

        let assignment = lvalue
            .then(
                just(Token::Eq)
                    .or(just(Token::ColonEq))
                    .to(None)
                    .or(just(Token::PlusEq).to(Some(BinaryOp::Add)))
                    .or(just(Token::MinusEq).to(Some(BinaryOp::Sub)))
                    .or(just(Token::StarEq).to(Some(BinaryOp::Mul)))
                    .or(just(Token::SlashEq).to(Some(BinaryOp::Div))),
            )
            .then(expr_parser())
            .map(|((target, maybe_op), value)| match maybe_op {
                None => Stmt::Assignment { target, value },
                Some(op) => Stmt::Assignment {
                    target: target.clone(),
                    value: Expr::Binary {
                        op,
                        lhs: Box::new(target),
                        rhs: Box::new(value),
                    },
                },
            });

        let block = stmt
            .clone()
            .repeated()
            .delimited_by(just(Token::LBrace), just(Token::RBrace))
            .map(Stmt::Block);
        let if_stmt = just(Token::If)
            .ignore_then(expr_parser())
            .then(block.clone())
            .then(just(Token::Else).ignore_then(block.clone()).or_not())
            .map(|((c, t), e)| Stmt::If {
                condition: c,
                then_block: Box::new(t),
                else_block: e.map(Box::new),
            });
        let while_stmt = just(Token::While)
            .ignore_then(expr_parser())
            .then(block.clone())
            .map(|(c, b)| Stmt::While {
                condition: c,
                body: Box::new(b),
            });
        let for_stmt = just(Token::For)
            .ignore_then(select! { Token::Identifier(n) => n })
            .then_ignore(just(Token::In))
            .then(expr_parser())
            .then(block.clone())
            .map(|((n, i), b)| Stmt::For {
                var_name: n,
                iterable: i,
                body: Box::new(b),
            });
        let print_stmt = just(Token::Printf)
            .ignore_then(
                select! { Token::String(s) => s }
                    .then(
                        just(Token::Comma)
                            .ignore_then(expr_parser())
                            .repeated()
                            .or_not(),
                    )
                    .delimited_by(just(Token::LParen), just(Token::RParen)),
            )
            .map(|(f, a)| {
                let format = f.trim_matches('"').replace("\\n", "\n").to_string();
                Stmt::Printf {
                    format,
                    args: a.unwrap_or_default(),
                }
            });
        let expr_stmt = expr_parser().map(Stmt::Expr);

        decl.or(assignment)
            .or(if_stmt)
            .or(while_stmt)
            .or(for_stmt)
            .or(print_stmt)
            .or(block)
            .or(expr_stmt)
            .boxed()
    })
}

fn expr_parser() -> impl Parser<Token, Expr, Error = Simple<Token>> {
    recursive(|expr| {
        let val = select! {
            Token::Int(n) => Expr::Literal(Literal::Int(n)),
            Token::Float(s) => Expr::Literal(Literal::Float(s.parse().unwrap())),
            Token::String(s) => Expr::Literal(Literal::String(s.trim_matches('"').to_string())),
            Token::True => Expr::Literal(Literal::Bool(true)),
            Token::False => Expr::Literal(Literal::Bool(false)),
            Token::Identifier(s) => Expr::Identifier(s),
        };

        let array_literal = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map(Expr::Array);

        let atom = val.or(array_literal).or(expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen)));

        let call = atom
            .clone()
            .then(
                expr.clone()
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    .or_not(),
            )
            .map(|(func, maybe_args)| match maybe_args {
                Some(args) => Expr::Call {
                    func: Box::new(func),
                    args,
                },
                None => func,
            });

        // Unary operators (!, not, -)
        let unary = just(Token::Not)
            .to(UnaryOp::Not)
            .or(just(Token::Minus).to(UnaryOp::Negate))
            .repeated()
            .then(call.clone())
            .foldr(|op, expr| Expr::Unary {
                op,
                expr: Box::new(expr),
            });

        let index_or_field = unary
            .clone()
            .then(
                expr.clone()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .map(|idx| (true, idx, String::new()))
                    .or(just(Token::Dot)
                        .ignore_then(select! { Token::Identifier(name) => name })
                        .map(|name| (false, Expr::Identifier("dummy".to_string()), name)))
                    .repeated(),
            )
            .foldl(|lhs, (is_index, expr_arg, field_name)| {
                if is_index {
                    match lhs {
                        Expr::Index { array, mut indices } => {
                            indices.push(expr_arg);
                            Expr::Index { array, indices }
                        }
                        _ => Expr::Index {
                            array: Box::new(lhs),
                            indices: vec![expr_arg],
                        },
                    }
                } else {
                    Expr::FieldAccess {
                        target: Box::new(lhs),
                        field: field_name,
                    }
                }
            })
            .boxed();

        let power = index_or_field
            .clone()
            .then(
                just(Token::Pow)
                    .to(BinaryOp::Pow)
                    .then(index_or_field)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        let product = power
            .clone()
            .then(
                just(Token::Star)
                    .to(BinaryOp::Mul)
                    .or(just(Token::Slash).to(BinaryOp::Div))
                    .or(just(Token::Percent).to(BinaryOp::Mod))
                    .then(power)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        let sum = product
            .clone()
            .then(
                just(Token::Plus)
                    .to(BinaryOp::Add)
                    .or(just(Token::Minus).to(BinaryOp::Sub))
                    .then(product)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        let bitwise = sum
            .clone()
            .then(
                just(Token::Ampersand)
                    .to(BinaryOp::BitAnd)
                    .or(just(Token::Pipe).to(BinaryOp::BitOr))
                    .or(just(Token::Caret).to(BinaryOp::BitXor))
                    .then(sum)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        let comparison = bitwise
            .clone()
            .then(
                choice((
                    just(Token::DoubleEq).to(BinaryOp::Eq),
                    just(Token::NotEq).to(BinaryOp::NotEq),
                    just(Token::Gt).to(BinaryOp::Gt),
                    just(Token::Lt).to(BinaryOp::Lt),
                    just(Token::GtEq).to(BinaryOp::GtEq),
                    just(Token::LtEq).to(BinaryOp::LtEq),
                ))
                .then(bitwise.clone())
                .repeated(),
            )
            .map(|(lhs, pairs)| {
                if pairs.is_empty() {
                    return lhs;
                }

                if pairs.len() == 1 {
                    let (op, rhs) = pairs[0].clone();
                    return Expr::Binary {
                        op,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }

                // Chained Comparison: 1 <= n <= 10  ->  (1 <= n) && (n <= 10)
                let (first_op, first_rhs) = pairs[0].clone();

                let mut final_expr = Expr::Binary {
                    op: first_op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(first_rhs.clone()),
                };

                let mut prev_rhs = first_rhs;

                for (op, rhs) in pairs.into_iter().skip(1) {
                    let next_comparison = Expr::Binary {
                        op,
                        lhs: Box::new(prev_rhs.clone()),
                        rhs: Box::new(rhs.clone()),
                    };

                    final_expr = Expr::Binary {
                        op: BinaryOp::LogicalAnd,
                        lhs: Box::new(final_expr),
                        rhs: Box::new(next_comparison),
                    };

                    prev_rhs = rhs;
                }

                final_expr
            })
            .boxed();

        // 8. Logic AND (&& or and)
        let logic_and = comparison
            .clone()
            .then(
                just(Token::And)
                    .to(BinaryOp::LogicalAnd)
                    .then(comparison)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        // 9. Logic OR (|| or or)
        let logic_or = logic_and
            .clone()
            .then(
                just(Token::Or)
                    .to(BinaryOp::LogicalOr)
                    .then(logic_and)
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
            .boxed();

        // 10. Range (1:10 or 1:2:10)
        let range_end_parser = logic_or.clone();
        let range_step_parser = logic_or.clone();

        let range = logic_or
            .clone()
            .then(
                just(Token::Colon)
                    .ignore_then(range_end_parser)
                    .then(just(Token::Colon).ignore_then(range_step_parser).or_not())
                    .or_not(),
            )
            .map(|(start, maybe_rest)| match maybe_rest {
                None => start, // Is not range
                Some((second, third_opt)) => match third_opt {
                    // start:end
                    None => Expr::Range {
                        start: Box::new(start),
                        end: Box::new(second),
                        step: None,
                    },
                    // start:step:end
                    Some(end) => Expr::Range {
                        start: Box::new(start),
                        end: Box::new(end),
                        step: Some(Box::new(second)),
                    },
                },
            });

        // 11. Ternary (condition ? true_expr : false_expr)
        // Use logic_or for branches to avoid conflict with range's colon
        let ternary = range
            .clone()
            .then(
                just(Token::Question)
                    .ignore_then(logic_or.clone())
                    .then_ignore(just(Token::Colon))
                    .then(logic_or.clone())
                    .or_not(),
            )
            .map(|(condition, maybe_branches)| match maybe_branches {
                None => condition,
                Some((then_expr, else_expr)) => Expr::Ternary {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                },
            });

        ternary.boxed()
    })
}
