// Error reporting with Ariadne for codegen
//
// This module provides beautiful error messages for code generation errors.

use crate::{CodegenError};
use ariadne::{Color, Label, Report, ReportKind, Source};

/// Format a CodegenError as a beautiful Ariadne report
pub fn report_codegen_error(
    filename: &str,
    source: &str,
    error: &CodegenError,
) {
    match error {
        CodegenError::LLVMError { operation, details, span } => {
            let report = Report::build(ReportKind::Error, filename, span.as_ref().map(|s| s.start).unwrap_or(0))
                .with_code("E101")
                .with_message(format!("LLVM Error in {}", operation))
                .with_help(details.clone());

            let report = if let Some(span) = span {
                report.with_label(
                    Label::new((filename, span.clone()))
                        .with_message(format!("LLVM operation '{}' failed here", operation))
                        .with_color(Color::Red)
                )
            } else {
                report
            };

            report.finish().print((filename, Source::from(source))).unwrap();
        }

        CodegenError::TypeError { expected, found, context, span } => {
            let report = Report::build(ReportKind::Error, filename, span.as_ref().map(|s| s.start).unwrap_or(0))
                .with_code("E102")
                .with_message(format!("Type Error in {}", context))
                .with_help(format!("Expected type '{}', but found '{}'", expected, found));

            let report = if let Some(span) = span {
                report.with_label(
                    Label::new((filename, span.clone()))
                        .with_message(format!("This expression has type '{}', not '{}'", found, expected))
                        .with_color(Color::Red)
                )
            } else {
                report
            };

            report.finish().print((filename, Source::from(source))).unwrap();
        }

        CodegenError::UndefinedSymbol { name, context, span } => {
            let report = Report::build(ReportKind::Error, filename, span.as_ref().map(|s| s.start).unwrap_or(0))
                .with_code("E103")
                .with_message(format!("Undefined symbol '{}'", name))
                .with_help(format!("Symbol '{}' is not defined in {}", name, context));

            let report = if let Some(span) = span {
                report.with_label(
                    Label::new((filename, span.clone()))
                        .with_message(format!("'{}' used here but not defined", name))
                        .with_color(Color::Red)
                )
            } else {
                report
            };

            report.finish().print((filename, Source::from(source))).unwrap();
        }

        CodegenError::InvalidOperation { operation, reason, span } => {
            let report = Report::build(ReportKind::Error, filename, span.as_ref().map(|s| s.start).unwrap_or(0))
                .with_code("E104")
                .with_message(format!("Invalid operation: {}", operation))
                .with_help(reason.clone());

            let report = if let Some(span) = span {
                report.with_label(
                    Label::new((filename, span.clone()))
                        .with_message(format!("'{}' cannot be used here", operation))
                        .with_color(Color::Red)
                )
            } else {
                report
            };

            report.finish().print((filename, Source::from(source))).unwrap();
        }

        CodegenError::MissingValue { what, context, span } => {
            let report = Report::build(ReportKind::Error, filename, span.as_ref().map(|s| s.start).unwrap_or(0))
                .with_code("E105")
                .with_message(format!("Missing {}", what))
                .with_help(format!("{} required in {}", what, context));

            let report = if let Some(span) = span {
                report.with_label(
                    Label::new((filename, span.clone()))
                        .with_message(format!("Missing {} here", what))
                        .with_color(Color::Red)
                )
            } else {
                report
            };

            report.finish().print((filename, Source::from(source))).unwrap();
        }

        CodegenError::General(msg) => {
            Report::<(&str, std::ops::Range<usize>)>::build(ReportKind::Error, filename, 0)
                .with_code("E100")
                .with_message(msg.clone())  // Use the message directly
                .finish()
                .print((filename, Source::from(source)))
                .unwrap();
        }
    }
}

/// Report multiple codegen errors
pub fn report_codegen_errors(
    filename: &str,
    source: &str,
    errors: &[CodegenError],
) {
    for error in errors {
        report_codegen_error(filename, source, error);
    }
}
