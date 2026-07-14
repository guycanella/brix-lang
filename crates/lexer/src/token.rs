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

    #[token("break")]
    Break,

    #[token("continue")]
    Continue,

    #[token("for")]
    For,

    #[token("in")]
    In,

    #[token("async")]
    Async,

    #[token("await")]
    Await,

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

    #[token("|>")]
    PipeGt, // Pipeline operator (|> has priority over |)

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

    #[token("...")]
    DotDotDot, // Array rest pattern ({ a, ...rest }) — "..." has priority over ".." and "."

    #[token("..<")]
    DotDotLt, // Exclusive range (..<  has priority over ..)

    #[token("..")]
    DotDot, // Inclusive range (.. has priority over .)

    #[token(".")]
    Dot,

    // --- Delimiters ---
    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    // `{` carries a bool flag: true when this brace is preceded (possibly through
    // whitespace/comments) by a newline since the previous non-trivia token, false
    // when it appears on the same line as whatever precedes it.
    //
    // This lets the parser distinguish `Point { x: 1 }` (struct-init continuation,
    // same line) from a `{` that starts a brand-new construct on its own line (e.g.
    // the next `match` arm's pattern), without needing to thread raw source text
    // through the parser. See the "Non-generic struct init" postfix rule in
    // parser.rs, which is the only place that cares about this distinction.
    #[token("{", |lex| {
        let start = lex.span().start;
        let before = &lex.source()[..start];
        let mut preceded_by_newline = false;
        for ch in before.chars().rev() {
            match ch {
                '\n' => { preceded_by_newline = true; break; }
                ' ' | '\t' | '\r' | '\x0c' => continue,
                _ => break,
            }
        }
        preceded_by_newline
    })]
    LBrace(bool),

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
