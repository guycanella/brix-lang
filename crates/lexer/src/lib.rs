pub mod token;

use logos::Logos;
use token::Token;

/// Tokenize a source string and return a Vec of tokens
pub fn lex(source: &str) -> Vec<Token> {
    Token::lexer(source)
        .filter_map(|t| t.ok())
        .collect()
}
