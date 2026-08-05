// Type system for Brix
//
// This module contains the BrixType enum and type-related utilities.
//
// REFACTORING NOTE (v1.2):
// - Extracted from lib.rs (originally part of 7,338-line monolith)
// - Type helper methods remain in lib.rs (need LLVM Context access):
//   * string_to_brix_type() - Parse type strings
//   * brix_type_to_llvm() - Convert to LLVM types
//   * are_types_compatible() - Type compatibility checking

/// Brix type system
#[derive(Debug, Clone, PartialEq)]
pub enum BrixType {
    Int,
    Float,
    String,
    Matrix,        // Matrix of f64 (double*)
    IntMatrix,     // Matrix of i64 (long*)
    StringMatrix,  // Array of BrixString* ({ ref_count, len, BrixString** data }) (v1.7)
    Complex,       // Complex number (struct { f64 real, f64 imag })
    ComplexArray,  // Array of Complex (1D)
    ComplexMatrix, // Matrix of Complex (2D)
    FloatPtr,
    Void,
    Tuple(Vec<BrixType>),                  // Multiple returns (stored as struct)
    Nil,                                   // Represents null/nil value (null pointer)
    Error,                                 // Error type (pointer to BrixError struct in runtime.c)
    Atom,                                  // Elixir-style atom (interned string, i64 ID)
    Struct(String),                        // User-defined struct (name stored as String)
    Optional(Box<BrixType>),               // Optional type: int?, String?, Matrix? (v1.4)
    Union(Vec<BrixType>),                  // Union type: int | float | string (v1.4)
    Intersection(Vec<BrixType>),           // Intersection type: Point & Label (v1.4)
    AsyncFuture, // async { } block result: state_ptr (i8*) with embedded poll_fn at offset 0 (v1.6 Phase 3b)
    Vector(Box<BrixType>), // Dynamic array Vector<T> (BrixVector*), v1.8 Grupo C; T in {Int, Float, String}
    Stack(Box<BrixType>), // Stack<T> (LIFO) — thin wrapper over BrixVector*, v1.8 Grupo D; T in {Int, Float, String}
    Queue(Box<BrixType>), // Queue<T> (FIFO ring buffer, BrixQueue*), v1.8 Grupo D; T in {Int, Float, String}
    MinHeap(Box<BrixType>), // MinHeap<T> — BrixVector* por baixo, ordem ascendente. v1.8 Grupo E; T in {Int, Float, String}
    MaxHeap(Box<BrixType>), // MaxHeap<T> — BrixVector* por baixo, ordem descendente. v1.8 Grupo E; T in {Int, Float, String}
    HashMap(Box<BrixType>, Box<BrixType>), // HashMap<K,V> — K in {Int, String}, V in {Int, Float, String}. v1.8 Grupo F
    DateTime,                              // DateTime struct pointer (BrixDateTime*), v1.9 Grupo A
    Json,                                  // JsonValue opaque pointer (JsonValue*), v1.9 Grupo B
}

// Type-related helper functions will be implemented as methods on Compiler
// in lib.rs. They are kept there because they need access to LLVM Context.

fn format_elem_type(inner: &BrixType) -> &'static str {
    match inner {
        BrixType::Int => "int",
        BrixType::Float => "float",
        BrixType::String => "string",
        _ => "unknown",
    }
}

/// Render a `BrixType` as the user-facing type name shown by `typeof()` and
/// the REPL's `:type` command — not `{:?}` Debug output.
pub fn format_brix_type(ty: &BrixType) -> String {
    match ty {
        BrixType::Int => "int".to_string(),
        BrixType::Float => "float".to_string(),
        BrixType::String => "string".to_string(),
        BrixType::Matrix => "matrix".to_string(),
        BrixType::IntMatrix => "intmatrix".to_string(),
        BrixType::StringMatrix => "stringmatrix".to_string(),
        BrixType::Complex => "complex".to_string(),
        BrixType::ComplexArray => "complexarray".to_string(),
        BrixType::ComplexMatrix => "complexmatrix".to_string(),
        BrixType::FloatPtr => "float_ptr".to_string(),
        BrixType::Void => "void".to_string(),
        BrixType::Tuple(_) => "tuple".to_string(),
        BrixType::Nil => "nil".to_string(),
        BrixType::Error => "error".to_string(),
        BrixType::Atom => "atom".to_string(),
        BrixType::Struct(name) => name.clone(),
        BrixType::Vector(inner) => format!("Vector<{}>", format_elem_type(inner)),
        BrixType::Stack(inner) => format!("Stack<{}>", format_elem_type(inner)),
        BrixType::Queue(inner) => format!("Queue<{}>", format_elem_type(inner)),
        BrixType::MinHeap(inner) => format!("MinHeap<{}>", format_elem_type(inner)),
        BrixType::MaxHeap(inner) => format!("MaxHeap<{}>", format_elem_type(inner)),
        BrixType::HashMap(key, val) => format!(
            "HashMap<{}, {}>",
            format_elem_type(key),
            format_elem_type(val)
        ),
        BrixType::Optional(inner) => format!("{}?", format_brix_type(inner)),
        BrixType::Union(types) => types
            .iter()
            .map(|t| match t {
                BrixType::Nil => "nil".to_string(),
                other => format_brix_type(other),
            })
            .collect::<Vec<_>>()
            .join(" | "),
        BrixType::Intersection(types) => types
            .iter()
            .map(format_brix_type)
            .collect::<Vec<_>>()
            .join(" & "),
        BrixType::AsyncFuture => "async_future".to_string(),
        BrixType::DateTime => "datetime".to_string(),
        BrixType::Json => "json".to_string(),
    }
}
