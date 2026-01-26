#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    Text(String),
    Expr {
        expr: Box<Expr>,
        format: Option<String>,
    },
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
pub enum UnaryOp {
    Not,
    Negate,
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

    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    Increment {
        expr: Box<Expr>,
        is_prefix: bool,  // true = ++x, false = x++
    },

    Decrement {
        expr: Box<Expr>,
        is_prefix: bool,  // true = --x, false = x--
    },

    FString {
        parts: Vec<FStringPart>,
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

    StaticInit {
        element_type: String,  // "int" or "float"
        dimensions: Vec<Expr>, // [n] for 1D, [r, c] for 2D
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

    Import {
        module: String,
        alias: Option<String>,
    },

    Printf {
        format: String,
        args: Vec<Expr>,
    },

    Print {
        expr: Expr,
    },

    Println {
        expr: Expr,
    },

    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
