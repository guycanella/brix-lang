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

**Interactive REPL (v1.9 Grupo F — "replay REPL", not incremental JIT):**
```bash
cargo run -- repl
```
Each entry recompiles and re-runs the whole accumulated session through the same object+cc+ld pipeline as `cargo run <file.bx>` — see the v1.9 Grupo F entry below for what this does and doesn't guarantee (side effects replay, no persistent process state).

**Run Rust unit tests:**
```bash
cargo test -p lexer -p parser -p codegen  # All unit tests: 317 + 220 + 860 = 1,397 (all passing)
cargo test -p lexer                       # Only lexer (317 tests)
cargo test -p parser                      # Only parser (220 tests)
cargo test -p codegen                     # Only codegen (860 tests)
cargo test -p codegen json_tests          # Specific test module in codegen
cargo test <pattern>                      # Tests matching pattern
cargo test -- --nocapture                 # Show println! output
```

**Run integration tests (must be sequential):**
```bash
cargo test --test integration_test -- --test-threads=1 # 272 tests passing
```

**Run Brix language tests (Test Library):**
```bash
cargo run -- test                   # All *.test.bx and *.spec.bx (32 suites passing)
cargo run -- test json              # Files matching "json" in path
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

The driver (`src/main.rs`, ~507 lines; REPL in `src/repl.rs`) orchestrates all stages: lexing, parsing, closure analysis, codegen, `cc`-compiling `runtime.c`, LLVM object emission, and linking with `-lm -llapack -lblas`.

### Workspace Structure

```
brix/
├── src/main.rs              # CLI + compilation pipeline driver
├── runtime.c                # C runtime (~5,974 lines) — must be in project root
├── crates/
│   ├── lexer/src/token.rs   # Token enum (logos)
│   ├── parser/src/
│   │   ├── ast.rs           # Expr { kind: ExprKind, span }, Stmt { kind: StmtKind, span }
│   │   ├── parser.rs        # chumsky parser (~2,064 lines)
│   │   ├── closure_analysis.rs  # Capture analysis pass (runs after parse)
│   │   └── error.rs         # Ariadne-based parse error reporting
│   └── codegen/src/
│       ├── lib.rs           # Main compiler (~19,458 lines — post-refactor 11,014, grew with v1.8–v2.0 + rustfmt expansion)
│       ├── stmt.rs          # Statement compilation (~1,509 lines)
│       ├── expr.rs          # Expression compilation + list comprehension (~2,028 lines)
│       ├── helpers.rs       # LLVM helpers
│       ├── error.rs         # CodegenError enum + CodegenResult<T>
│       ├── error_report.rs  # Ariadne codegen error formatting
│       ├── types.rs         # BrixType enum
│       ├── operators.rs     # Operator logic (refactor postponed)
│       └── builtins/        # math.rs, stats.rs, linalg.rs, string.rs, io.rs, matrix.rs, test.rs,
│                            #   iterator.rs, match_compiler.rs, async_compiler.rs, closure_compiler.rs
├── tests/
│   ├── integration/         # End-to-end .bx files (success/, parser_errors/, codegen_errors/, runtime_errors/, test_library_failures/)
│   └── brix/                # Language test files (*.test.bx) — 32 files, 529 tests, all passing
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

### Type System (current: v2.0 in progress — Grupo A Fases 0–2 done)

Core types: `Int` (i64), `Float` (f64), `String`, `Matrix` (f64, contiguous), `IntMatrix` (i64), `StringMatrix` (array of `BrixString*`, v1.7), `Complex`, `ComplexArray`, `ComplexMatrix`, `Tuple`, `Nil`, `Error`, `Atom` (i64 interned), `Void`, `Struct(String)`, `Optional(Box)` (desugars to Union), `Union(Vec<BrixType>)`, `Intersection(Vec<BrixType>)`, `AsyncFuture`, `FloatPtr`, `Vector(Box<BrixType>)` (v1.8 Grupo C — dynamic `Vector<T>`, `BrixVector*`), `Stack`/`Queue`/`MinHeap`/`MaxHeap(Box<BrixType>)` and `HashMap(Box, Box)` (v1.8 D–F), `DateTime` (v1.9 A), `Json` (v1.9 B), and `Embedding(u32)` (v2.0 Grupo A — `BrixEmbedding*`, dim is a const generic).

**Scientific notation (v1.8):** float literals accept exponents — `6.0e23`, `1.5e-10`, `6.02E+23`, and integer-mantissa `1e10`; imaginary too (`1e3i`). Lexer `Float`/`ImaginaryLiteral` regexes; parser converts via `str::parse::<f64>()`.

Key rules:
- `[1,2,3]` → `IntMatrix`; `[1, 2.5]` → `Matrix` (int→float promotion)
- `IntMatrix op Float` → promotes to `Matrix` via `intmatrix_to_matrix()`
- `T?` desugars to `Union(T, nil)`
- Matrix `*` is element-wise (NOT matrix multiply); use `matmul()` for that
- `int[]` / `float[]` in type annotations map to `IntMatrix` / `Matrix`
- `StringMatrix` has no type-annotation syntax yet (`string[]` still maps to `IntMatrix`) — only reachable via inference (`var parts := "a,b".split(",")`); see Status & Limitations

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

Jest-style framework. 17 matchers (all support `.not.`): `toBe`, `toEqual`, `toBeCloseTo`, `toBeTruthy`, `toBeFalsy`, `toBeGreaterThan`, `toBeLessThan`, `toBeGreaterThanOrEqual`, `toBeLessThanOrEqual`, `toContain`, `toHaveLength`, `toStartWith`, `toEndWith`, `toMatch` (glob, `*` only), `toHaveProperty` (resolved at compile time via `struct_defs` — the property-name argument must be a string literal in the AST, not a variable), `toBeNil`, `toThrow` (v1.7 Grupo H — `fork()` + `waitpid()` in `runtime.c`; only supports a synchronous, zero-parameter closure literal passed directly to `test.expect(...)`, not a variable holding a closure). Dispatch in `compile_test_matcher()` in `lib.rs`; C implementations in `runtime.c` `SECTION 8`.

**`.not.` parsing (bugfix, v1.7 Grupo G):** `not` lexes as the keyword `Token::Not` (used for the `not x` prefix operator), so the parser's field-access postfix rule — which only accepted `Token::Identifier(name)` after a `.` — could never parse `.not.` as a field. This meant `not.toBe`/etc. were documented as working since v1.5 but had never actually parsed. Fixed by accepting `Token::Not` as an alternative there, mapped to the literal field name `"not"` (parser.rs, postfix `Field` production).

## Adding Features

**New operator:** Token in `lexer/src/token.rs` → precedence in `parser/src/parser.rs` → handler in `compile_binary_op()` in `lib.rs`.

**New built-in function:** External declaration in codegen → C implementation in `runtime.c` (auto-recompiled). Register in `builtins/` module.

**New global constructor/function (e.g., `ones`, `linspace`):** Add `if fn_name == "foo"` dispatch block in `lib.rs` (~line 8,758, next to the `ones` block) → add `compile_foo()` method in `lib.rs` (after `compile_irand` ~line 13,845, before `compile_zip`) → add C implementation in `runtime.c`. For functions taking float args that may receive int literals, use `self.coerce_to_f64(val, &brix_type)` helper.

**New type:** use the `/add-type` skill checklist. Minimum: `BrixType` enum + `format_brix_type()` in `types.rs`; `brix_type_to_llvm()`, `is_ref_counted()`, `insert_retain()`/`insert_release()`, `typeof()` and `infer_expr_type_static()` in `lib.rs`; plus the two non-exhaustive `match BrixType` gap sites (`stmt.rs` variable-decl allocation match and `lib.rs` `ExprKind::Identifier` variable-load match) — see v1.8 Grupo D below.

**New iterator method on IntMatrix/Matrix:** Add match arm in `compile_iterator_method()` in `builtins/iterator.rs`; add method name to the `matches!(field.as_str(), ...)` dispatch guard in `lib.rs` (~line 6,835) — note this guard is shared between iterator methods and string methods (see `is_iter_method`/`is_str_method` split, added in v1.7 Grupo D to fix a struct-init grammar ambiguity — see Status & Limitations).

**Soft keywords** (context-sensitive, e.g., `step`): parsed as `Identifier("step")` in the lexer — no new `Token` variant needed. Match via `just(Token::Identifier("step".to_string()))` in `parser.rs`.

**New pattern variant (e.g., for `match`):** Add variant to `Pattern` enum in `parser/src/ast.rs` → parse it in the `pattern` recursive block in `parser.rs` (inside `let pattern = recursive(|_pat| { ... })`) → add match arm to `compile_pattern_match()` in `builtins/match_compiler.rs`. For sub-patterns, use `apply_sub_pattern()` helper. Also add the variant's binding names to `collect_pattern_binding_names()` (v1.7 fix — used to scope match-arm bindings so they don't leak into the next arm; see Status & Limitations).

## Status & Limitations (v1.9 complete, v2.0 in progress)

**Completed in v1.6 (Fases 0–4):**
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
- Pattern matching 2.0 — Phase 4 (Fases 4a/4b/4c): Three sub-features added:
  - **Destructuring patterns** (`{ x, y }`, `{ x, 0 }`, `{ _, y }`): `Pattern::Destructure(Vec<Pattern>)` in AST; parsed with `{ atomic, ... }` syntax; codegen handles `Tuple`, `Struct`, `IntMatrix`, `Matrix`. Helper `apply_sub_pattern()` dispatches Wildcard/Binding/recursive sub-patterns. Integration test 149; +4 parser unit tests; +3 codegen unit tests; +4 Test Library tests.
  - **Range patterns** (`18..64`, `0..<10`, `0.0..<0.5`): `Pattern::Range { start, end, inclusive }` in AST; numeric token followed by optional `..`/`..<` suffix; codegen uses LLVM SLE/SLT (int) and OLE/OLT (float). Integration tests 150–151; +3 codegen unit tests; +3 Test Library tests.
  - **Universal destructuring** (`var { a, b } := struct_or_array`): `compile_destructuring_decl_stmt()` in `stmt.rs` extended from Tuple-only to also handle `BrixType::Struct` (field index extraction) and `BrixType::IntMatrix`/`BrixType::Matrix` (GEP from data pointer, no bounds check). Integration test 152; +2 codegen unit tests.

**Test baseline (post Phase 4, pre-v1.7):** 1,194 unit + 152 integration + 390 Test Library (23 `.test.bx` files)

**Test baseline at v1.9 completion:** 1,357 unit (317 lexer + 209 parser + 831 codegen) + 264 integration + 517 Test Library (31 `.test.bx` files).

**Current test baseline (v2.0 Grupo A Fase 2 COMPLETE):** 1,397 unit (317 lexer + 220 parser + 860 codegen) + 272 integration + 529 Test Library (32 `.test.bx` files). All green.

**Completed in v1.9 (Grupo F):**
- **Grupo F — `brix repl` — COMPLETE, as a "replay REPL", not the roadmap's original incremental-JIT design:**
  - **Deliberate scope reduction from the roadmap:** the roadmap's draft (`ROADMAP_V1.9.md`, no longer in the repo) called for a true incremental JIT via inkwell's `ExecutionEngine` — per-line LLVM modules, prior variables redeclared as `extern`, no re-execution of old code. That has no existing foothold anywhere in this compiler (every other path emits an object file and shells out to `cc`/`ld`) and would need to solve cross-module linking and ARC lifetime across module boundaries from scratch — real, `High`-risk engineering the roadmap itself flags. Implemented instead as a **replay REPL**: each entry recompiles and re-runs the *entire* accumulated session history through the existing object+cc+ld pipeline. This preserves every type's exact semantics (ARC, `Vector<string>`, JSON, closures — anything) for free, since each evaluation is a real, complete `compile_program()` run, not a partial approximation. True incremental JIT is deferred to v2.0.
  - **This has real, load-bearing limitations, documented rather than glossed over:** (1) prior side effects (`println`, `input()`, randomness, `datetime.now()`, file I/O) genuinely re-execute on every entry — only the *current* entry's output is shown to the user, the underlying effects still repeat; (2) performance is `O(history length)` — a full recompile + link happens on every entry; (3) it is not a persistent process in the JIT sense — there is no long-lived state beyond the accumulated source text itself.
  - **Pipeline refactor (prerequisite):** `src/main.rs` gained `compile_and_run_isolated(source: &str, opt_level: u8, workdir: &Path, label: &str) -> Result<EvalOutcome, ReplError>` — takes source as an in-memory string (never written to a `.bx` file), writes all artifacts (`runtime.o`, `output.o`, the executable, always named `program`) into a caller-supplied directory instead of the project root, never calls `exit()` (every failure path returns one of 8 `ReplError` variants), and captures the child binary's stdout/stderr instead of inheriting the parent's. **Deliberately not unified with the legacy `compile_to_exe`/`run_exe` path** used by `run_file`/`run_tests`: that path's ~250-test integration suite depends on the compiled binary being named after the file stem *in the repo root* (e.g. `./246_mut_array_push_pop`), a contract `compile_and_run_isolated`'s fixed `workdir/program` naming can't satisfy without special-casing that would defeat the point of extracting it. Accepted as intentional duplication (~120 lines) rather than risk that test suite.
  - **Implicit expression printing:** an entry that parses standalone as exactly one `StmtKind::Expr` (checked via `is_bare_expr()` in `src/repl.rs`, a throwaway parse with no codegen) is wrapped as `println(<expr>)` *only for that evaluation* — the unwrapped original text is what gets appended to history, so re-running it later doesn't double-print. Declarations, imports, and function/struct defs are never wrapped.
  - **Output isolation via a per-evaluation boundary marker:** since the whole history replays on every entry, isolating "just this entry's output" requires a marker, not a fixed sentinel byte (`\x1E` is not a recognized escape in the Brix lexer/parser and would be taken literally). Each evaluation injects `println("__BRIX_REPL_BOUNDARY_<pid>_<counter>__")` right before its own code, then the driver does `stdout.rsplit_once(marker)` and shows only what follows. The `pid`+monotonic-counter marker is unique per evaluation, not just per process, ruling out any practical collision with a user's own printed output.
  - **State rollback:** `ReplState.history: Vec<String>` is only pushed to *after* a full success (`Ok(outcome)` with `exit_code == 0`) — a parse/codegen/runtime failure never mutates it, so the REPL's visible state is exactly what the last all-successful entries produced. `:clear` empties it explicitly.
  - **`:type <expr>` — static inference only, never executes:** a new public codegen API, `codegen::infer_type_of_expr_in_context(history_source: &str, expr_src: &str) -> Result<BrixType, String>` (`lib.rs`, free function after the `Compiler` impl block), reconstructs the symbol table by running a real (but throwaway, in-memory-only) `Compiler::compile_program()` over the history — **this does lower to LLVM IR internally** (needed to populate `self.variables`/function signatures the same way a real run would) **but never links, never executes, and never touches disk** — then runs the existing (private, same-module-accessible) `infer_expr_type_static()` against the standalone-parsed expression. Deliberately not "compile-and-discard": an expr like `input()` or a function call with real side effects must never run just because its type was asked for. A new `pub fn format_brix_type(&BrixType) -> String` (`types.rs`) renders user-facing type names (`Vector<int>`, `int?`, `HashMap<string, int>`, etc.) for `:type`'s output instead of leaking `{:?}` Debug formatting; the pre-existing inline `typeof()` match in `lib.rs` was left untouched (not refactored to call the new function) to avoid any behavior-change risk to existing `typeof()` tests — accepted as intentional duplication between the two.
  - Special commands: `:quit`/`:q`, `:clear`, `:type <expr>`, `:help`. EOF (Ctrl+D) on stdin is treated the same as `:quit`.
  - **Codegen crate gained two new direct dependencies** (`logos`, `chumsky`) to support `infer_type_of_expr_in_context`'s own lex+parse of history/expr source — previously only `main.rs` (the driver binary) did lexing/parsing directly; codegen only ever consumed an already-built AST.
  - Integration tests 251–259 (`test_251_repl_basic` … `test_259_repl_help_command`), driving `cargo run -- repl` via piped stdin in `tests/integration_test.rs` (new `run_brix_repl()` helper) — not `.bx`+`.expected` file pairs like every other integration test, since there's no `.bx` file to compile, only a REPL session transcript. `test_255` specifically confirms a malformed entry (`y +`) neither corrupts state nor prevents a later entry (`println(y)`) from seeing the prior successful declaration; `test_256` asserts the boundary marker actually hides replayed output rather than just checking `stdout.contains(...)` — it counts occurrences of a first entry's printed text and fails if it leaks into a later entry's visible output; `test_257`/`test_258`/`test_259` cover `:type`, `:clear`, and `:help`, which the original 5-test batch didn't touch at all.
  - **Post-review fixes (3 issues found before this landed):**
    1. **`compile_and_run_isolated` used a relative `"runtime.c"` path** passed straight to `cc -c`, inherited unmodified from the legacy `compile_to_exe` path — which only ever worked because every existing caller happened to run from the repo root. Since the REPL is meant to be a general-purpose interactive tool, not something tied to a cwd, this broke immediately when invoked from anywhere else (`clang: error: no such file or directory: 'runtime.c'`). Fixed by resolving `runtime.c` via `Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime.c")` — baked in at compile time of the `brix` binary itself, so it's correct regardless of the REPL's cwd at runtime. The legacy file-based path (`compile_to_exe`) was deliberately left untouched — it's still documented as requiring cwd = project root (see Troubleshooting).
    2. **`:type` coverage was far narrower than a user would reasonably expect.** `infer_expr_type_static()` was written years earlier for one purpose only — inferring a `.map()`/`.filter()` closure's return type from its body — and never needed to know about module functions or container methods, so it had zero cases for `math.*`, `json.*`, or `Vector`/`Stack`/`Queue`/`MinHeap`/`MaxHeap`/`HashMap` methods; `:type math.sqrt(16.0)` (after `import math`) returned `"could not infer type"` despite the return type being unambiguous and fixed. Extended the same `ExprKind::Call`/`FieldAccess` match arm in `infer_expr_type_static()` (not `infer_type_of_expr_in_context()` — the fix belongs in the shared inference function so `.map()` callbacks benefit too) to cover: `math.*` (mirrors the real dispatch in `compile_expr`'s `Call` handler — `lu`/`qr`/`svd` → `Tuple`, `cholesky`/`solve` → `Matrix`, `norm`/`norm_mat` → `Float`, `tr`/`inv`/`eye` → `Matrix`, `eigvals`/`eigvecs` → `ComplexMatrix`, everything else gated on `self.module.get_function(...)` actually existing → `Float`); `json.*` (all 15 dispatch names — `null`/`bool`/`int`/`float`/`string`/`array`/`object` → `Json`, `get`/`index`/`parse` → `Union(Json, Nil)`, `as_string` → `Union(String, Nil)`, `as_int`/`as_bool` → `Union(Int, Nil)`, `as_float` → `Union(Float, Nil)`, `set`/`array_push` → `Void`, `array_len`/`tag` → `Int`, `stringify`/`stringify_pretty` → `String`); and container methods on `Vector`/`Stack`/`MinHeap`/`MaxHeap` (`pop`/`peek` → `Union(elem, Nil)`, `get` → `elem`, `to_array` → the matching `IntMatrix`/`Matrix`/`StringMatrix`), `Queue` (`dequeue`/`front` → `Union(elem, Nil)`), and `HashMap` (`get` → `Union(val, Nil)`, `has`/`delete`/`len` → `Int`) — `keys()` is left unresolved (returns `None`) since its result type (`IntMatrix` vs `StringMatrix`) depends on the key type and isn't threaded through here. Genuinely runtime-dependent expressions (anything reading a variable whose value isn't known statically, e.g. `input()`) still correctly report "could not infer type" — that limitation was never the bug; the bug was coverage gaps for things that were always statically known.
    3. **The REPL's per-session workdir (`$TMPDIR/brix_repl_<pid>`, containing every entry's `runtime.o`/`output.o`/`program`) was never cleaned up**, leaking a growing directory of compiled artifacts on every session. Fixed with a `WorkdirGuard` RAII type (`src/repl.rs`) whose `Drop` impl removes the directory — covers both normal exit paths (`:quit`/`:q` and EOF/Ctrl+D). Does **not** cover a `SIGINT` (Ctrl+C) kill, since Rust's default signal handling terminates the process without running destructors; that residual leak-on-kill case is accepted as a known limitation rather than pulling in a signal-handling crate for a REPL convenience feature.
  - v1.9 is now feature-complete across all 6 groups (A–F).

**Completed in v1.9 (Grupo E):**
- **Grupo E — Mutabilidade Controlada de Arrays (`mut`) — COMPLETE:**
  - `mut` keyword token in lexer (`token.rs`).
  - Parser support for `mut` in type annotations (`mut int[]`, `mut float[]`), allowing both `=` and `:=` after explicit type annotations (`stmt.rs`), and parsing method calls with optional trailing `!` (e.g. `push!`, `pop!`, `insert!`, `remove!`, `sort!`, `reverse!`, `clear!`).
  - Aliasing & Variable Mutability Tracking: `Compiler::mutable_variables: HashSet<String>` tracks variables declared with `mut`. Calling in-place methods (`!`) on immutable variables returns a `CodegenError::InvalidOperation`.
  - Receiver scope restriction: in-place methods (`!`) are strictly restricted to plain variable identifiers (`ExprKind::Identifier`).
  - C Runtime (`runtime.c` `SECTION 1.9`): 14 in-place array functions (`intmatrix_push_inplace`, `intmatrix_pop_inplace`, `intmatrix_insert_inplace`, `intmatrix_remove_inplace`, `intmatrix_sort_inplace`, `intmatrix_reverse_inplace`, `intmatrix_clear_inplace`, and float `matrix_*_inplace` equivalents). All restricted to 1D arrays (`rows == 1`), raising runtime error on 2D matrices or empty `pop!` / OOB `insert!`/`remove!`.
  - Integration tests 246–250 (`246_mut_array_push_pop.bx`, `247_mut_array_sort_inplace.bx`, `248_mut_immut_mix.bx`, `mut_array_on_immutable.bx`, `mut_array_non_identifier.bx`). All tests passing 100%.

**Completed in v1.9 (Grupos B–D)** — summarized from commit messages (no dedicated doc entry was written at the time):
- **Grupo B — `import json`:** `BrixType::Json` + `runtime.c` `SECTION 2.9`. Constructors `json.null/bool/int/float/string/array/object`; `json.parse`/`get`/`index` → `Union(Json, Nil)`; `as_string`/`as_int`/`as_bool`/`as_float` → optional; `set`/`array_push`/`array_len`/`tag`; `stringify`/`stringify_pretty(indent)`; index sugar `data["key"]`. Hardening fixes in `22c7d33`: UTF-8 multi-byte overflow guard, `\u0000` rejected, full control-char + key escaping in `stringify`, and an ARC regression where `StringMatrix` indexing lost its retain on assignment. Integration tests 230–238; `json.test.bx`.
- **Grupo C — variadic params (`...T`):** `FunctionParam` AST; must be last, no default. Lowered at the call site to a 1D `IntMatrix`/`Matrix`/`StringMatrix` (N=0 → `1×0`); `StringMatrix` element ARC delegated to `string_matrix_set`. Integration tests 239–244; `variadic_v19.test.bx`.
- **Grupo D — static `is_function()`:** `infer_expr_type_static()` handles `ExprKind::Closure`; `is_function(x)` emits a constant `i64 1`/`0` without compiling the argument (no spurious closure alloc/retain). Integration test 245; `type_checking.test.bx`.

**Completed in v1.9 (Grupo A):**
- **Grupo A — `import datetime` — COMPLETE:**
  - Standard library datetime implementation (`runtime.c` `SECTION 2.8`).
  - Heap-allocated `BrixDateTime` struct `{ long ref_count, int year, int month, int day, int hour, int minute, int second, int tz_offset_minutes }` with native ARC memory management (`datetime_new`, `datetime_retain`, `datetime_release`).
  - `BrixType::DateTime` representation in Rust compiler (`types.rs`, `builtins/datetime.rs`, `stmt.rs`, `lib.rs`).
  - Module functions: `datetime.now()`, `datetime.today()`, `datetime.timestamp()`, `datetime.from_timestamp()`, `datetime.format()`, `datetime.parse()`, `datetime.add_days()`, `datetime.add_hours()`, `datetime.add_minutes()`, `datetime.add_seconds()`, `datetime.diff_seconds()`.
  - `datetime.parse()` returns `Union(DateTime, Nil)` (tagged struct `{ i64 tag, ptr val }`) — returns NULL / tag 1 on invalid parse input.
  - Transparent pointer extraction via `extract_datetime_ptr()` in codegen allows calling methods, performing field access, and running comparisons directly on both `DateTime` and `Union(DateTime, Nil)` instances.
  - Field access (`.year`, `.month`, `.day`, `.hour`, `.minute`, `.second`, `.tz_offset` / `.tz_offset_minutes`) via GEP load and sign extension to `i64`.
  - All 6 comparison operators (`<`, `<=`, `>`, `>=`, `==`, `!=`) implemented via `datetime_compare(a, b)` returning `-1, 0, 1` in `runtime.c`.
  - Local timezone contract: `now()`, `today()`, and `from_timestamp()` use `localtime_r` and local timezone offset in minutes.
  - Integration tests 224–229 in `tests/integration/success/`; +6 codegen unit tests in `crates/codegen/src/tests/datetime_tests.rs`; +11 Test Library tests in `datetime.test.bx` (including overflow protection, calendar validation for impossible dates, and Union ARC retain on copy). All tests passing 100%.

**Completed in v1.7 (Grupos A–I, all complete):**
- **Grupo A** `BrixType::StringMatrix` + `.split()` / `join()`: new type `{ ref_count, len, BrixString** data }` in `runtime.c` (`SECTION 2.3`), with `string_matrix_new/get/set/retain/release`, `brix_str_split`, `brix_str_join`. `split()` creates each `BrixString*` with `ref_count=1` and inserts it directly into `data[i]` (not via `string_matrix_set`, which retains — that helper exists for future use but is currently dead code in codegen). Wired into `lib.rs`: `brix_type_to_llvm`, `is_ref_counted`, `insert_retain`/`insert_release`, new `get_string_matrix_type()` helper, `.len` field access, indexing (`sm[i]` → `string_matrix_get`, borrowed pointer), `for part in string_matrix` iteration, `value_to_string` (formats as `["a", "b", "c"]`), `.split()` dispatch in `compile_string_method`, global `join(arr, sep)` dispatch. **ARC note:** indexing a `StringMatrix` returns a borrowed `BrixString*` still owned by the matrix — both `is_print_temp()` (lib.rs) and the bare-expression-statement release check (`compile_expr_stmt` in `stmt.rs`) special-case `ExprKind::Index` for `BrixType::String` to avoid releasing it. **Known limitations:** no type-annotation syntax for `StringMatrix` yet (`string[]` still maps to `IntMatrix` — only reachable via inference); `var x := "...".split(...)` leaks the matrix and its constituent strings, same pre-existing pattern as `var x := ones(...)` (see `should_retain` in `stmt.rs`, which excludes `Call` results from retain-adjustment) — not a regression, not yet fixed; no bounds checking on `sm[i]` (returns `NULL` silently out of range), consistent with `Matrix`/`IntMatrix` indexing (which also has zero bounds checking) — not a Grupo A regression. Integration tests 153–155; +2 codegen unit tests; +8 Test Library tests in `strings_v17.test.bx`. Post-review fixes: a CRITICAL use-after-free (`ExprKind::Index` missing from the "borrowed" check in `compile_expr_stmt` — a bare `parts[i]` statement released a string still owned by the matrix) and two Medium findings (`infer_expr_type_static()` didn't recognize `.split()`; `string_matrix_set()` had a self-assignment use-after-free), all fixed.

- **Grupo B** New array methods on `IntMatrix`/`Matrix`: `.sort()`, `.sort_desc()`, `.min()`, `.max()`, `.flatten()`, `.unique()`, `.reverse()`, `.append()`, `.prepend()`, `.count()`. 18 new C functions (`runtime.c` `SECTION 1.8`); `builtins/matrix.rs` populated for the first time with a `MatrixFunctions` trait (mirrors `builtins/string.rs`'s pattern); 10 dispatch arms + `compile_array_*` helpers in `compile_iterator_method()`. Integration tests 156–160; +10 codegen unit tests; +10 Test Library tests (`arrays_v17.test.bx`). Post-review fixes: `coerce_to_i64()` added (rejects non-Int args to `.append()`/`.prepend()` on `IntMatrix` with a `CodegenError` instead of panicking); `infer_expr_type_static()` extended to cover the new methods; the shared iterator/string-method dispatch (`is_iter_method`/`is_str_method`) now propagates receiver-compile errors via `?` instead of silently swallowing them.

- **Grupo C** Array slicing `arr[1..4]`/`arr[1..<4]` (closed range only) and negative indexing `arr[-1]`/`arr[i-1]` (adjusted at **runtime**, `idx < 0 ? idx + len : idx` — works for literals and computed expressions alike, not just static negative literals) for `IntMatrix`/`Matrix`, in both read and assignment paths. New `matrix_slice`/`intmatrix_slice` in `runtime.c`. **Descoped from the roadmap:** open-ended ranges (`arr[..<3]`, `arr[2..]`) would need `Range`'s `start`/`end` to become optional in the parser (a bigger, riskier change touching every existing `Range` consumer); 2D row extraction via `mat[1]` conflicts with the flat single-index semantics Fase 2b's `.map()`/`.filter()`/`.flatten()` already test and depend on. Integration tests 161–162; +7 codegen unit tests (4 original + 3 review-fix regressions below); +4 Test Library tests. Post-review fixes: clamp `start`/`end` in `matrix_slice`/`intmatrix_slice` (prevented a heap out-of-bounds read on negative/over-length ranges); reject stepped ranges (`arr[0..4 step 2]`) as a slice index instead of silently discarding `step`; reject non-`Int` slice bounds with a `CodegenError` instead of panicking on `into_int_value()`; fix negative single-index adjustment to use `rows*cols` (not bare `cols`), correct for a flat index into a 2D matrix.

- **Grupo D** Named field patterns in `match`: `{ x: px, y: 0 } -> ...` for structs — `Pattern::NamedField(Vec<(String, Pattern)>)` in AST, resolves field index by **name** via `struct_defs` (not position) in `compile_pattern_match()`, reusing `apply_sub_pattern()` from `Pattern::Destructure`. **Descoped:** Union type-tag matching (`int: n -> ...`) needs a `Pattern::TypeTag` variant that doesn't exist and wasn't built. Integration tests 164–165; +6 parser unit tests (4 for the feature + 2 for the grammar-ambiguity fix below) + 3 codegen unit tests; +4 Test Library tests. **Two bugs found and fixed along the way (not Grupo D regressions — pre-existing):** (1) Union's `max_type` sizing in `brix_type_to_llvm()` — `LLVMType::size_of()` on an aggregate isn't always constant-foldable, so the union's value field was silently undersized for any variant wider than 8 bytes (e.g. `Complex`'s `{f64,f64}`), overflowing the stack allocation on write; fixed with a structural size computation, `llvm_type_byte_size()`. (2) A grammar ambiguity: named-field pattern syntax (`{ x: 0, y: py }`) is byte-for-byte identical to struct-init syntax, so when a match arm's body was a bare identifier followed by a newline and the next arm was a named-field pattern, the parser greedily read it as `identifier { field: value }` struct-init, swallowing the next arm. Fixed by giving `Token::LBrace` a "preceded by newline" bool (`crates/lexer/src/token.rs`); non-generic struct-init only matches same-line braces (`lbrace_same_line()` helper in `parser.rs`, vs. `lbrace()` for the ~9 positions where the distinction doesn't matter).

- **Grupo E** Array rest patterns: `{ first, ...rest } -> ...` for `IntMatrix`/`Matrix` — `Pattern::ArrayRest { head: Vec<Pattern>, rest: String }` in AST. **Uses `{ }`, not `[ ]`** (deliberate deviation from the roadmap): array destructuring already used `{ }` via `Pattern::Destructure`, and introducing `[ ]` only for the rest case would've created two notations for the same thing. New `Token::DotDotDot` (`...`). Reuses `matrix_slice`/`intmatrix_slice` (Grupo C) for the `rest` sub-array. Integration tests 166–168; +5 codegen unit tests (3 original + 2 review-fix regressions) + 6 parser unit tests (4 original + 2 review-fix regressions) + 2 lexer unit tests; +3 Test Library tests. Post-review fixes: (1) reject `...rest` outside the last position and multiple `...rest` captures in one pattern at parse time (both used to be silently accepted with misleading semantics); (2) gate `head` element reads and the `rest` slice allocation behind real conditional branches (PHI-merge, same shape as the ternary operator) instead of a straight-line AND chain — the slice call is a real heap allocation that used to run unconditionally on every arm attempt, even ones failing the length check; (3) **critical fix:** the match-arm loop compiled a guard (`if cond ->`) unconditionally right after the pattern check, but `ArrayRest` only binds `rest` on its matched-path block — a guard referencing `rest` read an uninitialized pointer (SIGSEGV) whenever the length check failed. Fixed by branching on the pattern's boolean result *before* compiling the guard, so the guard only ever runs on the path where its bindings are valid — this is a general fix in the match-arm loop, not `ArrayRest`-specific.

- **Grupo F** Match exhaustiveness is now a compile error (`E102`) instead of a warning: every `match` needs a root-level `Wildcard` (`_`) or bare `Binding` arm **without a guard**, or it fails to compile. Fixes a pre-existing inconsistency where the old warning only recognized `Wildcard`, not `Binding` (a single bare-binding arm wrongly warned). **Descoped:** per-variant Union/Atom coverage checking (the roadmap's main example) isn't implementable — there's no `Pattern::TypeTag` to prove a Union variant was handled, and Atoms are a free-form/open set, not a closed enum — so the rule applies uniformly regardless of scrutinee type. +5 codegen unit tests (3 original + 2 review-fix regressions); +3 integration tests in `tests/integration/codegen_errors/` (05–07); 7 existing codegen unit tests and 27 `match` blocks across 6 existing test files needed a `_` arm added to keep compiling under the new rule. **Bug found and fixed:** a guarded catch-all arm (`n if cond -> ...`) used to satisfy exhaustiveness on its own even though the guard can fail at runtime with nothing to fall through to — fixed by requiring the catch-all arm to be unguarded.

- **Grupo G** Test matchers `toStartWith`, `toEndWith`, `toMatch` (simple glob, `*` only), `toHaveProperty` (resolved at compile time via `struct_defs`; the property-name argument must be a string literal in the AST) — all support `.not.` (widened from the roadmap's two). 8 new C functions in `runtime.c` `SECTION 8`. Integration tests 169–171; +6 codegen unit tests; +6 Test Library tests (`test_matchers_v17.test.bx`) + 2 fail-path regression tests in the new `tests/integration/test_library_failures/` directory. **Bug found and fixed (bigger than Grupo G):** `.not.` had never been parseable in *any* Test Library matcher since v1.5 — see the "Test Library" section above for the root cause and fix; +2 parser unit tests + 2 Test Library regression tests (`not.toBe`) pin it.

- **Grupo H** `panic(msg: string)` built-in (`fprintf` to stderr + `exit(1)`) and the `toThrow`/`not.toThrow` matcher via `fork()` + `waitpid()` — the child process calls the closure and `_exit(0)`s if it returns normally; `panic()` inside already `exit(1)`s directly. `fflush(NULL)` before `fork()` prevents duplicated buffered stdout between parent and child. **Scope restricted by design:** `toThrow` only supports a synchronous, zero-parameter closure **literal** passed directly to `test.expect(...)` — a closure stored in a variable loses its LLVM signature once `BrixType::Closure` collapses it to a generic `Tuple`, so a correctly-typed indirect call for it isn't safely buildable yet. Integration tests 172–173 + 1 fail-path test; +6 codegen unit tests (5 for `toThrow` + 1 regression for the bug below); +2 Test Library tests. **Bug found and fixed (broader than Grupo H — affects `.map()` too):** `compile_closure()` unconditionally declared any closure without an explicit `-> type` annotation as an LLVM `void`-returning function, regardless of what its body actually returned, while every caller of such a closure (`.map()`, now `toThrow`) independently infers the real return type and builds its indirect-call signature from that instead — producing IR where the function's own `define void` header disagreed with its `ret <value>` body (undefined behavior at the LLVM level, that only "worked" because caller and body happened to agree with each other). Fixed by having `compile_closure()` perform the same inference (`infer_return_type_from_body()`) its callers already assumed, falling back to `Void` only when the body has no return statement at all.

- **Grupo I** List comprehension result-type inference: `[x * 2 for x in [1, 2, 3]]` now produces `IntMatrix` instead of always defaulting to `Matrix` (float). In `compile_list_comprehension()`, the previously-hardcoded `result_elem_type` is now inferred per-generator (`Int` for an `IntMatrix` iterable, `Float` otherwise) via `infer_expr_type_static()`, with all of a generator's `var_names` bound to the same type — correct even for destructuring generators (`for a, b in m`, note: **no parentheses**), since a `Matrix`/`IntMatrix` row is always homogeneously typed. Falls back to `Float` when inference can't resolve something (preserves prior behavior for those cases; a non-`Matrix`/`IntMatrix` iterable like a `StringMatrix` from `.split()` is still rejected by the pre-existing iterable-type check regardless of what this inference produces). Integration test 174; +5 codegen unit tests; +3 Test Library tests. No regressions, no additional fixes needed — reviewed clean.

**`lib.rs` refactor — COMPLETE.** (`REFACTOR_LIB.md` is no longer in the repo.) Split `lib.rs` into dedicated modules across 6 extractions, zero behavior change, all test counts identical to baseline (lexer 312 + parser 202 + codegen 753 unit, 179 integration, 434 Test Library):

1. `compile_list_comprehension` + `generate_comp_loop` → `expr.rs`
2. `compile_test_matcher` + `try_compile_test_call` + helpers → `builtins/test.rs`
3. `compile_iterator_method` + `compile_array_*` + `call_array_*` → `builtins/iterator.rs`
4. `compile_pattern_match` + `apply_sub_pattern*` + `collect_pattern_binding_names` → `builtins/match_compiler.rs`
5. `compile_async_fn_def`/`_nested`/`_closure`/`_block` → `builtins/async_compiler.rs`
6. closure codegen (`compile_closure`, `closure_retain`/`_release`, `load_closure_fn_env`, `infer_closure_return_type`, `is_closure_type`) → `builtins/closure_compiler.rs`

Result: `lib.rs` **17,953 → 11,014 lines (−39%)**; codegen crate 12 → 20 files. The general per-type ARC dispatch (`is_ref_counted`/`insert_retain`/`insert_release`/`release_function_scope_vars`) and the `ExprKind::Match` handler + exhaustiveness check stay inline in `lib.rs`. Extracted functions that are called from `lib.rs` or other modules are `pub(crate)`; purely-internal helpers stay private. The `REFACTOR_LIB.md` <9,000-line target was aspirational and not reached — the cohesive extractable blocks totaled ~6,900 lines; the primary criterion (zero behavior change) is fully met.

## v1.8 Status (COMPLETE) — `ROADMAP_V1.8.md` is no longer in the repo

Order: Grupo A (physical constants) → B (LAPACK) → C (`Vector<T>`) → D (`Stack`/`Queue`) → E (heaps) → F (`HashMap`).

**Grupo A — COMPLETE** (`Phase v1.8 Grupo A completed`): 8 physical constants (`math.c_light`, `h_planck`, `G_grav`, `k_boltzmann`, `e_charge`, `g_earth`, `avogadro`, `R_gas`) as f64 globals in `register_math_constants` (`builtins/math.rs`) — no dimensional units (documented in comments). Also added **scientific-notation literals** (lexer, prerequisite). A dedicated `ROADMAP_UNITS.md` (exploratory, NOT scheduled) analyzes a compile-time units-of-measure system — decided **not worth it now**.

**Grupo B — COMPLETE** (LAPACK decompositions). 7 functions in `runtime.c` + `lib.rs`:
- `math.lu(A)` → `Tuple(Matrix L, Matrix U, IntMatrix P)` (dgetrf; P is a 0-based permutation vector; singular `info>0` still returns factors).
- `math.qr(A)` → `Tuple(Q m×m, R m×n)` (dgeqrf+dorgqr, full).
- `math.svd(A)` → `Tuple(U m×m, S vec, Vt n×n)` (dgesdd; S is a 1-D vector).
- `math.cholesky(A)` → `Matrix L` (dpotrf('L'), upper triangle zeroed).
- `math.solve(A, b)` → `Matrix x` (dgesv; b is n×nrhs or a length-n vector in either orientation; rejects singular).
- `math.norm(v)` → `Float` (L2); `math.norm_mat(A[, code])` → `Float` (int code: 0=Frobenius, 1=1-norm, 2=inf-norm; rejects other codes).
- Codegen: `compile_math_matrix_tuple` (shared helper for lu/qr/svd — opaque-pointer ABI, unpacks the heap `*Result` struct into a Brix `Tuple`, frees the container) + `compile_math_simple_builtin` (cholesky/solve/norm) + `compile_math_norm_mat`. All reject empty matrices. **matmul is not a Brix global** (`*` is element-wise) — reconstruction tests write out scalar dot products.
- Also fixed **tuple-destructuring ARC** (`stmt.rs`): `var { L, U, P } := math.lu(A)` registers ref-counted fields for release; only whitelisted fresh-tuple builtins (lu/qr/svd) transfer ownership, others (aliased returns like `fn dup(m)->(m,m)`) retain-and-keep (no double-free); ignored `_` fields released only for owned temporaries.

**Grupo C — `Vector<T>` — COMPLETE (Phases 1–5):**
- Runtime `BrixVector { ref_count, len, cap, elem_size, elem_kind, data }` (`runtime.c` SECTION 2.4), 2× growth. `elem_kind` (1=int/2=float/3=string) written from Phase 1 so element ARC (string) worked without rework. Funcs: `brix_vector_new/get_ptr(bounds-checked)/push/len/set(bounds-checked)/pop(transfers ownership)/clear/retain/release` (release reuses clear).
- `BrixType::Vector(Box<BrixType>)` (`types.rs`). `Vector<int>()` parses as `GenericCall` intercepted in `lib.rs` before monomorphization → `compile_vector_new` (gate: int/float/string only; rejects `Vector<Matrix>` etc.). Methods dispatched by receiver type in `compile_vector_method`: `push`/`pop`/`get`/`set`/`len`/`is_empty`/`clear`. `pop() → Union(T, Nil)` (the `T?`). Type-checked (`v.push("x")` on `Vector<int>` errors — strict, no int→float coercion).
- **Element ARC (Phase 4B):** push/set release the temp element after the runtime retains it (only for a ref-counted elem, only when the source is an owned temporary — via the dedicated `is_borrowed_ref_expr` helper, NOT `is_print_temp`); `get` retains → returns owned; `pop` transfers ownership.
- **Phase 4A (prerequisite, language-wide):** fixed `Union(ref-counted, Nil)` ARC — `string?` used to leak on decl/reassign and **dangle** on repeated Elvis. Now: `insert_union_release` (per-tag conditional release), Elvis returns a uniformly OWNED result (retains the borrowed not-nil branch and borrowed defaults, NOT owned temps like `pop()`), assignment releases the old union **after** compiling the RHS (so `x := x ?: "d"` is safe), scope-end releases union vars.
- **Phase 5 (final):** `to_array()` — 3 dedicated `runtime.c` wrappers (`brix_vector_to_intmatrix`/`_to_matrix`/`_to_string_matrix`, SECTION 2.4), dispatched by `elem_type` in the new `"to_array"` arm of `compile_vector_method`. The string variant retains (`string_retain`) each copied `BrixString*` so the Vector and the new `StringMatrix` co-own elements — `v.clear()` after `to_array()` doesn't invalidate the returned array. `for x in v { ... }` gets a new `BrixType::Vector(inner)` arm in the `for`-loop compiler (`lib.rs`), mirroring the existing `StringMatrix`/`String` iteration pattern: `brix_vector_len` for the bound, `brix_vector_get_ptr` + `build_load` per element (no bare `brix_vector_get` — that symbol doesn't exist). For a `String` element, the loaded value is retained (`insert_retain`) **before** the loop body compiles, so `v.clear()`/`v.pop()` called from inside the body can't invalidate the current `x` — the vector releases its own reference, but `x` holds a separate one. No release is emitted per iteration (consistent with the pre-existing `for ch in string` leak, not a new regression). **Post-review fix:** the loop bound was originally cached once before the loop (`vec_len`, a single `brix_vector_len` call) instead of reloaded per iteration; with 2+ elements, a body that shrinks the vector (`v.clear()`/`v.pop()`) left the cached bound stale, so the loop advanced its index past the vector's real (now smaller) length and `brix_vector_get_ptr` — which bounds-checks and aborts — crashed the process on the next iteration instead of ending the loop. Fixed by calling `brix_vector_len` fresh inside `cond_bb` on every check, same shape as the index reload already done there. Integration tests 199–202; +5 codegen unit tests; +7 Test Library tests in `collections_v18.test.bx` (including the 4 adversarial cases: `to_array()` + `clear()`, basic `for`, `clear()` mid-loop-body with 1 element, and the 2+-element regression that exercises the stale-bound bug).

**Grupo D — `Stack<T>` / `Queue<T>` — COMPLETE:**
- **`Stack<T>`** has **no new `runtime.c`** — it's a `BrixVector*` under the hood and every method (`push`/`pop`/`size`/`is_empty`) dispatches straight to the existing `brix_vector_push/pop/len` symbols (`compile_stack_new`/`compile_stack_method` in `lib.rs`). `peek()` is the one genuinely new method: `brix_vector_len - 1` + `brix_vector_get_ptr` + `insert_retain` if the element is ref-counted — no extra runtime code, and an empty stack aborts via `brix_vector_get_ptr`'s existing bounds check (message says `"Vector.get(-1) out of bounds"`, not `"Stack.peek"` — an accepted wording tradeoff from not adding a dedicated C function for a 5-line delegate).
- **`Queue<T>`** has a real new `BrixQueue` ring buffer (`runtime.c` SECTION 2.5): `{ ref_count, head, tail, len, cap, elem_size, elem_kind, data }`, cap starts at 4 (smaller than `Vector`'s 8, deliberately, to keep wraparound tests small). `brix_queue_new/enqueue/dequeue/front/size/retain/release`. **Growth relinearizes**: unlike `Vector` (which only ever grows at the tail), a full `Queue`'s physical buffer can be wrapped (`head > 0`), so `brix_queue_grow` copies the `len` elements in **logical** order (starting at `head`, wrapping) into a fresh doubled buffer starting at physical index 0, then resets `head=0`/`tail=len` — a plain `realloc` would have silently preserved the wrong byte order. `dequeue()` transfers ownership (mirrors `Vector.pop()`'s `Union(T, Nil)` tagged-struct build exactly); `front()` retains and returns owned (mirrors `Vector.get()`); an empty `front()` aborts with a dedicated message (`"Queue.front() called on empty queue"`).
- `BrixType::Stack(Box<BrixType>)` / `BrixType::Queue(Box<BrixType>)` (`types.rs`), both distinct from `Vector` (restricted APIs — no `get`/`set`/`clear`). Wired through the same 8-point type checklist as `Vector`: `string_to_brix_type` (`Stack<T>`/`Queue<T>` annotations), `brix_type_to_llvm`, `is_ref_counted`, `insert_retain`/`insert_release` (`Stack` → `brix_vector_retain/release`, `Queue` → `brix_queue_retain/release`), `typeof`, `GenericCall` interception before monomorphization, method dispatch guard.
- **Bug found and fixed during testing (not Grupo D-specific — pre-existing pattern extended incompletely by Grupo C too):** two *other*, non-exhaustive `match BrixType` sites outside `lib.rs`'s own `brix_type_to_llvm` — `compile_variable_decl_stmt`'s **separate** LLVM-allocation-type match in `stmt.rs` (`~line 791`, a second, independent "which LLVM type backs this `BrixType`" match that duplicates `brix_type_to_llvm`'s job) and the `ExprKind::Identifier` variable-load match in `lib.rs` (`~line 4213`) — both fall through to a wildcard `_ => Err(TypeError)` instead of erroring at compile time, so adding `Stack`/`Queue` to the "real" `brix_type_to_llvm` wasn't sufficient: `var s := Stack<int>()` failed with `"Type Error in Variable declaration"` until both sites got explicit `Stack(_) | Queue(_)` arms too. `compile_variable_decl_stmt`'s `Vector<T>` annotation-validation `if let` block was also generalized to an or-pattern (`Vector(inner) | Stack(inner) | Queue(inner)`) rather than duplicated three times. **Known, accepted, pre-existing gaps left untouched** (same wildcard-fallback shape, not exercised by any Grupo C or D test, and already missing `Vector` too): the nil-comparison `is_pointer_type` closure (`lib.rs ~5063`) and the `match`-expression PHI-type inference (`lib.rs ~9466`) — using `Stack`/`Queue`/`Vector` in `x == nil` or as a `match` arm result isn't supported by any of the three container types today.
- Integration tests 203–209 (`203_stack_basic`, `204_stack_string_arc`, `205_stack_peek_empty_aborts`, `206_queue_basic`, `207_queue_dequeue_twice`, `208_queue_front_empty_aborts`, `209_queue_wraparound_growth` — the last one traced by hand against the exact cap=4 ring-buffer implementation before being written, since it's the highest-risk scenario in the group); +8 codegen unit tests; +6 Test Library tests in `collections_v18.test.bx`. `for x in stack/queue` iteration is explicitly out of scope for Grupo D.

**Grupo E — `MinHeap<T>` / `MaxHeap<T>` — COMPLETE:**
- **No new C struct.** `MinHeap<T>`/`MaxHeap<T>` are, like `Stack<T>`, a `BrixVector*` under the hood — `brix_vector_new`/`retain`/`release`/`len` are reused directly (`size()`/`is_empty()` call `brix_vector_len`, no new symbol). The only difference between the two is the *direction* of comparison, passed as an `int is_max` parameter on every call (0=Min, 1=Max) — **never stored on the struct**; the codegen side knows statically which variant it's compiling and bakes in the literal constant. Rejected the roadmap's original idea of a user-supplied comparator via function pointer/closure callback — no such ABI exists in the compiler today, and `v1.8` only needs primitive comparisons (`int`/`float`/`string`, decided by the existing `elem_kind` field), so building that bridge would've been unused machinery.
- 3 new `runtime.c` functions (SECTION 2.6, after `BrixQueue`): `brix_heap_push(h, elem, is_max)` — delegates the append to `brix_vector_push` (reuses its grow-2x + string-retain logic verbatim), then sifts the new element up; `brix_heap_pop(h, out, is_max)` — extracts the root (ownership transfers to the caller, mirrors `Vector.pop`'s contract), moves the last element into the root slot via a **raw `memcpy`** (not `brix_vector_set`, which would retain/release and break the ownership transfer — the swap only relocates existing references, it doesn't create or destroy one), then sifts down; returns 0 on an empty heap without aborting (mirrors `Vector.pop`/`Queue.dequeue`, becomes `nil` via the tagged union on the codegen side). `brix_heap_peek(h)` — the one function with a **dedicated error message** (`"Heap.peek() called on empty heap"`), unlike `Stack.peek()` (Grupo D), which reuses `Vector.get_ptr`'s generic bounds-check message — justified here because `runtime.c` code was being written from scratch anyway, not purely delegated. Comparison is a `static` helper switching on `elem_kind` (`int`/`float` direct comparison, `string` via `strcmp` on `BrixString->data`); sift-up/down and the raw 8-byte slot swap are also `static` helpers, standard array-heap indexing (children at `2i+1`/`2i+2`, parent at `(i-1)/2`).
- `BrixType::MinHeap(Box<BrixType>)` / `MaxHeap(Box<BrixType>)` (`types.rs`), both distinct (kept apart in `typeof` output and error messages) but sharing a **single** Rust implementation — `compile_heap_new(..., is_max: bool)` / `compile_heap_method(..., is_max: bool, ...)` — since Min and Max only ever differ by that one boolean, unlike `Stack`/`Queue` (which have genuinely different method names and runtime symbols). `pop()` returns `Union(T, Nil)` exactly like `Vector.pop`/`Stack.pop`/`Queue.dequeue`; `peek()` retains and returns owned (same as `Vector.get`/`Queue.front`).
- Wired through the same type checklist as `Vector`/`Stack`/`Queue`, including the **two extra non-exhaustive-`match` sites found during Grupo D** (`stmt.rs`'s `compile_variable_decl_stmt` — both its `Vector<T>`-annotation-validation `if let` or-pattern and its separate LLVM-allocation-type match — and `lib.rs`'s `ExprKind::Identifier` variable-load match): both were extended proactively this time, with zero trial-and-error.
- Integration tests 210–216 (`minheap_basic`, `maxheap_basic`, `heap_peek_no_remove`, `heap_sort` — 10 shuffled values, deliberately more than `Vector`'s initial capacity of 8 so the growth path inside `brix_heap_push` is exercised too — `heap_peek_empty_aborts`, `minheap_string`, `heap_duplicates` — pushing repeated values and confirming the heap preserves multiplicity, no stability guarantee needed); +6 codegen unit tests; +6 Test Library tests in `collections_v18.test.bx`. **Descoped from the roadmap:** the suggested `193_dijkstra_example` integration test was dropped — `AdjacencyList`/graph support is already out of scope until v2.0 per this same roadmap, so there's no way to build a real graph to run Dijkstra over yet; replaced with the more direct correctness tests above.

**Grupo F — `HashMap<K,V>` — COMPLETE:**
- **Scope reduction from the roadmap:** values restricted to `V ∈ {Int, Float, String}` (keys stay `K ∈ {Int, String}` as originally planned) — "any Brix type" as a value would need runtime-dispatched ARC (a type tag + retain/release vtable resolved at runtime, since a generic hash table's C code can't know a value's Brix type at compile time), which doesn't exist anywhere in the compiler and isn't exercised by any of the roadmap's own tests. This keeps `HashMap` consistent with every other v1.8 container (`Vector`/`Stack`/`Queue`/`Heap`): 8-byte flat key/value slots, ARC resolved entirely at compile time from the static `BrixType::HashMap(K, V)`.
- **No comparator/hash function pointers** (the roadmap's original `long (*hash_fn)(...)`/`int (*eq_fn)(...)` struct fields) — same simplification as Grupo E's heap comparator: a `key_kind` field (reusing `BRIX_VEC_INT`/`BRIX_VEC_STRING`) drives an inline `switch` inside the hash/equality helpers instead.
- `runtime.c` (SECTION 2.7, after the Heap section): open addressing with linear probing + tombstones. `HashEntry { long key; long value; int occupied; int deleted; }` — 3 slot states: never-used `(0,0)` (probing stops, key absent), live `(1,0)`, tombstone `(0,1)` (probing continues past it, but available for insert reuse). `BrixHashMap { ref_count, len, cap, used, key_kind, val_kind, HashEntry *entries }` — `len` = live entries, `used` = live+tombstone (drives rehash independently of `len`, so delete-heavy churn still gets compacted). Hash: Knuth multiplicative for int, FNV-1a over content for string. Equality: `strcmp` content comparison for strings, never pointer comparison — two different `BrixString*` with the same text must collide as the same key. Rehash triggers when `used*10 >= cap*7`; grows to `cap*2` only if live entries alone would already occupy half the *old* capacity, otherwise rehashes into the *same* capacity purely to purge tombstones (bounds unbounded tombstone growth from delete-heavy usage without runaway table growth). Rehash is a raw relocation (recomputes each surviving entry's bucket in the new array) — no retain/release, ownership doesn't change, exactly like `Queue`'s growth relinearization.
- **ARC contracts (the part most likely to get wrong — spelled out explicitly, not left implicit):** `set()` on a **new** key retains the key (if string) and the value (if string). `set()` on an **existing** key (matched by content, even via a different `BrixString*` instance than the one stored) leaves the *stored* key reference completely untouched — releases only the old value and retains the new one. This is safe with a uniform codegen-side rule applied independently to the key arg and the value arg: release the source temp after the call iff it was an owned temporary (`!is_borrowed_ref_expr`) — on a new key, the C side retained it (ownership transfers cleanly, mirrors `Vector.push`); on an existing key, the C side never retained the incoming key, so the codegen's release just frees the caller's own single reference, correctly, without touching the map. `get()` retains the found value **in C, before writing to the output slot** — a deliberate divergence from `Vector.get()`'s pattern (which retains on the codegen side after loading) — so a successful `get()` always returns owned and the codegen must NOT call `insert_retain` again (would double-retain/leak). `delete()` is **idempotent**: a missing key is a silent no-op, not an error — releases key+value (if ref-counted) only when a live entry is actually found. Full `release()` walks live (non-tombstone) entries releasing key+value, skipping tombstones (already released at delete time).
- `BrixType::HashMap(Box<BrixType>, Box<BrixType>)` (`types.rs`) — the only v1.8 container type with **2** type parameters, which broke the single-parameter or-pattern shortcut used for `Vector`/`Stack`/`Queue`/`MinHeap`/`MaxHeap` in `stmt.rs`'s annotation-validation block; handled as a separate `if let BrixType::HashMap(key, val)` branch there instead. `compile_hashmap_new`/`compile_hashmap_method` (`set`/`get`/`has`/`delete`/`len`/`keys`) in `lib.rs`; `keys()` dispatches on the key type to return `IntMatrix` or `StringMatrix` (retaining each key for the string case, same co-ownership as `Vector.to_array()`).
- **Index sugar**, wired as two independent short-circuits rather than one shared code path, because reads and writes were *already* two entirely separate mechanisms in the compiler before this feature existed: `map[key]` (read) is added inside `compile_expr`'s existing `ExprKind::Index` dispatch (which already special-cases `Tuple`/`StringMatrix`/`Matrix` there) and desugars to `.get(key)` — returns `V?`, deliberately **not** a separate abort-on-missing path, so there's exactly one lookup contract instead of two behaviors for the same syntax. `map[key] = val` (write) short-circuits at the very top of `compile_assignment_stmt`, *before* `compile_lvalue_addr` — that function's "compute a raw address, then apply generic ARC + store" model doesn't fit `HashMap` (whose key/value ARC rules live entirely inside `brix_hashmap_set`/the `"set"` method path already built above), so duplicating or bending that generic path was avoided in favor of delegating straight to the same `compile_hashmap_method(..., "set", ...)`.
- Integration tests 217–223 (`hashmap_string_int`, `hashmap_int_keys`, `hashmap_overwrite_arc`, `hashmap_delete` — has/delete/re-set, `hashmap_iter`, `hashmap_index_syntax`, `hashmap_growth_rehash` — 12 string keys, deliberately crossing the initial capacity of 8 to force at least one rehash cycle); +8 codegen unit tests; +8 Test Library tests in `collections_v18.test.bx`.
- **Housekeeping note:** the prior "Phase v1.8 Grupo E" commit only captured the Rust codegen side of `MinHeap`/`MaxHeap` — its `runtime.c` changes (`SECTION 2.6: HEAP<T>`) were never actually committed, leaving that commit non-linkable in isolation. Both `runtime.c` sections (Heap + HashMap) are included together in the commit that lands this Grupo F work.

v1.8 is now feature-complete across all 6 groups (A–F).

**Working conventions this session (memory):** run `rustfmt` with the crate's own edition on every touched file so `rustfmt --check` passes (root/`codegen`/`parser` = `--edition 2024`, `lexer` = `--edition 2021`; rustfmt follows `mod` declarations, so formatting `lib.rs` touches every codegen module — confirm with `git diff --stat`) (the whole `codegen` crate was normalized in commit `rustfmt: format the codegen crate`); NEVER run two compile-producing suites concurrently (integration + Test Library clobber the shared `output.o`/`program` in repo root → bogus low counts + `ld: file is empty` — run each alone, sequentially); each phase is validated across all 3 test layers + a full integration run before commit.

## v2.0 Status (IN PROGRESS) — see `ROADMAP_V2.0.md` (untracked, local only)

Single group, **Grupo A — `Embedding<DIM>`**, in 5 phases. v2.0 ships **methods, not operators** (`@`/`<->`/`<=>` are v2.1 sugar).

**Fase 0 — COMPLETE** (`50cd02d`) — const generics grammar:
- No AST change: a numeric arg is stringified into the existing `GenericCall.type_args: Vec<String>`.
- Recognized **only** for the literal names `Embedding`/`EmbeddingBatch`, at the identifier-atom level (not the shared postfix generic-call combinator — widening that broke `a < 1 > (b)`), and **only when `<` is byte-adjacent** to the name (`id_span.end == lt_span.start`), so a variable named `Embedding` still works in a spaced comparison. After the adjacent `<` the parser commits (`then_with`/`rewind`/`boxed`): negative or `> u32::MAX` dims are named parse errors, never a fallback to comparison.
- Codegen guard `is_bare_integer_literal()` rejects numeric type args for every other generic (`Vector<1536>()` etc.).
- Parser test helper `parse_expr_with_real_spans` (the plain `parse_expr` uses token-index pseudo-spans, useless for whitespace tests). +9 parser unit tests.

**Fase 1 — COMPLETE** (`84910a9`) — type + conversions:
- `BrixType::Embedding(u32)`; `runtime.c` `SECTION 2.10`: `BrixEmbedding { ref_count, dim, double* }` (double = `Matrix` precision), `brix_embedding_from_matrix(Matrix*, long dim)` / `brix_embedding_to_matrix` (always copy), retain/release.
- `type_annotation_parser()` accepts a bare integer in `<...>` universally (no name gate needed — no comparison ambiguity in type position); `string_to_brix_type("Embedding<...>")` returns `BrixType::Error` for bad/zero dims instead of the silent default-to-`Int` fallback.
- Constructor takes `Matrix`, or `IntMatrix` promoted via `intmatrix_to_matrix()` (both the promoted temp and an owned source `IntMatrix` are released). Literal-length mismatch → `E104` at compile time; `Matrix` variable with wrong shape → runtime abort (`BrixType::Matrix` carries no shape). `Embedding<0>` → `E104`.
- `.to_matrix()` (arity-checked) is the only method so far. `infer_expr_type_static()` gained its first `ExprKind::GenericCall` arm (`:type Embedding<3>(...)` in the REPL). The two known gap sites are closed for `Embedding`; the nil-comparison / match-PHI gaps are inherited, same as `Vector`. `println(embedding)` not supported (no `value_to_string` arm, same as `Vector`).
- +9 codegen unit, +2 parser unit, integration tests 260–262 (incl. runtime-abort via subprocess exit code), +5 Test Library (`embedding.test.bx`).

**Fase 2 — COMPLETE** (`2734f43`) — similarity methods via BLAS:
- `a.dot_product(b)` / `a.euclidean_distance(b)` / `a.cosine_similarity(b)` → `Float` (f64). `runtime.c` `SECTION 2.10`: `brix_embedding_dot`/`_euclidean`/`_cosine` over Fortran-convention `ddot_`/`dnrm2_` (`extern`, pass-by-pointer, `int` lengths — same convention as the LAPACK wrappers, resolved by the existing `-llapack -lblas`). Euclidean builds a temp `a - b` buffer for `dnrm2_`. Zero-vector cosine returns `0.0`, never `NaN`.
- Static helper `brix_embedding_check_pair` aborts on `NULL` / dim mismatch / `dim > INT_MAX` — an internal-invariant defense only; user-facing dim mismatch is `E102` at compile time (`compile_embedding_similarity` in `lib.rs` requires the arg type to equal `Embedding(dim)` exactly). Arity ≠ 1 → `E104`.
- Dispatch: `is_embedding_method` guard widened to the 3 names; a user struct method with the same name (e.g. `fn (v: V2) dot_product(...)`) still dispatches to the struct (integration 266). `infer_expr_type_static()` Embedding arm: `to_matrix` → `Matrix`, the 3 methods → `Float` (`:type` in the REPL).
- ARC: both the receiver and the argument are released after the call when they are owned temporaries (`!is_borrowed_ref_expr`) — `Embedding<3>([...]).dot_product(a)` and `a.dot_product(Embedding<3>([...]))` don't leak. Note `.to_matrix()` and the Vector/Stack/Queue/Heap/HashMap methods still do NOT release a temporary receiver (pre-existing leak, untouched).
- BLAS link validated on macOS (Accelerate) with an isolated `ddot_`/`dnrm2_` program before any embedding code. **Not validated on Linux** — no CI exists in the repo.

**ARC conventions changed in Fase 2 (language-wide, found in review — read before touching returns/ternaries/closures):**
- **Single-value `return` always hands the caller an owned value:** `own_if_borrowed()` (`lib.rs`) retains it if `is_borrowed_ref_expr` (local, parameter, captured var, field, element), then `release_function_scope_vars()` runs, same as a void return. A returned local nets out (+1/−1). Before: `return param` gave the caller an unowned pointer (use-after-free once the caller released the "temporary" call result, e.g. `v.push(id(s)); v.clear(); println(s)`), and every other local of a non-void function leaked. **Tuple returns are unchanged** (caller-side "aliased returns" logic in `stmt.rs` destructuring still applies; they still don't release locals).
- **Closures and struct methods have their own ARC scope:** `function_scope_vars` is saved/cleared/restored (`std::mem::take`) in `compile_closure` (`builtins/closure_compiler.rs`) and in the method-definition path (`lib.rs`), as `compile_function_def` and the async compilers already did; both now also release their locals on implicit void return. Before: closure-body locals were pushed into the enclosing function's list, and a `return` inside a struct method released the *caller's* locals (invalid IR / segfault).
- **A ternary always yields an owned value** (same contract as Elvis since v1.8 Fase 4A): each borrowed branch is retained inside its own block via `own_if_borrowed` (`expr.rs`), and `is_borrowed_ref_expr` treats `Ternary` as an owned temporary. Before: `println(f ? s1 : s2)` / a discarded `f ? a : b` released the chosen variable (use-after-free — pre-existing for `string`).
- **Ternary PHI type** for pointer-backed types now comes from the branch value (`final_then_val.get_type()`); it used to fall back to `i64` for anything but String/Matrix/FloatPtr (invalid IR for Vector/HashMap/DateTime/Embedding/...).
- Source-based codegen unit tests: `ir_from_source()` / `function_body()` helpers in `tests/builtin_tests.rs` (lex+parse+compile real Brix, then `module.verify()`), plus `compile_error()` to pin a `CodegenError` variant (the older `compile_program` helper discards errors).

Tests: +19 codegen unit, integration 263–267 (263 dot incl. f64-only precision case, 264 euclidean, 265 cosine incl. zero vector, 266 return/ternary/struct-method ARC, 267 ternary ownership), +7 Test Library (`embedding.test.bx`).

**Known pre-existing bugs found during Fase 2, NOT fixed (separate work):** (1) a closure returning `string`, called through a variable (`var g := (n: int) -> string {...}; println(g(1))`), prints the pointer as an int — the `Closure`→`Tuple` collapse loses the signature; (2) a program combining top-level functions and closures that return strings (ternary over a captured var) crashes LLVM during object emission — reproduces identically on the pre-Fase-2 HEAD, root cause not isolated.

**Next — Fase 3:** `EmbeddingBatch<DIM>`: `add`/`get`/`find_nearest`/`len`/`is_empty` (see `ROADMAP_V2.0.md` contracts: `find_nearest` returns `IntMatrix` `1×min(k,len)` indices sorted by cosine desc, `O(n·DIM + n·log k)`). Then Fase 4 (informative benchmark).

## Troubleshooting

- **Linking errors**: run clean build (see above)
- **"runtime.c not found"**: must run from project root
- **LLVM errors**: requires LLVM 18 — `brew install llvm@18`
- **Panic on unwrap()**: remaining `unwrap()` calls are isolated in Option-returning I/O helpers; check stack trace location
- **Parser errors with valid code**: Brix uses **newlines** as statement separators, not semicolons
- **Integration tests must be sequential**: `--test-threads=1` required (all tests compile to the same directory)
