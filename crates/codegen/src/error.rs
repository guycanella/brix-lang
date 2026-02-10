// Error handling for code generation
//
// This module provides error types and utilities for robust error handling
// during LLVM code generation.

use std::fmt;

// Import Span type from parser AST
pub use parser::ast::Span;

/// Code generation error types
#[derive(Debug, Clone)]
pub enum CodegenError {
    /// LLVM operation failed (builder, module, etc.)
    LLVMError {
        operation: String,
        details: String,
        span: Option<Span>,
    },

    /// Type mismatch or incompatibility
    TypeError {
        expected: String,
        found: String,
        context: String,
        span: Option<Span>,
    },

    /// Variable or function not found in symbol table
    UndefinedSymbol {
        name: String,
        context: String,
        span: Option<Span>,
    },

    /// Invalid operation (e.g., range outside for loop)
    InvalidOperation {
        operation: String,
        reason: String,
        span: Option<Span>,
    },

    /// Missing required value (e.g., failed compilation)
    MissingValue {
        what: String,
        context: String,
        span: Option<Span>,
    },

    /// General error with message
    General(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::LLVMError { operation, details, .. } => {
                write!(f, "LLVM error in {}: {}", operation, details)
            }
            CodegenError::TypeError { expected, found, context, .. } => {
                write!(
                    f,
                    "Type error in {}: expected {}, found {}",
                    context, expected, found
                )
            }
            CodegenError::UndefinedSymbol { name, context, .. } => {
                write!(f, "Undefined symbol '{}' in {}", name, context)
            }
            CodegenError::InvalidOperation { operation, reason, .. } => {
                write!(f, "Invalid operation '{}': {}", operation, reason)
            }
            CodegenError::MissingValue { what, context, .. } => {
                write!(f, "Missing {} in {}", what, context)
            }
            CodegenError::General(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

impl CodegenError {
    /// Get the exit code for this error type
    /// Used by main.rs to return specific exit codes
    pub fn exit_code(&self) -> i32 {
        match self {
            CodegenError::General(_) => 100,
            CodegenError::LLVMError { .. } => 101,
            CodegenError::TypeError { .. } => 102,
            CodegenError::UndefinedSymbol { .. } => 103,
            CodegenError::InvalidOperation { .. } => 104,
            CodegenError::MissingValue { .. } => 105,
        }
    }
}

/// Convenient Result type for codegen operations
pub type CodegenResult<T> = Result<T, CodegenError>;
