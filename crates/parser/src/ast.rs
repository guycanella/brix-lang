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
        indices: Vec<Expr>,
    },

    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    FieldAccess {
        target: Box<Expr>,
        field: String,
    },

    Match {
        value: Box<Expr>,
        cases: Vec<(Expr, Expr)>,
    },

    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        step: Option<Box<Expr>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VariableDecl {
        name: String,
        type_hint: Option<String>,
        value: Expr,
        is_const: bool,
    },

    Assignment {
        target: Expr,
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

    For {
        var_name: String,
        iterable: Expr,
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
