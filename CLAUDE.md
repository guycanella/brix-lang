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

Brix has 9 core types (defined in `crates/codegen/src/lib.rs`):

```rust
pub enum BrixType {
    Int,       // i64
    Float,     // f64
    String,    // BrixString struct (in runtime.c)
    Matrix,    // Matrix struct (in runtime.c) - f64* data
    IntMatrix, // IntMatrix struct (in runtime.c) - i64* data
    FloatPtr,  // f64* (internal pointer type)
    Void,      // for functions with no return
    Tuple(Vec<BrixType>),  // Multiple return values (LLVM struct)
    Complex,   // Complex struct (in runtime.c) - double real, double imag
    ComplexMatrix, // ComplexMatrix struct (in runtime.c) - Complex* data
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

#### Destructuring in for loops (v0.9)

```brix
var a := [1, 2, 3]
var b := [10, 20, 30]

for x, y in zip(a, b) {
    println(f"x={x}, y={y}, sum={x + y}")
}
// Output: x=1, y=10, sum=11
//         x=2, y=20, sum=22
//         x=3, y=30, sum=33
```

### Arrays

```brix
var nums := [1, 2, 3, 4, 5]
var x := nums[0]           // Index access
```

### List Comprehensions (v0.9 - Jan 2026)

Python-style list comprehensions with full support for nested loops, multiple conditions, and destructuring.

#### Basic Syntax

```brix
var nums := [1.0, 2.0, 3.0, 4.0, 5.0]
var doubled := [x * 2.0 for x in nums]  // [2, 4, 6, 8, 10]
```

#### With Conditions

```brix
var evens := [x for x in nums if int(x) % 2 == 0]  // [2, 4]
```

#### Multiple Conditions (AND logic)

```brix
var filtered := [x for x in nums if x > 2.0 if x < 5.0]  // [3, 4]
```

#### Nested Loops

```brix
var a := [1.0, 2.0]
var b := [10.0, 20.0]
var products := [x * y for x in a for y in b]  // [10, 20, 20, 40]
```

#### With Destructuring

```brix
var a := [1.0, 2.0, 3.0]
var b := [10.0, 20.0, 30.0]
var sums := [x + y for x, y in zip(a, b)]  // [11, 22, 33]
```

#### Complex Example

```brix
var pairs := [x + y for x in a for y in b if x + y > 15.0]  // Nested + condition
```

**Design decisions:**
- Loop order: left-to-right = outer-to-inner (Python-style)
- Multiple conditions use AND logic: `if c1 if c2` → `if c1 && c2`
- Always returns Matrix (Float) currently (IntMatrix support planned)
- Hybrid allocation: pre-allocates max size, then resizes to actual size

### Functions (v0.8 - Jan 2026)

Brix supports user-defined functions with single and multiple return values, destructuring, and default parameters.

#### Basic Function Definition

```brix
function add(a: int, b: int) -> int {
    return a + b
}

var result := add(5, 3)  // 8
```

#### Void Functions

Functions without a return type are void and don't need explicit return statements:

```brix
function greet(name: string) {
    println(f"Hello, {name}!")
}

greet("Alice")  // Prints: Hello, Alice!
```

#### Multiple Return Values

Functions can return multiple values as tuples:

```brix
function calculations(a: int, b: int) -> (int, int, int) {
    return (a + b, a - b, a * b)
}

// Access via indexing
var result := calculations(10, 5)
println(f"sum = {result[0]}")       // 15
println(f"diff = {result[1]}")      // 5
println(f"product = {result[2]}")   // 50
```

#### Destructuring

Destructure multiple return values into separate variables using `{}`:

```brix
var { sum, diff, product } := calculations(10, 5)
println(f"sum = {sum}")       // 15
println(f"diff = {diff}")     // 5
println(f"product = {product}") // 50

// Ignore values with _
var { quotient, _ } := divmod(17, 5)  // Ignore remainder
```

#### Default Parameter Values

Parameters can have default values, allowing calls with fewer arguments:

```brix
function power(base: float, exp: float = 2.0) -> float {
    return base ** exp
}

println(f"power(5.0) = {power(5.0)}")           // 25 (uses default exp=2.0)
println(f"power(5.0, 3.0) = {power(5.0, 3.0)}") // 125

function greet(name: string, greeting: string = "Hello") {
    println(f"{greeting}, {name}!")
}

greet("Alice")          // Hello, Alice!
greet("Bob", "Hi")     // Hi, Bob!
```

**Design decisions:**
- Keyword: `function` (not `fn`)
- Return type: Required for non-void functions
- Single return: Parentheses optional in return statement (`return x` or `return (x)`)
- Multiple returns: Parentheses required (`return (a, b, c)`)
- Tuple access: Array-style indexing (`result[0]`, `result[1]`)
- Destructuring: Uses `{}` with `:=` operator
- Ignore values: Use `_` in destructuring
- Default values: Evaluated at call site, filled in order from left to right

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

// For Complex Numbers (v1.0+)
typedef struct {
    double real;
    double imag;
} Complex;

// For Complex Matrices (eigenvalues/eigenvectors)
typedef struct {
    long rows;
    long cols;
    Complex* data;  // Array of Complex structs
} ComplexMatrix;

// Future (v1.1+): For Text Data
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

**Arrays in f-strings (v0.9):**

```brix
var nums := [1, 2, 3, 4, 5]
println(f"nums = {nums}")               // Output: nums = [1, 2, 3, 4, 5]

var data := [1.5, 2.7, 3.9]
println(f"data = {data}")               // Output: data = [1.5, 2.7, 3.9]
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
- `string(x)`: Convert to string (works with all types, including Matrix/IntMatrix)
- `bool(x)`: Convert to boolean (0/0.0/empty string = false, rest = true)

**Array Operations:**

- `zip(arr1, arr2)`: Combine two arrays into pairs
  - Returns Matrix(n, 2) or IntMatrix(n, 2) depending on input types
  - Uses minimum length if arrays differ in size
  - Example: `zip([1,2,3], [4,5,6])` → Matrix with rows [1,4], [2,5], [3,6]
  - Type variants: ii (IntMatrix), if (Matrix), fi (Matrix), ff (Matrix)

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

**Functions (v0.8):**
- `function_test.bx`: Basic function definition and calls
- `void_test.bx`: Void functions without return values
- `multiple_return_test.bx`: Multiple return values with tuple indexing
- `destructuring_test.bx`: Destructuring tuples into variables
- `destructuring_ignore_test.bx`: Destructuring with `_` to ignore values
- `default_values_test.bx`: Default parameter values

**List Comprehensions (v0.9):**
- `zip_test.bx`: zip() function with type variants
- `destructuring_for_test.bx`: Destructuring in for loops
- `list_comp_simple_test.bx`: Basic list comprehension
- `list_comp_cond_test.bx`: List comprehension with conditions
- `list_comp_advanced_test.bx`: Nested loops and multiple conditions
- `list_comp_zip_test.bx`: List comprehension with zip and destructuring
- `list_comp_test.bx`: Comprehensive test (all 4 scenarios)

**Pattern Matching (v1.0):**
- `match_basic_test.bx`: Literal patterns (int, float, string, bool) and wildcard
- `match_guard_test.bx`: Guards with if conditions
- `match_or_test.bx`: Or-patterns (multiple values with |)
- `match_typeof_test.bx`: Match on typeof(value)
- `match_types_test.bx`: Type coercion (int→float promotion)

**Complex Numbers & LAPACK (v1.0):**
- `eigvals_simple_test.bx`: Eigenvalues of identity matrix
- `eigvals_rotation_test.bx`: Complex eigenvalues (rotation, symmetric, diagonal matrices)
- `eigvecs_test.bx`: Eigenvectors (5 different scenarios)

Run tests individually:

```bash
cargo run <test_file.bx>
```

**Note:** The compiler generates intermediate files (`runtime.o`, `output.o`) and an executable `program` in the project root during compilation.

## Project Status (v1.0 em progresso - Jan 2026)

### Progress: 92% MVP Complete

**Completed:**

- ✅ Compiler pipeline (Lexer → Parser → Codegen → Native binary)
- ✅ 9 primitive types with automatic casting (Int, Float, String, Matrix, IntMatrix, FloatPtr, Void, Complex, ComplexMatrix)
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
- ✅ **Math library** (38 functions + constants - see below)
- ✅ **Complex Numbers** (v1.0):
  - Complex struct with real and imag fields
  - ComplexMatrix for eigenvalue/eigenvector results
  - 2D matrix printing: `[[a+bi, c+di], [e+fi, g+hi]]`
  - LAPACK integration for linear algebra
- ✅ **User-defined functions** (v0.8):
  - Function definitions with `function` keyword
  - Single and multiple return values (tuples)
  - Destructuring with `{}`
  - Default parameter values
  - Void functions
- ✅ **List Comprehensions** (v0.9):
  - Full Python-style syntax: `[expr for var in iterable if cond]`
  - Multiple conditions (AND logic): `[x for x in arr if c1 if c2]`
  - Nested loops: `[x * y for x in a for y in b]`
  - Destructuring support: `[x + y for x, y in zip(a, b)]`
  - Hybrid allocation (pre-allocate max size, then resize)
- ✅ **zip() function** (v0.9):
  - Combines multiple arrays into pairs
  - Returns Matrix(n, 2) or IntMatrix(n, 2)
  - Type-aware: 4 variants (ii, if, fi, ff)
- ✅ **Destructuring in for loops** (v0.9):
  - Syntax: `for x, y in zip(arr1, arr2)`
  - Works with Matrix and IntMatrix
- ✅ **Array printing in f-strings** (v0.9):
  - `println(f"nums = {nums}")` → `nums = [1, 2, 3, 4, 5]`
  - Works for Matrix and IntMatrix
- ✅ **Pattern Matching** (v1.0):
  - Match expressions: `match value { pattern -> expr }`
  - Literal patterns: int, float, string, bool
  - Wildcard: `_`
  - Binding: `x` (captures value)
  - Or-patterns: `1 | 2 | 3`
  - Guards: `x if x > 10`
  - Type coercion: int→float automatic promotion
  - Exhaustiveness warning
  - Match on typeof(): `match typeof(value) { "int" -> ... }`

### ✅ **v0.7 - Import System + Math Library (COMPLETO - 26/01/2026)**

**Sistema de Imports:**
- ✅ `import math` - Import com namespace
- ✅ `import math as m` - Import com alias
- ✅ Suporte a `module.function(args)` e `module.constant`
- ✅ Flat symbol table com prefixos
- ✅ Auto-conversão Int→Float em funções math

**Math Library - 38 itens implementados:**

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

**6 Funções Álgebra Linear** (runtime.c + LAPACK):
- `math.det(M)` - Determinante (Gaussian elimination)
- `math.inv(M)` - Inversa de matriz (Gauss-Jordan)
- `math.tr(M)` - Transposta
- `math.eye(n)` - Matriz identidade n×n
- `math.eigvals(A)` - Autovalores (LAPACK dgeev, retorna ComplexMatrix)
- `math.eigvecs(A)` - Autovetores (LAPACK dgeev, retorna ComplexMatrix)

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
- `eigvals_simple_test.bx` - Autovalores de matriz identidade
- `eigvals_rotation_test.bx` - Autovalores complexos (rotação, simétrica, diagonal)
- `eigvecs_test.bx` - Autovetores (5 cenários diferentes)

**Adiado para versões futuras:**
- ⏳ Constantes físicas → v1.1+ (quando tivermos sistema de unidades)
- ⏳ Selective imports (`from math import sin`) → v0.7.1+

---

### ✅ **v0.8 - User-Defined Functions (COMPLETO - 26/01/2026)**

**Funcionalidades implementadas:**

1. **Definição de funções básicas:**
   - Keyword `function` para definir funções
   - Parâmetros tipados obrigatórios
   - Tipo de retorno obrigatório (exceto void)
   - Exemplo: `function add(a: int, b: int) -> int { return a + b }`

2. **Funções void:**
   - Funções sem tipo de retorno
   - Return statement opcional
   - Exemplo: `function greet(name: string) { println(f"Hello, {name}!") }`

3. **Múltiplos retornos (tuples):**
   - Retornar múltiplos valores como tupla
   - Sintaxe: `-> (int, int, int)`
   - Return com parênteses: `return (a, b, c)`
   - Acesso por índice: `result[0]`, `result[1]`, `result[2]`
   - Exemplo: `function calculations(a: int, b: int) -> (int, int, int)`

4. **Destructuring:**
   - Desempacotamento de tuplas em variáveis separadas
   - Sintaxe: `var { a, b, c } := func()`
   - Suporta `_` para ignorar valores
   - Exemplo: `var { sum, _, product } := calculations(10, 5)`

5. **Default parameter values:**
   - Parâmetros com valores padrão
   - Sintaxe: `param: type = default_value`
   - Avaliados no call site
   - Exemplo: `function power(base: float, exp: float = 2.0) -> float`
   - Chamada: `power(5.0)` usa exp=2.0, `power(5.0, 3.0)` usa exp=3.0

**Implementação técnica:**
- AST: `FunctionDef`, `Return`, `DestructuringDecl`
- Tuples implementadas como LLVM structs
- Function registry com metadata de parâmetros
- Default values expandidos no call site durante compilação
- Suporte completo a type inference para tuples

**Testes:**
- `function_test.bx` - Funções básicas ✅
- `void_test.bx` - Funções void ✅
- `multiple_return_test.bx` - Múltiplos retornos ✅
- `destructuring_test.bx` - Destructuring básico ✅
- `destructuring_ignore_test.bx` - Destructuring com `_` ✅
- `default_values_test.bx` - Default parameters ✅

---

### ✅ **v0.9 - List Comprehensions & zip() (COMPLETO - 27/01/2026)**

**Funcionalidades implementadas:**

1. **zip() Built-in Function:**
   - Combina dois arrays em pares: `zip([1,2,3], [4,5,6])` → Matrix 3×2 com cada linha sendo um par
   - 4 variantes para type safety:
     - `brix_zip_ii`: IntMatrix × IntMatrix → IntMatrix
     - `brix_zip_if`: IntMatrix × Matrix → Matrix
     - `brix_zip_fi`: Matrix × IntMatrix → Matrix
     - `brix_zip_ff`: Matrix × Matrix → Matrix
   - Usa comprimento mínimo quando arrays têm tamanhos diferentes
   - Exemplo:
     ```brix
     var a := [1, 2, 3]
     var b := [10, 20, 30]
     var pairs := zip(a, b)  // [[1,10], [2,20], [3,30]]
     ```

2. **Destructuring em for loops:**
   - Suporte a múltiplas variáveis em loops
   - Sintaxe: `for x, y in iterable { ... }`
   - Funciona com zip(): `for x, y in zip(a, b) { ... }`
   - Suporta Matrix e IntMatrix
   - Itera sobre linhas quando há múltiplas variáveis
   - Exemplo:
     ```brix
     var a := [1, 2, 3]
     var b := [10, 20, 30]
     for x, y in zip(a, b) {
         println(f"x={x}, y={y}, sum={x + y}")
     }
     // Output: x=1, y=10, sum=11
     //         x=2, y=20, sum=22
     //         x=3, y=30, sum=33
     ```

3. **List Comprehensions (sintaxe completa):**
   - Python-style syntax com todas as features:
     - Básica: `[expr for var in iterable]`
     - Com condição: `[expr for var in iterable if condition]`
     - Múltiplas condições (AND): `[x for x in arr if c1 if c2]`
     - Nested loops: `[expr for x in a for y in b]`
     - Com destructuring: `[x + y for x, y in zip(a, b)]`
   - Ordem de loops: esquerda-para-direita = outer-to-inner (estilo Python)
   - Alocação híbrida para performance:
     1. Pré-aloca array com tamanho máximo (produto de todos iterables)
     2. Preenche array conforme avalia condições
     3. Redimensiona para tamanho final
   - Suporta Matrix e IntMatrix como iterables
   - Type inference: sempre retorna Matrix (Float) por ora
   - Exemplos:
     ```brix
     // Básico
     var nums := [1.0, 2.0, 3.0, 4.0, 5.0]
     var doubled := [x * 2.0 for x in nums]  // [2, 4, 6, 8, 10]

     // Com condição
     var evens := [x for x in nums if int(x) % 2 == 0]  // [2, 4]

     // Múltiplas condições
     var filtered := [x for x in nums if x > 2.0 if x < 5.0]  // [3, 4]

     // Nested loops (produto cartesiano)
     var a := [1.0, 2.0]
     var b := [10.0, 20.0]
     var products := [x * y for x in a for y in b]  // [10, 20, 20, 40]

     // Com destructuring
     var sums := [x + y for x, y in zip(a, b)]  // [11, 22]
     ```

4. **Array printing em f-strings:**
   - Suporte para imprimir Matrix/IntMatrix em f-strings
   - Formato: `[elemento1, elemento2, ...]`
   - Funciona com `print()`, `println()`, e f-strings
   - Exemplo:
     ```brix
     var nums := [1, 2, 3, 4, 5]
     println(f"nums = {nums}")  // Output: nums = [1, 2, 3, 4, 5]
     ```

**Implementação técnica:**
- AST: `ListComprehension`, `ComprehensionGen` structs
- Parser: suporta sintaxe completa com generators aninhados
- Codegen:
  - `compile_list_comprehension()`: orquestra a compilação
  - `generate_comp_loop()`: gera loops aninhados recursivamente
  - Usa LLVM basic blocks para controle de fluxo
  - Implementa short-circuit evaluation para condições
- Runtime: 4 funções zip em `runtime.c`
- value_to_string: estendido para Matrix/IntMatrix (loop com concatenação)

**Testes:**
- `zip_test.bx` - zip() function ✅
- `destructuring_for_test.bx` - Destructuring em for loops ✅
- `list_comp_simple_test.bx` - Comprehension básica ✅
- `list_comp_cond_test.bx` - Com condição ✅
- `list_comp_advanced_test.bx` - Nested loops + múltiplas condições ✅
- `list_comp_zip_test.bx` - Zip + destructuring ✅
- `list_comp_test.bx` - Teste completo (4 cenários) ✅

---

### ✅ **v1.0 - Pattern Matching** ✅ **COMPLETO (27/01/2026)**

Sistema completo de pattern matching com guards, or-patterns, e type coercion.

**Sintaxe:**
```brix
match value {
    pattern -> expression
    pattern if guard -> expression
    pattern1 | pattern2 -> expression
    _ -> expression
}
```

**Features Implementadas:**

1. **Match como Expressão:**
   - Retorna valor: `var result := match x { ... }`
   - Todos os arms devem retornar tipos compatíveis
   - Type coercion automática (int→float)

2. **Patterns Suportados:**
   - **Literais**: `42`, `3.14`, `"text"`, `true`, `false`
   - **Wildcard**: `_` (matches anything)
   - **Binding**: `x` (captures value and binds to variable)
   - **Or-patterns**: `1 | 2 | 3` (matches any of the values)

3. **Guards:**
   - Condições com `if`: `x if x > 10`
   - Binding disponível no guard
   - Exemplo: `n if n > 0 && n < 100`

4. **Type Checking:**
   - Todos os arms devem retornar tipos compatíveis
   - Promoção automática int→float quando necessário
   - Erro de compilação para tipos incompatíveis (string + int)

5. **Exhaustiveness Warning:**
   - Warning (não bloqueia) quando falta wildcard
   - Sugere adicionar `_ -> ...`

6. **Match em typeof():**
   - Pattern matching em tipos: `match typeof(value) { "int" -> ... }`

**Exemplos:**

```brix
// Básico
var result := match x {
    1 -> "one"
    2 -> "two"
    3 -> "three"
    _ -> "other"
}

// Com guards
var category := match age {
    x if x < 18 -> "child"
    x if x < 60 -> "adult"
    _ -> "senior"
}

// Or-patterns
var day_type := match day {
    1 | 2 | 3 | 4 | 5 -> "weekday"
    6 | 7 -> "weekend"
    _ -> "invalid"
}

// Type coercion (int→float)
var num := match x {
    1 -> 10      // int
    2 -> 20.5    // float (promotes arm 1 to float)
    _ -> 0.0
}  // num: float

// Match em typeof()
match typeof(value) {
    "int" -> println("integer")
    "float" -> println("float")
    "string" -> println("string")
    _ -> println("other")
}
```

**Implementação Técnica:**
- AST: `Pattern` enum, `MatchArm` struct
- Parser: suporta sintaxe completa com guards e or-patterns
- Codegen: usa LLVM basic blocks + PHI nodes
- Type checking com promoção automática
- Binding de variáveis antes de guards

**Testes:**
- `match_basic_test.bx` - Literais e wildcard ✅
- `match_guard_test.bx` - Guards com if conditions ✅
- `match_or_test.bx` - Or-patterns ✅
- `match_typeof_test.bx` - Match em typeof() ✅
- `match_types_test.bx` - Type coercion ✅

**Futuro (v1.1+):**
- [ ] **Destructuring patterns**: `{ x: x, y: y }`, `(a, b, c)`, `[first, second, ...]`
- [ ] **Range patterns**: `1..10`, `'a'..'z'`
- [ ] **Exhaustiveness checking obrigatório**

---

### 🎯 **v1.0 - Advanced Features** ✅ **70% COMPLETO**

- [x] Pattern matching (`match` syntax) ✅ **COMPLETO (27/01/2026)**
- [x] Complex numbers & ComplexMatrix ✅ **COMPLETO (27/01/2026)**
- [x] LAPACK integration (eigvals/eigvecs) ✅ **COMPLETO (27/01/2026)**
- [ ] Closures and lambda functions ⏸️
- [ ] First-class functions ⏸️
- [ ] User-defined modules ⏸️

---

### ✅ **v1.0 - Complex Numbers & LAPACK** ✅ **COMPLETO (27/01/2026)**

Sistema completo de números complexos e integração LAPACK para álgebra linear avançada.

**Tipos Implementados:**

1. **Complex (struct):**
   - Campos: `double real`, `double imag`
   - Usado internamente para cálculos

2. **ComplexMatrix (struct):**
   - Campos: `long rows`, `long cols`, `Complex* data`
   - Retorno de `eigvals()` e `eigvecs()`

**Funções LAPACK:**

- **math.eigvals(A):** Calcula autovalores de matriz
  - Input: Matrix (f64)
  - Output: ComplexMatrix (n×1 vector)
  - Usa LAPACK dgeev
  - Exemplo: `var eigenvalues := math.eigvals(matrix)`

- **math.eigvecs(A):** Calcula autovetores de matriz
  - Input: Matrix (f64)
  - Output: ComplexMatrix (n×n matrix)
  - Autovetores nas colunas
  - Usa LAPACK dgeev
  - Exemplo: `var eigenvectors := math.eigvecs(matrix)`

**Implementação Técnica:**

1. **Runtime (runtime.c):**
   - Structs Complex e ComplexMatrix
   - Funções `brix_eigvals()` e `brix_eigvecs()`
   - Conversão row-major → column-major para LAPACK
   - Work array queries (two-pass LAPACK)
   - Handling de complex conjugate pairs

2. **Codegen:**
   - `BrixType::Complex` e `BrixType::ComplexMatrix`
   - `declare_eigen_function()` helper
   - Return type detection para eigvals/eigvecs
   - ComplexMatrix loading support
   - **CRITICAL FIX:** eye() passa i64 direto sem conversão int→float

3. **String Formatting (2D Matrix Printing):**
   - ComplexMatrix imprime como `[[elem1, elem2], [elem3, elem4]]`
   - Usa modulo arithmetic para detectar row boundaries
   - Adiciona `[` no início de cada row
   - Adiciona `]` no fim de cada row
   - Adiciona `, ` entre rows
   - Formato: `println(f"eigvecs = {eigvecs}")` → `[[1+0i, 0+0i], [0+0i, 1+0i]]`

**Exemplos:**

```brix
import math

// Identity matrix (autovalores reais)
var I := math.eye(3)
var eig_I := math.eigvals(I)
println(f"Eigenvalues: {eig_I}")  // [1+0i, 1+0i, 1+0i]

// Rotation matrix (autovalores complexos)
var R := zeros(2, 2)
R[0][1] = -1.0
R[1][0] = 1.0
var eig_R := math.eigvals(R)
println(f"Eigenvalues: {eig_R}")  // [0+1i, 0-1i]

// Eigenvectors
var vecs := math.eigvecs(I)
println(f"Eigenvectors: {vecs}")  // [[1+0i, 0+0i], [0+0i, 1+0i]]
```

**LAPACK Integration:**
- Links com `-llapack -lblas` em main.rs
- Usa `dgeev_` (double precision general eigenvalue)
- Column-major format conversion
- Complex eigenvector pair handling

**Testes:**
- `eigvals_simple_test.bx` - Matriz identidade ✅
- `eigvals_rotation_test.bx` - Autovalores complexos (rotação, simétrica, diagonal) ✅
- `eigvecs_test.bx` - Autovetores (5 cenários) ✅

**Design Decisions:**
- Autovalores sempre retornam ComplexMatrix (mesmo quando reais)
- Autovetores como colunas da matriz (convenção matemática padrão)
- Erro exit(1) para matrizes não-quadradas (futuro: Go-style (error, value) tuples)
- 2D printing para legibilidade (nested array format)

---

## Current Limitations (v1.0 70% completo)

- **No generics**: Only concrete types (int, float, string, matrix, complex, tuple)
- **Single-file compilation**: Multi-file imports not yet implemented (user modules coming in v1.1+)
- **No optimizations**: LLVM runs with `OptimizationLevel::None`
- **No closures**: Functions are not first-class (v1.1+ planned)
- **No structs**: User-defined types not implemented (v1.1+ planned)
- **Basic error handling**: Parse errors shown via debug output; LAPACK errors use exit(1) instead of Go-style (error, value) tuples
- **List comprehensions type inference**: Currently only returns Matrix (Float), IntMatrix support coming soon
- **Pattern matching destructuring**: Only scalar patterns supported (no struct/tuple/array destructuring yet)
- **Complex arithmetic**: Complex numbers exist but no +, -, *, / operators yet (only via LAPACK eigvals/eigvecs)

## Future Roadmap (from DOCUMENTATION.md)

### Planned Features

- Pattern matching (`when` syntax)
- Pipe operator (`|>`) for data pipelines
- SQL and JSON as native types
- Extension methods
- Null safety with `?` operator
- Dimensional units (`f64<m>`, `f64<s>`)
- Concurrency: `spawn`, `par for`, `par map`
- Closures and lambda functions
- First-class functions

### Implementation Phases

- ✅ v0.6: IntMatrix type, zeros/izeros, static initialization
- ✅ v0.7: Import system, math library (38 functions + constants)
- ✅ v0.8: User-defined functions (single/multiple returns, destructuring, default values)
- ✅ v0.9: List comprehensions, zip(), destructuring in for loops, array printing
- 🚧 v1.0: Pattern matching ✅, complex numbers ✅, LAPACK integration ✅, closures ⏸️, user-defined modules ⏸️ (70% complete)
- v1.1: Complex arithmetic operators, closures, first-class functions, user-defined modules
- v1.2: Generics, concurrency primitives
- v1.3: Full standard library with data structures (Stack, Queue, HashMap, Heap)

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
