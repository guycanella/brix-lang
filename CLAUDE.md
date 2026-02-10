# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Instructions for Claude Code

**CRITICAL**: Do not stop tasks early due to context limits. Always complete the full task even if it requires significant context usage. Use context efficiently but prioritize task completion.

## Quick Start

**Compile and run a Brix program:**
```bash
cargo run <file.bx>
```

This single command lexes, parses, generates LLVM IR, compiles runtime.c, links everything, and executes the binary.

**Build compiler only:**
```bash
cargo build          # Debug
cargo build --release
```

**Run tests:**
```bash
cargo test --all              # Run all unit tests (1001 tests total, 100% passing)
cargo test <pattern>          # Run tests matching pattern
cargo test -- --nocapture     # Show println! output
cargo test -p lexer           # Run only lexer tests
cargo test -p parser          # Run only parser tests
cargo test -p codegen         # Run only codegen tests
```

**Clean build (fixes most linking errors):**
```bash
rm -f runtime.o output.o program
cargo clean
cargo run <file.bx>
```

## Project Overview

**Brix** is a compiled programming language for Data Engineering and Algorithms, combining Python-like syntax with Fortran-level performance.

- **Extension**: `.bx`
- **Philosophy**: "Write like Python, execute like Fortran, scale like Go"
- **Stack**: Rust (compiler) + LLVM 18 (backend)
- **Memory Model**: ARC (Automatic Reference Counting)
- **Type System**: Strong static typing with aggressive type inference

## Architecture

### Compilation Pipeline

`.bx` source → **Lexer** → Tokens → **Parser** → AST → **Codegen** → LLVM IR → **Link** → Native Binary

### Workspace Structure

```
brix/
├── src/main.rs              # CLI driver, orchestrates compilation
├── runtime.c                # C runtime (MUST be in project root)
├── crates/
│   ├── lexer/               # Tokenization (logos)
│   │   └── src/token.rs     # Token enum
│   ├── parser/              # AST construction (chumsky)
│   │   └── src/{ast.rs, parser.rs, error.rs}
│   └── codegen/             # LLVM code generation (inkwell) - REFACTORED v1.2 + ERROR HANDLING
│       └── src/
│           ├── lib.rs       # Core compiler (~7,700 lines with error handling)
│           ├── error.rs     # Error types (CodegenError, CodegenResult) (84 lines)
│           ├── error_report.rs # Ariadne error formatting (131 lines)
│           ├── types.rs     # BrixType enum (33 lines)
│           ├── helpers.rs   # LLVM helpers with Result types (146 lines)
│           ├── stmt.rs      # Statement compilation with Result (528 lines)
│           ├── expr.rs      # Expression compilation with Result (285 lines)
│           ├── operators.rs # Operator logic (postponed, annotated)
│           └── builtins/    # Built-in function declarations
│               ├── mod.rs, math.rs, stats.rs, linalg.rs
│               └── string.rs, io.rs, matrix.rs
```

### Key Components

**1. Lexer (`crates/lexer`)**
- Uses `logos` crate for performance
- Token priority: `ImaginaryLiteral` (priority=3) > `Float` to avoid `2.0i` being parsed as float + identifier
- Atoms: `:atom_name` (priority=4) > `Colon`
- F-strings: `r#"f"(([^"\\]|\\.)*)"#` - accepts any escaped character

**2. Parser (`crates/parser`)**
- Uses `chumsky` parser combinators
- **Error reporting** via `Ariadne` - beautiful, colored error messages with source context
- **Semantic checks**: detects invalid operator sequences (like `1 ++ 2`) before parsing
- Operator precedence (lowest to highest, C-style):
  - Comparison/Logical: `<`, `<=`, `>`, `>=`, `==`, `!=`, `&&`, `||`
  - Bitwise: `&`, `|`, `^` (binds tighter than comparison - C-style)
  - Additive: `+`, `-`
  - Multiplicative: `*`, `/`, `%`
  - Power: `**` (right-associative, like Python/Fortran)
  - Atom: literals, identifiers, function calls, indexing
- **Postfix chaining**: `.field`, `[index]`, and `(args)` can be chained in any order
  - Examples: `get_matrix().rows`, `foo()()`, `arr[0].len`, `obj.get_nested()[0]()`
- For loops desugar to while loops during parsing
- Escape sequences processed via `process_escape_sequences()` helper

**3. Codegen (`crates/codegen`)**
- Uses `inkwell` (LLVM 18 bindings)
- Symbol table: `HashMap<String, (PointerValue, BrixType)>`
- All variables allocated via `alloca` on stack
- Control flow uses LLVM basic blocks (if/else, loops, match)
- **No PHI nodes for if/else** - values stored in alloca'd variables
- **PHI nodes used for**: ternary operator (`? :`), match expressions, logical short-circuit (`&&`, `||`)
- **Error Handling** (v1.2.1 - Feb 2026):
  - `CodegenError` enum with 6 variants: LLVMError, TypeError, UndefinedSymbol, InvalidOperation, MissingValue, General
  - Each variant (except General) includes `span: Option<Span>` for source location
  - `CodegenResult<T>` = `Result<T, CodegenError>` used throughout compilation pipeline
  - All expression compilation returns `CodegenResult<(BasicValueEnum, BrixType)>`
  - All statement compilation returns `CodegenResult<()>`
  - Proper error propagation with `?` operator instead of `.unwrap()`
  - LLVM operations use `.map_err()` for descriptive error messages
  - **Modules converted**: error.rs, expr.rs, stmt.rs, helpers.rs, lib.rs (nearly complete)
- **Error Reporting** (`error_report.rs` - Feb 2026):
  - Beautiful error messages using Ariadne library
  - `report_codegen_error()`: Formats CodegenError with source context
  - Error codes: E100 (General), E101 (LLVM), E102 (Type), E103 (UndefinedSymbol), E104 (InvalidOperation), E105 (MissingValue)
  - Colored labels pointing to exact source code spans
  - Contextual help messages for each error type
  - Integration: `Compiler::new()` accepts `filename` and `source` parameters

**4. Runtime (`runtime.c`)**
- Provides C implementations of built-in functions (~1,500 lines)
- Compiled to `runtime.o` by `src/main.rs` using system `cc`
- Linked with `-lm -llapack -lblas` for math/linear algebra
- Organized in sections: Atoms, Complex, Matrix, IntMatrix, ComplexMatrix, LAPACK, Errors, Strings, Stats, Linear Algebra, Zip
- **Matrix Operations**: 28 functions for element-wise arithmetic
  - Matrix with scalar: `matrix_add_scalar`, `matrix_mul_scalar`, etc. (6 ops)
  - IntMatrix with Int: `intmatrix_add_scalar`, `intmatrix_mul_scalar`, etc. (6 ops)
  - Matrix with Matrix: `matrix_add_matrix`, `matrix_mul_matrix`, etc. (6 ops)
  - IntMatrix with IntMatrix: `intmatrix_add_intmatrix`, etc. (6 ops)
  - Non-commutative: `scalar_sub_matrix`, `scalar_div_matrix`, `scalar_sub_intmatrix`
  - Conversion: `intmatrix_to_matrix()` for type promotion
- Critical structures:
  ```c
  typedef struct { long len; char* data; } BrixString;
  typedef struct { long rows; long cols; double* data; } Matrix;
  typedef struct { long rows; long cols; long* data; } IntMatrix;
  typedef struct { double real; double imag; } Complex;
  typedef struct { long rows; long cols; Complex* data; } ComplexMatrix;
  typedef struct { char* message; } BrixError;
  typedef struct { char** names; long count; long capacity; } AtomPool;
  ```

## Type System

**14 Core Types:**
- `Int` (i64), `Float` (f64), `String` (BrixString*)
- `Matrix` (f64*), `IntMatrix` (i64*), `FloatPtr` (f64*)
- `Complex` (real+imag), `ComplexMatrix` (Complex*)
- `Tuple(Vec<BrixType>)` - multiple return values
- `Nil` (i8* null), `Error` (BrixError*), `Atom` (i64 ID)
- `Void` (no return)

**Type Inference for Array Literals:**
- All ints → `IntMatrix`: `[1, 2, 3]`
- Mixed or all floats → `Matrix`: `[1, 2.5, 3.7]` (int→float promotion)

**Matrix Arithmetic:**
- **All 6 operators supported**: `+`, `-`, `*`, `/`, `%`, `**` (element-wise operations)
- **IntMatrix with Int**: Result is `IntMatrix` (integer division for `/`)
  - Example: `[1, 2, 3] * 2 = [2, 4, 6]`, `[1, 2, 3] / 2 = [0, 1, 1]`
- **IntMatrix with Float**: Automatic promotion to `Matrix`
  - Example: `[1, 2, 3] * 2.5 = [2.5, 5.0, 7.5]`
- **Matrix with scalar**: Element-wise operation
  - Example: `[1.0, 2.0] + 10.5 = [11.5, 12.5]`
- **Matrix with Matrix**: Element-wise operation (NOT matrix multiplication)
  - Example: `[1.0, 2.0] * [3.0, 4.0] = [3.0, 8.0]`

**Boolean Representation:**
- Stored as `i1` in LLVM, auto-extends to `i64` when stored in variables

## Important Implementation Details

### Symbol Table Management
- Flat symbol table with module prefixes: `math.sin` stored as `"math.sin"`
- Variables: `alloca` + `load`/`store`
- Imported modules create prefixed entries at compile time

### Control Flow
- **If/else**: Uses basic blocks (`then_block`, `else_block`, `merge_block`), NO PHI nodes
- **While loops**: Condition block + body block + merge block
- **For loops**: Desugared to while loops: `for i in start:step:end` → `var i := start; while i <= end { body; i += step }`
- **Match expressions**: Basic blocks per arm + PHI node in merge block
- **Ternary operator**: Creates merge block with PHI node for expression result
- **Logical operators**: `&&` and `||` use PHI nodes for short-circuit evaluation

### String Handling
- Literals create global constants
- F-strings parse `{}` expressions recursively
- Format specifiers: `:x` (hex), `:o` (octal), `:.2f` (precision), `:e` (scientific)
- Concatenation calls runtime `str_concat()`

### Complex Numbers
- Imaginary unit: `im` constant (not `i`) to avoid loop variable conflicts
- Parser recognizes `(expr)im` and converts to `expr * im`
- User variables shadow builtin constants
- LAPACK integration: `eigvals()`, `eigvecs()` return `ComplexMatrix`

### Pattern Matching
- AST: `Pattern` enum (Literal, Wildcard, Binding, Or)
- Codegen: Basic blocks per arm + type checking across arms
- Type coercion: int→float when arms have different types
- Guards: Binding occurs before guard evaluation

### Import System
- Zero-overhead: generates LLVM external declarations at compile time
- `import math` → adds `math.*` namespace to symbol table
- `import math as m` → adds `m.*` namespace
- Math functions link directly to C math.h (FSIN/FCOS CPU instructions)
- Symbol table is **flat with prefixes**, not hierarchical (e.g., `"math.sin"` is a single key)

### Matrix Operations
- **Element-wise arithmetic**: All 6 operators (`+`, `-`, `*`, `/`, `%`, `**`) work on matrices
- **Type promotion rules**:
  - `IntMatrix op Int` → stays `IntMatrix` (integer division for `/`)
  - `IntMatrix op Float` → promotes to `Matrix` via `intmatrix_to_matrix()`
  - `Matrix op Float` → stays `Matrix`
- **Runtime implementation**: 28 functions in runtime.c handle all combinations
  - Matrix-scalar, scalar-Matrix (non-commutative for `-`, `/`)
  - Matrix-Matrix (element-wise, NOT matrix multiplication)
  - IntMatrix-Int, IntMatrix-IntMatrix (similar operations)
- **Codegen detection**: Checks operand types and selects appropriate runtime function
- **NOT matrix multiplication**: `*` is element-wise, use `matmul()` for true matrix product

## Error Handling Architecture (v1.2.1)

**Philosophy**: All compilation errors use `Result` types with rich error information and precise source spans.

### Error Types (`CodegenError` enum)

| Variant | Exit Code | Description | Example |
|---------|-----------|-------------|---------|
| `General` | 100 | Generic error message | Internal compiler errors |
| `LLVMError` | 101 | LLVM operation failed | Builder/module operations |
| `TypeError` | 102 | Type mismatch | `"string" + 42` |
| `UndefinedSymbol` | 103 | Variable/function not found | `var x := undefined_var` |
| `InvalidOperation` | 104 | Invalid operation | Unsupported operator combination |
| `MissingValue` | 105 | Required value missing | Failed compilation step |
| Parser Errors | 2 | Syntax errors | `1 ++ 2`, missing tokens |
| Success | 0 | Compilation successful | - |

### Error Propagation Flow

```
Source Code (.bx)
    ↓
Lexer (logos) → Token stream
    ↓
Parser (chumsky) → AST with spans
    ↓ (Result<AST, ParseError>)
    ├─ Err → report_errors() → exit(2)
    └─ Ok → AST
        ↓
Codegen (inkwell) → LLVM IR
    ↓ (CodegenResult<()>)
    ├─ Err(e) → report_codegen_error() → exit(e.exit_code())
    └─ Ok → Compile & Link
        ↓
Binary Execution
    ├─ Runtime Error → exit(1) or crash
    └─ Success → exit(0)
```

### Error Reporting

**Parser Errors**: Use Ariadne to show beautiful syntax errors with source context
**Codegen Errors**: Use Ariadne with precise token-level spans (not expression-level)
**Runtime Errors**: Some have automatic checks (div/0), others are undefined behavior

### Span Precision

All errors include `span: Option<Span>` to point to exact source locations:
- Parser captures spans via chumsky's `.map_with_span()`
- Codegen propagates spans through AST nodes
- Ariadne uses spans to highlight exact tokens in error messages

Example: In `var x := a + foo * b`, error on `foo` highlights only `foo`, not entire expression.

### Runtime Safety Checks

| Operation | Check | Behavior |
|-----------|-------|----------|
| Int / 0 | ✅ Automatic | Exit with error message |
| Int % 0 | ✅ Automatic | Exit with error message |
| Float / 0.0 | ❌ None | Returns `inf` (IEEE 754) |
| Array bounds | ❌ None | Undefined behavior (like C) |

## Critical Architectural Decisions

**Why PHI nodes only for expressions, not if/else statements:**
- If/else statements don't produce values in Brix, so no merge needed
- Ternary operator (`cond ? a : b`) produces a value, requires PHI to merge branches
- Match expressions produce values, use PHI in merge block
- This keeps statement compilation simpler while enabling expression-level control flow

**Why all variables use alloca:**
- Stack allocation is faster than heap
- LLVM optimizes stack variables well
- ARC model doesn't require heap allocation for primitives
- Complex types (Matrix, String) store pointers to heap-allocated data, but the pointer itself is stack-allocated

**Why for loops desugar at parse time:**
- Simpler codegen (only needs to handle while loops)
- Easier to optimize in LLVM
- Reduces code duplication in codegen
- Range syntax sugar is purely syntactic

**Why atoms use i64 instead of strings:**
- O(1) comparison vs O(n) string comparison
- Interned at runtime in C (AtomPool)
- LLVM treats as integer constant
- Codegen calls `intern_atom("ok")` to get ID

## Development Workflow

**Before Making Changes:**
1. Run `cargo test --all` to verify baseline (should show 1001/1001 passing, 100% 🎉)
   - Lexer: 292 passing, 0 ignored
   - Parser: 150 passing, 0 ignored
   - Codegen: 559 passing, 0 ignored
2. Check which crate needs modification (lexer, parser, or codegen)
3. Review recent commits with `git log --oneline -10`
4. For new features: follow the Lexer → Parser → Codegen → Runtime order
5. See FIX_BUGS.md for bug fix history

**Debugging Checklist:**
1. Linking errors? Run clean build: `rm -f *.o program && cargo clean && cargo build`
2. Runtime errors? Check that `runtime.c` exists in project root
3. LLVM errors? Verify LLVM 18 installed: `brew list llvm@18`
4. Panic? Search for `unwrap()` calls in stack trace location

**Adding Features:**
1. **New operator**: Lexer token → Parser precedence → Codegen binary_op
2. **New built-in**: Codegen external declaration → Runtime C implementation
3. **New type**: Update `BrixType` enum, `infer_type()`, `cast_value()`, `get_llvm_type()`
4. Always add tests in corresponding test module

## Common Development Patterns

### Adding a New Operator
1. **Lexer**: Add token in `crates/lexer/src/token.rs`
   ```rust
   #[token("&&")]
   And,
   ```
2. **Parser**: Add to appropriate precedence level in `crates/parser/src/parser.rs`
3. **Codegen**: Handle in `compile_binary_op()` in `crates/codegen/src/lib.rs`

### Adding a New Built-in Function
1. **Codegen**: Declare external function
   ```rust
   fn get_sqrt(&self) -> FunctionValue<'ctx> {
       let fn_type = self.f64_type.fn_type(&[self.f64_type.into()], false);
       self.module.add_function("sqrt", fn_type, Some(Linkage::External))
   }
   ```
2. **Runtime** (if needed): Implement in `runtime.c`
3. Automatically recompiled by `src/main.rs`

### Type System Changes
1. Update `BrixType` enum in `crates/codegen/src/lib.rs`
2. Update `infer_type()` for type inference
3. Update `cast_value()` for automatic casting
4. Add LLVM type mapping in `get_llvm_type()`

## Testing

### Unit Tests

**Automated Unit Tests:** 1001 tests total, **1001 passing (100%)** 🎉
```bash
cargo test --all              # Run all tests
cargo test <pattern>          # Run tests matching pattern
cargo test -- --nocapture     # Show output from tests
```

**Test Organization:**
- `crates/lexer/src/tests/` - 5 modules (atoms, numbers, strings, tokens, edge cases)
- `crates/parser/src/tests/` - 7 modules, **149 passing, 1 ignored**
  - exprs, stmts, patterns, precedence, destructuring, errors, edge cases
- `crates/codegen/src/tests/` - 12 modules (560 tests), **559 passing, 1 ignored**:
  - builtin_tests.rs (100 tests) - Math, stats, linear algebra, type checking, I/O
  - complex_tests.rs (30 tests) - Complex numbers, ComplexMatrix, LAPACK
  - stmt_tests.rs (40 tests) - Declarations, assignments, imports, destructuring
  - function_tests.rs (50 tests) - Default params, multiple returns, recursion, scoping
  - pattern_tests.rs (37 tests) - Type coercion, typeof() matching, complex patterns
  - string_tests.rs (35 tests) - Format specifiers, escape sequences, operations
  - control_flow_tests.rs (40 tests) - Loops, comprehensions, zip(), constructors
  - type_tests.rs (45 tests) - Type inference, casting, numeric edge cases
  - matrix_tests.rs (65 tests) - Constructors, indexing, field access, list comprehensions, arithmetic
  - expr_tests.rs (60 tests) - Literals, operators, ternary, short-circuit, chained comparisons
  - edge_cases.rs (50 tests) - Overflow, precedence, division, boolean, negative numbers
  - integration_tests.rs (15 tests) - Complex feature combinations

**Remaining Ignored Tests:** None! 🎉 All 1001 tests passing (100%)

### Integration Tests

**End-to-End Tests:** 68 tests total, **68 passing (100%)** 🎉
```bash
# IMPORTANT: Must run sequentially to avoid file conflicts
cargo test --test integration_test -- --test-threads=1

# Run with output
cargo test --test integration_test -- --test-threads=1 --nocapture
```

**Test Categories** (`tests/integration/`):
- **Success cases** (64 tests) - Programs that compile and execute successfully (exit code 0)
  - Hello world, arithmetic, variables, control flow, functions, arrays, matrices, strings
  - Math operations, matrix operations, postfix chaining, atoms, default params
  - List comprehensions, pattern matching, complex numbers, type checking
  - F-strings, destructuring, multiple returns, imports, and more
- **Parser errors** (2 tests) - Syntax errors detected during parsing (exit code 2)
  - Invalid operator sequences, missing tokens
- **Codegen errors** (2 tests) - Type/undefined errors during code generation (exit codes 100-105)
  - Undefined variables, type mismatches
- **Runtime errors** (2 tests) - Errors during program execution (exit code 1)
  - Division by zero, modulo by zero

**What Integration Tests Cover:**
- ✅ End-to-end compilation pipeline (lex → parse → codegen → link → execute)
- ✅ Actual `.bx` file compilation and execution
- ✅ Exit code validation (0, 1, 2, 100-105)
- ✅ Ariadne error messages in real scenarios
- ✅ Runtime safety checks (division by zero)
- ✅ System integration (clang linking, LLVM backend)

**Limitation:** Tests must run sequentially (`--test-threads=1`) because they compile to the same directory.

**Recently Completed (Feb 2026):**
- ✅ **Phase 5: Integration Tests** (COMPLETE - Feb 2026)
  - 68 end-to-end tests covering success and error cases
  - Exit code propagation from executed programs
  - Framework for testing real `.bx` compilation and execution
- ✅ **Phase E7: Final Polish** (COMPLETE - Feb 2026)
  - Exit codes diferenciados por tipo de erro (100-105, parser=2)
  - Documented error handling architecture
  - Division by zero runtime checks (int/mod operations)
  - Type error fixes (String + Int now shows proper error)
- ✅ **Phase E6: Real Spans in Errors** (458 lines modified, precise error locations)
- ✅ **Span Granularity Fix** - Parser uses chumsky Stream with spans
  - Spans now point to exact tokens instead of expression-level ranges
  - Ariadne highlights precise source locations (e.g., `undefined_var` not whole line)
- ✅ **Ariadne error reporting** (beautiful error messages with source context)
- ✅ **Error handling infrastructure** (CodegenError with 6 variants, Result types throughout)
- ✅ Invalid operator sequence detection (`1 ++ 2` now properly detected)
- ✅ Power operator right-associativity (`2**3**2 = 512`)
- ✅ Range with variables (`start : end` with required spaces)
- ✅ Postfix operation chaining (`.field`, `[index]`, `(args)` in any order)
- ✅ Matrix arithmetic (28 runtime functions + codegen logic)
- ✅ IntMatrix → Matrix automatic promotion
- ✅ C-style bitwise precedence (bitwise > comparison)

## Current Limitations & Known Issues

- **~32 eprintln!() calls remaining** - All critical errors converted to CodegenError; remaining are warnings/debug messages
- **unwrap() calls in helpers** - Isolated in Option-returning I/O helper functions and test utilities
- **No LLVM optimizations** - runs with `OptimizationLevel::None`
- **Single-file compilation** - multi-file imports not yet implemented
- **Operator refactoring postponed** - Binary/Unary operators still in lib.rs (see operators.rs annotations)

## Recent Fixes (Feb 2026)

- ✅ **Parser Span Precision** - Fixed chumsky parser to preserve source code spans
  - **Problem**: Parser was receiving `Vec<Token>` without spans, causing chumsky to generate spans based on vector indices (0, 1, 2...) instead of source positions
  - **Solution**: Changed to use `Stream::from_iter()` with `(Token, Span)` pairs
  - **Impact**: Ariadne now highlights exact tokens in error messages instead of whole expressions
  - **File**: `src/main.rs` line 52-58

## Intentional Limitations (Design Decisions)

- **Nested ternary operators not supported** - Use `match` or `if/else` instead for better readability
  ```brix
  // ❌ Not supported (poor readability)
  var x := a > b ? 1 : c > d ? 2 : 3

  // ✅ Use match instead
  var x := match {
      a > b -> 1,
      c > d -> 2,
      _ -> 3
  }
  ```

- **Nested arrays (arrays of arrays) not supported** - Use `Matrix` instead for better performance
  ```brix
  // ❌ Not supported (poor performance, cache-unfriendly)
  var nested := [[1, 2], [3, 4]]

  // ✅ Use Matrix instead (contiguous memory, Fortran-level performance)
  var m := zeros(2, 2)
  m[0, 0] := 1; m[0, 1] := 2
  m[1, 0] := 3; m[1, 1] := 4

  // Or use constructor helpers
  var identity := eye(3)  // 3x3 identity matrix
  ```
  **Rationale:** Brix prioritizes "Fortran-level performance" for numerical computing. Nested arrays
  (like Python's `[[1,2],[3,4]]`) store data non-contiguously in memory, causing:
  - 10x slower performance (cache misses, pointer chasing)
  - Incompatible with BLAS/LAPACK (requires contiguous data)
  - Contradicts the philosophy "Write like Python, execute like Fortran"

  Brix's `Matrix` and `IntMatrix` types store data contiguously (like Fortran, MATLAB, NumPy),
  making them much faster for numerical operations while maintaining clean syntax.

- **Ranges with variables require spaces** - To avoid conflict with atoms
  ```brix
  // ✅ Numeric ranges - no space needed
  for i in 0:10 { }

  // ✅ Variable ranges - space required
  for i in start : end { }
  ```

## Troubleshooting

**"runtime.c not found"**
- Ensure `runtime.c` exists in project root
- Compiler looks in current working directory

**Parser errors with valid code**
- **Brix does NOT use semicolons (`;`)** - statements are separated by newlines
- Example: `println(42)` NOT `println(42);`
- If you see "found Error" at position X, check if you added a semicolon
- Keywords like `var`, `function`, `println` are recognized automatically

**LLVM Errors**
- Requires LLVM 18: `brew install llvm@18` (macOS)
- Ensure `inkwell` feature `llvm18-0` matches your LLVM version

**"cc: command not found"**
- Needs C compiler for runtime.c
- macOS: `xcode-select --install`
- Linux: `apt install build-essential`

**Linking errors**
- Run clean build: `rm -f runtime.o output.o program && cargo clean && cargo build`

**"cannot find function/type in scope"**
- Codegen functions may need `pub` visibility for tests
- Tests in separate module need proper imports

## Development Roadmap

**Current Focus (Feb 2026):** ✅ **v1.2.1 - Error Handling Implementation (COMPLETE!)**
- ✅ Phase 1: Lexer unit tests (completed)
- ✅ Phase 2: Parser unit tests (completed - 150 passing, 0 ignored)
- ✅ Phase 3: Codegen unit tests (completed - 1001/1001 passing, 100%!)
- ✅ Phase 3.5: Bug fix sprint (completed - fixed 8/10 issues, see FIX_BUGS.md)
- ✅ Phase 4: Ariadne integration (completed - beautiful error messages!)
- ✅ **Phase R: Codegen refactoring (COMPLETED!)** - 7,338 → 6,499 lines (-11.4%)
 - ✅ Types module (BrixType enum)
 - ✅ Helpers module (LLVM utilities)
 - ✅ Builtins modules (math, stats, linalg, string)
 - ✅ Statements module (10/12 statements)
 - ✅ Expressions module (literals, ternary, etc.)
 - ⏸️ Operators module (postponed - annotated for future work)
- ✅ **Phase E: Error Handling (COMPLETE!)** - Replace unwrap() with Result types
  - ✅ **E1: Core error infrastructure** (completed)
    - Created `error.rs` with `CodegenError` enum (6 variants)
    - Created `CodegenResult<T>` type alias
  - ✅ **E2: Core module conversion** (completed - ~2000 lines)
    - `expr.rs` - All expression methods return `CodegenResult`
    - `stmt.rs` - All 12 statement methods return `CodegenResult`
    - `helpers.rs` - LLVM helpers with proper error handling
    - `lib.rs` - Main compilation methods (`compile_expr`, `compile_stmt`, `value_to_string`, etc.)
    - **All 1001 tests passing!** ✅
  - ✅ **E3: Auxiliary function conversion** (completed - 325 → 14 unwrap() calls!)
    - Binary/unary operators converted to Result types
    - All matrix arithmetic operations (28 functions)
    - Complex number operations (arithmetic, power, promotion)
    - String operations (concat, equality)
    - Logical short-circuit operators (AND, OR with PHI nodes)
    - Built-in function calls (int(), float(), string(), bool(), typeof())
    - Type checking functions (is_nil, is_atom, is_boolean, etc.)
    - Match expression compilation + pattern matching
    - Increment/Decrement operations
    - F-string compilation
    - FieldAccess and Index compilation
    - Array literal compilation
    - List comprehension + generator loop compilation
    - `compile_pattern_match` converted from Option → CodegenResult
    - `generate_comp_loop` converted from Option → CodegenResult
    - **14 remaining unwrap() calls** are in Option-returning I/O functions (compile_input_*, compile_read_csv, compile_matrix_constructor, compile_zip) - isolated and safe
    - **All 1001 tests passing!** ✅
  - ✅ **E4a: Basic Error Propagation** (completed - Feb 2026)
    - `compile_program()` returns `CodegenResult<()>`
    - main.rs catches and displays structured error messages
    - Replaced ~11 eprintln!() calls with proper CodegenError returns in critical paths:
      - Identifier compilation errors (undefined symbols, unsupported types)
      - Type conversion functions (int, float, string, bool, typeof)
      - Type checking functions (is_nil, is_atom, is_boolean, is_integer, is_float, is_number, is_string, is_list)
      - Operator errors (complex numbers, string operations)
    - Error display in CLI with colored, structured messages (6 error variants)
    - **~54 eprintln!() calls remaining** (mostly in debugging/fallback paths)
    - **All 1001 tests passing!** ✅
  - ✅ **E4b: AST Migration with Spans** (COMPLETED - Feb 2026)
    - ✅ **AST Structure Updated:**
      - Added `Span = Range<usize>` type
      - `Expr` changed from enum to `struct { kind: ExprKind, span: Span }`
      - `Stmt` changed from enum to `struct { kind: StmtKind, span: Span }`
      - Added helper methods: `Expr::new()`, `Expr::dummy()`, `Stmt::new()`, `Stmt::dummy()`
    - ✅ **Parser Fully Updated:**
      - All ~930 lines converted to use new AST structure
      - Pattern matches updated from `match expr {` to `match &expr.kind {`
      - Uses `.map_with_span()` from chumsky to capture real spans
    - ✅ **Codegen Fully Updated:**
      - Main codegen logic (~7300 lines) updated for new AST
      - Pattern matches converted to use `.kind` field
    - ✅ **CodegenError with Spans:**
      - Added `span: Option<Span>` field to 5 error variants
      - All ~654 locations updated with `span: None`
    - ✅ **All Tests Restored:**
      - Parser tests: 150 passing ✅
      - Codegen tests: 559 passing ✅
      - All test files converted to use `Expr::dummy(ExprKind::...)` and `Stmt::dummy(StmtKind::...)`
    - **All 1001 tests passing!** ✅
  - ✅ **E4c: Complete Ariadne Integration** (COMPLETED - Feb 2026)
    - ✅ Created `error_report.rs` module with Ariadne formatting
    - ✅ Updated `Compiler::new()` to accept `filename: String` and `source: String`
    - ✅ Implemented `report_codegen_error()` with beautiful error messages
    - ✅ All 6 CodegenError variants formatted with:
      - Error codes (E100-E105)
      - Colored labels pointing to source code spans
      - Contextual help messages
    - ✅ Updated all 559 codegen tests to pass filename and source
    - **All 1001 tests passing!** ✅
  - ✅ **E4d: Integrate Ariadne in main.rs** (COMPLETED - Feb 2026)
    - ✅ main.rs calls `report_codegen_error()` instead of `eprintln!()`
    - ✅ Updated `UndefinedSymbol` errors to capture `expr.span`
    - ✅ Beautiful error messages visible to end users
    - ✅ Tested with `.bx` files showing proper Ariadne formatting
    - **All 1001 tests passing!** ✅
    - **Known limitation:** Spans capture entire expressions, not just identifiers (parser-level improvement needed)
  - ✅ **E5: Cleanup eprintln!() and unwrap()** (COMPLETE - Feb 2026)
    - ✅ Converted 22/54 critical eprintln!() to CodegenError (54 → 32)
      - Argument validation → `InvalidOperation`
      - Type mismatches → `TypeError`
      - Undefined symbols → `UndefinedSymbol`
    - ✅ Remaining unwrap() calls isolated in I/O helpers and test utilities
    - ✅ Remaining 32 eprintln!() are warnings/debug messages (non-critical)
    - **All 1001 tests passing!** ✅
  - ✅ **E6: Add Real Spans to Errors** (COMPLETE - Feb 2026)
    - ✅ Captured source positions during expression/statement compilation
    - ✅ Replaced `span: None` with actual spans from AST throughout compilation pipeline
    - ✅ 458 lines modified in lib.rs to propagate spans correctly
    - ✅ Beautiful error messages with precise source code highlighting
    - ✅ All CodegenError variants now include accurate source locations
    - **All 1001 tests passing!** ✅
  - ✅ **E7: Final integration & polish** (COMPLETE - Feb 2026)
    - ✅ Exit codes for different error types (0, 1, 2, 100-105)
    - ✅ Exit code propagation from executed programs
    - ✅ Documentation of error handling architecture
    - **All 1001 tests passing!** ✅
- ✅ **Phase 5: Integration Tests** (COMPLETE - Feb 2026)
  - ✅ 68 end-to-end tests covering success and error cases
  - ✅ Exit code validation across all error types
  - ✅ Framework for testing real `.bx` compilation and execution
  - ✅ Test categories: success (64), parser errors (2), codegen errors (2), runtime errors (2)
  - **All 68 tests passing!** ✅

**Next Steps:**
- ⏭️ LLVM optimizations (-O2, -O3) - Add optimization levels
- Phase 6: Property-based tests (~20 tests)
- Complete operator refactoring (see operators.rs TODOs)

**Future Features:**
- v1.2: Documentation system (@doc), panic(), advanced string functions
- v1.3: Generics, Result<T,E>, Structs, Closures
- v1.3+: **Test Library** - Jest-style testing framework (`import test`) implemented in runtime.c
  - Matchers: `test.expect(x).to_equal(y)`, `to_be_greater_than()`, etc.
  - Structure: `test.describe()`, `test.it()`, `test.run()`
  - Smart float precision based on expected value decimals
  - Beautiful Jest-like output with pass/fail summary
  - See DOCUMENTATION.md section "🧪 v1.3+ - Test Library" for full API
- v1.4+: Concurrency, pipe operator, optional types, LSP, REPL

## Version Summary

**v1.2.1 (COMPLETE - Feb 2026):**
- ✅ **AST Migration with Spans** (Phase E4b - COMPLETE)
  - AST structure: `Expr { kind: ExprKind, span: Span }`, `Stmt { kind: StmtKind, span: Span }`
  - Parser, codegen, and ALL tests fully converted
  - CodegenError has `span: Option<Span>` on all variants
  - **All 1001 unit tests passing!** ✅
- ✅ **Error Handling with Result types** (Phase E1-E7 COMPLETE) 🎉
  - ✅ E1: Core error infrastructure (CodegenError enum with 6 variants)
  - ✅ E2: Core module conversion (expr.rs, stmt.rs, helpers.rs, lib.rs)
  - ✅ E3: Auxiliary function conversion (unwrap() calls isolated in helpers)
  - ✅ E4a: Basic error propagation to main.rs
  - ✅ E4c: Ariadne integration (error_report.rs module, beautiful errors)
  - ✅ E4d: Ariadne in main.rs (user-facing error messages)
  - ✅ E5: Cleanup eprintln!() and unwrap() (22/54 critical errors converted)
  - ✅ E6: Add real spans to errors (458 lines modified, all errors have source positions)
  - ✅ E7: Final polish (exit codes, runtime checks, documentation)
  - **All 1001 unit tests passing!** ✅
  - **Phase E COMPLETE!** 🎉
- ✅ **Integration Tests** (Phase 5 COMPLETE) 🎉
  - ✅ 68 end-to-end tests (success, parser errors, codegen errors, runtime errors)
  - ✅ Exit code validation (0, 1, 2, 100-105)
  - ✅ Real `.bx` compilation and execution
  - **All 68 integration tests passing!** ✅
  - **Total: 1069 tests (1001 unit + 68 integration) - 100% passing!** 🎉

**v1.2 (COMPLETE - Feb 2026):**
- ✅ Codegen refactoring - modular architecture (7,338 → 6,499 lines)
- ✅ error.rs, types.rs, helpers.rs, stmt.rs, expr.rs, builtins/ modules
- ✅ Comprehensive unit tests (1001/1001 passing - 100%)

**v1.1 (COMPLETE - Feb 2026):**
- ✅ Atoms (Elixir-style: `:ok`, `:error`)
- ✅ Escape sequences (\n, \t, \r, \\, \", \b, \f)
- ✅ Type checking functions (is_nil, is_atom, is_boolean, etc.)
- ✅ String functions (uppercase, lowercase, capitalize, replace, etc.)
- ✅ F-string escape fix
- ✅ **Matrix arithmetic** (28 runtime functions, all 6 operators)
- ✅ **IntMatrix → Matrix promotion** (automatic type promotion)
- ✅ **Postfix operation chaining** (`.field`, `[index]`, `(args)` in any order)
- ✅ **Right-associative power operator** (`2**3**2 = 512`)
- ✅ **C-style bitwise precedence** (bitwise > comparison)
- ✅ **Range with variables** (requires spaces: `start : end`)

**v1.0 (Jan 2026):**
- Pattern matching (match expressions with guards)
- Complex numbers + ComplexMatrix
- LAPACK integration (eigvals, eigvecs)
- Nil/Error types (Go-style error handling)

**v0.9 (Jan 2026):**
- List comprehensions (Python-style)
- zip() function
- Destructuring in for loops

**v0.8 (Jan 2026):**
- User-defined functions
- Multiple return values (tuples)
- Default parameter values

**v0.7 (Jan 2026):**
- Import system
- Math library (38 functions + constants)

For complete version history and feature details, see DOCUMENTATION.md.
