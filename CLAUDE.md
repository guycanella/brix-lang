# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Instructions for Claude Code

**CRITICAL**: Do not stop tasks early due to context limits. Always complete the full task even if it requires significant context usage. Use context efficiently but prioritize task completion.

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
     - Comparison/Logical (`<`, `<=`, `>`, `>=`, `==`, `!=`, `&&`, `||`)
     - Bitwise (`&`, `|`, `^`)
     - Additive (`+`, `-`)
     - Multiplicative (`*`, `/`, `%`)
     - Power (`**`)
     - Atom (literals, identifiers, function calls, indexing)

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

Brix has 7 core types (defined in `crates/codegen/src/lib.rs`):

```rust
pub enum BrixType {
    Int,       // i64
    Float,     // f64
    String,    // BrixString struct (in runtime.c)
    Matrix,    // Matrix struct (in runtime.c) - f64* data
    IntMatrix, // IntMatrix struct (in runtime.c) - i64* data
    FloatPtr,  // f64* (internal pointer type)
    Void,      // for functions with no return
}
```

**Type Selection for Arrays/Matrices:**
- Literal `[1, 2, 3]` → `IntMatrix` (all integers)
- Literal `[1.0, 2.0]` or `[1, 2.5]` → `Matrix` (floats or mixed, with int→float promotion)
- Constructors: `zeros()` → `Matrix`, `izeros()` → `IntMatrix`

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

## Arrays and Matrices: Design Decisions (Jan 2026)

### Type Inference for Array Literals

The compiler analyzes literal elements to decide the most efficient memory allocation:

- **IntMatrix**: Created when all elements are integers
- **Matrix (Float)**: Created when all are floats OR mixed types (automatic int→float promotion)

```brix
// Creates IntMatrix (i64*)
var arr_int := [1, 2, 3]
var mat_int := [[1, 2], [3, 4]]

// Creates Matrix (f64*)
var arr_float := [1.0, 2.0, 3.0]
var arr_mixed := [1, 2, 3.5]  // Promotes ints to float
```

### Array Constructors

Brix provides multiple ways to create arrays and matrices, each with specific use cases:

#### 1. Array Literals (Type Inference)

```brix
var nums := [1, 2, 3, 4, 5]    // IntMatrix (all ints)
var vals := [1, 2.5, 3.7]      // Matrix (mixed → float promotion)
var x := nums[0]               // Index access
```

#### 2. zeros() and izeros() Functions

For semantic clarity between Engineering (Floats) and Discrete Math (Ints):

```brix
// Float matrices (f64) - default for engineering/math
var m1 := zeros(5)        // 1D array of 5 floats
var m2 := zeros(3, 4)     // 3x4 float matrix

// Integer matrices (i64) - for discrete data/indices
var i1 := izeros(5)       // 1D array of 5 ints
var i2 := izeros(3, 4)    // 3x4 int matrix
```

#### 3. Static Initialization Syntax (v0.6)

Concise syntax for allocating zeroed memory:

```brix
// Allocates array of 5 integers (initialized to 0)
var buffer := int[5]

// Allocates 2x3 float matrix (initialized to 0.0)
var grid := float[2, 3]

// Equivalent to izeros(5) and zeros(2, 3)
```

This is syntactic sugar that compiles to the same efficient calloc-based allocation as zeros()/izeros().

### Mutability and Safety

The keyword defines heap memory behavior:

**`var` (Mutable)**: Allows element rewriting

```brix
var m := [1, 2, 3]
m[0] = 99  // Valid
```

**`const` (Deep Immutability)**: Compiler blocks any store instructions to indices

```brix
const PI_VEC := [3.14, 6.28]
PI_VEC[0] = 1.0  // ❌ Compile Error: Cannot mutate const variable
```

### Internal Representation

To maintain "Fortran-level" performance, we use specialized C structures (not generic `void*` arrays):

**Runtime structures (runtime.c):**

```c
// For Engineering and Mathematics (Default)
typedef struct {
    long rows;
    long cols;
    double* data;  // 8 bytes (f64)
} Matrix;

// For Images, Indices, and Discrete Data
typedef struct {
    long rows;
    long cols;
    long* data;    // 8 bytes (i64)
} IntMatrix;

// Future (v0.8+): For Text Data
typedef struct {
    long rows;
    long cols;
    char** data;   // Array of pointers
} StringMatrix;
```

**Key Design Principle**: Matrices and arrays store homogeneous, contiguous data for CPU performance. JSON/heterogeneous data will use a separate `JsonValue` type (tagged union) for web interoperability, kept separate from mathematical structures.

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

### Format Specifiers

Format specifiers allow precise control over how values are converted to strings in f-strings:

**Integer formats:**

```brix
var num := 255
println(f"{num:x}")    // ff (hexadecimal lowercase)
println(f"{num:X}")    // FF (hexadecimal uppercase)
println(f"{num:o}")    // 377 (octal)
println(f"{num:d}")    // 255 (decimal, default)
```

**Float formats:**

```brix
var pi := 3.14159265359
println(f"{pi:.2f}")   // 3.14 (2 decimal places)
println(f"{pi:.6f}")   // 3.141593 (6 decimal places)
println(f"{pi:e}")     // 3.141593e+00 (scientific notation lowercase)
println(f"{pi:E}")     // 3.141593E+00 (scientific notation uppercase)
println(f"{pi:.2e}")   // 3.14e+00 (scientific with 2 decimals)
println(f"{pi:g}")     // 3.14159 (compact format, default)
println(f"{pi:G}")     // 3.14159 (compact format uppercase)
```

**Mixed formats:**

```brix
var x := 42
var y := 3.14159
println(f"x={x:x}, y={y:.2f}")  // x=2a, y=3.14
```

### Built-in Functions

**Output:**

- `printf(format, ...)`: Formatted output (C-style)
- `print(expr)`: Print any value without newline (auto-converts to string)
- `println(expr)`: Print any value with newline (auto-converts to string)

**Input:**

- `scanf(format, ...)`: Formatted input

**Type Introspection:**

- `typeof(expr)`: Returns type as string (e.g., "int", "float", "string")

**Type Conversion:**

- `int(x)`: Convert to int (truncates floats, parses strings)
- `float(x)`: Convert to float (promotes ints, parses strings)
- `string(x)`: Convert to string (works with all types)
- `bool(x)`: Convert to boolean (0/0.0/empty string = false, rest = true)

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

- Syntax: `f"text {expr} more text"` or `f"text {expr:format} more text"`
- Token: `FString` in lexer with regex `r#"f"([^"\\]|\\["\\bnfrt])*""#`
- AST: `FStringPart` enum with `Text(String)` and `Expr { expr: Box<Expr>, format: Option<String> }`
- Parser extracts expressions from `{}`, detects format specifier after `:`, tokenizes and parses them recursively
- Codegen converts each part to string using `value_to_string(val, type, format)`:
  - Int: Uses C `sprintf()` with format strings (`%lld`, `%x`, `%X`, `%o`)
  - Float: Uses C `sprintf()` with format strings (`%.Nf`, `%e`, `%E`, `%g`, `%G`)
  - String: Returns as-is
- Format specifiers are mapped to printf-style formats in codegen
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

### Type Conversion Functions Implementation

Built-in functions for explicit type conversion between primitive types.

**int(x):**

- Int → returns same value
- Float → `build_float_to_signed_int()` (truncates: 3.14 → 3)
- String → calls C `atoi()` for parsing ("123" → 123)
- Returns: i64

**float(x):**

- Float → returns same value
- Int → `build_signed_int_to_float()` (promotes: 42 → 42.0)
- String → calls C `atof()` for parsing ("3.14" → 3.14)
- Returns: f64

**string(x):**

- String → returns same value
- Int/Float → reuses `value_to_string()` with `sprintf()`
- Bool → converts to "0" or "1"
- Returns: BrixString

**bool(x):**

- Int → `x != 0` (0 = false, anything else = true)
- Float → `x != 0.0`
- String → `len > 0` (empty string = false, non-empty = true)
- Returns: i64 (0 or 1)

**Helper functions:**

- `get_atoi()`: Declares C `int atoi(const char*)`
- `get_atof()`: Declares C `double atof(const char*)`

### Import System and Standard Library (Planned - v0.7+)

**Architecture Design:**

The import system will provide zero-overhead access to standard library functions by using direct C library bindings. This approach prioritizes performance and code reuse over reimplementation.

**Import Statement Processing:**

1. **Parser**: Recognizes `import math` or `import math as m`
   - Token: `Token::Import`
   - AST: `Stmt::Import { module: String, alias: Option<String> }`

2. **Symbol Table**: Creates namespace for imported module
   - Example: `import math` → adds `math.*` namespace
   - Example: `import math as m` → adds `m.*` namespace

3. **Codegen**: Generates LLVM external function declarations
   ```rust
   // For import math, generate:
   let fn_type = f64_type.fn_type(&[f64_type.into()], false);
   module.add_function("sin", fn_type, Some(Linkage::External));
   ```

4. **Linking**: System linker resolves symbols at link-time
   ```bash
   cc output.o runtime.o -lm -llapack -lblas -o program
   ```

**Performance Characteristics:**

- **Compile-time only**: Import resolution has zero runtime cost
- **Direct calls**: `math.sin(x)` compiles to `call @sin(double %x)` - identical to C
- **LLVM optimization**: Can inline, vectorize, use CPU intrinsics (FSIN instruction)
- **Dead code elimination**: Unused functions never linked into final binary

**Runtime Bridge (runtime.c):**

The runtime acts as a thin bridge to C libraries:

```c
// Mathematical functions - direct passthroughs
#include <math.h>
double brix_sin(double x) { return sin(x); }
double brix_cos(double x) { return cos(x); }
double brix_sqrt(double x) { return sqrt(x); }

// Linear algebra - LAPACK bindings
#include <lapacke.h>
double brix_det(Matrix* A) {
    // Use LAPACK's optimized LU decomposition
    lapack_int ipiv[A->rows];
    LAPACKE_dgetrf(LAPACK_ROW_MAJOR, A->rows, A->cols,
                   A->data, A->cols, ipiv);
    // ... compute determinant from diagonal
}
```

**Standard Library Structure:**

```
stdlib/
├── math/
│   ├── basic.c      // sin, cos, sqrt (math.h wrappers)
│   ├── linalg.c     // det, inv, eigvals (LAPACK wrappers)
│   └── stats.c      // mean, median, std (custom or GSL)
└── ...
```

**Why This Approach:**

1. **Performance**: Leverages decades of hand-tuned assembly optimizations
2. **Reliability**: Battle-tested code used by NumPy, MATLAB, Julia, R
3. **Maintainability**: No need to maintain complex math implementations
4. **Ecosystem compatibility**: Easy to link with existing C/Fortran libraries

**Example Performance:**
- Matrix determinant (1000×1000): ~50ms with LAPACK vs ~5s naive implementation (100× faster)
- Trigonometric functions: CPU-native instructions (FSIN, FCOS) when possible

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

**Core Language Features:**
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
- `conversion_test.bx`: Type conversion functions (int, float, string, bool)
- `format_test.bx`: Format specifiers (hex, octal, decimal, scientific, precision)
- `zeros_test.bx`: zeros() and izeros() constructors (v0.6)
- `static_init_test.bx`: Static initialization syntax int[n], float[r,c] (v0.6)
- `array_constructors_test.bx`: Comprehensive test of all array constructor methods (v0.6)

**Math Library (v0.7):**
- `math_test.bx`: All 21 math.h functions + 6 constants
- `math_alias_test.bx`: Import with alias (import math as m)
- `physics_test.bx`: Physics simulation (projectile motion)
- `stats_linalg_test.bx`: Statistics and linear algebra functions
- `eye_test.bx`: Identity matrix creation and verification

Run tests individually:

```bash
cargo run <test_file.bx>
```

**Note:** The compiler generates intermediate files (`runtime.o`, `output.o`) and an executable `program` in the project root during compilation.

## Project Status (v0.7 - Jan 2026)

### Progress: 80% MVP Complete

**Completed:**

- ✅ Compiler pipeline (Lexer → Parser → Codegen → Native binary)
- ✅ 7 primitive types with automatic casting (Int, Float, String, Matrix, IntMatrix, FloatPtr, Void)
- ✅ Arrays and matrices with 2D indexing
- ✅ **IntMatrix type system** (v0.6):
  - Array literal type inference (all ints → IntMatrix, mixed → Matrix with promotion)
  - `zeros(n)` / `zeros(r,c)` - Float matrix constructors
  - `izeros(n)` / `izeros(r,c)` - Integer matrix constructors
  - Static initialization syntax: `int[5]`, `float[2,3]`
  - Full indexing and assignment support for both Matrix and IntMatrix
- ✅ Control flow (if/else, while, for loops)
- ✅ Operators (arithmetic, comparison, logical, bitwise, unary, inc/dec, string)
- ✅ Power operator (`**` for int and float)
- ✅ Chained comparisons (Julia-style)
- ✅ Ternary operator (`cond ? true_val : false_val`)
- ✅ Bitwise operators (`&`, `|`, `^` for integers)
- ✅ Unary operators (`!`, `not` for logical negation; `-` for arithmetic negation)
- ✅ Increment/Decrement (`++x`, `x++`, `--x`, `x--` - prefix and postfix)
- ✅ String interpolation (`f"text {expr}"` with automatic type conversion)
- ✅ Format specifiers (`f"{value:.2f}"`, `f"{num:x}"` - hex, octal, scientific notation, precision)
- ✅ Built-in functions (printf, scanf, typeof, matrix, read_csv, print, println)
- ✅ Type conversion functions (int(), float(), string(), bool())
- ✅ Runtime library (C) for matrix, intmatrix, and string operations
- ✅ **Import system** (`import math`, `import math as m`)
- ✅ **Math library** (36 functions + constants - see below)

### ✅ **v0.7 - Import System + Math Library (COMPLETO - 26/01/2026)**

**Sistema de Imports:**
- ✅ `import math` - Import com namespace
- ✅ `import math as m` - Import com alias
- ✅ Suporte a `module.function(args)` e `module.constant`
- ✅ Flat symbol table com prefixos
- ✅ Auto-conversão Int→Float em funções math

**Math Library - 36 itens implementados:**

**21 Funções math.h** (via LLVM external declarations):
- Trigonometria (7): `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`
- Hiperbólicas (3): `sinh`, `cosh`, `tanh`
- Exp/Log (4): `exp`, `log`, `log10`, `log2`
- Raízes (2): `sqrt`, `cbrt`
- Arredondamento (3): `floor`, `ceil`, `round`
- Utilidades (5): `fabs`, `fmod`, `hypot`, `fmin`, `fmax`

**6 Constantes Matemáticas:**
- `math.pi`, `math.e`, `math.tau`, `math.phi`, `math.sqrt2`, `math.ln2`

**5 Funções Estatísticas** (runtime.c):
- `math.sum(arr)` - Soma de elementos
- `math.mean(arr)` - Média aritmética
- `math.median(arr)` - Mediana
- `math.std(arr)` - Desvio padrão
- `math.variance(arr)` - Variância

**4 Funções Álgebra Linear** (runtime.c):
- `math.det(M)` - Determinante (Gaussian elimination)
- `math.inv(M)` - Inversa de matriz (Gauss-Jordan)
- `math.tr(M)` - Transposta
- `math.eye(n)` - Matriz identidade n×n

**Exemplos de uso:**
```brix
import math

// Funções básicas
var x := math.sin(math.pi / 2.0)  // 1.0
var y := math.sqrt(16)             // 4.0 (auto-converte int→float)

// Estatísticas
var data := [1.0, 2.0, 3.0, 4.0, 5.0]
var avg := math.mean(data)         // 3.0
var sd := math.std(data)           // 1.414...

// Álgebra linear
var I := math.eye(3)               // Matriz identidade 3×3
var det := math.det(matrix)        // Determinante
var inv := math.inv(matrix)        // Inversa

// Com alias
import math as m
var z := m.cos(0.0)                // 1.0
```

**Arquivos de teste:**
- `math_test.bx` - 21 funções + 6 constantes
- `math_alias_test.bx` - Import com alias
- `physics_test.bx` - Simulação de física (projétil)
- `stats_linalg_test.bx` - Estatísticas e álgebra linear
- `eye_test.bx` - Matriz identidade

**Adiado para versões futuras:**
- ⏳ `eigvals/eigvecs` → v0.8+ (requer tipo Complex)
- ⏳ Constantes físicas → v0.8+ (quando tivermos sistema de unidades)
- ⏳ Selective imports (`from math import sin`) → v0.7.1+

---

### 🎯 **PRÓXIMO PASSO: v0.8 - Functions**

- [ ] Functions (definition, calls, return values)
- [ ] Multiple return values (Go-style)
- [ ] Pattern matching (`when` syntax)
- [ ] List comprehensions

---

## Current Limitations (v0.7)

- **No generics**: Only concrete types (int, float, string, matrix)
- **Single-file compilation**: Multi-file imports not yet implemented (user modules coming in v0.8+)
- **No user-defined functions**: Function definitions coming in v0.8
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

- ✅ v0.7: Import system, math library (36 functions + constants)
- v0.8: Functions (definition, calls, return values), user-defined modules
- v0.9: Pattern matching, closures, complex numbers
- v1.0: Generics, concurrency primitives
- v1.2: Full standard library with data structures (Stack, Queue, HashMap, Heap)

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
