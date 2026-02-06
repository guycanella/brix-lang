// Arithmetic and comparison operators
//
// This module contains compilation logic for binary and unary operators.

// ============================================================================
// TODO: FASE 5 - OPERATORS (POSTPONED)
// ============================================================================
//
// This phase was postponed because operator logic is highly coupled with
// compile_expr and working perfectly (1001/1001 tests passing).
//
// CURRENT STATE (in lib.rs):
// - Expr::Unary (lines ~1518-1571, ~54 lines)
//   - UnaryOp::Not (logical NOT)
//   - UnaryOp::Negate (arithmetic negation for Int/Float)
//
// - Expr::Binary (lines ~1573-2341, ~768 lines)
//   - Logical operators with short-circuit (&&, ||) using PHI nodes
//   - IntMatrix → Matrix promotion (automatic type promotion)
//   - Matrix arithmetic (28 runtime functions):
//     * Matrix op scalar (Float/Int)
//     * scalar op Matrix (with commutative/non-commutative handling)
//     * Matrix op Matrix
//     * IntMatrix op Int
//     * Int op IntMatrix
//     * IntMatrix op IntMatrix
//   - Nil comparison (== nil, != nil for pointer types)
//   - Complex number pattern detection (3.0 + 4.0i)
//   - Complex arithmetic (add, sub, mul, div, pow with optimizations)
//   - Standard arithmetic (Int, Float operations)
//   - Bitwise operators (&, |, ^, <<, >>)
//   - Comparison operators (<, <=, >, >=, ==, !=)
//   - Atom comparison (compares i64 IDs)
//   - String concatenation (+)
//
// REFACTORING STRATEGY (when we return to this):
//
// Option 1 - Helper functions (recommended):
//   Create helper methods in this module that are called from compile_expr:
//   - compile_matrix_arithmetic() - Matrix/IntMatrix operations
//   - compile_complex_arithmetic() - Complex number operations
//   - compile_nil_comparison() - Nil pointer comparisons
//   - compile_logical_operator() - Short-circuit && and ||
//   - compile_standard_binary_op() - Basic Int/Float arithmetic
//   - compile_unary_op() - Negation and NOT
//
// Option 2 - Full extraction (more complex):
//   Create a trait OperatorCompiler with methods for each operator type,
//   but this requires careful design to avoid circular dependencies with
//   compile_expr (since operators need to recursively compile sub-expressions).
//
// COMPLEXITY NOTES:
// - Operator logic uses recursive compile_expr calls
// - Direct access to self.builder, self.context, self.module
// - Pattern matching integrated with Expr enum
// - PHI nodes for control flow (logical operators)
// - Type promotion and casting throughout
//
// Total estimated lines to move: ~822 lines
// Estimated time: 1.5-2 hours
// Risk level: MEDIUM-HIGH (complex logic, working perfectly, many tests depend on it)
//
// ============================================================================

// Module will be populated when we return to Phase 5
