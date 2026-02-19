// Error reporting with Ariadne
//
// This module provides beautiful error messages using Ariadne.

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Simple;
use lexer::token::Token;
use std::ops::Range;

/// Type alias for Chumsky parser errors
pub type ParseError = Simple<Token>;

/// Converts Chumsky errors to beautiful Ariadne reports
pub fn report_errors(
    filename: &str,
    source: &str,
    errors: Vec<ParseError>,
) {
    for error in errors {
        let span = error.span();
        let msg = format!("{}", error);

        let report = Report::build(ReportKind::Error, filename, span.start)
            .with_code("E001")
            .with_message("Parse Error")
            .with_label(
                Label::new((filename, span))
                    .with_message(msg)
                    .with_color(Color::Red)
            );

        // Add expected tokens if available (limit to 5 to avoid overwhelming output)
        let report = if error.expected().len() > 0 {
            let expected: Vec<String> = error
                .expected()
                .take(5)
                .map(|e| format_expected(e))
                .collect();

            let help_msg = if error.expected().len() > 5 {
                format!("Expected one of: {}, ...", expected.join(", "))
            } else {
                format!("Expected: {}", expected.join(", "))
            };

            report.with_help(help_msg)
        } else {
            report
        };

        // Print the report
        report
            .finish()
            .print((filename, Source::from(source)))
            .unwrap();
    }
}

/// Format expected token for human-readable output
fn format_expected(token: &Option<Token>) -> String {
    match token {
        Some(Token::Plus) => "'+'".to_string(),
        Some(Token::Minus) => "'-'".to_string(),
        Some(Token::Star) => "'*'".to_string(),
        Some(Token::Slash) => "'/'".to_string(),
        Some(Token::Percent) => "'%'".to_string(),
        Some(Token::Pow) => "'**'".to_string(),
        Some(Token::LParen) => "'('".to_string(),
        Some(Token::RParen) => "')'".to_string(),
        Some(Token::LBracket) => "'['".to_string(),
        Some(Token::RBracket) => "']'".to_string(),
        Some(Token::LBrace) => "'{'".to_string(),
        Some(Token::RBrace) => "'}'".to_string(),
        Some(Token::Comma) => "','".to_string(),
        Some(Token::Dot) => "'.'".to_string(),
        Some(Token::Colon) => "':'".to_string(),
        Some(Token::DoubleEq) => "'=='".to_string(),
        Some(Token::NotEq) => "'!='".to_string(),
        Some(Token::Lt) => "'<'".to_string(),
        Some(Token::Gt) => "'>'".to_string(),
        Some(Token::LtEq) => "'<='".to_string(),
        Some(Token::GtEq) => "'>='".to_string(),
        Some(Token::ColonEq) => "':='".to_string(),
        Some(Token::PlusPlus) => "'++'".to_string(),
        Some(Token::MinusMinus) => "'--'".to_string(),
        Some(Token::And) => "'&&'".to_string(),
        Some(Token::Or) => "'||'".to_string(),
        Some(Token::Ampersand) => "'&'".to_string(),
        Some(Token::Pipe) => "'|'".to_string(),
        Some(Token::Caret) => "'^'".to_string(),
        Some(Token::Question) => "'?'".to_string(),
        Some(Token::Arrow) => "'->'".to_string(),
        Some(Token::Var) => "keyword 'var'".to_string(),
        Some(Token::Function) => "keyword 'function'".to_string(),
        Some(Token::If) => "keyword 'if'".to_string(),
        Some(Token::Else) => "keyword 'else'".to_string(),
        Some(Token::While) => "keyword 'while'".to_string(),
        Some(Token::For) => "keyword 'for'".to_string(),
        Some(Token::In) => "keyword 'in'".to_string(),
        Some(Token::Return) => "keyword 'return'".to_string(),
        Some(Token::Match) => "keyword 'match'".to_string(),
        Some(Token::Import) => "keyword 'import'".to_string(),
        Some(Token::As) => "keyword 'as'".to_string(),
        Some(Token::Identifier(_)) => "identifier".to_string(),
        Some(Token::Int(_)) => "integer".to_string(),
        Some(Token::Float(_)) => "float".to_string(),
        Some(Token::String(_)) => "string".to_string(),
        Some(Token::FString(_)) => "f-string".to_string(),
        Some(Token::True) => "'true'".to_string(),
        Some(Token::False) => "'false'".to_string(),
        Some(Token::Nil) => "'nil'".to_string(),
        Some(Token::Atom(_)) => "atom".to_string(),
        Some(Token::ImaginaryLiteral(_)) => "imaginary literal".to_string(),
        Some(t) => format!("{:?}", t),
        None => "end of input".to_string(),
    }
}

/// Checks for invalid operator sequences (like `1 ++ 2`) and reports with Ariadne
/// This is a semantic check that chumsky might miss
pub fn check_and_report_invalid_sequences(
    filename: &str,
    source: &str,
    tokens: &[(Token, Range<usize>)],
) -> bool {
    for window in tokens.windows(3) {
        let (prev_tok, prev_span) = &window[0];
        let (curr_tok, curr_span) = &window[1];
        let (next_tok, next_span) = &window[2];

        // Helper: true if there is a newline in source between two byte offsets
        let has_newline_between = |a: usize, b: usize| -> bool {
            source.get(a..b).map_or(false, |s| s.contains('\n'))
        };

        // Check for: value ++ value on the same line (binary usage).
        // x++ at end of line is valid postfix — detected by newline between ++ and next token.
        if matches_value_token(prev_tok)
            && matches!(curr_tok, Token::PlusPlus)
            && matches_value_token(next_tok)
            && !has_newline_between(curr_span.end, next_span.start)
        {
            let combined_span = prev_span.start..curr_span.end;

            Report::build(ReportKind::Error, filename, combined_span.start)
                .with_code("E002")
                .with_message("Invalid operator sequence")
                .with_label(
                    Label::new((filename, combined_span.clone()))
                        .with_message("'++' cannot be used as a binary operator")
                        .with_color(Color::Red)
                )
                .with_help("Did you mean:\n  • Compound assignment: x += 1\n  • Prefix increment: ++x\n  • Postfix increment: x++")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();

            return true;
        }

        // Check for: value -- value on the same line (binary usage).
        // x-- at end of line is valid postfix.
        if matches_value_token(prev_tok)
            && matches!(curr_tok, Token::MinusMinus)
            && matches_value_token(next_tok)
            && !has_newline_between(curr_span.end, next_span.start)
        {
            let combined_span = prev_span.start..curr_span.end;

            Report::build(ReportKind::Error, filename, combined_span.start)
                .with_code("E002")
                .with_message("Invalid operator sequence")
                .with_label(
                    Label::new((filename, combined_span.clone()))
                        .with_message("'--' cannot be used as a binary operator")
                        .with_color(Color::Red)
                )
                .with_help("Did you mean:\n  • Compound assignment: x -= 1\n  • Prefix decrement: --x\n  • Postfix decrement: x--")
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();

            return true;
        }
    }

    false
}

/// Helper: check if token represents a value (literal, identifier, etc.)
fn matches_value_token(tok: &Token) -> bool {
    matches!(
        tok,
        Token::Int(_) | Token::Float(_) | Token::Identifier(_) | Token::True | Token::False
    )
}
