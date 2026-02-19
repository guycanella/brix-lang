// Built-in functions for Brix
//
// This module contains all built-in function declarations and implementations.
//
// REFACTORING NOTE (v1.2):
// - Extracted from lib.rs (originally ~350 lines)
// - Organized into domain-specific submodules
// - Each submodule provides trait for clean separation
//
// Module structure:
// - math.rs (112 lines) - Math library: trig, exponential, rounding, constants
// - stats.rs (26 lines) - Statistics: sum, mean, median, std, variance
// - linalg.rs (53 lines) - Linear algebra: det, inv, tr, eigvals, eigvecs
// - string.rs (133 lines) - String operations: uppercase, lowercase, replace
// - io.rs (5 lines) - I/O functions (placeholder for future)
// - matrix.rs (5 lines) - Matrix constructors (placeholder for future)

pub mod math;
pub mod stats;
pub mod linalg;
pub mod string;
pub mod io;
pub mod matrix;
pub mod test;
