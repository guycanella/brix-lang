// Span represents a range in the source code (start and end positions)
pub type Span = std::ops::Range<usize>;

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Complex(f64, f64), // (real, imag)
    Nil,               // Represents null/nil value
    Atom(String),      // Elixir-style atoms (:ok, :error, :atom_name)
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
    Literal(Literal), // 42, 3.14, "text", true
    Binding(String),  // x (captures value and binds to variable)
    Wildcard,         // _ (matches anything, doesn't bind)
    Or(Vec<Pattern>), // 1 | 2 | 3 (matches any of the patterns)
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>, // Optional 'if' guard condition
    pub body: Box<Expr>,          // Expression to execute if pattern matches
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
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
        is_prefix: bool, // true = ++x, false = x++
    },

    Decrement {
        expr: Box<Expr>,
        is_prefix: bool, // true = --x, false = x--
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

    GenericCall {
        func: Box<Expr>,
        type_args: Vec<String>, // Explicit type arguments: swap<int, float>
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

    StructInit {
        struct_name: String,
        type_args: Vec<String>,      // Type arguments for generic structs: Box<int>
        fields: Vec<(String, Expr)>, // (field_name, value)
    },

    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        step: Option<Box<Expr>>,
    },

    ListComprehension {
        expr: Box<Expr>,                   // Expression to evaluate
        generators: Vec<ComprehensionGen>, // for clauses
    },

    StaticInit {
        element_type: String,  // "int" or "float"
        dimensions: Vec<Expr>, // [n] for 1D, [r, c] for 2D
    },

    Closure(Closure),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComprehensionGen {
    pub var_names: Vec<String>, // Variables (supports destructuring)
    pub iterable: Box<Expr>,    // What to iterate over
    pub conditions: Vec<Expr>,  // if clauses (multiple allowed)
}

// Type parameter for generics (e.g., T, U, K, V)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: String,
}

// Struct definition (user-defined types)
#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub type_params: Vec<TypeParam>,                 // Generic type parameters
    pub fields: Vec<(String, String, Option<Expr>)>, // (field_name, type, default_value)
}

// Method definition (Go-style receivers)
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    pub receiver_name: String,  // "p" in fn (p: Point) distance()
    pub receiver_type: String,  // "Point"
    pub method_name: String,    // "distance"
    pub params: Vec<(String, String, Option<Expr>)>, // (param_name, type, default_value)
    pub return_type: Option<Vec<String>>, // None = void, Some(vec!["int"]) = single
    pub body: Box<Stmt>,
}

// Closure (anonymous function with capture)
// Syntax: (x: int, y: int) -> int { return x + y }
#[derive(Debug, Clone, PartialEq)]
pub struct Closure {
    pub params: Vec<(String, String)>,    // Type annotations required: (name, type)
    pub return_type: Option<String>,      // Optional return type
    pub body: Box<Stmt>,                  // Closure body is a block (Statement)
    pub captured_vars: Vec<String>,       // Filled by analysis pass
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    VariableDecl {
        name: String,
        type_hint: Option<String>,
        value: Expr,
        is_const: bool,
    },

    DestructuringDecl {
        names: Vec<String>, // Variable names to destructure into
        value: Expr,        // Expression that returns a tuple
        is_const: bool,     // var or const
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
        var_names: Vec<String>, // Support multiple variables: for x, y in ...
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
        type_params: Vec<TypeParam>,                 // Generic type parameters
        params: Vec<(String, String, Option<Expr>)>, // (param_name, type, default_value)
        return_type: Option<Vec<String>>, // None = void, Some(vec!["int"]) = single, Some(vec!["int", "float"]) = multiple
        body: Box<Stmt>,
    },

    StructDef(StructDef),

    MethodDef(MethodDef),

    Return {
        values: Vec<Expr>, // Empty for void, 1+ for returns
    },

    Expr(Expr),
}

// Helper methods for Expr construction
impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Expr { kind, span }
    }

    // Convenience method for dummy spans (used in tests)
    pub fn dummy(kind: ExprKind) -> Self {
        Expr { kind, span: 0..0 }
    }
}

// Helper methods for Stmt construction
impl Stmt {
    pub fn new(kind: StmtKind, span: Span) -> Self {
        Stmt { kind, span }
    }

    // Convenience method for dummy spans (used in tests)
    pub fn dummy(kind: StmtKind) -> Self {
        Stmt { kind, span: 0..0 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}