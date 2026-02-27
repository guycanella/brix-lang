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
cargo test -p lexer -p parser -p codegen  # All unit tests: 310 + 184 + 688 = 1,182 (all passing)
cargo test -p lexer                       # Only lexer (310 tests)
cargo test -p parser                      # Only parser (184 tests)
cargo test -p codegen                     # Only codegen (688 tests)
cargo test -p codegen arc_tests           # Specific test module in codegen
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
├── runtime.c                # C runtime (~2,769 lines) — must be in project root
├── crates/
│   ├── lexer/src/token.rs   # Token enum (logos)
│   ├── parser/src/
│   │   ├── ast.rs           # Expr { kind: ExprKind, span }, Stmt { kind: StmtKind, span }
│   │   ├── parser.rs        # chumsky parser (~1,450 lines)
│   │   ├── closure_analysis.rs  # Capture analysis pass (runs after parse)
│   │   └── error.rs         # Ariadne-based parse error reporting
│   └── codegen/src/
│       ├── lib.rs           # Main compiler (~13,656 lines)
│       ├── stmt.rs          # Statement compilation (~1,009 lines)
│       ├── expr.rs          # Expression compilation (~369 lines)
│       ├── helpers.rs       # LLVM helpers
│       ├── error.rs         # CodegenError enum + CodegenResult<T>
│       ├── error_report.rs  # Ariadne codegen error formatting
│       ├── types.rs         # BrixType enum
│       ├── operators.rs     # Operator logic (refactor postponed)
│       └── builtins/        # math.rs, stats.rs, linalg.rs, string.rs, io.rs, matrix.rs, test.rs
├── tests/
│   ├── integration/         # End-to-end .bx files (success/, parser_errors/, codegen_errors/, runtime_errors/)
│   └── brix/                # Language test files (*.test.bx) — 22 files, 372 tests, all passing
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
- **break/continue**: `Compiler` has `current_break_block` / `current_continue_block` (`Option<BasicBlock>`). Each loop saves the outer blocks, sets its own, restores after body. After emitting the unconditional branch, a dead basic block is appended to keep LLVM IR valid.

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
- **Closures**: heap-allocated `{ ref_count, fn_ptr, env_ptr, env_destructor }`, **capture-by-value for closures** (retain at capture time) / capture-by-reference for primitives, ARC via `closure_retain()`/`closure_release()`. `env_destructor` is generated when any captured var is itself a closure; it does a single `closure_release()` per captured closure field (no double-dereference).

### Test Library (`import test`)

Jest-style framework. 28 matchers across 14 categories: `toBe`, `not.toBe`, `toEqual`, `toBeCloseTo`, `toBeTruthy`, `toBeFalsy`, `toBeGreaterThan`, `toBeLessThan`, `toBeGreaterThanOrEqual`, `toBeLessThanOrEqual`, `toContain`, `toHaveLength`, `toBeNil`, `not.toBeNil`. Implemented in `builtins/test.rs` + `runtime.c`.

## Adding Features

**New operator:** Token in `lexer/src/token.rs` → precedence in `parser/src/parser.rs` → handler in `compile_binary_op()` in `lib.rs`.

**New built-in function:** External declaration in codegen → C implementation in `runtime.c` (auto-recompiled). Register in `builtins/` module.

**New global constructor/function (e.g., `ones`, `linspace`):** Add `if fn_name == "foo"` dispatch block in `lib.rs` (~line 6740, after the `eye` block) → add `compile_foo()` method in `lib.rs` (after `compile_eye`, before `compile_zip`) → add C implementation in `runtime.c`. For functions taking float args that may receive int literals, use `self.coerce_to_f64(val, &brix_type)` helper.

**New type:** Update `BrixType` enum in `types.rs`, `infer_type()`, `cast_value()`, `get_llvm_type()` in `lib.rs`.

**New iterator method on IntMatrix/Matrix:** Add match arm in `compile_iterator_method()` in `lib.rs`; add method name to the `matches!(field.as_str(), ...)` dispatch guard (~line 4292).

**Soft keywords** (context-sensitive, e.g., `step`): parsed as `Identifier("step")` in the lexer — no new `Token` variant needed. Match via `just(Token::Identifier("step".to_string()))` in `parser.rs`.

## Status & Limitations (v1.6 — Fases 0–3 complete)

**Completed in v1.6:**
- `break` / `continue` (Fase 0a): `Token::Break`/`Token::Continue`, `StmtKind::Break`/`StmtKind::Continue`, save/restore pattern on `Compiler`. Note: `break`/`continue` inside closures (e.g., `.map()` callbacks) is not supported.
- Nested closure ARC (Fase 0b): capture-by-value semantics; `env_dtor` uses single dereference; no double-free.
- String methods (Fase 1): `trim`, `ltrim`, `rtrim`, `starts_with`, `ends_with`, `contains`, `substring`, `reverse`, `repeat`, `index_of` (returns `int?`), `for ch in str` iteration. Implemented in `builtins/string.rs` + `runtime.c`.
- Matrix constructors (Fase 2a): `ones(n/r,c)`, `linspace(start,stop,n)`, `arange(start,stop,step)`, `rand(n/r,c)`, `irand(n,max)` — implemented in `runtime.c` + dispatched in `lib.rs` via `compile_ones/linspace/arange/rand/irand`. Helper `coerce_to_f64()` handles int→float coercion for float args. RNG seeded automatically via `__attribute__((constructor))`. Integration tests 124–129.
- 2D Matrix iteration (Fase 2b): `.map(fn)` preserves shape (allocates `matrix_new(rows, cols)`); `.filter(pred)` flattens to 1D; `.reduce()`, `.any()`, `.all()`, `.find()` iterate all `rows*cols` elements. Implemented in `compile_iterator_method()` in `lib.rs` — loads `rows` (field 1), computes `total = rows * cols`, uses `total` as flat loop bound. Integration tests 130–132; +4 codegen unit tests; +4 Test Library tests in `matrix.test.bx`.
- Float closure type inference (Fase 2c): `matrix.map((x: float) -> { return x * 2.0 })` works without explicit `-> float` annotation. Three new methods on `Compiler`: `infer_expr_type_static()` (static AST type walk), `collect_return_types()` (walks stmt tree gathering return expr types), `infer_return_type_from_body()` (drives inference with Float > String > Matrix > IntMatrix promotion). `infer_closure_return_type()` now falls through to body inference when `return_type` is `None`. Integration tests 133–135; +4 codegen unit tests; +4 Test Library tests in `matrix.test.bx`.
- `await` in nested control flow (Fase 3a): `await` inside `if`/`else` and `while` bodies within `async fn`. State machine extended with `var_field_map: HashMap<String, (u32, BrixType)>` for live variable preservation across suspension points. `WhileAwait` uses an `after_while_bb` merge block enabling multiple sequential `while` loops with `await`. Integration tests 136–141, 145; +4 codegen unit tests.
- `async { }` blocks (Fase 3b): Anonymous async state machines. Block struct layout has `poll_fn_ptr` at field 0 (enables indirect call by caller), `state` at field 1. Compiled by `compile_async_block()`. Integration tests 136–137; +4 codegen unit tests.
- Async closures (Fase 3c): `async (params) -> { await f() }` syntax. `is_async: bool` added to `Closure` AST node; parser detects `async` keyword before `(params) ->`. Compiled by `compile_async_closure()` — struct layout matches async blocks (poll_fn_ptr at field 0). Integration tests 142–143; +4 parser unit tests; +4 codegen unit tests.
- Async test matchers (Fase 3d): `test.it("name", async () -> { ... })` — codegen detects `BrixType::AsyncFuture` callback and calls `test_it_async(name, state_ptr, poll_fn)` instead of `test_it`. `test_it_async` drives the polling loop in `runtime.c`. Integration test 144; `async.test.bx` suite (3 tests).

**Current test baseline (post Fase 3):** 1,182 unit + 148 integration + 383 Test Library (23 `.test.bx` files)

**Planned for future phases:**
- `split`, `join` — require `StringMatrix` type (v1.7).
- Pattern matching destructuring and range patterns — v1.6 Phase 4.
- See `ROADMAP_V1.6.md` for detailed specs on Phase 4.

## Troubleshooting

- **Linking errors**: run clean build (see above)
- **"runtime.c not found"**: must run from project root
- **LLVM errors**: requires LLVM 18 — `brew install llvm@18`
- **Panic on unwrap()**: remaining `unwrap()` calls are isolated in Option-returning I/O helpers; check stack trace location
- **Parser errors with valid code**: Brix uses **newlines** as statement separators, not semicolons
- **Integration tests must be sequential**: `--test-threads=1` required (all tests compile to the same directory)
