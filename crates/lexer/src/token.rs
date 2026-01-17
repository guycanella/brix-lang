use logos::Logos;
use std::fmt;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")] // Ignore spaces, tabs and line breaks automatically
pub enum Token {
    // --- Keywords ---
    #[token("function")]
    Function,

    #[token("var")]
    Var,

    #[token("const")]
    Const,

    #[token("type")]
    Type,

    #[token("return")]
    Return,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("for")]
    For,

    #[token("in")]
    In,

    #[token("when")]
    When, // Pattern Matching (switch substitute)

    // --- Literals ---

    // Identifiers: variable names, functions (ex: "minha_variavel")
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // Integers (ex: 42, 100)
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok())]
    Int(i64),

    // Floats (ex: 3.14, 0.5)
    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    // Strings (ex: "Olá Brix")
    #[regex(r#""([^"\\]|\\["\\bnfrt])*""#, |lex| lex.slice().to_string())]
    String(String),

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

    #[token("?")]
    Question, // Ternary

    #[token(".")]
    Dot,

    #[token(",")]
    Comma,

    #[token(":")]
    Colon,

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
    LBracket, // Array

    #[token("]")]
    RBracket,
}

// This helps to show the token prettily in the print
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
