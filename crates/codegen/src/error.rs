// Error handling for code generation
//
// This module provides error types and utilities for robust error handling
// during LLVM code generation.

use std::fmt;

/// Code generation error types
#[derive(Debug, Clone)]
pub enum CodegenError {
    /// LLVM operation failed (builder, module, etc.)
    LLVMError { operation: String, details: String },

    /// Type mismatch or incompatibility
    TypeError { expected: String, found: String, context: String },

    /// Variable or function not found in symbol table
    UndefinedSymbol { name: String, context: String },

    /// Invalid operation (e.g., range outside for loop)
    InvalidOperation { operation: String, reason: String },

    /// Missing required value (e.g., failed compilation)
    MissingValue { what: String, context: String },

    /// General error with message
    General(String),
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodegenError::LLVMError { operation, details } => {
                write!(f, "LLVM error in {}: {}", operation, details)
            }
            CodegenError::TypeError { expected, found, context } => {
                write!(
                    f,
                    "Type error in {}: expected {}, found {}",
                    context, expected, found
                )
            }
            CodegenError::UndefinedSymbol { name, context } => {
                write!(f, "Undefined symbol '{}' in {}", name, context)
            }
            CodegenError::InvalidOperation { operation, reason } => {
                write!(f, "Invalid operation '{}': {}", operation, reason)
            }
            CodegenError::MissingValue { what, context } => {
                write!(f, "Missing {} in {}", what, context)
            }
            CodegenError::General(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

/// Convenient Result type for codegen operations
pub type CodegenResult<T> = Result<T, CodegenError>;
