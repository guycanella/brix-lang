// crates/parser/src/ast.rs

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    BitAnd,
    BitOr,
    BitXor,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    LogicalAnd,
    LogicalOr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),

    Identifier(String),

    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    Array(Vec<Expr>),

    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    Match {
        value: Box<Expr>,
        cases: Vec<(Expr, Expr)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VariableDecl {
        name: String,
        value: Expr,
        is_const: bool,
    },

    Assignment {
        target: String,
        value: Expr,
    },

    Block(Vec<Stmt>),

    If {
        condition: Expr,
        then_block: Box<Stmt>,
        else_block: Option<Box<Stmt>>,
    },

    While {
        condition: Expr,
        body: Box<Stmt>,
    },

    Printf {
        format: String,
        args: Vec<Expr>,
    },

    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
