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
cargo test --all              # Run all unit tests (560 tests total: 143 actual, 417 in codegen)
cargo test <pattern>          # Run tests matching pattern
cargo test -- --nocapture     # Show println! output
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
│   │   └── src/{ast.rs, parser.rs}
│   └── codegen/             # LLVM code generation (inkwell)
│       └── src/lib.rs       # Main codegen (7,154 lines - needs refactoring)
```

### Key Components

**1. Lexer (`crates/lexer`)**
- Uses `logos` crate for performance
- Token priority: `ImaginaryLiteral` (priority=3) > `Float` to avoid `2.0i` being parsed as float + identifier
- Atoms: `:atom_name` (priority=4) > `Colon`
- F-strings: `r#"f"(([^"\\]|\\.)*)"#` - accepts any escaped character

**2. Parser (`crates/parser`)**
- Uses `chumsky` parser combinators
- Operator precedence (lowest to highest):
  - Comparison/Logical: `<`, `<=`, `>`, `>=`, `==`, `!=`, `&&`, `||`
  - Bitwise: `&`, `|`, `^`
  - Additive: `+`, `-`
  - Multiplicative: `*`, `/`, `%`
  - Power: `**`
  - Atom: literals, identifiers, function calls, indexing
- For loops desugar to while loops during parsing
- Escape sequences processed via `process_escape_sequences()` helper

**3. Codegen (`crates/codegen`)**
- Uses `inkwell` (LLVM 18 bindings)
- Symbol table: `HashMap<String, (PointerValue, BrixType)>`
- All variables allocated via `alloca` on stack
- Control flow uses LLVM basic blocks (if/else, loops, match)
- **No PHI nodes for if/else** - values stored in alloca'd variables
- **PHI nodes used for**: ternary operator (`? :`), match expressions, logical short-circuit (`&&`, `||`)

**4. Runtime (`runtime.c`)**
- Provides C implementations of built-in functions (1,166 lines)
- Compiled to `runtime.o` by `src/main.rs` using system `cc`
- Linked with `-lm -llapack -lblas` for math/linear algebra
- Organized in sections: Atoms, Complex, Matrix, IntMatrix, ComplexMatrix, LAPACK, Errors, Strings, Stats, Linear Algebra, Zip
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
1. Run `cargo test --all` to verify baseline (should show 143 passing, 8 ignored)
2. Check which crate needs modification (lexer, parser, or codegen)
3. Review recent commits with `git log --oneline -10`
4. For new features: follow the Lexer → Parser → Codegen → Runtime order
5. See PARSER_BUGS.md for known parser issues (8 ignored tests)

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

**Automated Unit Tests:** 560 tests total (143 actual, 417 in codegen mock), 556 passing, 4 ignored
```bash
cargo test --all              # Run all tests
cargo test <pattern>          # Run tests matching pattern
cargo test -- --nocapture     # Show output from tests
```

**Test Organization:**
- `crates/lexer/src/tests/` - 5 modules (atoms, numbers, strings, tokens, edge cases)
- `crates/parser/src/tests/` - 7 modules (exprs, stmts, patterns, precedence, destructuring, errors, edge cases)
- `crates/codegen/src/tests/` - 12 modules (560 tests):
  - builtin_tests.rs (100 tests) - Math, stats, linear algebra, type checking, I/O
  - complex_tests.rs (30 tests) - Complex numbers, ComplexMatrix, LAPACK
  - stmt_tests.rs (40 tests) - Declarations, assignments, imports, destructuring
  - function_tests.rs (50 tests) - Default params, multiple returns, recursion, scoping
  - pattern_tests.rs (37 tests) - Type coercion, typeof() matching, complex patterns
  - string_tests.rs (35 tests) - Format specifiers, escape sequences, operations
  - control_flow_tests.rs (40 tests) - Loops, comprehensions, zip(), constructors
  - type_tests.rs (45 tests) - Type inference, casting, numeric edge cases
  - matrix_tests.rs (65 tests) - Constructors, indexing, field access, list comprehensions
  - expr_tests.rs (60 tests) - Literals, operators, ternary, short-circuit, chained comparisons
  - edge_cases.rs (50 tests) - Overflow, precedence, division, boolean, negative numbers
  - integration_tests.rs (15 tests) - Complex feature combinations

**Known Issues:**
- 8 parser tests ignored (see PARSER_BUGS.md):
  - Range with variables (lexer issue - 2 tests)
  - Nested ternary operators (not implemented - 1 test)
  - Function call chaining (not implemented - 1 test)
  - Field access on call result (not implemented - 1 test)
  - Bitwise precedence (design decision - 1 test)
  - Power associativity (should be right-associative - 1 test)
  - Error recovery (invalid operator sequence - 1 test)

## Current Limitations & Known Issues

- **595 unwrap() calls** - needs proper error handling with `Result<>`
- **Monolithic codegen** - 7,154-line lib.rs needs modularization into types.rs, builtins.rs, expr.rs, stmt.rs
- **Parse errors** - shown via debug output, Ariadne integration pending
- **No LLVM optimizations** - runs with `OptimizationLevel::None`
- **Single-file compilation** - multi-file imports not yet implemented
- **No integration tests** - only unit tests exist, need end-to-end `.bx` execution tests

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

**Current Focus (Feb 2026):** Test Infrastructure
- ✅ Phase 1: Lexer unit tests (completed)
- ✅ Phase 2: Parser unit tests (completed - 8 known issues documented)
- ✅ Phase 3: Codegen unit tests (completed - 560 tests, 556 passing, 4 ignored)
- 🚧 Phase 4: Integration/golden tests (next - end-to-end .bx execution)
- Phase 5: Property-based tests (~20 tests)

**After Testing:**
- Refactor codegen into modules (types.rs, builtins.rs, expr.rs, stmt.rs)
- Replace unwrap() with proper error handling
- Ariadne integration for beautiful error messages
- LLVM optimizations (-O2, -O3)

**Future Features:**
- v1.2: Documentation system (@doc), panic(), advanced string functions
- v1.3: Generics, Result<T,E>, Structs, Closures
- v1.4+: Concurrency, pipe operator, optional types, LSP, REPL

## Version Summary

**v1.1 (COMPLETE - Feb 2026):**
- ✅ Atoms (Elixir-style: `:ok`, `:error`)
- ✅ Escape sequences (\n, \t, \r, \\, \", \b, \f)
- ✅ Type checking functions (is_nil, is_atom, is_boolean, etc.)
- ✅ String functions (uppercase, lowercase, capitalize, replace, etc.)
- ✅ F-string escape fix

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
