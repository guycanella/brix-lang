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

        let assignment = select! { Token::Identifier(name) => name }
            .then_ignore(just(Token::Eq).or(just(Token::ColonEq)))
            .then(expr_parser())
            .map(|(target, value)| Stmt::Assignment { target, value });

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

        let expr_stmt = expr_parser().map(Stmt::Expr);

        decl.or(assignment).or(if_stmt).or(block).or(expr_stmt)
    })
}

fn expr_parser() -> impl Parser<Token, Expr, Error = Simple<Token>> {
    recursive(|expr| {
        // 1. Atoms
        let val = select! {
            Token::Int(n) => Expr::Literal(Literal::Int(n)),
            Token::Float(s) => Expr::Literal(Literal::Float(s.parse().unwrap())),
            Token::String(s) => Expr::Literal(Literal::String(s)),
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

        let index = atom
            .clone()
            .then(
                expr.clone()
                    .delimited_by(just(Token::LBracket), just(Token::RBracket))
                    .repeated(),
            )
            .foldl(|lhs, index_expr| Expr::Index {
                array: Box::new(lhs),
                index: Box::new(index_expr),
            });

        let power = index
            .clone()
            .then(just(Token::Pow).to(BinaryOp::Pow).then(index).repeated())
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });

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
            });

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
            });

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
            });

        bitwise
            .clone()
            .then(
                just(Token::DoubleEq)
                    .to(BinaryOp::Eq)
                    .or(just(Token::NotEq).to(BinaryOp::NotEq))
                    .or(just(Token::Gt).to(BinaryOp::Gt))
                    .or(just(Token::Lt).to(BinaryOp::Lt))
                    .or(just(Token::GtEq).to(BinaryOp::GtEq))
                    .or(just(Token::LtEq).to(BinaryOp::LtEq))
                    .then(bitwise)
                    .or_not(),
            )
            .map(|(lhs, maybe_op)| match maybe_op {
                None => lhs,
                Some((op, rhs)) => Expr::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
            })
    })
}
