# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**CRITICAL**: Do not stop tasks early due to context limits. Always complete the full task even if it requires significant context usage.

## Commands

**Compile and run a Brix program:**
```bash
cargo run <file.bx>
cargo run <file.bx> -O 3        # With optimization
cargo run <file.bx> --release   # Equivalent to -O3
```

**Build only:**
```bash
cargo build
cargo build --release
```

**Run Rust unit tests:**
```bash
cargo test --all                          # All ~1,133 tests (100% passing)
cargo test -p lexer                       # Only lexer (304 tests)
cargo test -p parser                      # Only parser (174 tests)
cargo test -p codegen                     # Only codegen (655 tests)
cargo test <pattern>                      # Tests matching pattern
cargo test -- --nocapture                 # Show println! output
```

**Run integration tests (must be sequential):**
```bash
cargo test --test integration_test -- --test-threads=1
```

**Run Brix language tests (Test Library):**
```bash
cargo run -- test                   # All *.test.bx and *.spec.bx
cargo run -- test math              # Files matching "math" in path
```

**Clean build (fixes most linking errors):**
```bash
rm -f runtime.o output.o program && cargo clean && cargo run <file.bx>
```

## Architecture

### Compilation Pipeline

```
.bx source → Lexer (logos) → Parser (chumsky) → AST → Codegen (inkwell/LLVM 18) → Object + runtime.o → Native Binary
```

The driver (`src/main.rs`, ~314 lines) orchestrates all stages: lexing, parsing, closure analysis, codegen, `cc`-compiling `runtime.c`, LLVM object emission, and linking with `-lm -llapack -lblas`.

### Workspace Structure

```
brix/
├── src/main.rs              # CLI + compilation pipeline driver
├── runtime.c                # C runtime (~2,508 lines) — must be in project root
├── crates/
│   ├── lexer/src/token.rs   # Token enum (logos)
│   ├── parser/src/
│   │   ├── ast.rs           # Expr { kind: ExprKind, span }, Stmt { kind: StmtKind, span }
│   │   ├── parser.rs        # chumsky parser (~930 lines)
│   │   ├── closure_analysis.rs  # Capture analysis pass (runs after parse)
│   │   └── error.rs         # Ariadne-based parse error reporting
│   └── codegen/src/
│       ├── lib.rs           # Main compiler (~10,837 lines)
│       ├── stmt.rs          # Statement compilation (~998 lines)
│       ├── expr.rs          # Expression compilation (~369 lines)
│       ├── helpers.rs       # LLVM helpers
│       ├── error.rs         # CodegenError enum + CodegenResult<T>
│       ├── error_report.rs  # Ariadne codegen error formatting
│       ├── types.rs         # BrixType enum
│       ├── operators.rs     # Operator logic (refactor postponed)
│       └── builtins/        # math.rs, stats.rs, linalg.rs, string.rs, io.rs, matrix.rs, test.rs
├── tests/
│   ├── integration/         # End-to-end .bx files (success/, parser_errors/, codegen_errors/, runtime_errors/)
│   └── brix/                # Language test files (*.test.bx) — 21 suites, all passing
└── examples/                # Example .bx programs
```

### AST Structure

Both `Expr` and `Stmt` are structs with a `kind` field (enum) and a `span: Range<usize>` for error reporting:

```rust
struct Expr { kind: ExprKind, span: Span }
struct Stmt { kind: StmtKind, span: Span }
```

In tests, use `Expr::dummy(ExprKind::...)` and `Stmt::dummy(StmtKind::...)`.

### Error Handling

All codegen functions return `CodegenResult<T>` = `Result<T, CodegenError>`. Six error variants: `General` (E100), `LLVMError` (E101), `TypeError` (E102), `UndefinedSymbol` (E103), `InvalidOperation` (E104), `MissingValue` (E105). Parser errors exit with code 2; success exits with 0.

Ariadne formats errors with colored source spans. `Compiler::new()` takes `filename: String` and `source: String` to enable this.

### Symbol Table

Flat `HashMap<String, (PointerValue, BrixType)>` with module prefixes. `import math` adds entries like `"math.sin"`. All variables use `alloca` + `load`/`store`.

### Control Flow Internals

- **if/else statements**: basic blocks, no PHI nodes (values stored via alloca)
- **ternary / match / logical `&&`/`||`**: PHI nodes in merge block
- **for loops**: desugared to while loops at parse time
- **match**: one basic block per arm + PHI in merge block

### Type System (current: v1.5)

19 core types: `Int` (i64), `Float` (f64), `String`, `Matrix` (f64, contiguous), `IntMatrix` (i64), `Complex`, `ComplexMatrix`, `Tuple`, `Nil`, `Error`, `Atom` (i64 interned), `Void`, `Struct(String)`, `Generic`, `Closure` (represented as `Tuple(Int,Int,Int)` = ref_count/fn_ptr/env_ptr), `TypeAlias(String)`, `Union(Vec<BrixType>)`, `Intersection(Vec<BrixType>)`, `FloatPtr`.

Key rules:
- `[1,2,3]` → `IntMatrix`; `[1, 2.5]` → `Matrix` (int→float promotion)
- `IntMatrix op Float` → promotes to `Matrix` via `intmatrix_to_matrix()`
- `T?` desugars to `Union(T, nil)`
- Matrix `*` is element-wise (NOT matrix multiply); use `matmul()` for that
- `int[]` / `float[]` in type annotations map to `IntMatrix` / `Matrix`

### Ranges and Iterators (v1.5)

**Range syntax** (`ExprKind::Range` has fields `start`, `end`, `step: Option`, `inclusive: bool`):
- `0..5` — inclusive (SLE predicate), produces 0, 1, 2, 3, 4, 5
- `0..<5` — exclusive (SLT predicate), produces 0, 1, 2, 3, 4
- `0..10 step 2` — explicit step (`step` is a soft keyword parsed via `Identifier("step")`)
- Auto-step: when `step` is `None`, direction is inferred at runtime (`start > end` → step = -1)
- `[1..5]` / `[1..<5]` — array range literals, produce `IntMatrix` via `compile_range_to_array()`

**Iterator methods** on `IntMatrix` and `Matrix` (dispatched in `compile_iterator_method()`):
- `.map(fn)` — returns new array of inferred type (return type from closure annotation)
- `.filter(pred)` — returns new array with elements passing the predicate
- `.reduce(init, fn)` — returns scalar; fold with explicit initial value
- `.any(pred)` — returns `Int` (1/0); early exits on first match
- `.all(pred)` — returns `Int` (1/0); early exits on first non-match
- `.find(pred)` — returns `Union(elem_type, Nil)` tagged struct `{i64 tag, elem value}`

**Pipeline operator** (`|>`): `lhs |> method(args)` desugars to `lhs.method(args)` in the parser (level between range and ternary). Zero codegen changes — AST identical to method chaining.

### Generics & Structs

- **Monomorphization**: `swap<int>` and `swap<float>` generate separate LLVM functions
- **Name mangling**: `Box<int>.get()` → `Box_int_get` in LLVM
- **Methods**: Go-style receivers — `fn (p: Point) distance() -> float { ... }`
- **Closures**: heap-allocated `{ ref_count, fn_ptr, env_ptr }`, capture by reference, ARC via `closure_retain()`/`closure_release()`

### Test Library (`import test`)

Jest-style framework. 28 matchers across 14 categories: `toBe`, `not.toBe`, `toEqual`, `toBeCloseTo`, `toBeTruthy`, `toBeFalsy`, `toBeGreaterThan`, `toBeLessThan`, `toBeGreaterThanOrEqual`, `toBeLessThanOrEqual`, `toContain`, `toHaveLength`, `toBeNil`, `not.toBeNil`. Implemented in `builtins/test.rs` + `runtime.c`.

## Adding Features

**New operator:** Token in `lexer/src/token.rs` → precedence in `parser/src/parser.rs` → handler in `compile_binary_op()` in `lib.rs`.

**New built-in function:** External declaration in codegen → C implementation in `runtime.c` (auto-recompiled). Register in `builtins/` module.

**New type:** Update `BrixType` enum in `types.rs`, `infer_type()`, `cast_value()`, `get_llvm_type()` in `lib.rs`.

**New iterator method on IntMatrix/Matrix:** Add match arm in `compile_iterator_method()` in `lib.rs`; add method name to the `matches!(field.as_str(), ...)` dispatch guard (~line 4292).

## Known Limitations (v1.6 planned)

- `break` / `continue` not yet in lexer/AST/codegen
- Nested closures have ARC double-free issue (fixed for one level of nesting in v1.5 Phase 0b)
- **Async/await (v1.5 Phase 2 complete)**: `async fn` and `await` are fully implemented via LLVM state machines. Each `async fn` compiles to `create_{name}(params) -> i8*` + `poll_{name}(i8*) -> {status, value}`. `async fn main()` is driven by `brix_run_to_completion` in the C runtime. Limitation: only `var x := await f(args)` at the top level of a linear Block body is supported; `await` in nested control flow is not yet supported. `async { }` blocks are not yet supported in codegen.
- String functions not yet implemented: `trim`, `split`, `join`, `starts_with`, `ends_with`, `contains`, `substring`, `reverse`
- Matrix constructors not yet implemented: `ones()`, `linspace()`, `arange()`, `rand()`
- Iterators on `Matrix` (float arrays) — `map`/`filter`/`reduce`/`any`/`all`/`find` work on `IntMatrix`; `Matrix` dispatch exists but float closures require `-> float` return type annotation
- String iteration and 2D Matrix iteration (planned for v1.6)

## Troubleshooting

- **Linking errors**: run clean build (see above)
- **"runtime.c not found"**: must run from project root
- **LLVM errors**: requires LLVM 18 — `brew install llvm@18`
- **Panic on unwrap()**: remaining `unwrap()` calls are isolated in Option-returning I/O helpers; check stack trace location
- **Parser errors with valid code**: Brix uses **newlines** as statement separators, not semicolons
- **Integration tests must be sequential**: `--test-threads=1` required (all tests compile to the same directory)
