# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Brix** is a compiled programming language designed for Data Engineering and Algorithms, combining Python-like syntax with Fortran-level performance. The language compiles to native binaries via LLVM.

- **Extension**: `.bx`
- **Philosophy**: "Write like Python, execute like Fortran, scale like Go"
- **Stack**: Rust (compiler) + LLVM 18 (backend)
- **Memory Model**: ARC (Automatic Reference Counting)
- **Type System**: Strong static typing with aggressive type inference

## Building and Running

### Compile and Execute a .bx File
```bash
cargo run <file.bx>
```

This single command:
1. Lexes and parses the source file
2. Generates LLVM IR
3. Compiles `runtime.c` to `runtime.o`
4. Emits native object code (`output.o`)
5. Links with runtime and executes the binary

Example:
```bash
cargo run types.bx
cargo run for_test.bx
```

### Build the Compiler Only
```bash
cargo build          # Debug build
cargo build --release # Release build
```

## Architecture

### Workspace Structure

The project uses a Cargo workspace with three main crates:

1. **`crates/lexer`**: Tokenization
   - Uses `logos` crate for high-performance lexing
   - Exports `Token` enum with all language tokens
   - Located: `crates/lexer/src/token.rs`

2. **`crates/parser`**: AST Construction
   - Uses `chumsky` parser combinator library
   - Defines AST nodes: `Expr`, `Stmt`, `Literal`, `BinaryOp`, etc.
   - Located: `crates/parser/src/{ast.rs, parser.rs}`
   - Implements operator precedence (lowest to highest):
     * Comparison/Logical (`<`, `<=`, `>`, `>=`, `==`, `!=`, `&&`, `||`)
     * Bitwise (`&`, `|`, `^`)
     * Additive (`+`, `-`)
     * Multiplicative (`*`, `/`, `%`)
     * Power (`**`)
     * Atom (literals, identifiers, function calls, indexing)

3. **`crates/codegen`**: LLVM Code Generation
   - Uses `inkwell` (LLVM 18 bindings)
   - Translates AST to LLVM IR
   - Manages symbol table (`HashMap<String, (PointerValue, BrixType)>`)
   - Implements control flow (if/else with basic blocks)
   - Located: `crates/codegen/src/lib.rs`

4. **`src/main.rs`**: CLI Driver
   - Orchestrates lexer → parser → codegen → linking
   - Uses `clap` for argument parsing
   - Handles compilation of `runtime.c` and native linking
   - Pipeline: `.bx` → Tokens → AST → LLVM IR → `output.o` → link with `runtime.o` → executable `program` → run

### Runtime Library

**File**: `runtime.c` (must be in project root)

Provides C implementations of built-in functions:

- **Matrix operations**: `matrix_new()`, `read_csv()`
- **String operations**: `str_new()`, `str_concat()`, `str_eq()`, `print_brix_string()`

The runtime is compiled to `runtime.o` and linked with each program automatically by `src/main.rs` using the system C compiler (`cc`).

## Type System

Brix has 6 core types (defined in `crates/codegen/src/lib.rs`):

```rust
pub enum BrixType {
    Int,      // i64
    Float,    // f64
    String,   // BrixString struct (in runtime.c)
    Matrix,   // Matrix struct (in runtime.c)
    FloatPtr, // f64* (internal pointer type)
    Void,     // for functions with no return
}
```

**Note:** `bool` is implemented as `i1` in LLVM and auto-extends to `i64` when stored as variables.

### Type Inference and Casting

- **Inference**: `var x := 10` → infers `int`
- **Explicit**: `var x: float = 3.14`
- **Auto-casting**:
  - `var x: int = 99.99` → truncates to 99
  - `var y: float = 50` → promotes to 50.0

### Boolean Representation

- `bool` is represented as `i1` in LLVM
- `true` → 1, `false` → 0
- Comparison operators return `i1` (auto-extended to `i64` when needed)

## Language Features Implemented

### Variables and Constants
```brix
var x := 10           // Inference
var y: float = 3.14   // Explicit type
const pi = 3.1415     // Immutable
```

### Operators
- Arithmetic: `+`, `-`, `*`, `/`, `%`, `**` (power)
- Unary: `!`, `not` (logical negation), `-` (arithmetic negation)
- Increment/Decrement: `++x`, `x++`, `--x`, `x--` (prefix and postfix)
- Bitwise: `&`, `|`, `^` (integer only)
- Logical: `&&`, `and`, `||`, `or`
- Comparison: `<`, `<=`, `>`, `>=`, `==`, `!=`
- Chained comparison: `10 < x <= 20` (Julia-style, compiles to `(10 < x) && (x <= 20)`)
- Ternary: `condition ? true_val : false_val` (supports type promotion int→float)

### Control Flow
```brix
if condition {
    // code
} else {
    // code
}
```

### Loops
```brix
for i in 1:5 { }           // Range: 1 to 5 inclusive
for i in 0:2:10 { }        // Step: 0, 2, 4, 6, 8, 10
for i in start:end { }     // Expressions allowed
```

Loops are Julia-style with inclusive ranges.

### Arrays
```brix
var nums := [1, 2, 3, 4, 5]
var x := nums[0]           // Index access
```

### Strings
```brix
var s := "hello"
var msg := s + " world"    // Concatenation
if s == "test" { }         // Comparison
```

### String Interpolation
```brix
var name := "Brix"
var greeting := f"Hello, {name}!"       // Simple interpolation
var x := 42
var msg := f"Answer: {x}"               // Integer interpolation
var pi := 3.14
var circle := f"Pi = {pi}"              // Float interpolation
var calc := f"5 * 2 = {5 * 2}"          // Expression interpolation
```

### Built-in Functions
- `printf(format, ...)`: Formatted output (C-style)
- `print(expr)`: Print any value without newline (auto-converts to string)
- `println(expr)`: Print any value with newline (auto-converts to string)
- `scanf(format, ...)`: Formatted input
- `typeof(expr)`: Returns type as string (e.g., "int", "float", "string")

## Important Implementation Details

### Symbol Table Management
- Variables are stored in `HashMap<String, (PointerValue, BrixType)>`
- Each variable is allocated on the stack via `alloca`
- Values are loaded/stored using LLVM's `load`/`store` instructions

### Control Flow Implementation
- If/else uses LLVM basic blocks: `then_block`, `else_block`, `merge_block`
- Conditional branching via `build_conditional_branch()`
- PHI nodes are NOT used; values are stored in alloca'd variables

### For Loop Lowering
For loops desugar to while loops:
```brix
for i in start:step:end { body }
```
Becomes:
```brix
var i := start
while i <= end {
    body
    i = i + step
}
```

### String Compilation
- String literals create global constants
- Runtime struct `BrixString` holds length and char pointer
- Concatenation and comparison call C runtime functions

### Ternary Operator Implementation
- Syntax: `condition ? then_expr : else_expr`
- Uses LLVM basic blocks: `tern_then`, `tern_else`, `tern_merge`
- PHI node in merge block unifies the two branch values
- Supports automatic type promotion (int → float when branches have different types)
- Parser uses `logic_or` level for branches to avoid conflict with range's colon

### String Interpolation Implementation
- Syntax: `f"text {expr} more text"`
- Token: `FString` in lexer with regex `r#"f"([^"\\]|\\["\\bnfrt])*""#`
- AST: `FStringPart` enum with `Text(String)` and `Expr(Box<Expr>)`
- Parser extracts expressions from `{}`, tokenizes and parses them recursively
- Codegen converts each part to string using `value_to_string()`:
  - Int/Float: Uses C `sprintf()` with format strings `%lld` / `%g`
  - String: Returns as-is
- All parts concatenated using runtime `str_concat()` function
- Supports nested expressions, arithmetic, and function calls inside `{}`

### Print Functions Implementation
- **print(expr)**: Prints any value without newline
- **println(expr)**: Prints any value with automatic newline
- AST: `Stmt::Print { expr }` and `Stmt::Println { expr }`
- Codegen:
  - Calls `value_to_string()` to convert any type to BrixString
  - Extracts `char*` from BrixString struct (field index 1)
  - Uses `printf("%s", ...)` for print, `printf("%s\n", ...)` for println
- Supports all types: int, float, string, bool (auto-converted)
- More user-friendly than printf for simple output

## Common Patterns

### Adding a New Operator
1. Add token to `crates/lexer/src/token.rs`
2. Add case to `crates/parser/src/parser.rs` in appropriate precedence level
3. Handle in `compile_binary_op()` in `crates/codegen/src/lib.rs`

### Adding a New Built-in Function
1. Declare external function in `Compiler::get_<function_name>()`
2. Implement in `runtime.c`
3. Recompile runtime.o during compilation

### Type System Changes
1. Update `BrixType` enum
2. Update type inference in `infer_type()`
3. Update casting logic in `cast_value()`
4. Add LLVM type mapping in codegen

## Testing

Test files are `.bx` files in the root directory. Common test files include:
- `types.bx`: Type inference, explicit types, casting, typeof()
- `for_test.bx`: Loop variants (range, step, nested)
- `logic_test.bx`: Boolean operators
- `chain_test.bx`: Chained comparisons
- `string_test.bx`: String operations
- `arrays_test.bx`: Array operations
- `csv_test.bx`: Matrix/CSV operations
- `bitwise_test.bx`: Bitwise operators (&, |, ^)
- `ternary_test.bx`: Ternary operator (basic, nested, type mixing)
- `negation_test.bx`: Logical negation (!, not) and unary minus
- `increment_test.bx`: Increment/decrement (++, --, prefix/postfix)
- `fstring_test.bx`: String interpolation (f"text {expr}")
- `print_test.bx`: Print and println functions (auto-conversion)

Run tests individually:
```bash
cargo run <test_file.bx>
```

**Note:** The compiler generates intermediate files (`runtime.o`, `output.o`) and an executable `program` in the project root during compilation.

## Project Status (v0.4 - Jan 2026)

### Progress: 60% MVP Complete

**Completed:**
- ✅ Compiler pipeline (Lexer → Parser → Codegen → Native binary)
- ✅ 6 primitive types with automatic casting
- ✅ Arrays and matrices with 2D indexing
- ✅ Control flow (if/else, while, for loops)
- ✅ Operators (arithmetic, comparison, logical, bitwise, unary, inc/dec, string)
- ✅ Power operator (`**` for int and float)
- ✅ Chained comparisons (Julia-style)
- ✅ Ternary operator (`cond ? true_val : false_val`)
- ✅ Bitwise operators (`&`, `|`, `^` for integers)
- ✅ Unary operators (`!`, `not` for logical negation; `-` for arithmetic negation)
- ✅ Increment/Decrement (`++x`, `x++`, `--x`, `x--` - prefix and postfix)
- ✅ String interpolation (`f"text {expr}"` with automatic type conversion)
- ✅ Built-in functions (printf, scanf, typeof, matrix, read_csv)
- ✅ Runtime library (C) for matrix and string operations

### Next Up (v0.5):
- [ ] Functions (definition, calls, return values)
- [ ] Multiple return values (Go-style)
- [ ] Pattern matching (`when` syntax)
- [ ] List comprehensions

### Planned for v0.6+ (Numeric & Type System Enhancements):
- [ ] **Format Specifiers in String Interpolation**: `f"{value:.6f}"`, `f"{num:x}"` (hexadecimal), `f"{val:.2e}"` (scientific notation)
  - Extends current f-string syntax to support printf-style format specifiers after `:`
  - Examples: `.2f` (2 decimal places), `.6f` (6 significant digits), `x` (hex), `o` (octal), `e` (scientific)

- [ ] **Type Conversion Functions**: Explicit conversion between primitive types
  - `float(x)`: Convert int/string to float
  - `int(x)`: Convert float/string to int (truncation)
  - `string(x)`: Convert any type to string
  - `bool(x)`: Convert to boolean

- [ ] **Complex Numbers** (Julia-style for physics/engineering calculations):
  - Literal syntax: `z := 1 + 2im` (imaginary unit `im`)
  - Built-in functions:
    - `real(z)`: Extract real part
    - `imag(z)`: Extract imaginary part
    - `conj(z)`: Complex conjugate
    - `abs(z)`: Magnitude (distance from origin)
    - `abs2(z)`: Squared magnitude (avoids sqrt for performance)
    - `angle(z)`: Phase angle in radians
  - Arithmetic: Full support for `+`, `-`, `*`, `/`, `**` with complex numbers
  - New type: `BrixType::Complex` (stored as struct with real/imag f64 fields)

## Current Limitations (v0.4)

- **No generics**: Only concrete types (int, float, string, matrix)
- **Single-file compilation**: No imports or modules
- **No optimizations**: LLVM runs with `OptimizationLevel::None`
- **No pattern matching**: `when` syntax not yet implemented
- **No closures**: Functions are not first-class
- **No structs**: User-defined types not implemented
- **Basic error handling**: Parse errors shown via debug output

## Future Roadmap (from DOCUMENTATION.md)

### Planned Features
- Pattern matching (`when` syntax)
- Multiple return values (Go-style)
- Pipe operator (`|>`) for data pipelines
- List comprehensions
- SQL and JSON as native types
- Extension methods
- Null safety with `?` operator
- Dimensional units (`f64<m>`, `f64<s>`)
- Concurrency: `spawn`, `par for`, `par map`

### Implementation Phases
- v0.2: Multi-file support, imports, basic structs
- v0.3: Pattern matching, closures, generics
- v0.4: Concurrency primitives
- v1.0: Full standard library with data structures (Stack, Queue, HashMap, Heap)

## Troubleshooting

### Compilation Fails with "runtime.c not found"
- Ensure `runtime.c` exists in the project root directory
- The compiler looks for it in the current working directory

### LLVM Errors
- The project requires LLVM 18 to be installed
- On macOS: `brew install llvm@18`
- Ensure `inkwell` feature `llvm18-0` matches your LLVM version

### "cc: command not found"
- The compiler requires a C compiler (gcc/clang) to compile `runtime.c`
- On macOS: Install Xcode Command Line Tools (`xcode-select --install`)
- On Linux: Install `build-essential` (Debian/Ubuntu) or `gcc` (other distros)

### Parse Errors Show Only Debug Output
- Error reporting with Ariadne is planned but not yet implemented
- Current errors display using Rust's `Debug` format (`{:?}`)
