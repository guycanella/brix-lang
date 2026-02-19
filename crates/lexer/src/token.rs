use logos::Logos;
use std::fmt;

#[derive(Logos, Debug, PartialEq, Eq, Hash, Clone)]
#[logos(skip r"[ \t\n\f]+")] // Ignore spaces, tabs and line breaks automatically
#[logos(skip r"//.*")] // Ignore comments
pub enum Token {
    // --- Keywords ---
    #[token("function")]
    #[token("fn")]
    Function,

    #[token("var")]
    Var,

    #[token("const")]
    Const,

    #[token("type")]
    Type,

    #[token("struct")]
    Struct,

    #[token("return")]
    Return,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("true")]
    True,

    #[token("false")]
    False,

    #[token("nil")]
    Nil,

    #[token("&&")]
    #[token("and")]
    And,

    #[token("||")]
    #[token("or")]
    Or,

    #[token("!")]
    #[token("not")]
    Not,

    #[token("while")]
    While,

    #[token("for")]
    For,

    #[token("in")]
    In,

    #[token("import")]
    Import,

    #[token("as")]
    As,

    #[token("match")]
    Match, // Pattern Matching (switch substitute)

    #[token("printf")]
    Printf,

    #[token("print")]
    Print,

    #[token("println")]
    Println,

    // --- Literals ---

    // Identifiers: variable names, functions (ex: "minha_variavel")
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // Imaginary literals (ex: 2i, 3.14i, 0.5i)
    // CRITICAL: Higher priority to match before Float/Int!
    #[regex(r"[0-9]+\.[0-9]+i|[0-9]+i", priority = 3, callback = |lex| lex.slice().to_string())]
    ImaginaryLiteral(String),

    // Integers (ex: 42, 100)
    #[regex(r"[0-9]+", priority = 1, callback = |lex| lex.slice().parse::<i64>().ok())]
    Int(i64),

    // Floats (ex: 3.14, 0.5)
    #[regex(r"[0-9]+\.[0-9]+", priority = 2, callback = |lex| lex.slice().to_string())]
    Float(String),

    // Strings (ex: "Olá Brix")
    // Updated regex to accept any escaped character (\.)
    #[regex(r#""(([^"\\]|\\.)*)""#, |lex| lex.slice().to_string())]
    String(String),

    // F-Strings (ex: f"Value: {x}")
    // Updated regex to accept any escaped character (\.)
    #[regex(r#"f"(([^"\\]|\\.)*)""#, |lex| lex.slice().to_string())]
    FString(String),

    // Atoms (ex: :ok, :error, :atom_name)
    // CRITICAL: Higher priority than Colon to match first!
    // Note: Ranges with variables require space (start : end) to avoid conflict with atoms
    #[regex(r":[a-zA-Z_][a-zA-Z0-9_]*", priority = 4, callback = |lex| {
        let s = lex.slice();
        s[1..].to_string()  // Remove leading ':'
    })]
    Atom(String),

    // --- Operators ---
    #[token(":=")]
    ColonEq, // Walrus operator (x := 10)

    #[token("=")]
    Eq,

    #[token("==")]
    DoubleEq,

    #[token("!=")]
    NotEq,

    #[token("+")]
    Plus,

    #[token("++")]
    PlusPlus, // Increment (x++)

    #[token("-")]
    Minus,

    #[token("--")]
    MinusMinus, // Decrement (x--)

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Percent,

    #[token("**")]
    Pow,

    #[token("+=")]
    PlusEq,

    #[token("-=")]
    MinusEq,

    #[token("*=")]
    StarEq,

    #[token("/=")]
    SlashEq,

    #[token("%=")]
    PercentEq,

    #[token("^")]
    Caret,

    #[token("|")]
    Pipe,

    #[token(">")]
    Gt,

    #[token("<")]
    Lt,

    #[token(">=")]
    GtEq,

    #[token("<=")]
    LtEq,

    #[token("->")]
    Arrow, // Function return (fn -> int)

    #[token("&")]
    Ampersand, // Intersection (TypeA & TypeB)

    #[token("?:")]
    QuestionColon, // Elvis operator (a ?: b)

    #[token("?")]
    Question, // Ternary

    #[token(",")]
    Comma,

    #[token(":")]
    Colon,

    #[token(".")]
    Dot,

    // --- Delimiters ---
    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("ERROR")]
    Error,
}

// This helps to show the token prettily in the print
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
