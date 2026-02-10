# Integration Tests

End-to-end tests that compile and execute `.bx` files to validate the complete compilation pipeline.

## Test Categories

### 1. Success Cases (`success/`)
Programs that should compile and execute successfully (exit code 0).

- `01_hello_world.bx` - Basic println
- `02_arithmetic.bx` - Simple arithmetic
- `03_variables.bx` - Variable declarations and f-strings
- `04_if_else.bx` - Conditional statements
- `05_while_loop.bx` - While loops
- `06_for_loop.bx` - For loops (desugared to while)
- `07_function.bx` - User-defined functions
- `08_array.bx` - Array literals and indexing
- `09_matrix.bx` - Matrix arithmetic
- `10_string_ops.bx` - String built-in functions

### 2. Parser Errors (`parser_errors/`)
Syntax errors detected during parsing (exit code 2).

- `01_invalid_operator.bx` - Invalid operator sequence (++)
- `02_missing_token.bx` - Missing expression after :=

### 3. Codegen Errors (`codegen_errors/`)
Type errors and undefined symbols detected during code generation (exit codes 100-105).

- `01_undefined_var.bx` - Undefined variable (exit code 103)
- `02_type_error.bx` - Type mismatch String + Int (exit code 102)

### 4. Runtime Errors (`runtime_errors/`)
Errors detected during program execution (exit code 1).

- `01_division_by_zero.bx` - Integer division by zero
- `02_modulo_by_zero.bx` - Integer modulo by zero

## Running Tests

**IMPORTANT:** Integration tests must run sequentially to avoid file conflicts.

```bash
# Run all integration tests (sequential execution required)
cargo test --test integration_test -- --test-threads=1

# Run specific test
cargo test --test integration_test test_hello_world -- --test-threads=1

# Run with output
cargo test --test integration_test -- --test-threads=1 --nocapture
```

**Why sequential?** All tests compile to the same directory, causing conflicts when run in parallel. This is a known limitation of integration tests that compile actual binaries.

## Exit Codes

| Code | Type | Description |
|------|------|-------------|
| 0 | Success | Program compiled and executed successfully |
| 1 | Runtime Error | Division/modulo by zero, other runtime errors |
| 2 | Parser Error | Invalid syntax, unexpected tokens |
| 100 | General | Generic codegen error |
| 101 | LLVM Error | LLVM operation failed |
| 102 | Type Error | Type mismatch in operations |
| 103 | Undefined Symbol | Variable/function not found |
| 104 | Invalid Operation | Unsupported operation |
| 105 | Missing Value | Required value missing |

## Test Coverage

These integration tests validate:
- ✅ End-to-end compilation pipeline (lex → parse → codegen → link → execute)
- ✅ Error reporting with Ariadne (beautiful error messages)
- ✅ Exit codes match error types
- ✅ Runtime safety checks (division by zero)
- ✅ Type system enforcement
- ✅ Control flow (if/else, loops)
- ✅ Functions and arrays
- ✅ Matrix operations
- ✅ String operations

What unit tests DON'T cover:
- Actual `.bx` file compilation and execution
- Runtime behavior of generated binaries
- Integration between lexer, parser, and codegen
- System exit codes
