#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Complex(f64, f64),  // (real, imag)
    Nil,                // Represents null/nil value
    Atom(String),       // Elixir-style atoms (:ok, :error, :atom_name)
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

// Pattern Matching support
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(Literal),           // 42, 3.14, "text", true
    Binding(String),            // x (captures value and binds to variable)
    Wildcard,                   // _ (matches anything, doesn't bind)
    Or(Vec<Pattern>),           // 1 | 2 | 3 (matches any of the patterns)
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,  // Optional 'if' guard condition
    pub body: Box<Expr>,           // Expression to execute if pattern matches
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
        arms: Vec<MatchArm>,
    },

    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        step: Option<Box<Expr>>,
    },

    ListComprehension {
        expr: Box<Expr>,                    // Expression to evaluate
        generators: Vec<ComprehensionGen>,  // for clauses
    },

    StaticInit {
        element_type: String,  // "int" or "float"
        dimensions: Vec<Expr>, // [n] for 1D, [r, c] for 2D
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComprehensionGen {
    pub var_names: Vec<String>,  // Variables (supports destructuring)
    pub iterable: Box<Expr>,     // What to iterate over
    pub conditions: Vec<Expr>,   // if clauses (multiple allowed)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VariableDecl {
        name: String,
        type_hint: Option<String>,
        value: Expr,
        is_const: bool,
    },

    DestructuringDecl {
        names: Vec<String>,  // Variable names to destructure into
        value: Expr,         // Expression that returns a tuple
        is_const: bool,      // var or const
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
        var_names: Vec<String>,  // Support multiple variables: for x, y in ...
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

    FunctionDef {
        name: String,
        params: Vec<(String, String, Option<Expr>)>,  // (param_name, type, default_value)
        return_type: Option<Vec<String>>,  // None = void, Some(vec!["int"]) = single, Some(vec!["int", "float"]) = multiple
        body: Box<Stmt>,
    },

    Return {
        values: Vec<Expr>,  // Empty for void, 1+ for returns
    },

    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}
