use crate::ast::{BinaryOp, Expr, Literal, Program, Stmt};
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
            .then_ignore(just(Token::ColonEq))
            .then(expr_parser())
            .map(|((is_const, name), value)| Stmt::VariableDecl {
                name,
                value,
                is_const,
            });

        // --- Assignment (x = 10 or x += 10) ---
        let assignment = select! { Token::Identifier(name) => name }
            .then(
                // Option A: Simple assignment "=" or ":="
                just(Token::Eq)
                    .or(just(Token::ColonEq))
                    .to(None)
                    // Option B: Compound assignment "+=", "-=", etc.
                    .or(just(Token::PlusEq).to(Some(BinaryOp::Add)))
                    .or(just(Token::MinusEq).to(Some(BinaryOp::Sub)))
                    .or(just(Token::StarEq).to(Some(BinaryOp::Mul)))
                    .or(just(Token::SlashEq).to(Some(BinaryOp::Div))),
            )
            .then(expr_parser())
            .map(|((name, maybe_op), value)| {
                match maybe_op {
                    // Case 1: Simple assignment (x = 10)
                    None => Stmt::Assignment {
                        target: name,
                        value,
                    },

                    // Case 2: Compound assignment (x += 10)
                    Some(op) => Stmt::Assignment {
                        target: name.clone(),
                        value: Expr::Binary {
                            op,
                            lhs: Box::new(Expr::Identifier(name)),
                            rhs: Box::new(value),
                        },
                    },
                }
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
            .map(|((condition, then_block), else_block)| Stmt::If {
                condition,
                then_block: Box::new(then_block),
                else_block: else_block.map(Box::new),
            });

        let while_stmt = just(Token::While)
            .ignore_then(expr_parser())
            .then(block.clone())
            .map(|(condition, body)| Stmt::While {
                condition,
                body: Box::new(body),
            });

        // --- For Loop (for i in 1:10) ---
        let for_stmt = just(Token::For)
            .ignore_then(select! { Token::Identifier(name) => name })
            .then_ignore(just(Token::In))
            .then(expr_parser())
            .then(block.clone())
            .map(|((var_name, iterable), body)| Stmt::For {
                var_name,
                iterable,
                body: Box::new(body),
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
            .map(|(format_raw, args)| {
                let format = format_raw
                    .trim_matches('"')
                    .replace("\\n", "\n")
                    .to_string();

                Stmt::Printf {
                    format,
                    args: args.unwrap_or_default(),
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

        let index_or_field = call
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
            .foldl(|lhs, (is_index, index_expr, field_name)| {
                if is_index {
                    Expr::Index {
                        array: Box::new(lhs),
                        index: Box::new(index_expr),
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

        range.boxed()
    })
}
