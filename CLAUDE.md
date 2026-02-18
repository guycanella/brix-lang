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
cargo test --all              # Run all unit tests (1089 tests total, 100% passing)
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
- Provides C implementations of built-in functions (~1,600 lines)
- Compiled to `runtime.o` by `src/main.rs` using system `cc`
- Linked with `-lm -llapack -lblas` for math/linear algebra
- Organized in sections: Memory Allocation, ARC (Closures), Atoms, Complex, Matrix, IntMatrix, ComplexMatrix, LAPACK, Errors, Strings, Stats, Linear Algebra, Zip
- **ARC for Closures (v1.3):**
  - `BrixClosure` typedef: `{ ref_count, fn_ptr, env_ptr }`
  - `closure_retain()`: Increments reference count
  - `closure_release()`: Decrements and frees when ref_count = 0
  - `brix_malloc()`, `brix_free()`: Heap allocation wrappers
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
  typedef struct { long ref_count; void* fn_ptr; void* env_ptr; } BrixClosure;  // v1.3
  ```

## Type System

**19 Core Types:**
- `Int` (i64), `Float` (f64), `String` (BrixString*)
- `Matrix` (f64*), `IntMatrix` (i64*), `FloatPtr` (f64*)
- `Complex` (real+imag), `ComplexMatrix` (Complex*)
- `Tuple(Vec<BrixType>)` - multiple return values
- `Nil` (i8* null), `Error` (BrixError*), `Atom` (i64 ID)
- `Void` (no return)
- `Struct(String)` - user-defined types (v1.3)
- `Generic` - type parameters in generic functions/structs (v1.3)
- **Closure** - represented as `Tuple(Int, Int, Int)` internally (ref_count, fn_ptr, env_ptr) (v1.3)
- **TypeAlias(String)** - aliases for existing types (v1.4)
- **Union(Vec<BrixType>)** - tagged unions for sum types (v1.4)
- **Intersection(Vec<BrixType>)** - struct merging for composite types (v1.4)

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

### Closures (v1.3)
- **Syntax:** `(x: int, y: int) -> int { return x + y }` (parentheses-based)
- **Type annotations:** REQUIRED - no type inference for closure signatures
- **Capture:** By reference (pointers) for efficiency
- **Recursion:** PROHIBITED - use regular `function` declarations instead
- **Generic closures:** ALLOWED - `<T>(x: T) -> T { return x }`
- **Memory model:** Heap-allocated closures and environments
- **ARC:** Automatic Reference Counting via ref_count field
  - `closure_retain()` on load (copying reference)
  - `closure_release()` on reassignment
  - Memory freed when ref_count reaches 0
- **Representation:** `BrixType::Tuple(vec![Int, Int, Int])` - { ref_count, fn_ptr, env_ptr }
- **Calling:** Indirect calls via LLVM `build_indirect_call`
  - Extract fn_ptr and env_ptr from closure struct
  - Pass env_ptr as first argument to closure function

### Structs (v1.3)
- **Syntax:** `struct Point { x: int; y: int }` (multi-line or inline with semicolons)
- **Default values:** `struct Config { timeout: int = 30 }`
- **Construction:** `Point{ x: 10, y: 20 }` or `Config{ url: "..." }` (partial init with defaults)
- **Methods:** Go-style receivers - `fn (p: Point) distance() -> float { ... }`
- **Mutability:** All methods can modify receiver (no `mut` keyword needed)
- **Generic structs:** `struct Box<T> { value: T }` with type inference on construction
- **Memory model:** Heap-allocated via runtime (planned for v1.4+)
- **Name mangling:** Methods use receiver type - `Point_distance`, `Box_int_get`

### Generics (v1.3)
- **Functions:** `fn swap<T>(a: T, b: T) -> (T, T) { ... }`
- **Structs:** `struct Box<T> { value: T }`
- **Methods:** `fn (b: Box<T>) get() -> T { ... }`
- **Type inference:** Automatic from arguments - `swap(1, 2)` infers `T = int`
- **Constraints:** NONE - duck typing approach (compile error if operation not supported)
- **Monomorphization:** Compile-time specialization (like C++ templates, Rust generics)
- **Caching:** Aggressive to prevent code bloat
- **Name mangling:** `Box<int>` → `Box_int`, `swap<float>` → `swap_float`

### Type Aliases (v1.4)
- **Syntax:** `type MyInt = int`, `type Point2D = Point`
- **Zero overhead:** Resolved completely at compile time
- **Full transparency:** Alias is 100% equivalent to original type
- **Not a new type:** `MyInt` and `int` are interchangeable
- **Supports all types:** Primitives, structs, generics, unions, closures
- **Implementation:** Alias table in Compiler, recursive alias resolution

### Union Types (v1.4)
- **Syntax:** `var x: int | float = 42`, `var result: int | float | string = "error"`
- **Tagged union:** LLVM struct `{ i64 tag, largest_type value }`
- **Type safety:** Tag ensures runtime safety (0=first type, 1=second, etc.)
- **Pattern matching:** Full integration with match expressions
- **Nil support:** `int | nil` replaces Optional types
- **Optional refactoring:** `T?` is now syntactic sugar for `Union(T, nil)`
- **Implementation:** Tag checking, value extraction via LLVM struct operations

### Intersection Types (v1.4)
- **Syntax:** `var x: Point & Label = Point{...} & Label{...}`
- **Struct merging:** Combines fields from multiple structs
- **Field access:** All fields from both structs available
- **Method merging:** Methods from both structs accessible
- **Name collision:** Compile error if structs have same field name
- **Generic support:** Works with generic structs
- **Implementation:** LLVM struct with concatenated fields

### Elvis Operator (v1.4)
- **Syntax:** `a ?: b` (null coalescing operator)
- **Behavior:** Returns `a` if not nil, otherwise returns `b`
- **Short-circuit:** Doesn't evaluate `b` if `a` is not nil
- **Compatible with Union:** Works with any Union containing nil
- **Compatible with Optional:** Works with `T?` (which is `Union(T, nil)`)
- **Type safety:** Result has type of non-nil value
- **❌ Chaining NOT supported:** `a ?: b ?: c` is prohibited (use `match` instead)
- **Implementation:** Nil checking (Union tag check or pointer null check) + conditional branching + PHI node

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

**Automated Unit Tests:** 1089 tests total, **1089 passing (100%)**
```bash
cargo test --all              # Run all tests
cargo test <pattern>          # Run tests matching pattern
cargo test -- --nocapture     # Show output from tests
```

**Test Organization:**
- `crates/lexer/src/tests/` - 5 modules (atoms, numbers, strings, tokens, edge cases) - **292 passing**
- `crates/parser/src/tests/` - 7 modules - **158 passing**
  - exprs, stmts, patterns, precedence, destructuring, errors, edge cases
- `crates/codegen/src/tests/` - 13 modules - **588 passing**:
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
  - generic_tests.rs (21 tests) - Generic functions, structs, type inference, monomorphization, generic methods

**Remaining Ignored Tests:** None! All 1089 tests passing (100%)

### Integration Tests

**End-to-End Tests:** 95 tests total, **95 passing (100%)**
```bash
# IMPORTANT: Must run sequentially to avoid file conflicts
cargo test --test integration_test -- --test-threads=1

# Run with output
cargo test --test integration_test -- --test-threads=1 --nocapture
```

**Test Categories** (`tests/integration/`):
- **Success cases** (88 tests) - Programs that compile and execute successfully (exit code 0)
  - Hello world, arithmetic, variables, control flow, functions, arrays, matrices, strings
  - Math operations, matrix operations, postfix chaining, atoms, default params
  - List comprehensions, pattern matching, complex numbers, type checking
  - F-strings, destructuring, multiple returns, imports
  - Type aliases, union types, intersection types, Elvis operator, and more
- **Parser errors** (2 tests) - Syntax errors detected during parsing (exit code 2)
  - Invalid operator sequences, missing tokens
- **Codegen errors** (2 tests) - Type/undefined errors during code generation (exit codes 100-105)
  - Undefined variables, type mismatches
- **Runtime errors** (3 tests) - Errors during program execution (exit code 1)
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
- ✅ **v1.4 - Advanced Type System (COMPLETE - Feb 2026):**
  - ✅ **Type Aliases** - `type MyInt = int`, zero overhead, full transparency
  - ✅ **Union Types** - `int | float | string`, tagged unions with pattern matching
  - ✅ **Intersection Types** - `Point & Label`, struct merging via composition
  - ✅ **Elvis Operator** - `a ?: b`, null coalescing operator
  - ✅ **Optional → Union** - `int?` is now `Union(int, nil)`
  - All 1089 unit tests + 95 integration tests passing (100%)
- ✅ **v1.3 - Type System Expansion (COMPLETE - Feb 2026):**
  - ✅ **Structs** - Go-style receivers, default values, generic struct support
  - ✅ **Generics** - Monomorphization, type inference, generic methods
  - ✅ **Closures** - Capture by reference, heap allocation, full ARC
  - All 1038 unit tests + 69 integration tests passing (100%)
- ✅ **Phase 2.6: Generic Methods** (COMPLETE - Feb 2026)
  - Generic methods on generic structs (e.g., `Box<T>.get()`)
  - Monomorphization of methods per struct instantiation
  - User implemented parser solution for function/method disambiguation
  - 2 new unit tests, multiple integration tests
- ✅ **Phase 5: Integration Tests** (COMPLETE - Feb 2026)
  - 69 end-to-end tests covering success and error cases
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
- **Single-file compilation** - multi-file imports not yet implemented
- **Operator refactoring postponed** - Binary/Unary operators still in lib.rs (see operators.rs annotations)
- **ARC for closures and ref-counted types** - String, Matrix, IntMatrix, ComplexMatrix use ARC with `release_function_scope_vars()` at function exit and loop re-declaration release. Scope-level release (block-level) planned for v1.5+.

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

- **Chained Elvis operators not supported** - Use `match` or `if/else` for clarity
  ```brix
  // ❌ Not supported (poor readability, hard to debug)
  var x := a ?: b ?: c ?: d

  // ✅ Use match instead for multiple fallbacks
  var x := match {
      a != nil -> a,
      b != nil -> b,
      c != nil -> c,
      _ -> d
  }

  // ✅ Or if/else for simple cases
  var x := if a != nil { a } else if b != nil { b } else { c }
  ```
  **Rationale:** Chaining Elvis operators (`a ?: b ?: c`) creates hard-to-read code and makes debugging
  difficult. Using `match` or `if/else` provides clearer intent and better error messages.

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

**Current Focus (Feb 2026):** ✅ **v1.4 - Advanced Type System (COMPLETE)**

**✅ Completed Phases:**
- ✅ v1.2.1 - Error Handling Implementation (COMPLETE!)
- ✅ Phase 1: Lexer unit tests (completed)
- ✅ Phase 2: Parser unit tests (completed - 158 passing)
- ✅ Phase 3: Codegen unit tests (completed - 1038/1038 passing, 100%!)
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
- ✅ **LLVM Optimizations** (COMPLETE - Feb 2026)
  - ✅ Optimization levels: `-O0`, `-O1`, `-O2`, `-O3`
  - ✅ `--release` flag (equivalent to `-O3`)
  - ✅ Usage: `cargo run file.bx -O 3` or `cargo run file.bx --release`
  - ✅ Zero-overhead flag parsing via clap
  - ✅ All 1184 tests passing with optimizations enabled (1089 unit + 95 integration)
  - See DOCUMENTATION.md section "1.1. LLVM Optimizations" for details

**v1.3 - Type System Expansion (COMPLETE - Feb 2026):**
- ✅ **Phase 1: Structs (COMPLETE)** - 2-3 weeks
  - ✅ Phase 1.1: Parser (struct definitions, field initialization, Go-style receivers)
  - ✅ Phase 1.2: AST updates (StructDef, MethodDef, StructInit nodes)
  - ✅ Phase 1.3: Codegen (LLVM struct types, method compilation, field access)
  - ✅ Phase 1.4: Default values (compile-time initialization)
  - ✅ Phase 1.5: Generic structs integration (works with existing generics)
  - **Struct tests passing** (definitions, methods, defaults, generic structs)
  - **All 1038 unit tests + 69 integration tests passing** ✅

- ✅ **Phase 2: Generics (COMPLETE)** - 3-4 weeks
  - ✅ Phase 2.1: Generic Functions (parser, monomorphization, type inference)
  - ✅ Phase 2.2: Generic Function Calls (explicit and inferred types)
  - ✅ Phase 2.3: Type Inference System (deduce T from arguments)
  - ✅ Phase 2.4: Type Substitution (replace type params in signatures)
  - ✅ Phase 2.5: Generic Structs (struct definitions, construction, field access)
  - ✅ Phase 2.6: Generic Methods (Go-style receivers, monomorphization)
  - **21 generic tests passing** (functions, structs, methods, inference)
  - **All 1038 unit tests + 69 integration tests passing** ✅

- ✅ **Phase 3: Closures (COMPLETE)** - 4-5 weeks
  - ✅ Phase 3.1: AST implementation (`Closure` struct with params, return_type, body, captured_vars)
  - ✅ Phase 3.2: Parser (expression-based closure syntax with blocks)
  - ✅ Phase 3.3: Capture Analysis (automatic detection of captured variables)
  - ✅ Phase 3.4: Codegen - Closure Compilation
    - Environment struct creation with captured variable pointers
    - Closure function generation with env_ptr parameter
    - Closure struct: `{ ref_count, fn_ptr, env_ptr }`
  - ✅ Phase 3.5: Closure Calling (indirect calls via function pointers)
  - ✅ Phase 3.6: Heap Allocation (malloc/free for closures and environments)
  - ✅ Phase 3.7: ARC for Closures
    - Automatic reference counting (retain/release)
    - Memory management via ref_count field
    - Automatic retain on load, release on reassignment
  - **Closure tests passing** (capture, calls, ARC, heap allocation)
  - **All 1038 unit tests + 69 integration tests passing** ✅

- ✅ **v1.4 - Advanced Type System (COMPLETE - Feb 2026):**
  - ✅ **Task #1: Type Aliases (COMPLETE)** - 1 semana
    - Type alias syntax: `type MyInt = int`
    - Zero overhead compilation
    - Alias table with recursive resolution
    - 2 unit tests + 1 integration test
  - ✅ **Task #2: Union Types (COMPLETE)** - 2 semanas
    - Union type syntax: `int | float | string`
    - Tagged unions via LLVM struct
    - Pattern matching integration
    - 5 unit tests + 2 integration tests
  - ✅ **Task #3: Intersection Types (COMPLETE)** - 1.5 semanas
    - Intersection syntax: `Point & Label`
    - Struct merging via field concatenation
    - Method merging support
    - 3 unit tests + 1 integration test
  - ✅ **Task #4: Optional → Union (COMPLETE)** - 1 semana
    - Desugar `T?` to `Union(T, nil)`
    - Remove `BrixType::Optional`
    - Backward compatibility maintained
    - Tests verify all Optional code still works
  - ✅ **Task #5: Elvis Operator (COMPLETE)** - 1 semana
    - Elvis operator syntax: `a ?: b`
    - Nil checking (Union tag check, pointer null check)
    - Conditional branching with PHI nodes
    - 1 integration test + unit tests
  - **All 1089 unit tests + 95 integration tests passing** ✅

**Next Steps:**
- v1.5: Async/Await, Test Library, Iterators
- Phase 4: Additional documentation
- Phase 5: More stress tests
- Phase 6: Operator refactoring (cleanup lib.rs)
- LTO and PGO support (future optimization enhancements)

## v1.3 - Type System Expansion (Design Decisions)

**Status:** Planned after testing infrastructure completion

This version introduces fundamental type system features: **Closures**, **Structs**, and **Generics**. All design decisions documented below (Feb 2026).

### 1. Closures

**Syntax:**
```brix
// Single expression - needs braces
var double := (x: int) -> int { return x * 2 }

// Multi-line body - needs braces
var complex := (a: int, b: int) -> int {
    var result := a + b
    return result * 2
}

// As function parameter
fn map(arr: [int], fn: (int) -> int) -> [int] {
    // implementation
}
```

**Type Annotations:** REQUIRED - no type inference for closure signatures
```brix
var add := (x: int, y: int) -> int { return x + y }  // ✅ Required
```

**Variable Capture:**
- **By Reference** - closures capture pointers to variables (not copies)
- Rationale: Efficient for large types (Matrix, String)
- ARC manages lifetimes automatically
- Example:
  ```brix
  var matriz := zeros(1000, 1000)  // 8MB
  var sum := 0
  var closure := (x: int) -> int {
      return x + sum  // Captures pointer to 'sum' (8 bytes)
  }
  ```

**Recursion:** PROHIBITED in closures
- Recursive closures create circular type inference
- Use regular `function` declarations for recursion instead
- Example:
  ```brix
  // ❌ NOT ALLOWED
  var fib := (n: int) -> int {
      if n <= 1 { return n }
      return fib(n-1) + fib(n-2)  // ERROR: recursion in closure
  }

  // ✅ Use function instead
  function fib(n: int) -> int {
      if n <= 1 { return n }
      return fib(n-1) + fib(n-2)
  }
  ```

**Generic Closures:** ALLOWED
```brix
var identity := <T>(x: T) -> T { return x }

identity<int>(42)        // 42
identity<string>("hi")   // "hi"
```

---

### 2. Structs

**Syntax:**
```brix
// Multi-line: no commas
struct Point {
    x: int
    y: int
}

// Inline: semicolons
struct Point { x: int; y: int }

// With default values
struct Config {
    timeout: int = 30
    retries: int = 3
    url: string          // No default - required
}
```

**Construction:**
```brix
// All defaults
var cfg1 := Config{ url: "https://example.com" }

// Partial override
var cfg2 := Config{
    timeout: 60,
    url: "https://example.com"
}  // Uses default retries=3

// All fields specified
var point := Point{ x: 10, y: 20 }
```

**Methods (Go-style Receivers):**
```brix
struct Point {
    x: int
    y: int
}

// Receiver syntax: fn (receiver: Type) method_name()
fn (p: Point) distance() -> float {
    return sqrt(float(p.x**2 + p.y**2))
}

// Method call (dot notation)
var point := Point{ x: 3, y: 4 }
var dist := point.distance()  // 5.0
```

**Mutability:** NO `mut` keyword needed - all methods can modify receiver
```brix
fn (p: Point) move(dx: int, dy: int) {
    p.x += dx  // ✅ Allowed - modifies receiver
    p.y += dy
}

var point := Point{ x: 2, y: 3 }
point.move(5, 10)  // point is now {7, 13}
```

**Design Choice:** Go-style receivers instead of `extend` blocks
- Rationale: Follow Go conventions for consistency
- Simpler syntax for method definitions
- No need for extension namespacing

---

### 3. Generics ✅ **IMPLEMENTED (Feb 2026)**

**Generic Functions:**
```brix
// Angle bracket syntax with explicit types
fn swap<T>(a: T, b: T) -> (T, T) {
    return (b, a)
}

// Explicit type arguments
var result := swap<int>(1, 2)  // (2, 1)

// Type inference from arguments
var result := swap(1, 2)  // Infers T = int
```

**Generic Structs:**
```brix
// Single type parameter
struct Box<T> {
    value: T
}

// Multiple type parameters
struct Pair<A, B> {
    first: A
    second: B
}

// Construction - type inference from values
var box := Box{ value: 42 }           // Infers Box<int>
var pair := Pair{ first: 1, second: 3.14 }  // Infers Pair<int, float>
```

**Type Constraints:** NONE - duck typing approach
- No trait bounds or interface constraints
- Compilation error if type doesn't support required operations
- Example:
  ```brix
  fn add<T>(a: T, b: T) -> T {
      return a + b  // Compiles only if T has operator+
  }

  add(1, 2)        // ✅ int has operator+
  add("a", "b")    // ✅ string has operator+ (concat)
  add(:ok, :err)   // ❌ Compile error: Atom doesn't support operator+
  ```

**Monomorphization:** Code generation strategy
- Generates specialized code for each concrete type used
- Similar to C++ templates and Rust generics
- Trade-off: Larger binary size for better runtime performance
- Example: `Box<int>` and `Box<string>` generate separate LLVM functions
- Aggressive caching to prevent code bloat

**Generic Methods:**
```brix
struct Box<T> {
    value: T
}

// Method on generic struct
fn (b: Box<T>) get() -> T {
    return b.value
}

// Usage
var int_box := Box{ value: 42 }
println(int_box.get())  // 42 (calls Box_int_get)
```

**Implementation Details:**
- **Name Mangling:** `Box<int>.get()` → `Box_int_get()` in LLVM
- **Monomorphization Cache:** Prevents duplicate instantiations
- **Type Substitution:** Replaces type parameters (T → int) in signatures
- **Parser Solution:** Combined `fn_or_method` parser disambiguates via distinct tokens
  - Method path: starts with `LParen` (receiver syntax)
  - Function path: starts with `Identifier` (function name)
  - No token consumption conflict due to distinct starting tokens
- **Codegen:** Methods compiled when struct is instantiated

---

### 4. Error Handling (NO Result<T,E>)

**Decision:** Continue using Go-style error handling with tuples and nil
- No `Result<T, E>` type in v1.3
- Rationale: Already have working pattern with Error type and nil checking

**Pattern:**
```brix
fn divide(a: int, b: int) -> (float, Error) {
    if b == 0 {
        return (0.0, Error{ message: "division by zero" })
    }
    return (float(a) / float(b), nil)
}

// Usage
var result, err := divide(10, 2)
if err != nil {
    println(err.message)
} else {
    println(result)  // 5.0
}
```

---

**Future Features:**
- v1.2: Documentation system (@doc), panic(), advanced string functions
- v1.5+: **Async/Await** - High-performance concurrency via compile-time state machine transformation
  - Target: 0.2-0.3 MB/task (12x better than Go goroutines)
  - Compile-time transformation to state machines (like Rust tokio)
  - Runtime minimalista em C (~300 lines) com event loop
  - Syntax: `async function`, `.await`, `spawn { }`
  - See DOCUMENTATION.md section "🚀 v1.5+ - Concorrência e Paralelismo" for full design
- v1.5+: **Test Library** - Jest-style testing framework (`import test`) implemented in runtime.c
  - Matchers: `test.expect(x).to_equal(y)`, `to_be_greater_than()`, etc.
  - Structure: `test.describe()`, `test.it()`, `test.run()`
  - Smart float precision based on expected value decimals
  - Beautiful Jest-like output with pass/fail summary
  - See DOCUMENTATION.md section "🧪 v1.5+ - Test Library" for full API
- v1.5+: Pipe operator, iterators (map, filter, reduce), LSP, REPL

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
  - **All 69 integration tests passing!** ✅
- ✅ **v1.3 - Type System Expansion (COMPLETE - Feb 2026)** 🎉
  - ✅ **Structs:** Go-style receivers, default values, generic support
  - ✅ **Generics:** Functions, structs, methods with monomorphization
  - ✅ **Closures:** Capture by reference, heap allocation, full ARC
  - ✅ All type system features integrated and working together
  - ✅ 21+ generic tests, closure tests, struct tests
  - **All 1038 unit tests + 69 integration tests passing!** ✅
  - **Total: 1107 tests (1038 unit + 69 integration) - 100% passing!** 🎉

**v1.2 (COMPLETE - Feb 2026):**
- ✅ Codegen refactoring - modular architecture (7,338 → 6,499 lines)
- ✅ error.rs, types.rs, helpers.rs, stmt.rs, expr.rs, builtins/ modules
- ✅ Comprehensive unit tests (1001/1001 passing - 100%)

**v1.4 (COMPLETE - Feb 2026):**
- ✅ **Type Aliases (COMPLETE)** - `type MyInt = int`, zero overhead, full transparency
- ✅ **Union Types (COMPLETE)** - `int | float | string`, tagged unions with pattern matching
- ✅ **Intersection Types (COMPLETE)** - `Point & Label`, struct merging via composition
- ✅ **Elvis Operator (COMPLETE)** - `a ?: b`, null coalescing operator
- ✅ **Optional → Union (COMPLETE)** - `int?` is now `Union(int, nil)`
- **Total: 1184 tests (292 lexer + 158 parser + 639 codegen + 95 integration) - 100% passing!** 🎉

**v1.3 (COMPLETE - Feb 2026):**
- ✅ **Structs (COMPLETE)** - Go-style receivers, default values, generic struct support
- ✅ **Generics (COMPLETE)** - Monomorphization, type inference, generic methods
- ✅ **Closures (COMPLETE)** - Capture by reference, heap allocation, ARC memory management

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
