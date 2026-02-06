// Type system for Brix
//
// This module contains the BrixType enum and type-related utilities.

/// Brix type system
#[derive(Debug, Clone, PartialEq)]
pub enum BrixType {
    Int,
    Float,
    String,
    Matrix,        // Matrix of f64 (double*)
    IntMatrix,     // Matrix of i64 (long*)
    Complex,       // Complex number (struct { f64 real, f64 imag })
    ComplexArray,  // Array of Complex (1D)
    ComplexMatrix, // Matrix of Complex (2D)
    FloatPtr,
    Void,
    Tuple(Vec<BrixType>), // Multiple returns (stored as struct)
    Nil,                  // Represents null/nil value (null pointer)
    Error,                // Error type (pointer to BrixError struct in runtime.c)
    Atom,                 // Elixir-style atom (interned string, i64 ID)
}

// Type-related helper functions will be implemented as methods on Compiler
// in lib.rs. They are kept there because they need access to LLVM Context.
