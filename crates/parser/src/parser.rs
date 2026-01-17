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
    // Declaration: var x := ...
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

    // Assignment: x = ...
    let assignment = select! { Token::Identifier(name) => name }
        .then_ignore(just(Token::Eq).or(just(Token::ColonEq)))
        .then(expr_parser())
        .map(|(target, value)| Stmt::Assignment { target, value });

    // Expression statement
    let expr_stmt = expr_parser().map(Stmt::Expr);

    decl.or(assignment).or(expr_stmt)
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

        let array = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map(Expr::Array);

        let atom = val
            .or(array)
            .or(expr.delimited_by(just(Token::LParen), just(Token::RParen)));

        // 2. Power (**). Highest precedence.
        // Note: Mathematical power is usually right-associative (2^3^4 = 2^(3^4)),
        // but for simplicity in v0.1 we are keeping it left-associative like others.
        let power = atom
            .clone()
            .then(just(Token::Pow).to(BinaryOp::Pow).then(atom).repeated())
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });

        // 3. Product (*, /, %) - Operates on 'power', not 'atom'
        let product = power // <--- Change here: consumes 'power'
            .clone()
            .then(
                just(Token::Star)
                    .to(BinaryOp::Mul)
                    .or(just(Token::Slash).to(BinaryOp::Div))
                    .or(just(Token::Percent).to(BinaryOp::Mod))
                    .then(power) // <--- and here
                    .repeated(),
            )
            .foldl(|lhs, (op, rhs)| Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });

        // 4. Sum (+, -) - Operates on 'product'
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

        // 5. Bitwise (&, |, ^) - Lower precedence than Sum (Standard C/Rust behavior)
        // ex: 5 + 1 & 2  becomes  (5+1) & 2 = 6 & 2
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

        // 6. Comparison (==, <, >) - Lowest precedence. Operates on 'bitwise'
        bitwise
            .clone() // <--- Change here: consumes 'bitwise'
            .then(
                just(Token::DoubleEq)
                    .to(BinaryOp::Eq)
                    .or(just(Token::NotEq).to(BinaryOp::NotEq))
                    .or(just(Token::Gt).to(BinaryOp::Gt))
                    .or(just(Token::Lt).to(BinaryOp::Lt))
                    .or(just(Token::GtEq).to(BinaryOp::GtEq))
                    .or(just(Token::LtEq).to(BinaryOp::LtEq))
                    .then(bitwise) // <--- and here
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
