# Brix Language (Design Document v1.0)

> ✅ **Status do Projeto (Fev 2026):** O compilador Brix **v1.5 COMPLETO** — Todas as três features principais entregues: **Iterators & Pipeline**, **Test Library** e **Async/Await**. v1.5 implementa Ranges Unificados (`..`/`..<`/`step`), Iteradores (`map`, `filter`, `reduce`, `any`, `all`, `find`), Pipeline Operator (`|>`), Test Library Jest-style (28 matchers, 21 suites), e Async/Await via state machines LLVM (`async fn`, `await`, `brix_run_to_completion`). **1.133 unit tests + 110 integration tests passando (100%).** Próxima: v1.6 — `break`/`continue`, String Library, Async Closures.

## Status Atual (Fevereiro 2026)

### ✅ **Funcionalidades Implementadas (v1.0-v1.5):**
- Compilação completa `.bx` → binário nativo via LLVM
- **LLVM Optimizations**: `-O0`, `-O1`, `-O2`, `-O3`, `--release`
- **v1.5 Async/Await (COMPLETE - Feb 2026):**
  - **`async fn`**: Compilado para state machine LLVM via `create_{name}(params) -> i8*` + `poll_{name}(i8*) -> {status, value}`
  - **`await`**: `var x := await f(args)` em sequência linear no body de `async fn`
  - **`async fn main()`**: Dirigido por `brix_run_to_completion` no runtime C (loop de polling síncrono)
  - **Stackless coroutines**: State struct mínimo (~40–300 bytes/task), zero overhead em runtime
  - Limitação: `await` em control flow aninhado e `async { }` blocks → v1.6
- **v1.5 Test Library (COMPLETE - Feb 2026):**
  - **Jest-style framework**: `test.describe()`, `test.it()`, `test.expect()`
  - **28 matchers**: `toBe`, `toEqual`, `toBeCloseTo`, `toBeTruthy`, `toBeFalsy`, `toBeGreaterThan`, `toBeLessThan`, `toContain`, `toHaveLength`, `toBeNil`, e variantes `not.*`
  - **`cargo run -- test`**: Executa todos os `*.test.bx` e `*.spec.bx`
  - **21 suites** em `tests/brix/` cobrindo toda a linguagem
- **v1.5 Iterators & Pipeline (COMPLETE - Feb 2026):**
  - **Array Type Syntax**: `int[]`, `float[]` em anotações de tipo
  - **Unified Ranges**: `0..5` (inclusivo), `0..<5` (exclusivo), `0..10 step 2`, auto-step decrescente
  - **Array Range Literals**: `[1..5]` → `IntMatrix`, `[1..<5]` → `IntMatrix`
  - **Iterators em IntMatrix/Matrix**: `.map(fn)`, `.filter(pred)`, `.reduce(init, fn)`, `.any(pred)`, `.all(pred)`, `.find(pred) -> T?`
  - **Pipeline Operator**: `arr |> map(fn) |> filter(pred) |> reduce(0, fn)`
  - **Closures como parâmetros**: Passagem e chamada de closures em funções resolvida (Phase 0a)
- **v1.4 Advanced Type System (COMPLETE - Feb 2026):**
  - **Type Aliases**: `type MyInt = int`, `type Point2D = Point`
  - **Union Types**: `int | float | string` com tagged unions
  - **Intersection Types**: `Point & Label` com struct merging
  - **Elvis Operator**: `a ?: b` (null coalescing)
  - **Optional Refactoring**: `int?` agora é `Union(int, nil)`
- **v1.3 Type System (COMPLETE):**
  - **Structs**: Go-style receivers, default values, generic support
  - **Generics**: Functions, structs, methods com monomorphization
  - **Closures**: Capture by reference, heap allocation, ARC
- 19 tipos core (Int, Float, String, Matrix, IntMatrix, Complex, ComplexMatrix, Atom, Nil, Error, Struct, Generic, Closure, Union, Intersection, TypeAlias, Void, FloatPtr, Tuple)
- Operadores completos (aritméticos, lógicos, bitwise, power operator `**`, Elvis `?:`)
- Funções definidas pelo usuário com múltiplos retornos
- Pattern matching com guards
- List comprehensions
- Import system (zero-overhead)
- 38 funções matemáticas (math module)
- Integração LAPACK (eigvals, eigvecs)
- Atoms estilo Elixir (`:ok`, `:error`)
- F-strings com format specifiers
- Ariadne error reporting (parser + codegen)

### ✅ **Completado (v1.2.1 - Phase E7 COMPLETE):**
- **Error Handling with Result Types (COMPLETE - Feb 2026):**
  - ✅ All core compilation functions use `CodegenResult<T>`
  - ✅ CodegenError enum with 6 variants + span information
  - ✅ AST Migration with Spans (Expr/Stmt structs with source positions)
  - ✅ **Ariadne Integration for Codegen Errors:**
    - `error_report.rs` module with beautiful error formatting
    - Error codes (E100-E105) with colored labels
    - Source code context in error messages
    - **Integrated in main.rs** - end users see beautiful errors
  - ✅ **Real Spans in All Errors (Phase E6 - COMPLETE):**
    - All CodegenError variants now capture real source spans from AST
    - 458 lines modified in lib.rs to propagate spans through compilation
    - Precise error highlighting in Ariadne error messages
  - ✅ **Span Granularity Fix (Feb 2026):**
    - Fixed parser to use chumsky Stream with spans instead of Vec<Token>
    - Spans now point to exact tokens (e.g., `undefined_var`) not whole expressions
    - Ariadne highlights precise source locations with surgical accuracy
  - ✅ **eprintln!() Cleanup:** 54 → 32 (22 critical errors converted to CodegenError)
  - ✅ **unwrap() Cleanup:** Remaining calls isolated in I/O helpers and test utilities
  - ✅ **Exit Codes Diferenciados (Phase E7):**
    - E100-E105: Códigos específicos por tipo de erro
    - Parser errors: exit code 2
    - Runtime div/0: exit code 1 com mensagem clara
  - ✅ **Division by Zero Runtime Checks:**
    - Detecção automática em operações inteiras (/, %)
    - Mensagem clara: "❌ Runtime Error: Division by zero"
  - ✅ **Type Error Fixes:**
    - String + Int agora retorna erro bonito (antes causava panic)
  - ✅ **Error Handling Architecture:**
    - Documentação completa em CLAUDE.md
    - Fluxo de propagação de erros
    - Tabela de exit codes
  - ✅ **1050/1050 testes unitários passando** (Lexer: 292, Parser: 158, Codegen: 600)
  - ✅ **85/85 testes de integração passando** (success: 79, parser errors: 4, codegen errors: 4, runtime errors: 3)
  - ✅ **Phase E COMPLETE!** 🎉
- **v1.3 - Type System Expansion (COMPLETE - Feb 2026):**
  - ✅ **Structs (Phase 1):**
    - Go-style receivers: `fn (p: Point) distance() -> float { ... }`
    - Default field values: `struct Config { timeout: int = 30 }`
    - Generic struct support: `struct Box<T> { value: T }`
    - Name mangling for methods: `Point_distance`, `Box_int_get`
  - ✅ **Generics (Phase 2):**
    - Generic functions com type parameters
    - Type inference from arguments
    - Generic structs com type inference on construction
    - Generic methods com monomorphization
    - Duck typing (no trait bounds)
    - 21+ generic tests
  - ✅ **Closures (Phase 3):**
    - Capture by reference (pointers)
    - Heap allocation for closures and environments
    - **ARC (Automatic Reference Counting) - FULL IMPLEMENTATION**
    - Automatic retain/release on assignment for ALL heap types
    - Indirect calls via function pointers
    - Closure tests (capture, calls, ARC)
    - **Bug Fix:** Closure analysis now accumulates scope correctly (segfault fix)
  - ✅ **ARC Implementation (February 2026):**
    - ✅ Implemented for: String, Matrix, IntMatrix, ComplexMatrix, Closures
    - ✅ Runtime functions: `*_retain()`, `*_release()` for each type
    - ✅ Codegen: Automatic retain on copy, release on reassignment
    - ✅ All constructors return with ref_count=1 (ownership transfer)
    - ✅ 10 ARC unit tests + 4 integration tests
    - ✅ Stress tests: 100k iterations validated
    - ✅ **Memory leak in loops FIXED** (see Section 9.1.1)
    - ✅ `release_function_scope_vars()` fully operational with null-init allocas
  - ✅ **Stress Tests (Phase 4):**
    - Closures: 10 captured variables, 3 levels nesting, 5 closure chain
    - Structs: 15 fields, 10 default values
    - Generics: 3 type parameters
    - Integration tests: Complex combinations of all v1.3 features
    - 7 unit stress tests + 4 integration stress tests
  - ✅ **All 1089 unit tests + 95 integration tests passing!** 🎉
  - ✅ **Total: 1135 tests (100% passing)** 🎉
- **LLVM Optimizations (COMPLETE - Feb 2026):**
  - ✅ Optimization levels: `-O0`, `-O1`, `-O2`, `-O3`
  - ✅ `--release` flag (equivalent to `-O3`)
  - ✅ Zero-overhead flag parsing via clap
  - ✅ Optimizations applied by LLVM TargetMachine during code generation
  - ✅ **All 68 integration tests passing** with optimizations enabled
  - Usage: `cargo run file.bx -O 3` or `cargo run file.bx --release`

### ✅ **v1.3 - Type System Expansion (COMPLETE - Feb 2026):**
- ✅ **Structs (COMPLETE)** - Go-style receivers, default values, generic support
- ✅ **Generics (COMPLETE)** - Functions, structs, methods com monomorphization
- ✅ **Closures (COMPLETE)** - Capture by reference, heap allocation, ARC, bug fix for scope accumulation
- ✅ **Stress Tests (COMPLETE)** - Edge cases for all v1.3 features
- **Total: 1129 tests (1050 unit + 79 integration) - 100% passing!** 🎉

### ✅ **v1.4 Advanced Type System (COMPLETE - Feb 2026):**
- ✅ Type Aliases (`type MyInt = int`)
- ✅ Union Types (`int | float | string`)
- ✅ Intersection Types (`Point & Label`)
- ✅ Elvis Operator (`a ?: b`)
- ✅ Optional → Union refactoring
- **Total: 1184 tests (292 lexer + 158 parser + 639 codegen + 95 integration) - 100% passing!** 🎉

### ✅ **v1.5 - Iterators, Pipeline, Test Library & Async/Await (COMPLETE - Feb 2026):**
- Test Library Jest-style (`describe`, `it`, `expect`, 28 matchers, 21 suites)
- Unified Ranges (`..` / `..<` / `step`)
- Array Range Literals (`[1..5]`)
- Iterators (`map`, `filter`, `reduce`, `any`, `all`, `find`)
- Pipeline Operator (`|>`)
- Async/Await (state machine LLVM, `async fn`, `await`, `brix_run_to_completion`)
- **Total: 1.133 unit + 110 integration = 1.243 tests (100% passing)**

### 🔮 **Planejado (v1.6):**
- `break` / `continue` em loops
- String Library: `trim`, `ltrim`, `rtrim`, `starts_with`, `ends_with`, `contains`, `substring`, `reverse`, `repeat`, `index_of`
- String iteration (`for ch in "hello"`)
- Matrix constructors: `ones()`, `linspace()`, `arange()`, `rand()`
- 2D Matrix iteration (`.map(fn)` preservando shape)
- Async Closures (`async () -> { await f() }`) e Async Test Matchers
- `await` em control flow aninhado (`if`/`while`/`for` dentro de `async fn`)
- `async { }` blocks
- Pattern Matching 2.0 (destructuring de structs/tuples, range patterns)

### 🔮 **Planejado (v1.7+):**
- `split` / `join` (requer `StringMatrix` como novo BrixType)
- Complex arithmetic operators (`+`, `-`, `*`, `/` em Complex numbers)
- HashMap / Vector / Stack como tipos built-in
- Error handling estilo Go (`result, err := f()`)
- LTO / PGO / SIMD

---

## Identidade

- **Nome:** Brix
- **Extensão de Arquivo:** `.bx`
- **Slogan:** "Doce como Python, Sólido como Fortran."

## Visão e Filosofia

**Objetivo:** Brix é uma linguagem definitiva para Engenharia de Dados e Algoritmos.
Combina a facilidade de prototipagem com a performance bruta.

- **Stack:** Rust + LLVM
- **Gerenciamento de Memória:** ARC (Automatic Reference Counting)

## 1. Visão Geral

- **Paradigma:** Imperativa, Estruturada, Data-Oriented (Array First).
- **Compilação:** AOT (Ahead-of-Time) para Binário Nativo (via LLVM).
- **Linguagem do Compilador:** Rust.
- **Filosofia:** "Escreve-se como Python, executa como Fortran, escala como Go."

---

## 2. Sistema de Tipos e Variáveis

A linguagem possui **Tipagem Forte** e **Estática**, mas com **Inferência de Tipos** agressiva para reduzir a verbosidade.

### Declaração (Influência: TypeScript & Go)

- `const`: Define valores imutáveis (preferencial).
- `var`: Define valores mutáveis.
- `:=`: Declaração rápida com inferência.

```z
// Inferência: 'pi' é f64, imutável
const pi = 3.1415

// Declaração explícita
var count: int = 0
count++  // Operador de incremento suportado
```

### Composição de Tipos (Influência: TypeScript)

Não há herança de classes. O sistema utiliza composição de Structs via tipos de interseção.

```
type Point2D = { x: f64, y: f64 }
type Label = { text: string }

// Composição: NamedPoint tem x, y e text num bloco só de memória
type NamedPoint = Point2D & Label

type User = {
    name: string
    age: int
}

type Admin = {
    role: string
    permissions: [string]
}

// O tipo 'SuperUser' contém todos os campos de User e Admin
// Na memória, isso é uma struct única plana (sem ponteiros extras)
type SuperUser = User & Admin
```

## 3. Estruturas de Dados Fundamentais

### Arrays e Vetores (Influência: Python & Fortran)

O cidadão de primeira classe. Foco em **SIMD e Acesso Contíguo**.

- Slicing: `arr[start:end]` cria uma _View_ (não copia dados).
- Índices Negativos: `arr[-1]` acessa o último elemento.
- Broadcasting: Operações matemáticas aplicadas ao array inteiro.

```
nums := [10, 20, 30, 40, 50]

// Slicing
subset := nums[1:4]  // [20, 30, 40]

// Operação Vetorial (Sem loop explícito)
doubled := nums * 2  // [20, 40, 60, 80, 100]
mask := data > 25         // [false, false, true, true]
```

### Decisões de Design: Arrays e Matrizes (23/01/2026)

#### 1. Tipagem e Inferência de Literais

O compilador analisa elementos literais para decidir a alocação de memória mais eficiente:

- **IntMatrix (i64*)**: Criado quando todos os elementos são inteiros
- **Matrix (f64*)**: Criado quando todos são floats OU há mistura (promoção automática int→float)

```brix
// Cria IntMatrix (i64*)
var arr_int := [1, 2, 3]
var mat_int := [[1, 2], [3, 4]]

// Cria Matrix (f64*)
var arr_float := [1.0, 2.0, 3.0]
var arr_misto := [1, 2, 3.5]  // Promove ints para float
```

#### 2. Construtores de Arrays

Brix oferece múltiplas formas de criar arrays e matrizes:

##### a) Literais de Array (Inferência Automática)

```brix
var nums := [1, 2, 3, 4, 5]    // IntMatrix (todos ints)
var vals := [1, 2.5, 3.7]      // Matrix (mixed → promoção float)
```

##### b) Funções zeros(), izeros() e eye()

Para clareza semântica entre Engenharia (Floats) e Matemática Discreta (Ints):

```brix
// Matrizes Float (f64) - padrão para engenharia/matemática
var m1 := zeros(5)        // Array 1D de 5 floats
var m2 := zeros(3, 4)     // Matriz 3x4 de floats

// Matrizes Int (i64) - para dados discretos/índices
var i1 := izeros(5)       // Array 1D de 5 ints
var i2 := izeros(3, 4)    // Matriz 3x4 de ints

// Matriz identidade n×n (Float) - implementado
var id := eye(3)           // Matriz identidade 3×3
```

**Construtores não implementados ainda (planejados):** `ones()`, `linspace()`, `arange()`, `rand()`

##### c) Inicialização Estática (v0.6 - Implementado)

Sintaxe concisa para alocar memória zerada:

```brix
// Aloca array de 5 inteiros (inicializado com 0)
var buffer := int[5]

// Aloca matriz 2x3 de floats (inicializado com 0.0)
var grid := float[2, 3]

// Equivalente a izeros(5) e zeros(2, 3)
// Compila para a mesma alocação eficiente com calloc
```

**Nota:** Esta sintaxe é açúcar sintático que compila diretamente para zeros()/izeros(), mantendo a mesma performance.

#### 4. Mutabilidade e Segurança

A palavra-chave define o comportamento da memória alocada na Heap:

**`var` (Mutável)**: Permite reescrita de elementos

```brix
var m := [1, 2, 3]
m[0] = 99  // Válido
```

**`const` (Imutabilidade Profunda)**: O compilador bloqueia qualquer tentativa de escrita em índices (Store Instruction)

```brix
const PI_VEC := [3.14, 6.28]
PI_VEC[0] = 1.0  // ❌ Erro de Compilação: Cannot mutate const variable
```

#### 5. Representação Interna

Para manter a performance de "Fortran", não usamos arrays genéricos (`void*`). Utilizamos estruturas C especializadas:

**Estruturas no `runtime.c`:**

```c
// Para Engenharia e Matemática (Padrão)
typedef struct {
    long rows;
    long cols;
    double* data;  // 8 bytes (f64)
} Matrix;

// Para Imagens, Índices e Dados Discretos
typedef struct {
    long rows;
    long cols;
    long* data;    // 8 bytes (i64)
} IntMatrix;

// Para Números Complexos (v1.0+)
typedef struct {
    double real;
    double imag;
} Complex;

// Para Matrizes Complexas (autovalores/autovetores)
typedef struct {
    long rows;
    long cols;
    Complex* data;  // Array de Complex structs
} ComplexMatrix;

// Futuro (v1.1+): Para Textos
typedef struct {
    long rows;
    long cols;
    char** data;   // Array de ponteiros
} StringMatrix;
```

#### 6. Estratégia para Web e JSON

Matrizes e JSON são entidades distintas no Brix:

- **Matriz/Array**: Dados homogêneos e contíguos na memória (Performance CPU)
- **JSON**: Dados heterogêneos em estrutura de árvore

Não forçaremos JSON dentro de `Matrix`. Será criado um tipo `JsonValue` (Tagged Union) específico para interoperabilidade Web, tratado separadamente das estruturas matemáticas.

**Princípio de Design**: Arrays e matrizes armazenam dados homogêneos e contíguos para máxima performance. JSON/dados heterogêneos usarão tipos separados.

---

### Biblioteca Padrão Nativa (Estruturas de Dados)

Estruturas de dados essenciais vêm "na caixa", implementadas sobre Arrays para máxima performance (Cache Locality).

**Removido:** LinkedList/DoublyLinkedList (foco em performance).

**Estruturas Lineares**

- **Vector:** Array dinâmico redimensionável (Padrão da linguagem).
- Stack (Pilha): Implementada sobre Vector.
  - `s := new Stack<int>() -> push(), pop(), peek().`
- **Queue (Fila):** Implementada como Ring Buffer (Array Circular).
  - `q := new Queue<int>() -> enqueue(), dequeue().`

**Estruturas de Busca e Ordenação**

- **HashMap:** Tabela Hash para chave-valor O(1).
- **MinHeap / MaxHeap:** Fila de prioridade (binária) sobre array. Essencial para algoritmos como Dijkstra.
  - `pq := new MinHeap<f64>()`

**Grafos**

- **AdjacencyList:** Implementação otimizada para grafos, onde nós e arestas residem em vetores contíguos (Arena Allocation) em vez de ponteiros dispersos.

## 4. Controle de Fluxo

### ✅ Pattern Matching & Complex Numbers (v1.0 - Implementado - 27/01/2026)

#### Pattern Matching

Pattern matching em Brix substitui `switch/case` complexos com uma sintaxe poderosa e segura.

**Sintaxe:**
```brix
match value {
    pattern -> expression
    pattern if guard -> expression
    pattern1 | pattern2 -> expression
    _ -> expression
}
```

**Patterns Suportados (v1.0):**

- **Literais**: `42`, `3.14`, `"text"`, `true`, `false`
- **Wildcard**: `_` (matches anything, ignora valor)
- **Binding**: `x` (captura valor e vincula a variável)
- **Or-patterns**: `1 | 2 | 3` (match em qualquer um dos valores)
- **Guards**: `x if x > 10` (condições adicionais)

**Exemplos:**

```brix
// Match básico com literais
var result := match x {
    1 -> "one"
    2 -> "two"
    3 -> "three"
    _ -> "other"
}

// Match com guards (condições)
var category := match age {
    x if x < 18 -> "child"
    x if x < 60 -> "adult"
    _ -> "senior"
}

// Or-patterns (múltiplos valores)
var day_type := match day {
    1 | 2 | 3 | 4 | 5 -> "weekday"
    6 | 7 -> "weekend"
    _ -> "invalid"
}

// Type coercion automática (int→float)
var num := match x {
    1 -> 10      // int
    2 -> 20.5    // float (promove arm 1 para float)
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

**Características:**

- **Match como expressão**: Retorna valor que pode ser atribuído
- **Type checking**: Todos os arms devem retornar tipos compatíveis
- **Type coercion**: Promoção automática int→float quando necessário
- **Exhaustiveness warning**: Warning (não bloqueia) quando falta wildcard
- **Guards**: Binding disponível dentro do guard

**Futuro (v1.1+):**
- Destructuring patterns: `{ x: x, y: y }`, `(a, b, c)`, `[first, second, ...]`
- Range patterns: `1..10`
- Exhaustiveness checking obrigatório

---

#### Complex Numbers & LAPACK Integration

Sistema completo de números complexos e integração LAPACK para álgebra linear avançada.

**Tipos Implementados:**

1. **Complex (struct):**
   ```c
   typedef struct {
       double real;
       double imag;
   } Complex;
   ```
   - Usado internamente para cálculos
   - Acessível via LAPACK eigenvalue functions

2. **ComplexMatrix (struct):**
   ```c
   typedef struct {
       long rows;
       long cols;
       Complex* data;
   } ComplexMatrix;
   ```
   - Retorno de `math.eigvals()` e `math.eigvecs()`
   - Printing 2D: `[[a+bi, c+di], [e+fi, g+hi]]`

**Funções LAPACK:**

```brix
import math

// Autovalores (eigenvalues)
var A := zeros(2, 2)
A[0][1] = -1.0
A[1][0] = 1.0
var eigenvalues := math.eigvals(A)
println(f"Eigenvalues: {eigenvalues}")  // [[0+1i], [0-1i]]

// Autovetores (eigenvectors)
var I := math.eye(3)
var eigenvectors := math.eigvecs(I)
println(f"Eigenvectors: {eigenvectors}")  // [[1+0i, 0+0i, 0+0i], ...]
```

**Implementação Técnica:**

- **LAPACK dgeev:** Double precision general eigenvalue solver
- **Column-major conversion:** Converte row-major (Brix) → column-major (Fortran/LAPACK)
- **Work array queries:** Two-pass LAPACK (query optimal size, then compute)
- **Complex conjugate pairs:** LAPACK armazena eigenvectors complexos como pares conjugados
- **2D Matrix Printing:** Usa modulo arithmetic para detectar row boundaries e formatar como `[[row1], [row2]]`

**Características:**

- ✅ Autovalores sempre retornam ComplexMatrix (mesmo quando reais)
- ✅ Autovetores nas colunas da matriz (convenção matemática)
- ✅ Links com `-llapack -lblas`
- ✅ Formato 2D para legibilidade visual
- ⚠️ Erro handling: exit(1) para matrizes não-quadradas (futuro: Go-style (error, value) tuples)

**Testes:**
- `eigvals_simple_test.bx` - Identity matrix ✅
- `eigvals_rotation_test.bx` - Complex eigenvalues ✅
- `eigvecs_test.bx` - 5 diferentes cenários ✅

**Limitações Atuais:**
- Complex arithmetic operators (+, -, *, /) não implementados ainda
- Complex numbers só acessíveis via eigvals/eigvecs
- Planned for v1.1: Full complex number support with operators

### Loops (Implementado - v1.5)

```brix
// Iteração sobre range inclusivo (0, 1, 2, 3, 4, 5)
for i in 0..5 {
    println(i)
}

// Iteração sobre range exclusivo (0, 1, 2, 3, 4)
for i in 0..<5 {
    println(i)
}

// Range com step
for i in 0..10 step 2 {
    println(i)   // 0, 2, 4, 6, 8, 10
}

// Range decrescente (step auto = -1)
for i in 5..0 {
    println(i)   // 5, 4, 3, 2, 1, 0
}

// Iteração sobre array
for i in 0..<arr.cols {
    println(arr[i])
}

// While loop
while condition {
    // ...
}

// Array range literal
var nums := [1..5]     // IntMatrix: [1, 2, 3, 4, 5]
var nums2 := [1..<5]   // IntMatrix: [1, 2, 3, 4]
```

## 5. Funções e Tratamento de Erro

### ✅ User-Defined Functions (v0.8 - Implementado - 26/01/2026)

Brix suporta funções definidas pelo usuário com sintaxe clara e funcionalidades modernas.

#### Funções Básicas

```brix
function add(a: int, b: int) -> int {
    return a + b
}

var result := add(5, 3)  // 8
```

**Características:**
- Keyword: `function` (não `fn`)
- Parâmetros tipados obrigatórios
- Tipo de retorno obrigatório para funções não-void

#### Funções Void

Funções sem retorno não precisam de tipo de retorno explícito:

```brix
function greet(name: string) {
    println(f"Hello, {name}!")
}

greet("Alice")  // Hello, Alice!
```

#### Retornos Múltiplos (Implementado)

Funções podem retornar múltiplos valores como tuples:

```brix
function calculations(a: int, b: int) -> (int, int, int) {
    return (a + b, a - b, a * b)
}

// Acesso via indexing
var result := calculations(10, 5)
println(f"sum = {result[0]}")       // 15
println(f"diff = {result[1]}")      // 5
println(f"product = {result[2]}")   // 50
```

**Sintaxe:**
- Tipo de retorno: `-> (type1, type2, type3)`
- Return statement: `return (value1, value2, value3)` (parênteses obrigatórios)
- Acesso: Array-style indexing `result[0]`, `result[1]`, etc.

#### Destructuring

Desempacotar múltiplos retornos em variáveis separadas:

```brix
var { sum, diff, product } := calculations(10, 5)
println(f"sum = {sum}")       // 15
println(f"diff = {diff}")     // 5
println(f"product = {product}") // 50

// Ignorar valores com _
var { quotient, _ } := divmod(17, 5)  // Ignora remainder
```

**Sintaxe:**
- Destructuring: `var { name1, name2, name3 } := func()`
- Ignorar valores: Use `_` na posição desejada
- Número de variáveis deve corresponder ao número de retornos (exceto `_`)

#### Default Parameter Values

Parâmetros podem ter valores padrão:

```brix
function power(base: float, exp: float = 2.0) -> float {
    return base ** exp
}

println(power(5.0))          // 25.0 (usa exp=2.0 padrão)
println(power(5.0, 3.0))     // 125.0 (sobrescreve exp)

function greet(name: string, greeting: string = "Hello") {
    println(f"{greeting}, {name}!")
}

greet("Alice")          // Hello, Alice!
greet("Bob", "Hi")     // Hi, Bob!
```

**Características:**
- Sintaxe: `param: type = default_value`
- Default values são avaliados no call site
- Parâmetros com defaults preenchidos da esquerda para direita
- Erro de compilação se faltarem parâmetros obrigatórios

### Tratamento de Erro (Planejado - v0.9+)

Sistema de erro inspirado em Go será implementado em versões futuras:

```brix
// Planejado para v0.9+
function divide(a: f64, b: f64) -> (f64, error) {
    if b == 0.0 {
        return 0.0, error("Divisão por zero")
    }
    return a / b, nil
}

res, err := divide(10.0, 2.0)
```

## 6. Syntactic Sugar (Facilidades)

- **Ternário:** `val = condition ? trueVal : falseVal`
- **Elvis Operator:** `name = inputName ?: "Default"`
- **String Interpolation:** `msg = f"User: {user.name}"`
- **List Comprehension:** `evens := [x for x in nums if x % 2 == 0]`
- **Métodos Funcionais:** `map`, `filter`, `reduce` (Lazy evaluation).
- **Chained Comparison:** Verificação matemática de intervalos com sintaxe limpa.
  - _Código:_ `if 10 < x <= 20 { ... }`
  - _Compilação:_ Traduzido automaticamente para `(10 < x) && (x <= 20)`, garantindo avaliação única do termo central (side-effect safety).

## 7. Roteiro Técnico (Stack do Compilador)

- **Linguagem de Implementação:** Rust.
- **Backend:** LLVM (via crate `inkwell` ou `llvm-sys`).
- **Lexer:** Logos (Rust crate) ou escrito à mão.
- **Parser:** Chumsky (Parser Combinator) ou LALRPOP.

## 8. Stack Tecnológica

- **Linguagem do Compilador:** Rust 🦀
- **Backend:** LLVM (via `inkwell`).
- **Lexer:** Crate `logos` (Performance extrema).
- **Parser:** Crate `chumsky`.
- **Gerenciamento de Memória:** ARC (Automatic Reference Counting).

## 9. Gerenciamento de Memória e Passagem de Dados

O Brix adota uma filosofia de "Smart Defaults" (Padrões Inteligentes). O compilador toma as decisões difíceis de alocação para garantir performance e segurança, mas oferece controle total sobre mutabilidade.

### 9.1. Modelo de Memória: ARC (Automatic Reference Counting)

Optamos por **ARC** em vez de Garbage Collection (GC) ou Gerenciamento Manual (`malloc/free`).

- **Determinismo:** Não há pausas aleatórias ("Stop the world") do GC. A memória é liberada no exato momento em que a última variável para de usá-la.
- **Performance:** O compilador otimiza incrementos/decrementos de contagem para evitar overhead em loops críticos.

#### 9.1.1. Implementação de ARC (Fevereiro 2026)

**Status:** ✅ **COMPLETO** - ARC implementado para todos os tipos heap-allocated (String, Matrix, IntMatrix, ComplexMatrix, Closures)

**Tipos com ARC:**
- `String` (BrixString)
- `Matrix` (Matrix - f64*)
- `IntMatrix` (IntMatrix - i64*)
- `ComplexMatrix` (ComplexMatrix - Complex*)
- `Closure` (BrixClosure - closures com ambiente capturado)

**Estrutura Interna:**

Todos os tipos ref-counted têm `ref_count` como **primeiro campo**:

```c
typedef struct {
    long ref_count;  // ← ARC reference counter
    long len;
    char* data;
} BrixString;

typedef struct {
    long ref_count;  // ← ARC reference counter
    long rows;
    long cols;
    double* data;
} Matrix;

// Similar para IntMatrix, ComplexMatrix, BrixClosure
```

**Operações ARC:**

1. **Criação (Construtores):**
   - `str_new()`, `matrix_new()`, `intmatrix_new()` retornam objetos com `ref_count = 1`
   - **Ownership transfer**: sem retain na declaração inicial

2. **Cópia (Assignment):**
   ```brix
   var s1 := "hello"    // ref_count = 1 (constructor)
   var s2 := s1          // ref_count = 2 (retain automático)
   ```
   - Codegen detecta cópia de variável e insere `string_retain()` automaticamente

3. **Reassignment:**
   ```brix
   var s := "hello"     // ref_count = 1
   s := "world"         // release("hello"), ref_count de "world" = 1
   ```
   - Codegen insere `string_release()` no valor antigo antes de atribuir o novo

4. **Release Automático:**
   - Runtime libera memória quando `ref_count` chega a 0:
   ```c
   void string_release(BrixString* str) {
       if (!str) return;
       str->ref_count--;
       if (str->ref_count == 0) {
           if (str->data) free(str->data);
           free(str);
       }
   }
   ```

**Implementação Técnica:**

| Arquivo | Mudanças | Descrição |
|---------|----------|-----------|
| `runtime.c` | +200 linhas | Retain/release para 4 tipos, construtores com ref_count=1 |
| `crates/codegen/src/lib.rs` | +150 linhas | `insert_retain()`, `insert_release()`, `is_ref_counted()` |
| `crates/codegen/src/stmt.rs` | +30 linhas | Retain em variable_decl, release em assignment |
| `crates/codegen/src/tests/arc_tests.rs` | 365 linhas | 10 unit tests para ARC |
| `tests/integration/success/71-74_arc_*.bx` | 4 arquivos | Integration tests para ARC |

**Testes de Validação:**

✅ **Unit Tests (10 testes):**
- `test_string_arc_basic` - Criação e ownership transfer
- `test_string_arc_reassignment` - Release automático
- `test_string_arc_copy` - Retain em cópia
- `test_matrix_arc_basic` - Matrix com ref_count
- `test_matrix_arc_reassignment` - Release em reassignment
- `test_intmatrix_arc_basic` - IntMatrix ARC
- `test_intmatrix_arc_reassignment` - IntMatrix release
- `test_mixed_arc_types` - Múltiplos tipos juntos
- `test_no_arc_for_primitives` - Primitivos sem retain/release
- `test_string_concat_arc` - Concatenação cria nova string

✅ **Integration Tests (4 testes):**
- `71_arc_string_basic.bx` - String copy e reassignment
- `72_arc_matrix_reassignment.bx` - Matrix ARC
- `73_arc_intmatrix_basic.bx` - IntMatrix ARC
- `74_arc_mixed_types.bx` - Múltiplos tipos ref-counted

✅ **Stress Tests:**
- 1,000 iterações: ~2s sem crashes
- 10,000 iterações: ~4.8 MB máximo
- 100,000 iterações: sem leak (corrigido Fev 2026)

**✅ RESOLVIDO - Memory Leak em Loops (Fevereiro 2026):**

**Problema Original:**

Existia um memory leak (~0.17 MB por 10,000 iterações) causado por variáveis ref-counted (String, Matrix, IntMatrix, ComplexMatrix) criadas dentro de loops que nunca eram liberadas. A função `release_function_scope_vars()` existia mas estava desabilitada porque causava SIGSEGV em 64 testes.

**Causas Raiz Identificadas:**

1. **Allocas não inicializadas em caminhos condicionais:** Variáveis declaradas dentro de `if` tinham alloca criada no entry block mas o store era condicional. Se o branch não era tomado, a alloca continha lixo → `release(lixo)` → SIGSEGV.

2. **`function_scope_vars` não preservado entre funções aninhadas:** Ao compilar uma função interna, `function_scope_vars.clear()` destruía o tracking da função externa.

3. **Valores antigos em loops nunca liberados:** Cada iteração de loop sobrescrevia o ponteiro na alloca sem liberar o valor anterior → leak acumulativo.

**Solução Implementada:**

1. **Null-inicialização de allocas ref-counted** (`helpers.rs`):
   - Nova função `create_null_init_entry_block_alloca()` cria alloca + store null no entry block
   - Garante que o alloca sempre contém null ou um ponteiro válido (nunca lixo)
   - Release functions em C já checam null → seguro para caminhos condicionais

2. **Release antes de sobrescrever em loops** (`stmt.rs`):
   - Antes de armazenar novo valor, carrega e libera o valor antigo
   - Na primeira execução: carrega null → release(null) → no-op
   - Em iterações subsequentes: carrega ponteiro anterior → libera → sem leak

3. **Save/restore de `function_scope_vars`** (`lib.rs`):
   - `function_scope_vars` salvo/restaurado junto com `variables` ao compilar funções aninhadas
   - Previne corrupção do tracking da função externa

4. **Deduplicação no release** (`lib.rs`):
   - `release_function_scope_vars()` usa HashSet para processar cada variável apenas uma vez
   - Previne double-free

5. **Release em retornos void explícitos** (`stmt.rs`):
   - `return` sem valor em funções void agora chama `release_function_scope_vars()`
   - Previne leak em retornos antecipados

**Comportamento Atual:**
```brix
var i := 0
while i < 1000 {
    var s := "temp string"    // Aloca com ref_count=1
    var m := [1.0, 2.0, 3.0]  // Aloca com ref_count=1
    i := i + 1
    // ✅ Na próxima iteração, s e m são liberados antes de receber novos valores
}
// ✅ Ao final da função, os últimos valores de s e m são liberados
```

6. **Release de temporários em print/println** (`stmt.rs`):
   - `value_to_string()` cria BrixString temporário para tipos não-String (Int, Float, etc.)
   - Agora liberado após `printf()` via `insert_release()`
   - Strings temporárias (literais, f-strings, concatenações) também liberadas
   - Apenas referências a variáveis (`Identifier`, `FieldAccess`) são preservadas

7. **Release de temporários em expressões descartadas** (`stmt.rs`):
   - `compile_expr_stmt` agora libera valores ref-counted que não são armazenados
   - Chamadas de função retornando String/Matrix que são descartadas são liberadas

8. **Release no programa principal** (`lib.rs`):
   - `compile_program()` agora chama `release_function_scope_vars()` antes de retornar
   - Variáveis de top-level são liberadas ao final da execução

**Status:** Todos os 1089 unit tests + 95 integration tests passando (100%)

### 9.2. Passagem de Parâmetros (Cópia vs. Referência)

O usuário não precisa gerenciar ponteiros manualmente (`*ptr` ou `&ref`). O compilador decide a estratégia mais eficiente baseada no tipo do dado:

1.  **Tipos Primitivos (`int`, `f64`, `bool`):** Passagem por **Valor (Copy)**.
    - _Custo:_ Zero (registradores da CPU).
2.  **Tipos Complexos (`Arrays`, `Structs`):** Passagem por **Referência (View)**.
    - O compilador passa um ponteiro silencioso ("fat pointer") contendo endereço e tamanho. Não há cópia profunda de dados.

### 9.3. Imutabilidade e Controle (`mut`)

Por padrão, referências a tipos complexos são **Imutáveis (Read-Only)**. Isso previne efeitos colaterais acidentais (o erro mais comum em concorrência).

```rust
// Padrão: Leitura (Rápido e Seguro)
fn ler_dados(dados: [int]) {
    print(dados[0])
    // dados[0] = 99  <-- ERRO DE COMPILAÇÃO!
}

// Explícito: Escrita (Mutável)
fn zerar_dados(mut dados: [int]) {
    dados[0] = 0 // Permitido. Altera o dado original na memória.
}
```

### 9.4. Estruturas Recursivas e Heap (Linked Lists)

Para criar estruturas de dados como Árvores ou Listas Encadeadas, o Brix evita a complexidade de Box<T> (Rust) ou ponteiros manuais (C).

Utilizamos o sistema de tipos (`?` / `nil`) para inferir alocação na Heap.

- **Regra:** Se uma Struct contém um campo do seu próprio tipo, o compilador exige que ele seja opcional (`?`).
- **Otimização:** O compilador detecta a recursão e, automaticamente, transforma esse campo em um **Ponteiro Gerenciado**.

```rust
type Node = {
    val: int,
    // O '?' sinaliza ao compilador: "Aloque isso na Heap como um ponteiro gerenciado"
    next: Node?
}

// O usuário escreve código limpo, sem asteriscos (*) ou alocações manuais.
var lista := Node { val: 10, next: Node { val: 20, next: nil } }
```

## 10. Status do Desenvolvimento (Atualizado - Jan 2026)

### 📊 Progresso Geral: v0.9 Completo (90% MVP Completo)

---

## ✅ IMPLEMENTADO (v0.1 - v0.3)

### 1. Arquitetura do Compilador

- ✅ **Workspace Cargo:** Separação em crates (`lexer`, `parser`, `codegen`)
- ✅ **Lexer (Logos):** Tokenização completa com comentários, operadores e literais
- ✅ **Parser (Chumsky):** Parser combinator com precedência de operadores correta
- ✅ **Codegen (Inkwell/LLVM 18):** Geração de LLVM IR e compilação nativa
- ✅ **Runtime C:** Biblioteca com funções de Matrix e String

### 1.1. LLVM Optimizations (v1.2.1 - Feb 2026)

**Status:** COMPLETE ✅ - Suporte completo para otimizações LLVM via flags de compilação

**Níveis de Otimização:**

| Nível | Flag | Descrição | Uso Recomendado |
|-------|------|-----------|-----------------|
| `-O0` | Default | Sem otimizações, compilação rápida | Debug, desenvolvimento |
| `-O1` | `-O 1` | Otimizações básicas, tamanho reduzido | Builds intermediários |
| `-O2` | `-O 2` | Otimizações padrão, performance balanceada | Maioria dos casos |
| `-O3` | `-O 3` or `--release` | Otimizações agressivas, máxima performance | Production, benchmarks |

**Exemplos de Uso:**

```bash
# Debug mode (sem otimizações)
cargo run program.bx

# Otimização básica
cargo run -- program.bx -O 1

# Otimização padrão
cargo run -- program.bx -O 2

# Otimização máxima
cargo run -- program.bx -O 3
cargo run -- program.bx --release  # Equivalente a -O3
```

**Implementação Técnica:**

- **TargetMachine OptimizationLevel:** Otimizações aplicadas durante geração de código objeto
- **Zero Overhead:** Flags processadas via clap sem impacto em performance
- **LLVM 18 Backend:** Aproveita otimizações modernas do LLVM (GVN, DCE, inlining, etc.)
- **Compatibilidade:** Todos os 1184 testes (1089 unit + 95 integration) passam com `-O3`

**O que LLVM Otimiza:**

- **-O1 (Less):** Constant folding, dead code elimination básico, simplificação de CFG
- **-O2 (Default):** GVN (Global Value Numbering), loop optimizations, function inlining
- **-O3 (Aggressive):** Vetorização, unrolling agressivo, otimizações interprocedurais

**Benefícios Observados:**

- Código gerado mais compacto e eficiente
- Melhor uso de registradores CPU
- Eliminação de código morto
- Inline de funções pequenas (reduz overhead de chamadas)

**Limitações:**

- Tamanho do binário similar entre níveis (runtime.c é a maior parte)
- Ganhos de performance dependem da complexidade do código Brix
- Tempos de compilação ligeiramente maiores em `-O3`

**Roadmap Futuro:**

- [ ] **LTO (Link-Time Optimization):** Otimizações cross-module
- [ ] **PGO (Profile-Guided Optimization):** Otimizações baseadas em profiling
- [ ] **Size Optimization (-Os, -Oz):** Flags para minimizar tamanho do binário

### 2. Sistema de Tipos

- ✅ **Tipos Primitivos:** `int` (i64), `float` (f64), `bool` (i1→i64), `string` (struct), `matrix` (struct f64*), `intmatrix` (struct i64*), `void`, `tuple` (struct - múltiplos retornos)
- ✅ **Inferência de Tipos:** `var x := 10` detecta automaticamente o tipo
- ✅ **Tipagem Explícita:** `var x: float = 10`
- ✅ **Casting Automático:**
  - `var x: int = 99.9` → trunca para 99 (float→int)
  - `var y: float = 50` → promove para 50.0 (int→float)
  - Promoção automática em operações mistas (int + float → float)
- ✅ **Introspecção:** `typeof(x)` retorna string do tipo em compile-time
- ✅ **Inferência para Arrays/Matrizes (v0.6+):**
  - `[1, 2, 3]` → IntMatrix (todos inteiros)
  - `[1.0, 2.0]` ou `[1, 2.5]` → Matrix (floats ou mistos com promoção)

### 3. Estruturas de Dados

- ✅ **Arrays Literais:** `var v := [10, 20, 30]` (IntMatrix para ints, Matrix para floats/mistos)
- ✅ **Matrizes Dinâmicas:** `var m := matrix(3, 4)` (alocação heap via Runtime C)
- ✅ **Indexação:**
  - Linear: `v[0]`
  - 2D: `m[0][0]` (cálculo `row * cols + col`)
  - L-Value: `m[0][0] = 5.5` (atribuição funcional)
- ✅ **Field Access:**
  - String: `.len`
  - Matrix: `.rows`, `.cols`, `.data`

### 4. Operadores

- ✅ **Aritméticos:** `+`, `-`, `*`, `/`, `%`, `**` (potência)
- ✅ **Unários:** `!`, `not` (negação lógica), `-` (negação aritmética)
- ✅ **Increment/Decrement:** `++x`, `x++`, `--x`, `x--` (pré e pós-fixo)
- ✅ **Comparação:** `<`, `<=`, `>`, `>=`, `==`, `!=`
- ✅ **Chained Comparison:** `if 1 < x <= 10` (açúcar sintático → `1 < x && x <= 10`)
- ✅ **Lógicos:** `&&`, `and`, `||`, `or` (com short-circuit evaluation)
- ✅ **Ternário:** `cond ? true_val : false_val` (com promoção automática de tipos)
- ✅ **Bitwise:** `&`, `|`, `^` (apenas para inteiros)
- ✅ **Strings:** `+` (concatenação), `==` (comparação)
- ✅ **Compound Assignment (Parser):** `+=`, `-=`, `*=`, `/=` (desugared para `x = x + y`)

### 5. Controle de Fluxo

- ✅ **If/Else:** Com blocos aninhados e LLVM Basic Blocks
- ✅ **While Loop:** Implementação completa com header/body/after blocks
- ✅ **For Loop - Range Numérico (Julia Style):**
  - `for i in 1:10` (1 a 10, inclusive)
  - `for i in 0:2:10` (com step customizado)
  - Suporte a expressões: `for k in (start + 1):end`
- ✅ **For Loop - Iteração de Matriz:**
  - `for val in lista` (detecta tipo automaticamente)
  - Itera sobre arrays/matrizes linearmente
- ✅ **For Loop - Destructuring (v0.9):**
  - `for x, y in zip(a, b)` (múltiplas variáveis)
  - Itera sobre linhas quando há múltiplas variáveis
  - Funciona com Matrix e IntMatrix

### 6. Funções Built-in

**Nota:** Para funções definidas pelo usuário, veja seção "## 5. Funções e Tratamento de Erro" ✅ v0.8

**Output:**
- ✅ **printf:** Saída formatada estilo C (`printf("x: %d", x)`)
- ✅ **print:** Imprime qualquer valor sem newline, com conversão automática (`print(42)`, `print("text")`)
- ✅ **println:** Imprime qualquer valor COM newline automático (`println(x)`)

**Input:**
- ✅ **scanf/input:** Entrada tipada (`input("int")`, `input("float")`, `input("string")`)

**Type System:**
- ✅ **typeof:** Retorna tipo como string (`typeof(x)` → "int")
- ✅ **int(x):** Converte para int - trunca floats, parseia strings (`int(3.14)` → 3, `int("42")` → 42)
- ✅ **float(x):** Converte para float - promove ints, parseia strings (`float(10)` → 10.0, `float("3.14")` → 3.14)
- ✅ **string(x):** Converte qualquer tipo para string (`string(42)` → "42")
- ✅ **bool(x):** Converte para boolean - 0/0.0/string vazia = false (`bool(0)` → 0, `bool(42)` → 1)

**Type Checking (v1.1):**
- ✅ **is_nil(x):** Verifica se valor é nil (`is_nil(nil)` → 1, `is_nil(10)` → 0)
- ✅ **is_atom(x):** Verifica se valor é atom (`is_atom(:ok)` → 1, `is_atom(42)` → 0)
- ✅ **is_boolean(x):** Verifica se int é 0 ou 1 (`is_boolean(1)` → 1, `is_boolean(42)` → 0)
- ✅ **is_number(x):** Verifica se é int ou float (`is_number(10)` → 1, `is_number("text")` → 0)
- ✅ **is_integer(x):** Verifica se é int (`is_integer(10)` → 1, `is_integer(3.14)` → 0)
- ✅ **is_float(x):** Verifica se é float (`is_float(3.14)` → 1, `is_float(10)` → 0)
- ✅ **is_string(x):** Verifica se é string (`is_string("hi")` → 1, `is_string(10)` → 0)
- ✅ **is_list(x):** Verifica se é Matrix ou IntMatrix (`is_list([1,2,3])` → 1)
- ✅ **is_tuple(x):** Verifica se é tuple (`is_tuple((10,20))` → 1)
- ✅ **is_function(x):** Verifica se é função (sempre retorna 0 por enquanto - funções não são first-class)

**String Functions (v1.1):**
- ✅ **uppercase(str):** Converte para maiúsculas (`uppercase("hello")` → "HELLO")
- ✅ **lowercase(str):** Converte para minúsculas (`lowercase("HELLO")` → "hello")
- ✅ **capitalize(str):** Primeira letra maiúscula (`capitalize("hello world")` → "Hello world")
- ✅ **byte_size(str):** Tamanho em bytes (`byte_size("Brix")` → 4)
- ✅ **length(str):** Número de caracteres UTF-8 (`length("Hello, 世界!")` → 10)
- ✅ **replace(str, old, new):** Substitui primeira ocorrência (`replace("hello world", "world", "Brix")` → "hello Brix")
- ✅ **replace_all(str, old, new):** Substitui todas ocorrências (`replace_all("hi hi", "hi", "bye")` → "bye bye")

**Data Structures:**
- ✅ **matrix:** Construtor de matriz vazia (`matrix(rows, cols)`)
- ✅ **read_csv:** Lê arquivo CSV como matriz (via runtime C)
- ✅ **zip (v0.9):** Combina dois arrays em pares (`zip([1,2,3], [4,5,6])` → Matrix 3×2 com linhas [1,4], [2,5], [3,6])

### 7. Memória e Performance

- ✅ **Tabela de Símbolos:** HashMap com `(PointerValue, BrixType)` para cada variável
- ✅ **Stack Allocation:** Variáveis alocadas via `alloca` no entry block
- ✅ **Heap (Runtime C):** Matrizes e Strings alocadas dinamicamente
- ✅ **Constant Folding:** LLVM otimiza constantes automaticamente (ex: `2 + 3` → `5`)

### 8. Type Checking e String Operations (v1.1)

#### Type Checking Functions

Sistema completo de verificação de tipos em tempo de execução:

```brix
// Type checking básico
var x := 42
var y := 3.14
var msg := "hello"

println(f"is_integer({x}) = {is_integer(x)}")  // 1
println(f"is_float({y}) = {is_float(y)}")      // 1
println(f"is_string({msg}) = {is_string(msg)}")  // 1

// Type checking combinado
var num := 100
if is_number(num) {
    println("É um número!")  // Verifica int OU float
}

// Boolean validation
var flag := 1
if is_boolean(flag) {
    println("É um boolean válido!")  // Verifica se é 0 ou 1
}

// Nil checking
var err := nil
if is_nil(err) {
    println("Sem erro!")
}

// Atom checking
var status := :ok
if is_atom(status) {
    println("É um atom!")
}
```

#### String Manipulation

Operações completas de string com suporte UTF-8:

```brix
// Transformações de caso
var msg := "hello world"
println(uppercase(msg))    // "HELLO WORLD"
println(lowercase(msg))    // "hello world"
println(capitalize(msg))   // "Hello world"

// Análise de strings
var text := "Hello, 世界!"
println(f"byte_size = {byte_size(text)}")  // 14 (bytes)
println(f"length = {length(text)}")        // 10 (caracteres UTF-8)

// Substituição de texto
var greeting := "Hello world world"
println(replace(greeting, "world", "Brix"))      // "Hello Brix world"
println(replace_all(greeting, "world", "Brix"))  // "Hello Brix Brix"

// Edge cases
var empty := ""
println(f"length(\"\") = {length(empty)}")  // 0

var no_match := replace("abc", "xyz", "123")
println(no_match)  // "abc" (sem mudança)
```

**Características:**
- ✅ **UTF-8 aware:** `length()` conta caracteres corretamente, não bytes
- ✅ **Seguro:** Retorna cópias, strings originais imutáveis
- ✅ **Eficiente:** Implementado em C com malloc/strcpy otimizados

---

## 🚧 ROADMAP: O QUE FALTA IMPLEMENTAR

---

### ✅ **v0.4 - Operadores e Expressões Avançadas** (COMPLETO)

**Prioridade Alta:**

- [x] **Increment/Decrement:** `x++`, `x--`, `++x`, `--x` ✅ **IMPLEMENTADO**
- [x] **Bitwise Operators:** `&`, `|`, `^` ✅ **IMPLEMENTADO**
- [x] **Operador Ternário:** `cond ? true_val : false_val` ✅ **IMPLEMENTADO**
- [x] **Negação Lógica:** `!condition` ou `not condition` ✅ **IMPLEMENTADO**
- [x] **Operador de Potência:** `**` para int e float (usa LLVM intrinsic `llvm.pow.f64`) ✅ **IMPLEMENTADO**
- [ ] **Elvis Operator:** `val ?: default` (para null coalescing futuro - adiado para v0.8 com null safety)

**Açúcar Sintático:**

- [x] **String Interpolation:** `f"Valor: {x}"` com conversão automática de tipos ✅ **IMPLEMENTADO**

---

### ✅ **v0.8 - User-Defined Functions** ✅ **COMPLETO (26/01/2026)**

Sistema completo de funções com múltiplos retornos, destructuring e default values.

**Core:**

- [x] **Declaração de Funções:** `function add(a: int, b: int) -> int { return a + b }` ✅ **IMPLEMENTADO**
- [x] **Chamada de Funções:** `var result := add(10, 20)` ✅ **IMPLEMENTADO**
- [x] **Return Statement:** `return value` ✅ **IMPLEMENTADO**
- [x] **Funções Void:** Funções sem retorno `function greet(name: string) { println(...) }` ✅ **IMPLEMENTADO**
- [x] **Escopo Local:** Variáveis dentro de funções com symbol table save/restore ✅ **IMPLEMENTADO**

**Avançado:**

- [x] **Retornos Múltiplos (Tuples):** `function calc(a, b) -> (int, int, int)` ✅ **IMPLEMENTADO**
- [x] **Tuple Indexing:** Acesso via `result[0]`, `result[1]`, `result[2]` ✅ **IMPLEMENTADO**
- [x] **Destructuring:** `var { sum, diff, product } := calc(10, 5)` ✅ **IMPLEMENTADO**
- [x] **Ignore Values:** `var { quotient, _ } := divmod(17, 5)` ✅ **IMPLEMENTADO**
- [x] **Default Parameters:** `function power(base: float, exp: float = 2.0) -> float` ✅ **IMPLEMENTADO**

**Implementação Técnica:**
- AST: `FunctionDef`, `Return`, `DestructuringDecl`
- Tuples como LLVM structs para múltiplos retornos
- Function registry com metadata de parâmetros
- Default values expandidos no call site
- Type inference completo para tuples

**Testes:**
```brix
// Teste básico
function add(a: int, b: int) -> int { return a + b }
println(add(5, 3))  // 8

// Múltiplos retornos
function calculations(a: int, b: int) -> (int, int, int) {
    return (a + b, a - b, a * b)
}
var result := calculations(10, 5)
println(result[0])  // 15

// Destructuring
var { sum, diff, product } := calculations(10, 5)
println(sum)  // 15

// Default values
function power(base: float, exp: float = 2.0) -> float {
    return base ** exp
}
println(power(5.0))      // 25.0 (usa default)
println(power(5.0, 3.0)) // 125.0
```

**Arquivos de Teste:**
- `function_test.bx` - Funções básicas ✅
- `void_test.bx` - Funções void ✅
- `multiple_return_test.bx` - Múltiplos retornos ✅
- `destructuring_test.bx` - Destructuring básico ✅
- `destructuring_ignore_test.bx` - Destructuring com `_` ✅
- `default_values_test.bx` - Default parameters ✅

**Futuro (v1.6+):**
- [ ] **Error Type:** `function divide(a, b) -> (float, error)` (requer null safety)
- [ ] **Funções Variádicas:** `function sum(nums: ...int)`
- [x] **Closures:** `var fn := (x: int) -> int { return x * 2 }` ✅ **COMPLETO (v1.3)**
- [x] **First-class functions:** Passar funções como parâmetros ✅ **COMPLETO (v1.5 Phase 0a)**

---

### ✅ **v0.9 - List Comprehensions & zip()** ✅ **COMPLETO (27/01/2026)**

Sistema completo de list comprehensions estilo Python com nested loops, múltiplas condições e destructuring.

**Core Features:**

- [x] **zip() Built-in Function:** Combina dois arrays em pares ✅ **IMPLEMENTADO**
  - 4 variantes type-safe: `brix_zip_ii`, `brix_zip_if`, `brix_zip_fi`, `brix_zip_ff`
  - Retorna Matrix(n, 2) ou IntMatrix(n, 2)
  - Usa comprimento mínimo quando arrays diferem
  - Exemplo: `zip([1,2,3], [10,20,30])` → Matrix com linhas [1,10], [2,20], [3,30]

- [x] **Destructuring em for loops:** Múltiplas variáveis ✅ **IMPLEMENTADO**
  - Sintaxe: `for x, y in zip(a, b) { ... }`
  - Itera sobre linhas quando há múltiplas variáveis
  - Suporta Matrix e IntMatrix

- [x] **List Comprehensions:** Sintaxe completa ✅ **IMPLEMENTADO**
  - Básica: `[x * 2 for x in nums]`
  - Com condição: `[x for x in nums if x > 10]`
  - Múltiplas condições (AND): `[x for x in nums if c1 if c2]`
  - Nested loops: `[x * y for x in a for y in b]`
  - Com destructuring: `[x + y for x, y in zip(a, b)]`
  - Loop order: esquerda→direita = outer→inner (Python-style)

- [x] **Array Printing em f-strings:** Matrix/IntMatrix em strings ✅ **IMPLEMENTADO**
  - `println(f"nums = {nums}")` → `nums = [1, 2, 3, 4, 5]`
  - Funciona com `print()`, `println()`, e f-strings

**Implementação Técnica:**
- AST: `ListComprehension`, `ComprehensionGen` structs
- Parser: sintaxe completa com generators aninhados
- Codegen:
  - `compile_list_comprehension()`: orquestra compilação
  - `generate_comp_loop()`: gera loops recursivamente
  - LLVM basic blocks para controle de fluxo
  - Short-circuit evaluation para condições
- Alocação híbrida: pré-aloca max size, preenche conforme condições, redimensiona ao final
- Runtime: 4 funções zip em `runtime.c`
- `value_to_string()`: estendido para Matrix/IntMatrix

**Testes e Exemplos:**

```brix
// 1. Básico
var nums := [1.0, 2.0, 3.0, 4.0, 5.0]
var doubled := [x * 2.0 for x in nums]  // [2, 4, 6, 8, 10]

// 2. Com condição
var evens := [x for x in nums if int(x) % 2 == 0]  // [2, 4]

// 3. Múltiplas condições
var filtered := [x for x in nums if x > 2.0 if x < 5.0]  // [3, 4]

// 4. Nested loops (produto cartesiano)
var a := [1.0, 2.0]
var b := [10.0, 20.0]
var products := [x * y for x in a for y in b]  // [10, 20, 20, 40]

// 5. Com zip e destructuring
var sums := [x + y for x, y in zip(a, b)]  // [11, 22]

// 6. Nested loops com condição
var pairs := [x + y for x in a for y in b if x + y > 15.0]  // [21, 22]

// 7. Array printing
println(f"nums = {nums}")  // Output: nums = [1, 2, 3, 4, 5]
```

**Arquivos de Teste:**
- `zip_test.bx` - zip() function ✅
- `destructuring_for_test.bx` - Destructuring em for loops ✅
- `list_comp_simple_test.bx` - Comprehension básica ✅
- `list_comp_cond_test.bx` - Com condição ✅
- `list_comp_advanced_test.bx` - Nested + múltiplas condições ✅
- `list_comp_zip_test.bx` - Zip + destructuring ✅
- `list_comp_test.bx` - Teste completo (4 cenários) ✅

**Limitações Atuais:**
- Type inference: sempre retorna Matrix (Float) - IntMatrix support planejado
- Sem suporte a matrix comprehension 2D ainda: `[[i+j for j in 1:n] for i in 1:m]`

**Futuro (v1.0+):**
- [ ] **IntMatrix type inference:** Retornar IntMatrix quando expr é int
- [ ] **Matrix Comprehension 2D:** Gerar matrizes 2D diretamente
- [ ] **Generator expressions:** Lazy evaluation com `(x for x in nums)`

---

### 🎨 **v0.6 - IntMatrix Type System & Format Specifiers** ✅ **COMPLETO**

**Motivação:** Adicionar suporte nativo para arrays de inteiros com type inference e complementar o sistema de output com format specifiers.

#### IntMatrix Type System ✅ **IMPLEMENTADO (25/01/2026)**

Sistema completo de arrays tipados com inferência automática e múltiplos construtores:

**1. Type Inference Automático:**
```brix
var int_arr := [1, 2, 3]        // IntMatrix (todos ints)
var float_arr := [1.0, 2.0]     // Matrix (todos floats)
var mixed := [1, 2.5, 3]        // Matrix (misturado → promoção int→float)
```

**2. Construtores zeros() e izeros():**
```brix
var m1 := zeros(5)         // Matrix 1D de 5 floats
var m2 := zeros(3, 4)      // Matrix 3×4 de floats
var i1 := izeros(5)        // IntMatrix 1D de 5 ints
var i2 := izeros(3, 4)     // IntMatrix 3×4 de ints
```

**3. Static Initialization Syntax:**
```brix
var buffer := int[5]       // IntMatrix de 5 elementos (zerado)
var grid := float[2, 3]    // Matrix 2×3 de floats (zerada)
// Syntactic sugar para izeros() e zeros()
```

**4. Indexing e Assignment:**
```brix
var arr := int[10]
arr[0] = 42                // Assignment funciona
var val := arr[0]          // Indexing retorna Int

var mat := float[3, 3]
mat[1][2] = 3.14           // 2D assignment
```

**✅ Implementação Completa:**
- `BrixType::IntMatrix` adicionado ao enum de tipos
- Runtime `IntMatrix` struct em runtime.c (i64* data)
- Funções `intmatrix_new()` e `matrix_new()` com calloc
- Type inference completo em array literals
- Parser para sintaxe `int[n]` e `float[r,c]`
- Indexing e assignment para IntMatrix e Matrix
- typeof() retorna "intmatrix"

**Testes validados:**
- `zeros_test.bx` - zeros() e izeros()
- `static_init_test.bx` - int[n], float[r,c]
- `array_constructors_test.bx` - teste abrangente

#### Format Specifiers ✅ **IMPLEMENTADO**

Atualmente, f-strings convertem valores automaticamente mas sem controle de formato. Precisamos de especificadores printf-style:

**Sintaxe proposta:** `f"{expr:format}"`

**Exemplos:**
```brix
var pi := 3.14159265
var msg := f"Pi com 2 casas: {pi:.2f}"           // "Pi com 2 casas: 3.14"
var precise := f"Pi preciso: {pi:.10f}"          // "Pi preciso: 3.1415926500"

var num := 255
var hex := f"Hex: {num:x}"                       // "Hex: ff"
var oct := f"Octal: {num:o}"                     // "Octal: 377"

var big := 1234567.89
var sci := f"Científico: {big:.2e}"              // "Científico: 1.23e+06"
```

**Formatos suportados:**
- `.Nf`: N casas decimais (float)
- `.Ne`: Notação científica com N dígitos
- `x`: Hexadecimal (lowercase)
- `X`: Hexadecimal (uppercase)
- `o`: Octal
- `b`: Binário

**Implementação:**
- Modificar parser para detectar `:format` após expressões em `{}`
- Estender `FStringPart::Expr` para incluir `Option<String>` com formato
- No codegen, usar formato especificado no `sprintf()` em vez de formato fixo

#### Funções de Conversão de Tipo ✅ **IMPLEMENTADO**

Conversões explícitas entre tipos primitivos já estão funcionando:

```brix
// Float para Int (truncamento)
var x := 3.14
var i := int(x)           // i = 3

// Int para Float
var n := 42
var f := float(n)         // f = 42.0

// String para Int/Float (parsing)
var s := "123"
var num := int(s)         // num = 123
var decimal := float("3.14")  // decimal = 3.14

// Qualquer tipo para String
var msg := string(42)     // "42"
var txt := string(3.14)   // "3.14"

// Conversão para Boolean
var b := bool(1)          // true (1)
var b2 := bool(0)         // false (0)
var b3 := bool("")        // false (string vazia)
var b4 := bool("hello")   // true (string não vazia)
```

**✅ Implementação concluída:**
- Built-in functions no codegen
- Usa lógica similar a `typeof()` mas retorna valores convertidos
- Parsing de strings via funções C: `atoi()`, `atof()`
- `string()` reutiliza `value_to_string()` com `sprintf()`

#### Format Specifiers ✅ **IMPLEMENTADO**

Sistema completo de format specifiers em f-strings foi implementado:

```brix
// Integers
var num := 255
println(f"{num:x}")    // ff (hexadecimal lowercase)
println(f"{num:X}")    // FF (hexadecimal uppercase)
println(f"{num:o}")    // 377 (octal)
println(f"{num:d}")    // 255 (decimal)

// Floats
var pi := 3.14159265359
println(f"{pi:.2f}")   // 3.14 (2 decimals)
println(f"{pi:.6f}")   // 3.141593 (6 decimals)
println(f"{pi:e}")     // 3.141593e+00 (scientific)
println(f"{pi:.2e}")   // 3.14e+00 (scientific with precision)
println(f"{pi:g}")     // 3.14159 (compact)
```

**✅ Status v0.6: 100% COMPLETO**
- AST estendido com campo `format: Option<String>` em `FStringPart::Expr`
- Parser detecta `:format` em expressões f-string
- Codegen mapeia formatos para sprintf printf-style
- Arquivo de teste `format_test.bx` validado

**📋 Decisões de Design Adicionadas (23/01/2026):**
- **IntMatrix vs Matrix**: Inferência automática baseada em literais
- **Inicialização estática**: `int[5]`, `float[2][3]`
- **Construtores**: `zeros()` → Matrix, `izeros()` → IntMatrix
- **Mutabilidade profunda**: `const` bloqueia modificação de elementos
- **Separação JSON**: Arrays homogêneos ≠ JSON heterogêneo

---

### 🧮 **v0.7 - Sistema de Imports e Biblioteca Matemática**

**Status:** 🎯 PRÓXIMO PASSO - Planejamento completo, pronto para implementação (26/01/2026)

**Motivação:** Brix é voltado para Engenharia, Física e Ciência de Dados. Precisamos de um sistema de módulos limpo e funções matemáticas performáticas que não reinventem a roda.

**📋 Decisões Finais (25/01/2026):**

**Implementar em v0.7:**
- ✅ Import com namespace: `import math`
- ✅ Import com alias: `import math as m`
- ✅ 21 funções math.h (trig, exp, log, round, utils)
- ✅ 5 funções estatísticas (sum, mean, median, std, var)
- ✅ 3 funções álgebra linear (det, inv, tr)
- ✅ 6 constantes matemáticas (pi, e, tau, phi, sqrt2, ln2)
- ✅ Total: 29 funções + 6 constantes = 35 itens no namespace math

**Adiado para versões futuras:**
- ⏳ `eigvals(A)` / `eigvecs(A)` → v0.8+ (requer tipo BrixType::Complex para autovalores complexos)
- ⏳ Constantes físicas (c_light, h_planck, G_grav, etc.) → v0.8+ (quando tivermos sistema de unidades)
- ⏳ Selective imports: `from math import sin, cos` → v0.7.1+

---

#### Decisão Arquitetural: Zero-Overhead C Bindings

**Princípio:** Não reimplementar código matemático já otimizado. Usar bibliotecas C battle-tested (math.h, BLAS, LAPACK) através de bindings diretos.

**Performance:**
- ✅ **Zero overhead runtime**: Chamadas diretas via LLVM external declarations
- ✅ **Otimizações nativas**: LLVM pode inline, vetorizar, usar instruções CPU (FSIN, FCOS)
- ✅ **Battle-tested**: Mesmo código usado por NumPy, MATLAB, Julia, R
- ✅ **Dead code elimination**: Funções não usadas não entram no binário final

**Exemplo de performance:**
- Determinante 1000×1000: ~50ms (LAPACK) vs ~5s (implementação naive) → **100× mais rápido**
- Funções trigonométricas: Instruções nativas CPU quando possível

#### Sistema de Imports

**Sintaxe:**

```brix
// Import completo com namespace
import math
var y := math.sin(3.14)
var det := math.det(matrix)

// Import com alias
import math as m
var y := m.sin(3.14)

// Selective import (futuro)
from math import sin, cos, sqrt
var y := sin(3.14)
```

**Arquitetura de Implementação:**

1. **Parser**: Reconhece `import` statement
   ```rust
   Token::Import
   Stmt::Import { module: String, alias: Option<String> }
   ```

2. **Symbol Table**: Cria namespace para módulo importado
   ```rust
   // import math → adiciona namespace "math.*"
   // import math as m → adiciona namespace "m.*"
   ```

3. **Codegen**: Gera declarações LLVM externas
   ```rust
   // Para import math, gera:
   let fn_type = f64_type.fn_type(&[f64_type.into()], false);
   module.add_function("sin", fn_type, Some(Linkage::External));
   ```

4. **Linking**: Linker resolve símbolos em link-time
   ```bash
   cc output.o runtime.o -lm -llapack -lblas -o program
   ```

**Características:**
- ✅ Compile-time only: Import não tem custo em runtime
- ✅ Namespace limpo: Evita poluição global de nomes
- ✅ Explícito: Código autodocumentado (sabe de onde vem cada função)

#### Biblioteca Matemática (import math)

**Runtime como Bridge (runtime.c):**

O runtime.c age como ponte thin para bibliotecas C:

```c
// Funções matemáticas básicas - passthroughs diretos
#include <math.h>

double brix_sin(double x) { return sin(x); }
double brix_cos(double x) { return cos(x); }
double brix_sqrt(double x) { return sqrt(x); }
double brix_exp(double x) { return exp(x); }
double brix_log(double x) { return log(x); }

// Álgebra linear - bindings LAPACK
#include <lapacke.h>

double brix_det(Matrix* A) {
    // Usa LU decomposition otimizada do LAPACK
    lapack_int ipiv[A->rows];
    LAPACKE_dgetrf(LAPACK_ROW_MAJOR, A->rows, A->cols,
                   A->data, A->cols, ipiv);

    // Calcula determinante do produto diagonal
    double det = 1.0;
    for (int i = 0; i < A->rows; i++) {
        det *= A->data[i * A->cols + i];
        if (ipiv[i] != i + 1) det = -det;
    }
    return det;
}
```

**Estrutura da Biblioteca:**

```
stdlib/math/
├── basic.c       // sin, cos, sqrt, exp, log (wrappers math.h)
├── linalg.c      // det, inv, eigvals, tr (bindings LAPACK/BLAS)
└── stats.c       // mean, median, std, variance
```

#### Funções Matemáticas (v0.7)

**Trigonométricas (7 funções via math.h):**
```brix
import math
math.sin(x), math.cos(x), math.tan(x)       // Funções trigonométricas
math.asin(x), math.acos(x), math.atan(x)    // Inversas trigonométricas
math.atan2(y, x)                             // Arco tangente de y/x (4 quadrantes)
```

**Hiperbólicas (3 funções via math.h):**
```brix
import math
math.sinh(x), math.cosh(x), math.tanh(x)    // Hiperbólicas
```

**Exponenciais e Logaritmos (4 funções via math.h):**
```brix
import math
math.exp(x)      // e^x
math.log(x)      // Logaritmo natural (base e)
math.log10(x)    // Logaritmo base 10
math.log2(x)     // Logaritmo base 2
```

**Raízes (2 funções via math.h):**
```brix
import math
math.sqrt(x)     // Raiz quadrada
math.cbrt(x)     // Raiz cúbica
// Nota: pow(x, y) NÃO será implementado - use operador ** já existente
```

**Arredondamento (3 funções via math.h):**
```brix
import math
math.floor(x)    // Arredonda para baixo
math.ceil(x)     // Arredonda para cima
math.round(x)    // Arredonda para o inteiro mais próximo
```

**Utilidades (5 funções via math.h):**
```brix
import math
math.abs(x)       // Valor absoluto (int ou float)
math.fmod(x, y)   // Módulo float (diferente de %)
math.hypot(x, y)  // sqrt(x² + y²) otimizado
math.min(a, b)    // Mínimo de dois valores
math.max(a, b)    // Máximo de dois valores
```

**Constantes Matemáticas (6 constantes):**
```brix
import math
math.pi     // 3.14159265358979323846...
math.e      // 2.71828182845904523536...
math.tau    // 6.28318530717958647692... (2π)
math.phi    // 1.61803398874989484820... (golden ratio)
math.sqrt2  // 1.41421356237309504880...
math.ln2    // 0.69314718055994530942...
```

**Estatística (5 funções - implementação custom):**
```brix
import math
math.sum(arr)     // Soma de elementos
math.mean(arr)    // Média aritmética
math.median(arr)  // Mediana
math.std(arr)     // Desvio padrão
math.var(arr)     // Variância
```

**Álgebra Linear (5 funções - runtime.c + LAPACK):**
```brix
import math
math.det(A)       // Determinante (Gaussian elimination)
math.inv(A)       // Inversa de matriz (Gauss-Jordan)
math.tr(A)        // Transposta (implementação custom)
math.eigvals(A)   // Autovalores (LAPACK dgeev, retorna ComplexMatrix) ✅ v1.0
math.eigvecs(A)   // Autovetores (LAPACK dgeev, retorna ComplexMatrix) ✅ v1.0
```

**Total v0.7+: 31 funções + 6 constantes = 37 itens**

---

#### ⏳ Adiado para v1.1+ (Decomposições Avançadas)

```brix
// ADIADO - Decomposições matriciais avançadas
math.lu(A)        // Decomposição LU
math.qr(A)        // Decomposição QR
math.svd(A)       // Singular Value Decomposition
math.cholesky(A)  // Decomposição de Cholesky
```

**Motivo do adiamento:**
- Requer retorno de múltiplas matrizes (tuples complexos)
- QR retorna (Q, R), SVD retorna (U, Sigma, V)
- Planejado para v1.1+ após melhorias em tuple handling

---

#### ⏳ Adiado para Futuro (Constantes Físicas)

Constantes físicas foram **ADIADAS** até termos sistema de unidades de medida:

```brix
// ADIADO - Aguardando sistema de unidades dimensional
math.c_light      // Velocidade da luz (299792458 m/s)
math.h_planck     // Constante de Planck (6.62607015e-34 J⋅s)
math.G_grav       // Constante gravitacional (6.67430e-11 m³/(kg⋅s²))
math.k_boltzmann  // Constante de Boltzmann (1.380649e-23 J/K)
math.e_charge     // Carga elementar (1.602176634e-19 C)
math.g_earth      // Aceleração gravitacional Terra (9.80665 m/s²)
// ... outras constantes físicas
```

**Motivo do adiamento:**
- Constantes físicas têm unidades (m/s, J⋅s, etc.)
- Seria confuso ter valores sem unidades explícitas
- Aguardando implementação de sistema de unidades dimensionais (v0.9+)
- Quando tivermos: `var c: float<m/s> = physics.c_light`

---

#### ✅ Números Complexos (v1.0 - COMPLETO)

**Status:** Sistema completo de números complexos com literais, operadores, 16+ funções e LAPACK integration.

**Literais e Sintaxe:**
```brix
// Imaginary literals
var i1 := 2.0i        // 0+2im
var i2 := 3i          // 0+3im

// Complex literals (real + imaginary)
var z := 3.0 + 4.0i   // 3+4im
var w := 1.0 - 2.0i   // 1-2im

// Complex constructor
var z3 := complex(5.0, 12.0)  // 5+12im

// Imaginary unit constant (Julia-style)
var unit := im        // 0+1im (built-in constant)

// Implicit multiplication with im
var euler := exp((pi / 2.0)im)  // (pi/2)*im automatically
```

**Nota sobre `im`:**
- Constante builtin `im` = 0+1i (similar ao Julia)
- Evita conflito com loop variables: `for i in 1:10` ainda funciona
- Variáveis do usuário têm prioridade: `var im := 5.0` sobrescreve
- Multiplicação implícita: `(expr)im` → `expr * im` (parser automático)

**Operadores Aritméticos:**
```brix
var z1 := 3.0 + 4.0i
var z2 := 1.0 - 2.0i

// Todos os operadores suportam Complex
var soma := z1 + z2       // 4+2im
var diff := z1 - z2       // 2+6im
var prod := z1 * z2       // 11-2im
var quot := z1 / z2       // -1+2im
var pow := z1 ** 2.0      // Potência

// Auto-conversão Float/Int → Complex
var z3 := z1 + 5.0        // 8+4im
var z4 := 10.0 - z1       // 7-4im
```

**Funções Complexas (16+):**
```brix
// Propriedades
var r := real(z)      // Parte real (retorna Float)
var i := imag(z)      // Parte imaginária (retorna Float)
var mag := abs(z)     // Magnitude |z| (Float)
var theta := angle(z) // Fase/ângulo (Float)
var z_conj := conj(z) // Conjugado (Complex)
var mag_sq := abs2(z) // |z|² (Float)

// Funções exponenciais/logarítmicas
var exp_z := exp(z)   // e^z
var log_z := log(z)   // ln(z)
var sqrt_z := sqrt(z) // √z

// Funções trigonométricas
var sin_z := csin(z)
var cos_z := ccos(z)
var tan_z := ctan(z)

// Funções hiperbólicas
var sinh_z := csinh(z)
var cosh_z := ccosh(z)
var tanh_z := ctanh(z)

// Potência complexa
var pow_z := cpow(z, n)  // z^n
```

**LAPACK Integration:**
```brix
import math

// Autovalores retornam ComplexMatrix
var A := zeros(2, 2)
A[0][1] = -1.0
A[1][0] = 1.0
var eigenvalues := math.eigvals(A)   // ComplexMatrix
var eigenvectors := math.eigvecs(A)  // ComplexMatrix

// Printing automático em formato 2D
println(f"Eigenvalues: {eigenvalues}")  // [[0+1im], [0-1im]]
println(f"Eigenvectors: {eigenvectors}") // [[a+bim, c+dim], [e+fim, g+him]]
```

**Implementação v1.0:**
- ✅ Tipo `BrixType::Complex` e `BrixType::ComplexMatrix`
- ✅ Struct LLVM { f64 real, f64 imag }
- ✅ Imaginary literals (`2.0i`, `3i`)
- ✅ Complex literals (`3.0 + 4.0i`)
- ✅ Constante `im` (imaginary unit)
- ✅ Multiplicação implícita `(expr)im`
- ✅ Operadores aritméticos (+, -, *, /, **)
- ✅ 16+ funções complexas (exp, log, sqrt, trig, hyperbolic)
- ✅ Auto-conversão Float/Int → Complex
- ✅ LAPACK integration (eigvals/eigvecs)
- ✅ 2D matrix printing para ComplexMatrix
- ✅ String format com "im" suffix

**Performance:** SIMD-friendly (2 floats = 16 bytes, cabe em registradores)

---

### 📦 **v0.8 - Arrays Avançados e Slicing**

**Slicing:**

- [ ] **Slicing Básico:** `arr[1:4]` retorna view (sem cópia)
- [ ] **Índices Negativos:** `arr[-1]` pega último elemento
- [ ] **Step em Slicing:** `arr[0:10:2]` (elementos pares)
- [ ] **Omissão de Índices:** `arr[:5]`, `arr[5:]`, `arr[:]`

**Broadcasting:**

- [ ] **Operações Escalar-Vetor:** `vetor * 2` multiplica todos os elementos
- [ ] **Operações Vetor-Vetor:** `v1 + v2` (elemento a elemento)

**Construtores Especiais:**

- [ ] **zeros(n):** Cria array/matriz de zeros
- [ ] **ones(n):** Cria array/matriz de uns
- [ ] **eye(n):** Cria matriz identidade
- [ ] **linspace(start, end, n):** Array espaçado linearmente
- [ ] **arange(start, end, step):** Similar ao range do NumPy

---

### 🗂️ **v0.9 - Structs e Tipos Customizados**

**Structs Básicos:**

- [ ] **Definição:** `type Point = { x: float, y: float }`
- [ ] **Criação:** `var p := Point { x: 10.0, y: 20.0 }`
- [ ] **Field Access:** `p.x`, `p.y`
- [ ] **Field Assignment:** `p.x = 15.0`

**Composição de Tipos (TypeScript Style):**

- [ ] **Intersection Types:** `type NamedPoint = Point & Label`
- [ ] **Herança via Composição:** Campos de múltiplos tipos em um único struct

**Null Safety:**

- [ ] **Tipos Opcionais:** `var x: string?` (pode ser `nil`)
- [ ] **Safe Navigation:** `x?.length`
- [ ] **Elvis com Nil:** `x ?: "default"`

---

### 🎭 **v1.0 - Pattern Matching** ✅ **COMPLETO (27/01/2026)**

#### Pattern Matching Fase 1 (Scalar Patterns) ✅

**Substituir switch/case complexos:**

- [x] **Match Básico (literais):**
  ```brix
  match x {
      1 -> "one"
      2 -> "two"
      _ -> "other"
  }
  ```
- [x] **Wildcard:** `_` (matches anything)
- [x] **Binding:** `x` (captura valor)
- [x] **Or-patterns:** `1 | 2 | 3`
- [x] **Guards (Condições):** `x if x > 10 -> ...`
- [x] **Type coercion:** int→float automático
- [x] **Match em typeof():** `match typeof(value) { "int" -> ... }`
- [x] **Exhaustiveness warning**

#### Pattern Matching Fase 2 (Destructuring) - v1.1+

- [ ] **Struct patterns:** `{ status: 200, body: b } -> ...`
- [ ] **Tuple patterns:** `(a, b, c) -> ...`
- [ ] **Array patterns:** `[first, second, ...rest] -> ...`
- [ ] **Range patterns:** `1..10 -> ...`
- [ ] **Exhaustiveness checking obrigatório**

---

### 🎯 **v1.0 - Advanced Features** ✅ **COMPLETO (28/01/2026)**

**Status Geral:**
- [x] Pattern matching (`match` syntax) ✅ **COMPLETO**
- [x] Complex numbers (literals, operators, 16+ functions) ✅ **COMPLETO**
- [x] LAPACK integration (eigvals, eigvecs) ✅ **COMPLETO**
- [x] Nil/Error handling (Go-style) ✅ **COMPLETO**
- [x] Closures and lambda functions ✅ **COMPLETO (v1.3)**
- [x] First-class functions ✅ **COMPLETO (v1.3 - via closures)**
- [ ] User-defined modules ⏸️ **Adiado para v1.7+**

**O que foi implementado em v1.0:**

1. **Pattern Matching Completo:**
   - Scalar patterns (literais, wildcard, binding)
   - Or-patterns (`1 | 2 | 3`)
   - Guards (`x if x > 10`)
   - Type coercion automática
   - Match em typeof()
   - Exhaustiveness warning

2. **Complex Numbers Completo:**
   - Tipos Complex e ComplexMatrix
   - Imaginary literals: `2.0i`, `3i`
   - Complex literals: `3.0 + 4.0i`
   - Constante `im` (imaginary unit, Julia-style)
   - Multiplicação implícita: `(expr)im`
   - Operadores: +, -, *, /, **
   - 16+ funções: exp, log, sqrt, sin/cos/tan (complex), sinh/cosh/tanh, real, imag, abs, angle, conj, abs2
   - Auto-conversão Float/Int → Complex
   - String format com "im" suffix

3. **LAPACK Integration:**
   - Funções `math.eigvals()` e `math.eigvecs()`
   - LAPACK dgeev integration
   - 2D matrix printing para ComplexMatrix
   - Column-major conversion
   - Work array queries

**Próximo:** v1.1 - Type checkers, String functions

---

### ✅ **v1.1 - Atoms & Escape Sequences** ✅ **COMPLETO (29/01/2026)**

Sistema de atoms estilo Elixir com atom pool global e correção completa de escape sequences em strings.

**Atoms (Elixir-style):**

Atoms são constant values (interned strings) usados para representar estados e identificadores imutáveis.

**Sintaxe:**
```brix
// Atom literals
var status := :ok
var msg := :error
var custom := :my_custom_atom

// Comparações O(1)
if status == :ok {
    println("Success!")
}

// Pattern matching
match status {
    :ok -> println("All good")
    :error -> println("Something failed")
    :pending -> println("Waiting...")
    _ -> println("Unknown status")
}

// typeof
println(typeof(status))  // "atom"
```

**Características:**

1. **Interned Strings:**
   - Atoms são armazenados em pool global
   - Cada atom recebe ID único (i64)
   - Strings duplicadas compartilham mesmo ID

2. **O(1) Comparison:**
   - Comparação por ID (não por string)
   - Performance superior a string comparison

3. **Memory Efficient:**
   - Cada atom armazenado uma única vez
   - IDs pequenos (8 bytes)

**Implementação Técnica:**

1. **Lexer (token.rs):**
   ```rust
   #[regex(r":[a-zA-Z_][a-zA-Z0-9_]*", priority = 4, callback = |lex| {
       let s = lex.slice();
       s[1..].to_string()  // Remove leading ':'
   })]
   Atom(String),
   ```

2. **Parser (ast.rs):**
   ```rust
   pub enum Literal {
       // ... existing
       Atom(String),  // :ok, :error, :atom_name
   }
   ```

3. **Runtime (runtime.c):**
   ```c
   typedef struct {
       char** names;
       long count;
       long capacity;
   } AtomPool;

   // Global atom pool
   static AtomPool ATOM_POOL = {NULL, 0, 0};

   // Intern string and return ID
   long atom_intern(const char* name) {
       // Search for existing atom
       for (long i = 0; i < ATOM_POOL.count; i++) {
           if (strcmp(ATOM_POOL.names[i], name) == 0) {
               return i;
           }
       }
       // Add new atom with dynamic realloc
       // ... (implementation in runtime.c)
       return ATOM_POOL.count++;
   }

   // Get atom name from ID
   const char* atom_name(long id);
   ```

4. **Codegen:**
   - `BrixType::Atom` → i64 in LLVM
   - Calls `atom_intern()` during compilation
   - Pattern matching via ID comparison
   - typeof() returns "atom"

**Escape Sequences Fix:**

Implementado processamento completo de escape sequences em todos os contextos de strings.

**Função Helper:**
```rust
fn process_escape_sequences(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    'b' => result.push('\u{0008}'),
                    'f' => result.push('\u{000C}'),
                    _ => {
                        result.push('\\');
                        result.push(next);
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}
```

**Escape Sequences Suportados:**
- `\n` - Newline (line feed)
- `\t` - Tab horizontal
- `\r` - Carriage return
- `\\` - Backslash literal
- `\"` - Double quote
- `\b` - Backspace
- `\f` - Form feed

**Aplicado em:**
- String literals: `"hello\nworld"`
- F-strings: `f"text {expr}"`
- Pattern literals: `"line1\nline2"`
- Printf format strings

**Lexer String Fix (v1.1 - 03/02/2026):**

Correção no lexer para aceitar aspas escapadas em f-strings e strings regulares:

```rust
// ANTES (limitado):
#[regex(r#"f"([^"\\]|\\["\\bnfrt])*""#, |lex| lex.slice().to_string())]
FString(String),

// DEPOIS (aceita qualquer escape):
#[regex(r#"f"(([^"\\]|\\.)*)""#, |lex| lex.slice().to_string())]
FString(String),
```

Agora funciona corretamente:
```brix
var msg := f"He said \"Hello\" to me"  // ✅ Funciona!
var text := "Quote: \"text\""           // ✅ Funciona!
```
- Printf format strings: `printf("Name:\t%s\n", name)`
- Atom names (edge case): `:atom_with_\n`

**Exemplos:**
```brix
// String literals
var msg := "Hello\nWorld"
println(msg)
// Output:
// Hello
// World

// Pattern matching
var text := "Line 1\nLine 2"
match text {
    "Line 1\nLine 2" -> println("Match!")
    _ -> println("No match")
}

// Printf
printf("Name:\t%s\nAge:\t%d\n", "Alice", 30)
// Output:
// Name:   Alice
// Age:    30
```

**Testes:**
- `atom_simple_test.bx` - Atoms básicos ✅
- `atom_test_v2.bx` - Pattern matching ✅
- `atom_test_fixed.bx` - Suite completa ✅
- `atom_with_newlines_test.bx` - Atoms com \n ✅
- `newline_test.bx` - Validação de \n ✅
- `escape_test.bx` - Todos os escapes ✅

**Design Decisions:**
- **Atom representation:** i64 ID (não string) para performance
- **Atom pool:** Global static pool com dynamic realloc
- **Comparison:** ID equality (O(1))
- **Memory:** Shared strings (atoms duplicados = mesmo ID)
- **Pattern matching:** Full support
- **Escape sequences:** Processados no parser (não no lexer)
- **Compatibility:** Atoms podem conter chars escapados (raro mas suportado)

**Performance:**
- Atom interning: O(n) worst case (linear search)
- Atom comparison: O(1) (ID equality)
- Memory overhead: 8 bytes per atom ID + shared string storage

**Futuro (v1.2+):**
- [ ] **Atom GC:** Cleanup de atoms não usados (low priority)
- [ ] **Atom limits:** Warning quando pool cresce demais
- [ ] **Hash table:** Substituir linear search por hash table para O(1) interning

---

### ✅ **v1.1 - Type Checkers & String Functions** ✅ **COMPLETO (03/02/2026)**

**Status:** 100% completo! Todas as features planejadas foram implementadas.

**Implementado:**
- [x] Atoms (Elixir-style) ✅ **COMPLETO (29/01/2026)**
- [x] Escape sequences (\n, \t, \r, \\, \", \b, \f) ✅ **COMPLETO (29/01/2026)**
- [x] Lexer string fix (aspas escapadas \" em f-strings) ✅ **COMPLETO (03/02/2026)**
- [x] Type checking functions (10 funções: is_nil, is_atom, is_boolean, is_number, is_integer, is_float, is_string, is_list, is_tuple, is_function) ✅ **COMPLETO (03/02/2026)**
- [x] String functions (7 funções: uppercase, lowercase, capitalize, byte_size, length, replace, replace_all) ✅ **COMPLETO (03/02/2026)**

**Notas:**
- `split()` e `join()` foram adiadas para v1.2 pois requerem o tipo `StringMatrix` que ainda não existe
- Todas as 18 features têm testes completos e funcionando
- Arquivos de teste: `fstring_escape_test.bx`, `type_check_test.bx`, `string_functions_test.bx`

### 🎯 **INFRAESTRUTURA DE TESTES** (2-3 semanas) 🚧 **EM ANDAMENTO (03/02/2026)**

**MUDANÇA ESTRATÉGICA:**

Antes de implementar novas features (v1.2+), vamos focar em **infraestrutura de qualidade** para garantir robustez do código existente.

**Motivação:**
- ❌ Zero testes automatizados (só 49+ testes manuais .bx)
- ❌ 573 unwrap() calls que podem crashar
- ❌ 6,093-line monolithic codegen/lib.rs
- ❌ Mensagens de erro ruins (Ariadne unused)

**Objetivo:** Implementar **~1,520 testes automatizados** em 5 fases.

---

#### **Fase 1: Lexer Tests** (3-4 dias) 🎯 **EM ANDAMENTO**

**Unit Tests para tokenização:**
- ~400 tests cobrindo todos os 80+ tokens
- Edge cases: empty strings, escape sequences, números extremos
- Testes de precedência (ImaginaryLiteral vs Float+Identifier)
- Validação de regex patterns

**Arquivos a criar:**
```
crates/lexer/src/tests/
  mod.rs              # Test module setup
  token_tests.rs      # Basic token recognition (~200 tests)
  number_tests.rs     # Int/Float/Imaginary edge cases (~50 tests)
  string_tests.rs     # String/FString/Escape sequences (~80 tests)
  atom_tests.rs       # Atom literals edge cases (~30 tests)
  edge_cases.rs       # Weird inputs, malformed tokens (~40 tests)
```

---

#### **Fase 2: Parser Tests** (4-5 dias)

**Unit Tests para AST construction:**
- ~480 tests cobrindo todas as expressões e statements
- Operator precedence completo (power > mul > add > bitwise > cmp > logical)
- Pattern matching edge cases
- Destructuring validation
- Error recovery (continuar parsing após erro)

**Edge cases:**
- Expressões aninhadas: `((((1 + 2) * 3) / 4) ** 5)`
- Chained comparisons: `1 < x < 10 < 100`
- Nested f-strings: `f"outer {f"inner {x}"} end"`
- Match exhaustiveness
- Empty blocks: `if x { }`
- Trailing commas: `[1, 2, 3,]`

---

#### **Fase 3: Codegen Tests** (5-6 dias)

**Unit Tests para geração LLVM IR:**
- ~560 tests cobrindo todas as 60+ built-in functions
- Type inference e casting (int→float, etc)
- Complex numbers e matrix operations
- Control flow (if/else, loops, match)
- Function calls (user-defined, defaults, multiple returns)
- String interpolation com format specifiers

**Edge cases:**
- Division by zero (compile OK, runtime error)
- Integer overflow (i64 limits)
- Type mismatches (int + string)
- Null pointer checks (is_nil)
- Empty arrays: `[]`
- 1D vs 2D matrix indexing

---

#### **Fase 4: Integration Tests** (2-3 dias)

**Golden File Tests:**
- ~60 testes end-to-end (compile + run + output comparison)
- Converter todos os 49+ arquivos .bx existentes
- Adicionar testes para features v1.1 (type checking, strings, atoms)
- Programs com Unicode, múltiplas funções, imports, errors, pattern matching

**Estrutura:**
```
tests/
  integration_test.rs
  golden/
    arithmetic.bx
    arithmetic.expected
    (50+ test pairs)
```

---

#### **Fase 5: Property-Based Tests** (2-3 dias) - OPCIONAL

**Geração automática com proptest:**
- ~20 proptests validando propriedades matemáticas
- Comutatividade: `a + b == b + a`
- Associatividade: `(a + b) + c == a + (b + c)`
- Roundtrip: `int(float(x)) == x`

---

### 📊 Total de Testes: ~1,520 | Tempo: 16-21 dias

**Distribuição:**
- Lexer: ~400 tests (3-4 dias)
- Parser: ~480 tests (4-5 dias)
- Codegen: ~560 tests (5-6 dias)
- Integration: ~60 tests (2-3 dias)
- Property-based: ~20 tests (2-3 dias, opcional)

**Próximos passos após testes:**
1. Refatoração arquitetural (modularizar codegen)
2. Error handling (substituir unwrap() por Result<>)
3. Ariadne integration (mensagens bonitas)
4. LSP + REPL
5. Então: v1.2 (docs, panic, modules)

---

### ⏸️ **v1.2 - Documentation & Advanced Features** (ADIADO - Após Testes)

**NOTA:** Esta versão foi adiada para priorizar infraestrutura de testes. Closures foram movidas para v1.3.

#### Documentation System (planejado)

- [ ] **@doc annotations:** Documentação inline no código
- [ ] **Doc generation:** Gerar documentação HTML/Markdown
- [ ] **Examples em docs:** Código executável em documentação

#### Advanced Functions (planejado)

- [ ] **panic():** Error handling alternativo para erros irrecuperáveis
- [ ] **Advanced string functions:** split(), join(), trim(), etc.

#### User-Defined Modules (planejado)

- [ ] **Sintaxe de módulo:** `module mymod { ... }`
- [ ] **Export/import:** `export function foo()`, `import mymod`
- [ ] **Multi-file compilation**

---

### ✅ **v1.3 - Type System Expansion (Closures, Structs, Generics)** **COMPLETE (Feb 2026)** 🎉

**Status:** Implementação finalizada (Feb 2026)
- ✅ **Phase 1: Structs (COMPLETE)** - Go-style receivers, default values, generic support
- ✅ **Phase 2: Generics (COMPLETE)** - All 6 sub-phases complete
- ✅ **Phase 3: Closures (COMPLETE)** - Full implementation with ARC
- ✅ **Phase 4: Stress Tests (COMPLETE)** - Edge cases and performance limits

Esta versão introduz features fundamentais do sistema de tipos: **Closures**, **Structs** e **Generics**. Todas as 3 features implementadas e testadas (Fev 2026). **Total: 1129 tests (1050 unit + 79 integration) - 100% passing!** 🎉

---

#### **1. Closures (Lambda Functions)** ✅ **COMPLETE**

**Status:** Fully implemented (Feb 2026)
- ✅ Phase 3.1: AST implementation complete
- ✅ Phase 3.2: Parser complete (expression-based with block support)
- ✅ Phase 3.3: Capture analysis complete (automatic detection)
- ✅ Phase 3.4: Codegen complete (environment struct, closure function, closure struct)
- ✅ Phase 3.5: Closure calling complete (indirect calls)
- ✅ Phase 3.6: Heap allocation complete (malloc/free for closures and environments)
- ✅ Phase 3.7: ARC complete (automatic retain/release)

**Implementation Details:**
- **Memory model:** Heap-allocated closures and environments via `brix_malloc()`
- **ARC:** Automatic Reference Counting with `ref_count` field
- **Closure struct:** `{ ref_count: i64, fn_ptr: ptr, env_ptr: ptr }`
- **Automatic retain:** On load from variable (copying reference)
- **Automatic release:** On reassignment (replaces old value)
- **Memory freed:** When `ref_count` reaches 0
- **Runtime functions:** `closure_retain()`, `closure_release()`, `brix_malloc()`, `brix_free()`

**Sintaxe:**
```brix
// Corpo multi-linha - chaves obrigatórias
var double := (x: int) -> int { return x * 2 }

// Corpo complexo
var complex := (a: int, b: int) -> int {
    var result := a + b
    return result * 2
}

// Como parâmetro de função
fn map(arr: [int], fn: (int) -> int) -> [int] {
    // implementação
}
```

**Type Annotations:** OBRIGATÓRIAS - sem inferência de tipo para assinaturas de closures
```brix
var add := (x: int, y: int) -> int { return x + y }  // ✅ Obrigatório
```

**Captura de Variáveis:**
- [x] **Por Referência** - closures capturam ponteiros para variáveis (não cópias)
- **Decisão:** Eficiente para tipos grandes (Matrix, String)
- ARC gerencia lifetimes automaticamente
- Exemplo:
  ```brix
  var matriz := zeros(1000, 1000)  // 8MB
  var sum := 0
  var closure := (x: int) -> int {
      return x + sum  // Captura ponteiro para 'sum' (8 bytes)
  }
  ```

**Recursão em Closures:**
- [x] **PROIBIDA** - closures recursivas criam inferência circular
- **Decisão:** Usar `function` declarations para recursão
- Exemplo:
  ```brix
  // ❌ NÃO PERMITIDO
  var fib := (n: int) -> int {
      if n <= 1 { return n }
      return fib(n-1) + fib(n-2)  // ERRO: recursão em closure
  }

  // ✅ Use function ao invés
  function fib(n: int) -> int {
      if n <= 1 { return n }
      return fib(n-1) + fib(n-2)
  }
  ```

**Closures Genéricos:**
- [x] **PERMITIDO** - closures podem ter type parameters
```brix
var identity := <T>(x: T) -> T { return x }

identity<int>(42)        // 42
identity<string>("hi")   // "hi"
```

---

#### **2. Structs (User-Defined Types)**

**Sintaxe:**
```brix
// Multi-linha: sem vírgulas
struct Point {
    x: int
    y: int
}

// Inline: usa ponto-e-vírgula
struct Point { x: int; y: int }

// Com default values
struct Config {
    timeout: int = 30
    retries: int = 3
    url: string          // Sem default - obrigatório
}
```

**Construção:**
```brix
// Usa todos os defaults
var cfg1 := Config{ url: "https://example.com" }

// Override parcial
var cfg2 := Config{
    timeout: 60,
    url: "https://example.com"
}  // Usa default retries=3

// Todos os campos especificados
var point := Point{ x: 10, y: 20 }
```

**Methods (Go-style Receivers):**
```brix
struct Point {
    x: int
    y: int
}

// Sintaxe de receiver: fn (receiver: Type) method_name()
fn (p: Point) distance() -> float {
    return sqrt(float(p.x**2 + p.y**2))
}

// Chamada de método (dot notation)
var point := Point{ x: 3, y: 4 }
var dist := point.distance()  // 5.0
```

**Mutabilidade:**
- [x] **SEM keyword `mut`** - todo método pode modificar o receiver
- **Decisão:** Simplicidade - não precisa declarar mutabilidade
```brix
fn (p: Point) move(dx: int, dy: int) {
    p.x += dx  // ✅ Permitido - modifica receiver
    p.y += dy
}

var point := Point{ x: 2, y: 3 }
point.move(5, 10)  // point agora é {7, 13}
```

**Escolha de Design:** Go-style receivers ao invés de `extend` blocks
- **Decisão:** Seguir convenções do Go para consistência
- Sintaxe mais simples para definição de métodos
- Sem necessidade de namespacing de extensões

---

#### **3. Generics (Parametric Polymorphism)** ✅ **IMPLEMENTED (Feb 2026)**

**Status:** COMPLETE - All phases implemented and tested
- ✅ Phase 2.1: Generic Functions (parser, AST, monomorphization)
- ✅ Phase 2.2: Generic Function Calls (explicit + inferred types)
- ✅ Phase 2.3: Type Inference System (deduce T from arguments)
- ✅ Phase 2.4: Type Substitution (replace type params in signatures)
- ✅ Phase 2.5: Generic Structs (definitions, construction, field access)
- ✅ Phase 2.6: Generic Methods (Go-style receivers, monomorphization)
- **21 generic tests + 1 integration test passing** ✅

**Generic Functions:**
```brix
// Angle brackets com tipos explícitos
fn map<T, U>(arr: [T], fn: (T) -> U) -> [U] {
    // implementação
}

// Múltiplos type parameters
fn zip<A, B>(arr1: [A], arr2: [B]) -> [(A, B)] {
    // implementação
}
```

**Generic Structs:**
```brix
// Single type parameter
struct Box<T> {
    value: T
}

// Múltiplos type parameters
struct Pair<A, B> {
    first: A
    second: B
}

// Construção - inferência de tipo a partir dos valores
var box := Box{ value: 42 }           // Infere Box<int>
var pair := Pair{ first: 1, second: 3.14 }  // Infere Pair<int, float>
```

**Type Constraints:**
- [x] **NENHUM** - abordagem duck typing
- **Decisão:** Sem trait bounds ou interface constraints
- Erro de compilação se tipo não suporta operações requeridas
- Exemplo:
  ```brix
  fn add<T>(a: T, b: T) -> T {
      return a + b  // Compila apenas se T tem operator+
  }

  add(1, 2)        // ✅ int tem operator+
  add("a", "b")    // ✅ string tem operator+ (concat)
  add(:ok, :err)   // ❌ Erro de compilação: Atom não suporta operator+
  ```

**Monomorphization:**
- [x] **Estratégia de geração de código**
- Gera código especializado para cada tipo concreto usado
- Similar a templates C++ e generics Rust
- Trade-off: Binário maior para melhor performance em runtime
- Exemplo: `Box<int>` e `Box<string>` geram funções LLVM separadas

**Generic Methods:**
```brix
struct Box<T> {
    value: T
}

// Método pode introduzir type parameters adicionais
fn (b: Box<T>) map<U>(fn: (T) -> U) -> Box<U> {
    return Box{ value: fn(b.value) }
}

// Uso
var int_box := Box{ value: 42 }
var str_box := int_box.map<string>((x: int) -> string {
    return string(x)
})  // Box<string>{ value: "42" }
```

---

#### **4. Error Handling (SEM Result<T,E>)**

**Decisão:** Continuar usando error handling estilo Go com tuplas e nil
- **NÃO vai ter tipo `Result<T, E>` na v1.3**
- **Justificativa:** Já temos padrão funcionando com Error type e nil checking

**Padrão:**
```brix
fn divide(a: int, b: int) -> (float, Error) {
    if b == 0 {
        return (0.0, Error{ message: "division by zero" })
    }
    return (float(a) / float(b), nil)
}

// Uso
var result, err := divide(10, 2)
if err != nil {
    println(err.message)
} else {
    println(result)  // 5.0
}
```

---

#### **Roadmap de Implementação v1.3** ✅ **COMPLETE**

**✅ Fase 1: Structs (COMPLETA - 2-3 semanas)**
1. ✅ Lexer: Token `struct` (already exists)
2. ✅ Parser: Struct definitions, field initialization, Go-style receivers
3. ✅ Codegen: LLVM struct types, field accessors, method compilation
4. ✅ Codegen: Default field values, generic struct support
5. ✅ Tests: Struct tests (constructors, methods, defaults, generic structs)

**✅ Fase 2: Generics (COMPLETA - 3-4 semanas)**
1. ✅ Parser: Angle bracket type parameters `<T, U>`
2. ✅ Type inference: Infer concrete types from usage
3. ✅ Codegen: Monomorphization - generate specialized code per type
4. ✅ Codegen: Generic methods, nested generics
5. ✅ Tests: 21 testes (functions, structs, methods, duck typing errors)

**✅ Fase 3: Closures (COMPLETA - 4-5 semanas)**
1. ✅ AST: `Closure` struct complete (params, return_type, body, captured_vars)
2. ✅ Parser: Expression-based closure syntax with block support
3. ✅ Capture Analysis: Automatic detection of captured variables
4. ✅ Codegen: Environment struct creation, closure function generation, closure struct
5. ✅ Closure Calling: Indirect calls via function pointers
6. ✅ Heap Allocation: `brix_malloc()` and `brix_free()` for closures and environments
7. ✅ ARC: Automatic Reference Counting (retain/release)
8. ✅ Tests: Closure tests (capture, calls, ARC, heap allocation)

**Progresso final:** Todas as 4 fases completas (100%)! 🎉
**Total de testes:** 1129 (1050 unit + 79 integration) - 100% passing
**v1.3 Type System Expansion:** **COMPLETE (Feb 2026)** ✅

---

### ✅ **v1.4 - Advanced Type System (Type Aliases, Union, Intersection, Elvis)** **COMPLETE (Feb 2026)** 🎉

**Status:** Implementação finalizada (Feb 2026)
- ✅ **Task #1: Type Aliases (COMPLETE)** - Aliases para tipos existentes
- ✅ **Task #2: Union Types (COMPLETE)** - Tagged unions com suporte a múltiplos tipos
- ✅ **Task #3: Intersection Types (COMPLETE)** - Struct merging via composition
- ✅ **Task #4: Optional → Union (COMPLETE)** - Refatoração de Optional para usar Union
- ✅ **Task #5: Elvis Operator (COMPLETE)** - Null coalescing operator

Esta versão adiciona sistema de tipos avançado sobre a base sólida do v1.3. Todas as 5 tasks implementadas e testadas (Fev 2026). **Total: 1184 tests (292 lexer + 158 parser + 639 codegen + 95 integration) - 100% passing!** 🎉

---

#### **1. Type Aliases** ✅ **COMPLETE**

**Status:** Fully implemented (Feb 2026)

Type aliases permitem criar nomes alternativos para tipos existentes, melhorando legibilidade e facilitando refatoração.

**Sintaxe:**
```brix
// Aliases para tipos primitivos
type MyInt = int
type Coordinate = float

// Aliases para tipos complexos
type Point2D = Point
type UserID = int
type Callback = (int) -> int

// Aliases para tipos genéricos (resolução em tempo de uso)
type IntBox = Box<int>
```

**Características:**
- **Zero overhead:** Resolvido completamente em tempo de compilação
- **Transparência total:** Alias é 100% equivalente ao tipo original
- **Não cria novo tipo:** `MyInt` e `int` são intercambiáveis
- **Suporta todos os tipos:** Primitivos, structs, generics, unions, closures

**Uso:**
```brix
type UserID = int

fn get_user(id: UserID) -> string {
    return "User " + string(id)
}

var user_id: UserID = 42
println(get_user(user_id))  // "User 42"
```

**Implementação:**
- **Lexer:** Token `Type` já existente
- **Parser:** `parse_type_alias()` em `parser.rs`
- **AST:** `TypeAlias { name: String, target: String }` em `ast.rs`
- **Codegen:** Alias table em `Compiler`, resolução recursiva de aliases
- **Type System:** `BrixType::TypeAlias(String)` em `types.rs`

---

#### **2. Union Types** ✅ **COMPLETE**

**Status:** Fully implemented (Feb 2026)

Union types permitem que um valor seja de um dentre vários tipos possíveis, usando tagged unions para type safety.

**Sintaxe:**
```brix
// Union de tipos primitivos
var x: int | float = 42
x := 3.14  // OK - pode mudar para float

// Union de múltiplos tipos
var result: int | float | string = "error"

// Union com nil (similar a Optional)
var maybe_num: int | nil = nil
```

**Representação Interna (Tagged Union):**
```llvm
// LLVM struct: { i64 tag, largest_type value }
{ i64 tag, double value }  // Para int | float

// Índices de tag:
// 0 = primeiro tipo (int)
// 1 = segundo tipo (float)
// 2 = nil (se presente)
```

**Type Checking:**
```brix
var x: int | float = 42

match x {
    i: int -> println("Int: " + string(i)),
    f: float -> println("Float: " + string(f))
}
```

**Características:**
- **Type safety:** Tag garante segurança em tempo de execução
- **Pattern matching:** Integração completa com match expressions
- **Nil support:** Union com nil substitui Optional
- **Zero runtime overhead:** Tag é um simples i64

**Implementação:**
- **Lexer:** Token `Pipe` (`|`) já existente
- **Parser:** `parse_union_type()` em `parser.rs`
- **AST:** `Union(Vec<String>)` em type annotations
- **Codegen:** Tagged union via LLVM struct, tag checking, value extraction
- **Type System:** `BrixType::Union(Vec<BrixType>)` em `types.rs`

---

#### **3. Intersection Types** ✅ **COMPLETE**

**Status:** Fully implemented (Feb 2026)

Intersection types combinam múltiplos structs em um único tipo via composition (struct merging).

**Sintaxe:**
```brix
struct Point {
    x: int
    y: int
}

struct Label {
    name: string
}

// Intersection type combina ambos os structs
var labeled_point: Point & Label = Point{ x: 10, y: 20 } & Label{ name: "Origin" }

// Acesso a campos de ambos os structs
println(labeled_point.x)     // 10
println(labeled_point.name)  // "Origin"
```

**Representação Interna (Struct Merging):**
```llvm
// LLVM struct resultante: { i64 x, i64 y, BrixString* name }
// Combina campos de Point e Label
```

**Construção:**
```brix
// Sintaxe de construção: struct1{...} & struct2{...}
var point_label := Point{ x: 5, y: 10 } & Label{ name: "A" }
```

**Características:**
- **Field merging:** Todos os campos de ambos os structs ficam disponíveis
- **Method merging:** Métodos de ambos os structs são acessíveis
- **Name collision:** Erro de compilação se houver campos com mesmo nome
- **Generic support:** Funciona com structs genéricos

**Implementação:**
- **Lexer:** Token `Ampersand` (`&`) já existente (usado para bitwise AND)
- **Parser:** `parse_intersection_type()` em `parser.rs`
- **AST:** `Intersection(Vec<String>)` em type annotations
- **Codegen:** Struct merging via LLVM, field concatenation
- **Type System:** `BrixType::Intersection(Vec<BrixType>)` em `types.rs`

---

#### **4. Optional → Union Refactoring** ✅ **COMPLETE**

**Status:** Fully implemented (Feb 2026)

Optional types (`T?`) agora são implementados como syntactic sugar para `Union(T, nil)`.

**Antes (v1.3):**
```brix
var x: int? = 42
// Implementação: Custom Optional type
```

**Depois (v1.4):**
```brix
var x: int? = 42
// Implementação: Union(int, nil) - syntactic sugar
```

**Mudanças:**
- **Parser:** `int?` desugars para `Union(vec!["int", "nil"])`
- **Type System:** `BrixType::Optional` removido, usa `BrixType::Union`
- **Codegen:** Optional usa mesma infraestrutura de Union (tagged union)
- **Compatibilidade:** Sintaxe `T?` continua funcionando (backward compatible)

**Vantagens:**
- **Menos código:** Reutiliza implementação de Union
- **Mais flexível:** Union pode ter mais de 2 tipos (`int | float | nil`)
- **Consistência:** Um único sistema de tipos tagged

---

#### **5. Elvis Operator** ✅ **COMPLETE**

**Status:** Fully implemented (Feb 2026)

O Elvis Operator (`?:`) é um null coalescing operator que retorna o lado esquerdo se não for nil, caso contrário retorna o lado direito.

**Sintaxe:**
```brix
var x: int? = 42
var y: int? = nil

var result1 := x ?: 100  // 42 (x não é nil)
var result2 := y ?: 200  // 200 (y é nil)
```

**Características:**
- **Short-circuit:** Não avalia lado direito se lado esquerdo não é nil
- **Compatível com Union:** Funciona com qualquer Union que contém nil
- **Compatível com Optional:** Funciona com `T?` (que é `Union(T, nil)`)
- **Type safety:** Resultado tem tipo do valor não-nil

**Comportamento:**
```brix
// Com Optional
var opt: int? = nil
var value := opt ?: 999  // value = 999

// Com Union
var multi: int | float | nil = nil
var result := multi ?: 42  // result = 42

// Com ref-counted types
var str: string? = nil
var default := str ?: "default"  // default = "default"
```

**Limitações de Design:**
- **❌ Chained Elvis NÃO suportado:** `a ?: b ?: c` causa erro de compilação
- **Decisão:** Elvis encadeado prejudica legibilidade - use `match` ou `if/else` para casos complexos

**Implementação:**
- **Lexer:** Token `QuestionColon` (`?:`) em `token.rs`
- **Parser:** `BinaryOp::Elvis` em `ast.rs`, precedência entre LogicalOr e Range
- **Codegen:** Nil checking baseado em tipo (Union tag check, pointer null check)
  - Union types: Extract tag (field 0) e compare com nil_index
  - Ref-counted types: `build_is_null()` no ponteiro
  - Literal nil: Sempre true
  - Non-nullable: Sempre false
- **Control Flow:** Basic blocks (lhs_not_nil_bb, rhs_bb, merge_bb) + PHI node

**Exemplo de Compilação:**
```brix
var x: int? = 42
var result := x ?: 100
```

```llvm
; LLVM IR gerado:
; 1. Check if x.tag == nil_index
; 2. Branch: if nil -> rhs_bb, else -> lhs_not_nil_bb
; 3. lhs_not_nil_bb: Extract x.value (field 1)
; 4. rhs_bb: Evaluate rhs (100)
; 5. merge_bb: PHI node seleciona resultado
```

---

#### **Roadmap de Implementação v1.4** ✅ **COMPLETE**

**✅ Task #1: Type Aliases (COMPLETA - 1 semana)**
1. ✅ Lexer: Token `Type` (already exists)
2. ✅ Parser: `type Name = TargetType` syntax
3. ✅ Codegen: Alias table, recursive alias resolution
4. ✅ Tests: 2 unit tests + 1 integration test

**✅ Task #2: Union Types (COMPLETA - 2 semanas)**
1. ✅ Lexer: Token `Pipe` (`|`) (already exists)
2. ✅ Parser: `int | float | string` syntax
3. ✅ Codegen: Tagged unions (LLVM struct with tag + value)
4. ✅ Pattern matching integration
5. ✅ Tests: 5 unit tests + 2 integration tests

**✅ Task #3: Intersection Types (COMPLETA - 1.5 semanas)**
1. ✅ Lexer: Token `Ampersand` (`&`) (already exists)
2. ✅ Parser: `Point & Label` syntax
3. ✅ Codegen: Struct merging (field concatenation)
4. ✅ Method merging
5. ✅ Tests: 3 unit tests + 1 integration test

**✅ Task #4: Optional → Union (COMPLETA - 1 semana)**
1. ✅ Parser: Desugar `T?` to `Union(T, nil)`
2. ✅ Type System: Remove `BrixType::Optional`
3. ✅ Codegen: Use Union infrastructure for Optional
4. ✅ Tests: Verify backward compatibility

**✅ Task #5: Elvis Operator (COMPLETA - 1 semana)**
1. ✅ Lexer: Token `QuestionColon` (`?:`)
2. ✅ Parser: `a ?: b` syntax, precedence rules
3. ✅ Codegen: Nil checking + conditional branching + PHI node
4. ✅ Tests: 1 integration test + unit tests

**Progresso final:** Todas as 5 tasks completas (100%)! 🎉
**Total de testes:** 1184 (292 lexer + 158 parser + 639 codegen + 95 integration) - 100% passing
**v1.4 Advanced Type System:** **COMPLETE (Feb 2026)** ✅

---

### ✅ **v1.5 - Iterators & Pipeline (Closures, Ranges, map/filter/reduce, |>)** **COMPLETE (Feb 2026)** 🎉

#### **Phase 0 — Prerequisitos (COMPLETE)**

- ✅ **Closures como parâmetros de função** — Symbol table fix: closures agora podem ser passadas e chamadas como argumentos de funções regulares.
- ✅ **Nested closures ARC double-free** — Corrigido para um nível de nesting.
- ✅ **Method calls em expressões arbitrárias** — `[1,2,3].map(fn).filter(pred)` funciona via chain postfix.

#### **1. Array Type Syntax (`int[]`, `float[]`)** ✅ **COMPLETE**

Permite usar `int[]` e `float[]` em anotações de tipo para struct fields e parâmetros de função.

```brix
function process(nums: int[]) -> float[] { ... }
struct Collection { items: int[], size: int }
```

**Implementação:**
- `type_annotation_parser()` em `parser.rs`: consome `[]` opcional após o tipo base
- `string_to_brix_type()` em `lib.rs`: `"int[]"` → `BrixType::IntMatrix`, `"float[]"` → `BrixType::Matrix`

#### **2. Unified Ranges (`..` / `..<` / `step`)** ✅ **COMPLETE**

Substitui a sintaxe antiga `start:end` por ranges estilo Swift/Kotlin.

```brix
for i in 0..5 { }           // inclusivo: 0, 1, 2, 3, 4, 5
for i in 0..<5 { }          // exclusivo: 0, 1, 2, 3, 4
for i in 0..10 step 2 { }   // com passo: 0, 2, 4, 6, 8, 10
for i in 10..0 { }          // decrescente: 10, 9, ..., 0 (step auto = -1)
var nums := [1..5]           // array range literal → IntMatrix [1, 2, 3, 4, 5]
var nums := [1..<5]          // exclusivo → [1, 2, 3, 4]
```

**AST:** `ExprKind::Range { start, end, step: Option<Expr>, inclusive: bool }`

**Tokens adicionados** (em `token.rs`, com prioridade de matching via logos):
- `DotDotLt` (`..<`) — exclusive range, prioridade sobre `DotDot`
- `DotDot` (`..`) — inclusive range, prioridade sobre `Dot`
- `PipeGt` (`|>`) — pipeline operator, prioridade sobre `Pipe`

**`step` como soft keyword:** reconhecido via `select! { Token::Identifier(s) if s == "step" => () }` — não reservado.

**Codegen for-loop:**
- Predicado dinâmico: `inclusive` → `SLE`/`SGE`; exclusivo → `SLT`/`SGT`
- Auto-step: `select` LLVM instrução escolhe +1 ou -1 baseado em `start > end`
- `compile_range_to_array()`: gera loop que preenche `IntMatrix` via `intmatrix_new()`

**Testes:** 6 novos tokens (lexer), 6 novos expr tests (parser), 8 novos codegen tests, 2 integration tests (`99_range_for_loops.bx`, `100_range_array_literal.bx`)

#### **3. Core Iterators (`map`, `filter`, `reduce`)** ✅ **COMPLETE**

Métodos funcionais em `IntMatrix` e `Matrix` com closures ou funções nomeadas.

```brix
var doubled := [1, 2, 3].map((x: int) -> int { return x * 2 })        // [2, 4, 6]
var evens   := [1, 2, 3, 4].filter((x: int) -> bool { return x % 2 == 0 })  // [2, 4]
var sum     := [1, 2, 3].reduce(0, (acc: int, x: int) -> int { return acc + x })  // 6

// Chaining
var result := [1, 2, 3, 4, 5]
    .map((x: int) -> int { return x * 2 })
    .filter((x: int) -> bool { return x > 5 })
    .reduce(0, (acc: int, x: int) -> int { return acc + x })
```

**Implementação** (`compile_iterator_method()` em `lib.rs`):
- Dispatch antes do struct method dispatch (guarda `matches!(field, "map" | "filter" | ...)`)
- `infer_closure_return_type()`: lê `return_type` da closure ou registry de funções
- `load_closure_fn_env()`: extrai `fn_ptr` e `env_ptr` do closure struct `{ref_count, fn_ptr, env_ptr, destructor}`
- `map`: aloca resultado com `intmatrix_new(1, len)` ou `matrix_new(1, len)`, loop com `build_indirect_call`
- `filter`: dois passes — temp array + count; depois copia para resultado de tamanho exato
- `reduce`: `acc_alloca` + loop com call; retorna escalar

#### **4. Pipeline Operator (`|>`)** ✅ **COMPLETE**

Syntactic sugar: `a |> method(args)` → `a.method(args)` (desugar no parser, zero mudanças no codegen).

```brix
var result := [1, 2, 3, 4, 5]
    |> map((x: int) -> int { return x * 2 })
    |> filter((x: int) -> bool { return x > 5 })
    |> reduce(0, (acc: int, x: int) -> int { return acc + x })
```

**Parser** (`parser.rs`): nível de precedência entre `range` e `ternary`, usando `foldl` sobre `repeated()`:
```rust
range.then(
    just(Token::PipeGt)
        .ignore_then(select! { Token::Identifier(name) => name })
        .then(args.delimited_by(LParen, RParen))
        .repeated()
).foldl(|lhs, (method, args)| {
    Call { func: FieldAccess { target: lhs, field: method }, args }
})
```

#### **5. Additional Iterators (`any`, `all`, `find`)** ✅ **COMPLETE**

```brix
var has_even := [1, 2, 3].any((x: int) -> bool { return x % 2 == 0 })   // 1 (true)
var all_pos  := [1, 2, 3].all((x: int) -> bool { return x > 0 })         // 1 (true)
var found    := [1, 3, 4, 7].find((x: int) -> bool { return x % 2 == 0 }) // int? (não nil)
```

- `any` / `all`: early return via basic blocks adicionais (`any_found` / `all_false`)
- `find`: retorna `Union(Int, Nil)` — tagged struct `{ i64 tag, i64 value }` (tag=0=Int, tag=1=Nil)

**Roadmap de Implementação v1.5** ✅ **COMPLETE**

| Fase | Feature | Status |
|------|---------|--------|
| 0a | Closures como parâmetros | ✅ |
| 0b | Nested closures ARC | ✅ |
| 0c | Method calls em expr arbitrárias | ✅ |
| 1 | Array Type Syntax (`int[]`) | ✅ |
| 2 | Ranges Unificados | ✅ |
| 3 | Core Iterators (map/filter/reduce) | ✅ |
| 4 | Pipeline Operator (`\|>`) | ✅ |
| 5 | Additional Iterators (any/all/find) | ✅ |

**Total de testes:** 1.108 unit (298 lexer + 164 parser + 646 codegen) + 107 integration + 21 Brix test suites = **100% passing** 🎉

---

### 📚 **v1.2 - Standard Library (Stdlib)**

**Estruturas de Dados Nativas:**

- [ ] **Vector<T>:** Array dinâmico com `push()`, `pop()`, `insert()`, `remove()`
- [ ] **Stack<T>:** Pilha (LIFO) implementada sobre Vector
- [ ] **Queue<T>:** Fila (FIFO) como Ring Buffer
- [ ] **HashMap<K, V>:** Tabela hash O(1) com FNV/SipHash
- [ ] **HashSet<T>:** Conjunto sem duplicatas
- [ ] **MinHeap<T> / MaxHeap<T>:** Fila de prioridade (para Dijkstra, etc)
- [ ] **AdjacencyList:** Grafo otimizado com Arena Allocation

**Math Library:**

- [ ] **Funções Básicas:** `sqrt`, `pow`, `log`, `exp`, `abs`, `floor`, `ceil`
- [ ] **Trigonometria:** `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`
- [ ] **Estatística:** `mean`, `median`, `std_dev`, `variance`, `min`, `max`
- [ ] **Helpers:** `clamp`, `lerp`, `map_range`, `sign`

**Date & Time:**

- [ ] **Armazenamento:** Unix Timestamp (i64) para performance
- [ ] **Parsing/Formatting:** ISO 8601 (`"2024-01-15T10:30:00Z"`)
- [ ] **Timezones:** UTC por padrão, conversões via IANA timezone DB
- [ ] **Aritmética:** `date.add(2.days)`, `date.sub(1.week)`

---

### 🧪 **Test Library (Biblioteca de Testes) — COMPLETA (Feb 2026)**

**Status:** ✅ IMPLEMENTADA — 28 funções de matcher, 20 arquivos `.test.bx` passando

**Objetivo:** Biblioteca de testes nativa em Brix, inspirada no Jest, para criar testes unitários e de integração diretamente na linguagem.

#### Como usar

```brix
import test

test.describe("Calculator", () -> {
    test.it("adds two numbers", () -> {
        var result := 2 + 2
        test.expect(result).toBe(4)
    })

    test.it("handles floats", () -> {
        test.expect(3.14159).toBeCloseTo(3.14)
    })
})
```

```bash
cargo run -- test            # Roda todos os *.test.bx e *.spec.bx
cargo run -- test math       # Filtra por nome de arquivo
```

#### API Completa

**Estrutura:**
- `test.describe(name: string, fn: closure)` — Agrupa testes relacionados
- `test.it(name: string, fn: closure)` — Define um teste individual
- `test.expect(value)` — Inicia uma cadeia de matchers

**Matchers de Igualdade (6 funções C):**
- `test.expect(x).toBe(expected)` — Igualdade estrita (int, float, string)
- `test.expect(x).not.toBe(expected)` — Negação (int, float, string)

**Igualdade de Arrays (2 funções C):**
- `test.expect(arr).toEqual(expected)` — Comparação elemento a elemento (IntMatrix, Matrix)

**Float com Precisão Inteligente (1 função C):**
- `test.expect(x).toBeCloseTo(expected)` — Precisão automática baseada nas casas decimais do valor esperado
  - `toBeCloseTo(3.14)` → compara com 2 casas decimais
  - `toBeCloseTo(9.5)` → compara com 1 casa decimal

**Truthiness (2 funções C):**
- `test.expect(x).toBeTruthy()` — Verifica se é truthy (≠ 0)
- `test.expect(x).toBeFalsy()` — Verifica se é falsy (== 0)

**Comparações Numéricas (8 funções C — int + float cada):**
- `test.expect(x).toBeGreaterThan(n)`
- `test.expect(x).toBeLessThan(n)`
- `test.expect(x).toBeGreaterThanOrEqual(n)`
- `test.expect(x).toBeLessThanOrEqual(n)`

**Containment (3 funções C):**
- `test.expect(s).toContain(sub)` — Substring em string
- `test.expect(arr).toContain(val)` — Elemento em int array ou float array

**Tamanho de Coleção (3 funções C):**
- `test.expect(s).toHaveLength(n)` — Tamanho de string (chars UTF-8)
- `test.expect(arr).toHaveLength(n)` — Tamanho de int array ou float array

**Nil (2 funções C):**
- `test.expect(x).toBeNil()` — Verifica se é nil
- `test.expect(x).not.toBeNil()` — Verifica se não é nil

#### Output Format (Estilo Jest)

```
Calculator
  ✓ adds two numbers (0ms)
  ✗ handles division (0ms)

      Expected: 2
      Received: 2.5

      at ./tests/brix/mytest.test.bx:12

Test Suites: 0 passed, 1 failed, 1 total
Tests:       1 passed, 1 failed, 2 total
Time:        0.001s
```

#### Implementação

- **Runtime:** `runtime.c` linhas ~2090–2509 (28 funções C)
- **Codegen:** `crates/codegen/src/lib.rs` → `compile_test_matcher()` (~300 linhas)
- **Declarações:** `crates/codegen/src/builtins/test.rs` (70 linhas)
- **Estrutura interna C:** `TestResult { name, passed, error_msg, duration_ms }` + `TestSuite { results, count, describe, start_time }`
- **Expect chain:** `test.expect(x)` armazena valor em estado global C; matchers leem e avaliam
- **Comportamento de falha:** Acumula erros sem interromper; mostra sumário com exit code 1 se algum falhou

#### Matchers NÃO implementados (planejados para v1.6+)

- `toThrow` / `toThrowError` — requer suporte a exceções
- `toMatch(regex)` — requer implementação de regex
- `toStartWith` / `toEndWith` — strings
- `toHaveProperty` — structs
- Matchers de mock/spy (`toHaveBeenCalled`, etc.)
- Matchers async (`resolves`, `rejects`)

---

### 🔧 **v1.6 - Extensions (Planejado)**

**Status:** Planejado — implementações confirmadas como pendentes (ver BRIX_TESTS.md)

#### String Extensions

As seguintes funções de string estão planejadas mas **ainda não implementadas**:

| Função | Descrição |
|--------|-----------|
| `trim(s)` | Remove espaços do início e fim |
| `trim_start(s)` | Remove espaços do início |
| `trim_end(s)` | Remove espaços do fim |
| `split(s, delim)` | Divide string por delimitador (retorno como array?) |
| `join(arr, sep)` | Une array de strings com separador |
| `starts_with(s, prefix)` | Verifica prefixo |
| `ends_with(s, suffix)` | Verifica sufixo |
| `contains(s, sub)` | Verifica se substring existe |
| `substring(s, start, len)` | Extrai substring |
| `reverse(s)` | Inverte a string |

#### Matrix Constructors

| Função | Descrição |
|--------|-----------|
| `ones(n)` / `ones(r, c)` | Matrix de uns (float) |
| `linspace(start, stop, n)` | N valores igualmente espaçados |
| `arange(start, stop, step)` | Array com step (como Python `range` para floats) |
| `rand(n)` / `rand(r, c)` | Matrix com valores aleatórios [0, 1) |

#### Control Flow Extensions

**`break` e `continue` em loops:**
- Requer: token `break`/`continue` no lexer, `StmtKind::Break` + `StmtKind::Continue` no AST
- Codegen: jump para `break_block` (após loop) ou `continue_block` (cabeçalho do while)
- Compilador precisa rastrear o bloco de destino do loop atual

#### Closure Fixes

**Closures aninhadas (ARC):**
- Criar closure dentro de outra closure causa segfault por double-free no ARC
- Requer revisão do ciclo de vida em escopos aninhados

**Closures como parâmetros de função:**
- `fn map(arr: intmatrix, f: (int) -> int)` falha no codegen
- Closure não é registrada no symbol table quando passada como argumento

---

### 🚀 **Concorrência e Paralelismo**

#### Async/Await (High-Performance I/O) ✅ COMPLETO — v1.5

**Status:** ✅ Implementado em v1.5 via compile-time state machine transformation. `async fn`, `await`, e `brix_run_to_completion` funcionando. Extensões (await em control flow aninhado, async closures) planejadas para v1.6.

#### Paralelismo de Dados (Planejado v1.7+)

- [ ] **par for:** Distribui iterações entre threads automaticamente
- [ ] **par map:** Map paralelo sobre arrays
- [ ] **Threads Nativas:** `spawn { ... }` (estilo Go)

#### Async/Await — Detalhes de Design

**Decisão de Design (Feb 2026):** Implementação via **compile-time state machine transformation**, seguindo modelo do Rust para atingir performance competitiva (0.2-0.3 MB/task vs 2.66 MB/task do Go).

**Motivação:**
- Benchmarks mostram Rust async (0.21 MB/task) sendo 12x mais eficiente que Go goroutines (2.66 MB/task) em 1M tasks concorrentes
- Brix precisa suportar alta concorrência para web servers e I/O intensivo
- Performance é pilar da linguagem ("execute like Fortran")

**Abordagem Técnica:**

1. **Compile-Time Transformation (Zero Overhead Runtime)**
   - Parser detecta `async function` e `await` expressions
   - Codegen transforma em state machine (struct com enum de estados)
   - LLVM otimiza state machine para código nativo eficiente
   - Tamanho por task: ~32-128 bytes (apenas variáveis capturadas + estado)

2. **State Machine via LLVM**
   ```brix
   // Código Brix
   async function fetch_data() -> String {
       var response := http_get("url").await
       var data := parse(response).await
       return data
   }

   // Transformado em (pseudo-código LLVM)
   struct FetchDataFuture {
       int state;           // 0=Start, 1=WaitingHttp, 2=WaitingParse, 3=Done
       void* http_future;   // Subfuture se ainda esperando
       String* response;    // Dados capturados
       String* result;      // Resultado final
   }

   PollResult fetch_data_poll(FetchDataFuture* self) {
       switch(self->state) {
           case 0: /* Start - inicia http_get */
           case 1: /* WaitingHttp - poll http, avança para parse */
           case 2: /* WaitingParse - poll parse, retorna resultado */
           case 3: /* Done */
       }
   }
   ```

3. **Runtime Minimalista (runtime.c)**
   - Event loop simples (~200-300 linhas)
   - Executor com fila de tasks
   - Poll cooperativo (não cria threads por task)
   - Integração com epoll/kqueue para I/O assíncrono
   - Overhead total: ~50KB código + array de ponteiros

4. **Syntax Proposta**
   ```brix
   // Definição async
   async function handle_request(req: Request) -> Response {
       var user := db.get_user(req.user_id).await
       var posts := db.get_posts(user.id).await
       return render_template(posts)
   }

   // Spawning tasks
   spawn handle_request(request)  // Lança no executor

   // Await inline
   var result := some_async_fn().await

   // Executor principal
   function main() {
       var executor := Executor.new()
       executor.spawn(server_loop())
       executor.run()  // Block até todas tasks completarem
   }
   ```

**Vantagens sobre Rust:**
- ✅ **Sem Borrow Checker:** Não precisa de `Pin<Box<dyn Future>>`, `Send`, `Sync`
- ✅ **ARC Automático:** Ownership resolvido em runtime, syntax mais simples
- ✅ **Runtime em C:** Mais fácil debugar e integrar com bibliotecas existentes
- ✅ **Mesma Performance:** LLVM otimiza state machines igualmente

**Vantagens sobre Go:**
- ✅ **12x Menos Memória:** State machines vs goroutine stacks
- ✅ **Compile-Time Transform:** Sem overhead de scheduler em runtime
- ✅ **Stack Inline:** Futures pequenas ficam na stack (0 bytes heap)

**Performance Esperada:**
- Target: **0.2-0.3 MB/task** (competitivo com Rust tokio)
- 1M tasks concorrentes: ~200-300 MB total
- Latência: Microsegundos (poll cooperativo, sem context switch)

**Roadmap de Implementação:**
1. Parser: Detectar `async` e `await` keywords
2. AST: Adicionar `AsyncFunction` e `Await` variants
3. Codegen: State machine transformation via LLVM
4. Runtime: Event loop + executor em runtime.c (~300 linhas)
5. Stdlib: Async I/O primitives (file, network, timers)

**Dependências:**
- ✅ Closures (v1.3 COMPLETE) - Para callbacks em async context
- ✅ Generics (v1.3 COMPLETE) - Para `Future<T>` type
- ⏸️ Result<T,E> (v1.7+) - Para error handling em async (continua padrão Go por enquanto)

**Referência:**
- Análise de performance: https://pkolaczk.github.io/memory-consumption-of-async/
- Rust async internals: https://rust-lang.github.io/async-book/
- Decisão tomada: Fevereiro 2026

---

### 🌟 **v1.2+ - Features Experimentais**

**SQL e JSON como Tipos Nativos (Zero-ORM):**

- [ ] **SQL Typed:**
  ```brix
  var users := sql {
      SELECT name, email FROM usuarios WHERE active = true
  }
  ```
- [ ] **JSON Validation:** Objetos JSON validados em compile-time

**Extension Methods:**

- [ ] **Estender Tipos Existentes:**
  ```brix
  extension float {
      fun to_percent() -> string { return f"{self * 100}%" }
  }
  ```

**Unidades de Medida (Dimensional Safety):**

- [ ] **Tipos com Unidades:** `var distancia: float<m> = 100.0`
- [ ] **Inferência Dimensional:** `var velocidade := distancia / tempo` → `float<m/s>`
- [ ] **Erro de Compilação:** `distancia + tempo` → `Cannot add float<m> to float<s>`

---

### 📝 **Backlog (Sem Versão Definida)**

- [ ] **Módulos e Imports:** Sistema de pacotes (`import math from "std/math"`)
- [ ] **Generics:** `function map<T, U>(arr: [T], fn: T -> U) -> [U]`
- [ ] **Traits/Interfaces:** Polimorfismo sem herança
- [ ] **Macros:** Metaprogramação compile-time
- [ ] **Package Manager:** Gerenciador de dependências (estilo Cargo/npm)
- [ ] **REPL:** Modo interativo para testes rápidos
- [ ] **LSP (Language Server Protocol):** Autocomplete, go-to-definition, etc
- [ ] **Debugger:** Integração com GDB/LLDB

---

## 11. Cronograma Visual de Desenvolvimento

```
v0.1 ████████████████████ 100% ✅ Lexer, Parser, Codegen básico
v0.2 ████████████████████ 100% ✅ Tipos, Casting, Operadores
v0.3 ████████████████████ 100% ✅ Matrizes, Loops, typeof()
v0.4 ████████████████████ 100% ✅ Operadores avançados, string interpolation
v0.5 ████████████████████ 100% ✅ Format specifiers
v0.6 ████████████████████ 100% ✅ IntMatrix type system
v0.7 ████████████████████ 100% ✅ Import system, math library (38 functions)
v0.8 ████████████████████ 100% ✅ User-defined functions, multiple returns
v0.9 ████████████████████ 100% ✅ List comprehensions, zip(), destructuring
v1.0 ████████████████████ 100% ✅ Pattern matching, Complex, LAPACK, Nil/Error
v1.1 ████████████████████ 100% ✅ Atoms, Escapes, Type checkers (10), Strings (7)
v1.2.1 ██████████████████ 100% ✅ Error Handling (Result types, 1129 tests)
TESTES ████████████████████ 100% ✅ Testing Infrastructure (1129 tests) - COMPLETO
v1.2 ░░░░░░░░░░░░░░░░░░░░   0% ⏸️ Docs, panic, modules (ADIADO)
v1.3 ████████████████████ 100% ✅ Structs, Generics, Closures, Stress Tests 🎉
v1.4 ████████████████████ 100% ✅ Type Aliases, Union, Intersection, Elvis 🎉
v1.5 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Async/Await, Test Library, Iterators (PLANEJADO)
```

**Legenda:**
- ✅ Completo
- 🚧 Em desenvolvimento
- 📋 Planejado
- 🎯 Meta principal

---

## 12. Diferenciais Competitivos (The "Killer Features")

Para destacar o Brix no cenário atual, a linguagem adota três pilares de inovação que resolvem dores latentes de Engenharia de Dados e Backend.

### 12.1. Pipeline First (`|>`) ✅ **IMPLEMENTADO (v1.5)**

Inspirado em Elixir e F#, mas focado em processamento de dados massivos. O operador pipe transforma código aninhado complexo em um fluxo linear de leitura natural.

- **Conceito:** `a |> method(args)` é syntactic sugar para `a.method(args)` — desugar no parser, zero overhead em runtime.
- **Implementação atual:** Funciona com todos os métodos de array (`.map()`, `.filter()`, `.reduce()`, etc.) e qualquer method call.

```brix
// Funcionando hoje em Brix:
var result := [1, 2, 3, 4, 5]
    |> map((x: int) -> int { return x * 2 })
    |> filter((x: int) -> bool { return x > 5 })
    |> reduce(0, (acc: int, x: int) -> int { return acc + x })
// result = 30
```

- **Paralelismo Implícito:** Planejado para v1.6+ — injetar paralelismo automaticamente via `par` em cadeias de pipes.

```brix
// Visão futura (v1.6+):
"vendas_2024.csv"
    |> io::read_csv()
    |> par map((x: Row) -> Row { x.total *= 1.1 })  // paralelo
    |> filter((x: Row) -> bool { x.total > 100 })
    |> json::serialize()
    |> http::post("api/vendas")
```

### 12.2. SQL e JSON como Tipos Nativos (Zero-ORM)

O Brix elimina a necessidade de ORMs lentos e a insegurança de strings SQL puras. O compilador entende a estrutura do banco de dados e valida queries em tempo de build.

- **JSON Typed:** Objetos literais são validados estaticamente.
- **SQL Checked:** Se a coluna não existe no banco, o código não compila.

```rust
// JSON é validado na compilação
var config = {
    "host": "localhost",
    "retries": 3
}

// O retorno 'users' é inferido automaticamente como:
// Array<{ name: string, email: string }>
var users := sql {
    SELECT name, email
    FROM usuarios
    WHERE active = true
}
```

### 12.3. Unidades de Medida (Dimensional Safety)

Focado em sistemas críticos (Engenharia, Finanças, Física), o sistema de tipos impede erros semânticos de grandezas.

- **Segurança:** Impossível somar Metros com Segundos ou Reais com Dólares acidentalmente.
- **Custo Zero:** As unidades existem apenas no compilador. No binário final, são apenas números f64 puros (sem overhead de performance).

```rust
// Definição de grandezas
var distancia: f64<m> = 100.0
var tempo: f64<s> = 9.58

// Operação válida (Inferência: velocidade é f64<m/s>)
var velocidade := distancia / tempo

// Erro de Compilação: "Cannot add type f64<m> to f64<s>"
// var erro := distancia + tempo
```

## 13. Modern Developer Experience (Influência Kotlin & Swift)

Para garantir a adoção por desenvolvedores mobile e modernos, o Brix adota padrões de sintaxe que priorizam segurança e legibilidade fluida.

### 13.1. Null Safety (`?`)

O sistema de tipos elimina o erro de "referência nula" por design. Tipos são não-nulos por padrão.

```rust
var a: string = "Safe" // Nunca será null
var b: string? = nil  // Pode ser null

// Safe Call Operator
var len := b?.length ?: 0 // Se b for null, retorna 0 (Elvis Operator)
```

### 13.2. Extension Methods

Permite estender tipos existentes (incluindo primitivos) com novas funcionalidades, mantendo o código organizado sem herança complexa.

```rust
extension f64 {
    fun to_percent() -> string {
        return f"{self * 100}%"
    }
}

var taxa := 0.75
print(taxa.to_percent()) // Saída: "75%"
```

### 13.3. Trailing Closures (Sintaxe de DSL)

Se o último argumento de uma função for uma closure (função anônima), os parênteses podem ser omitidos. Isso habilita a criação de APIs declarativas elegantes.

```rust
// Sintaxe limpa para iteradores e builders
users.filter { u ->
    u.active == true
}.map { u ->
    u.email
}
```

---

## 14. Sumário de Progresso e Próximos Passos

### ✅ O que já temos (v0.7 COMPLETO):

1. **Compilador funcional completo:** Lexer → Parser → Codegen → Binário nativo
2. **Sistema de tipos robusto:** 7 tipos primitivos (int, float, string, matrix, intmatrix, floatptr, void) com casting automático inteligente
3. **Operadores matemáticos completos:** `+`, `-`, `*`, `/`, `%`, `**` (potência para int e float)
4. **Operadores bitwise:** `&`, `|`, `^` (apenas para inteiros)
5. **Operadores unários:** `!`, `not` (negação lógica), `-` (negação aritmética)
6. **Increment/Decrement:** `++x`, `x++`, `--x`, `x--` (pré e pós-fixo)
7. **Operador ternário:** `cond ? true_val : false_val` com promoção automática de tipos
8. **String interpolation:** `f"Valor: {x}"` com conversão automática de tipos
9. **Format specifiers:** `f"{pi:.2f}"`, `f"{num:x}"` (hex, octal, científica, precisão) ✅ **NOVO v0.6**
10. **Controle de fluxo:** If/Else, While, For (range e iteração)
11. **Chained comparisons:** `10 < x <= 20` (estilo Julia)
12. **Matrizes e Arrays:** Com indexação 2D e field access
13. **Strings:** Com concatenação, comparação e introspection
14. **Runtime C:** Funções de matriz e string otimizadas
15. **typeof():** Introspecção de tipos em compile-time
16. **print() e println():** Output simplificado com conversão automática de tipos
17. **Funções de conversão:** `int()`, `float()`, `string()`, `bool()` para conversão explícita entre tipos
18. **Import system:** `import math`, `import math as m` ✅ **NOVO v0.7**
19. **Math library:** 36 funções matemáticas (trig, stats, linalg) + 6 constantes ✅ **NOVO v0.7**

### 🎯 Próximo Passo: v0.8 - User Functions

**Decisão Arquitetural Aprovada:**

Sistema de módulos com zero-overhead usando bindings diretos para bibliotecas C (math.h, BLAS, LAPACK):

```brix
// Sintaxe de import
import math
import math as m

// Funções matemáticas (via C math.h)
math.sin(x), math.cos(x), math.sqrt(x), math.exp(x), math.log(x)
math.floor(x), math.ceil(x), math.round(x), math.abs(x)

// Álgebra linear (via LAPACK/BLAS)
math.det(A), math.tr(A), math.inv(A)
math.eigvals(A), math.eigvecs(A)

// Estatística
math.sum(arr), math.mean(arr), math.median(arr), math.std(arr)
```

**Características:**
- ✅ **Zero overhead runtime**: Chamadas diretas via LLVM external declarations
- ✅ **Performance nativa C**: Mesma velocidade de C puro (det 1000×1000 em ~50ms)
- ✅ **Battle-tested**: Usa código usado por NumPy, MATLAB, Julia, R
- ✅ **Namespace limpo**: Evita poluição global de funções

**Implementação:**
1. Parser: `Token::Import`, `Stmt::Import { module, alias }`
2. Symbol table: Namespaces por módulo
3. Codegen: LLVM external declarations
4. Runtime: Thin wrappers em runtime.c chamando math.h/LAPACK

### Próximas Features (v1.1+):

**v1.2 - Documentation & Modules:**
- Documentation system: `@doc` annotations
- User-defined modules: `module mymod { ... }`
- Advanced string functions: split(), join(), trim()

**v1.3 - Type System Expansion:** ✅ **COMPLETE (Feb 2026)**
- ✅ Closures: `var fn := (x: int) -> int { return x * 2 }` com capture by reference + ARC
- ✅ Structs: `struct Point { x: int; y: int }` com Go-style receivers e default values
- ✅ Generics: `function swap<T>(a: T, b: T) -> (T, T)` com monomorphization
- ✅ Error handling: Continua padrão Go (sem Result<T,E>)

**v1.4 - Advanced Type System:** ✅ **COMPLETE (Feb 2026)**
- ✅ Type Aliases: `type MyInt = int`, `type Point2D = Point`
- ✅ Union Types: `int | float | string` com tagged unions
- ✅ Intersection Types: `Point & Label` com struct merging
- ✅ Elvis Operator: `a ?: b` (null coalescing)
- ✅ Optional → Union: `int?` agora é `Union(int, nil)`

**v1.5 - Concurrency & Advanced Features:**
- Async/Await: State machine transformation
- Concurrency: `spawn`, async functions
- Test Library: Jest-style testing framework
- Iterators: map, filter, reduce, pipeline operator

**Qualidade (qualquer versão):**
- Testes de integração automatizados
- Mensagens de erro melhores (Ariadne)
- Otimizações LLVM (-O2, -O3)

### 📊 Estatísticas do Projeto:

- **Linhas de Código (Rust):** ~6000 linhas (compiler core + advanced type system + atoms + type checkers + string functions)
- **Linhas de Código (C Runtime):** ~1200 linhas (math + matrix + complex + LAPACK + error handling + atoms + string functions)
- **Arquivos de Teste (.bx):** 95+ (core + math + functions + pattern matching + complex + nil/error + atoms + type checking + strings + type system)
- **Tipos Implementados:** 17 (Int, Float, String, Matrix, IntMatrix, Complex, ComplexMatrix, FloatPtr, Void, Tuple, Nil, Error, Atom, Struct, Generic, Union, Intersection, TypeAlias, Closure)
- **Built-in Functions:** 60+ (I/O, type system, type checking, conversions, math, stats, linalg, complex, string operations)
- **Features Implementadas:** ~160+ (v1.5 100% completo ✅)
- **Features v1.5:** Test Library + Iterators + Pipeline + Ranges + Async/Await = 5 features principais
- **Features Planejadas v1.6+:** break/continue, String Library, Async Closures, Pattern Matching 2.0
- **Versão Atual:** v1.5 ✅ **COMPLETO (Fev 2026)** 🎉
- **Versão Anterior:** v1.4 ✅ **COMPLETO (18/02/2026)**
- **Progresso MVP:** 100%
- **Próxima Versão:** v1.6 (break/continue, String Library, Async Closures, Pattern Matching 2.0)
- **Última Atualização:** Fev 2026

---

### 🚧 Resumo v1.2.1 (Em Progresso - 06/02/2026)

A versão 1.2.1 está implementando error handling robusto com Result types no compilador:

**✅ Phase E1-E2: Core Error Infrastructure & Module Conversion (Completo):**
- `CodegenError` enum com 6 variantes de erro:
  - `LLVMError` - Falhas em operações LLVM
  - `TypeError` - Incompatibilidade de tipos
  - `UndefinedSymbol` - Variável/função não encontrada
  - `InvalidOperation` - Operação inválida (ex: range fora de for loop)
  - `MissingValue` - Valor ausente/compilação falhou
  - `General` - Erros gerais com mensagem
- `CodegenResult<T>` = `Result<T, CodegenError>` usado em toda pipeline
- **Módulos convertidos (~2000 linhas):**
  - `error.rs` (61 linhas) - Infraestrutura de erros
  - `expr.rs` (285 linhas) - Compilação de expressões com Result
  - `stmt.rs` (528 linhas) - Compilação de statements com Result (12 métodos)
  - `helpers.rs` (146 linhas) - LLVM helpers com error handling
  - `lib.rs` - Métodos principais (`compile_expr`, `compile_stmt`, `value_to_string`)
- **Todos os 1001 testes passando!** ✅
- Redução de ~595 → ~350-400 unwrap() calls

**🔲 Phase E3-E6: Próximos Passos:**
- E3: Converter funções auxiliares restantes (~350-400 unwrap() calls)
- E4: Integrar Ariadne para pretty error printing
- E5: Propagar erros até main.rs para mensagens user-friendly
- E6: Substituir todos eprintln!() por erros estruturados

**📊 Impacto até agora:**
- ~2000 linhas convertidas de Option/() para Result
- Error propagation com `?` operator
- Mensagens de erro descritivas em cada LLVM operation
- Base sólida para error reporting user-facing

---

### 🎯 Resumo v1.2 (Completo - 05/02/2026)

A versão 1.2 realizou uma grande refatoração do codegen para arquitetura modular:

**✅ Codegen Refactoring (Phase R - Completo):**
- Divisão do monólito lib.rs (7,338 linhas) em módulos especializados
- **Redução de 11.4% no tamanho** (7,338 → 6,499 linhas)
- **Novos módulos criados:**
  - `types.rs` (33 linhas) - BrixType enum
  - `helpers.rs` (146 linhas) - LLVM helper functions
  - `stmt.rs` (528 linhas) - Statement compilation (12 métodos)
  - `expr.rs` (285 linhas) - Expression compilation (4 métodos)
  - `builtins/` (357 linhas) - Built-in function declarations
    - `math.rs`, `stats.rs`, `linalg.rs`, `string.rs`, `io.rs`, `matrix.rs`
  - `operators.rs` - Annotations (refactoring postponed)
- **Pattern de organização:** Trait-based separation
- **1001/1001 testes passando durante toda refatoração** ✅

**✅ Bug Fixes & Improvements:**
- 8/10 bugs críticos resolvidos (ver FIX_BUGS.md)
- Ariadne integration - Beautiful error messages no parser
- Invalid operator sequence detection (`1 ++ 2`)
- Matrix arithmetic - 28 runtime functions
- IntMatrix → Matrix automatic promotion
- Postfix operation chaining (`.field`, `[index]`, `(args)`)
- Right-associative power operator (`2**3**2 = 512`)
- C-style bitwise precedence

**📊 Impacto:**
- Arquitetura mais limpa e manutenível
- Melhor separação de responsabilidades
- Base sólida para error handling (v1.2.1)
- Zero regressões - 100% backward compatible

---

### 🎯 Resumo v1.1 (Completo - 03/02/2026)

A versão 1.1 trouxe melhorias importantes em type checking e manipulação de strings:

**✅ Lexer String Fix:**
- Correção do regex para aceitar aspas escapadas em f-strings
- Mudança: aceita qualquer caractere escapado (`\\.`) ao invés de lista fixa
- Impacto: f-strings agora suportam `\"` corretamente

**✅ Type Checking Functions (10 funções):**
- `is_nil()` - Verifica valores nulos (runtime check para ponteiros)
- `is_atom()` - Verifica atoms
- `is_boolean()` - Valida se int é 0 ou 1
- `is_number()` - Detecta int ou float
- `is_integer()` - Detecta int
- `is_float()` - Detecta float
- `is_string()` - Detecta string
- `is_list()` - Detecta Matrix ou IntMatrix
- `is_tuple()` - Detecta tuples
- `is_function()` - Placeholder (sempre retorna 0)

**✅ String Functions (7 funções):**
- `uppercase()`, `lowercase()`, `capitalize()` - Transformações de caso
- `byte_size()` - Tamanho em bytes
- `length()` - Número de caracteres (UTF-8 aware)
- `replace()` - Substitui primeira ocorrência
- `replace_all()` - Substitui todas ocorrências

**📊 Impacto:**
- 18 novas features implementadas
- 3 novos arquivos de teste
- ~200 linhas adicionadas ao runtime.c
- ~2000 linhas adicionadas ao codegen
- 100% dos testes passando

**⏸️ Adiado para v1.2:**
- `split()` e `join()` (requerem tipo StringMatrix)

---

### Onde vamos começar? (Histórico - Jan 2024)

Como você escolheu **Rust**, nosso fluxo de trabalho muda um pouco. Em vez de escrever scripts soltos, vamos criar um projeto estruturado com `cargo`.

A arquitetura do seu compilador em Rust será mais ou menos assim:

1.  **Crate `lexer`**: Transforma texto em `Enum` (Tokens).
2.  **Crate `parser`**: Transforma Tokens em `Structs` (AST).
3.  **Crate `codegen`**: Transforma Structs em chamadas LLVM.

### O Escopo da Versão 0.1 (MVP)

Para não ficarmos paralisados tentando fazer tudo, vamos definir o que NÃO vai entrar na primeira versão:

- ❌ Sem Generics (`<T>`) agora: Vamos fazer funcionar só com `i64` e `f64` primeiro. Generics adicionam uma complexidade absurda no compilador.
- ❌ Sem Strings complexas: Vamos tratar strings apenas como arrays de bytes por enquanto. Nada de Regex ou manipulação Unicode avançada na v0.1.
- ❌ Sem Otimizador: O código gerado vai ser "feio" (não otimizado), mas vai funcionar. Deixamos o LLVM limpar a sujeira depois.
- Compilação baseada em **Arquivo Único** para o MVP.
- Suporte a múltiplos arquivos e imports será adicionado na v0.2.

---

## 15. AI-Native Features 🤖 (Planejado v2.0+)

**Data Engineering + AI Era**

Com o boom de RAG, LLMs e Vector Databases, Brix visa se tornar **a linguagem nativa para Data Engineering e AI**. As features abaixo aproveitarão a arquitetura existente (Matrix, BLAS/LAPACK, SIMD) para entregar performance brutal em workflows de AI.

---

### 15.1. Native Vector/Embedding Operations ⭐ (Mais Promissor)

**Motivação:**
- RAG e LLMs explodiram em 2024-2025
- Trabalhar com embeddings é crucial para semantic search, vector databases, similarity search
- Nenhuma linguagem tem embeddings como tipo de primeira classe
- Python é lento para isso (~10-100x), Rust é verbose demais

**Sintaxe Proposta:**

```brix
// Tipo nativo para embeddings (vetores de alta dimensão)
var embedding1 := embed[1536]([0.1, 0.2, ...])  // OpenAI ada-002 dimension
var embedding2 := embed[1536]([0.3, 0.4, ...])

// Operações built-in otimizadas (SIMD, AVX-512)
var similarity := embedding1 @ embedding2  // cosine similarity (operador @)
var distance := embedding1 <-> embedding2  // euclidean distance

// Batch operations (Fortran-level performance)
var batch := EmbeddingBatch(1000, 1536)  // 1000 embeddings de dimensão 1536
var top_k := batch.find_nearest(query, k=10)  // SIMD-optimized nearest neighbors
```

**Características:**
- ✅ **Tipo de primeira classe:** `Embedding[DIM]` com dimensão fixa
- ✅ **Operadores nativos:** `@` (cosine sim), `<->` (euclidean distance), `<=>` (dot product)
- ✅ **SIMD-optimized:** AVX-512, ARM NEON para performance brutal
- ✅ **Batch operations:** Processa milhares de embeddings em paralelo
- ✅ **Zero-copy:** Compatível com BLAS/LAPACK existente

**Performance esperada:**
- Cosine similarity: ~10-100x mais rápido que Python/NumPy
- Batch search (1M embeddings): Sub-segundo com SIMD
- Integração nativa com vector databases

**Por que é diferencial:**
- Nenhuma linguagem tem embeddings nativos
- Sinérgico com Data Engineering: Dados → Embeddings → Vector DB → Analytics
- Aproveita arquitetura existente: Matrix, BLAS/LAPACK, forte em numérico
- Timing perfeito: RAG é o futuro de LLMs

---

### 15.2. Native Vector Database Integration 🔥

**Motivação:**
- Brix já terá SQL nativo (planejado)
- Por que não ter Vector DB nativo também?
- Vector search é tão importante quanto SQL para AI/ML pipelines

**Sintaxe Proposta:**

```brix
// Conectar a vector databases (Pinecone, Weaviate, Milvus)
connect vectordb "pinecone://api-key@environment/index"

// Query semântica com sintaxe nativa
var results := query vectordb {
    similar_to: user_query_embedding,
    limit: 10,
    filter: { category: "docs", year: 2024 }
}

// Upsert de embeddings
vectordb.upsert([
    { id: "doc1", values: emb1, metadata: { title: "..." } },
    { id: "doc2", values: emb2, metadata: { title: "..." } }
])

// Hybrid search (vector + metadata filtering)
var hybrid := query vectordb {
    similar_to: query_emb,
    filter: { price: { $gt: 100, $lt: 500 } },
    limit: 20
}
```

**Características:**
- ✅ **Type-safe queries:** Compile-time validation de schemas
- ✅ **Zero-overhead bindings:** Chamadas diretas via LLVM (como math.h)
- ✅ **Multi-provider support:** Pinecone, Weaviate, Milvus, Chroma
- ✅ **Streaming results:** Lazy evaluation para datasets grandes
- ✅ **Built-in batching:** Otimiza automaticamente upserts em lote

**Performance esperada:**
- Latência de query: ~10-50ms (network-bound, mas sem overhead de Python)
- Batch upserts: 10,000+ vectors/segundo

**Por que é diferencial:**
- Mesma importância de SQL para AI/ML
- Sintaxe declarativa, type-safe
- Zero-overhead como SQL nativo
- First-class citizen ao lado de SQL

---

### 15.3. Native ONNX Runtime Integration

**Motivação:**
- Executar modelos de ML sem overhead de Python
- Latência 10-100x menor para inferência
- Essencial para edge computing, real-time AI

**Sintaxe Proposta:**

```brix
import onnx

// Carregar modelo ONNX
var model := onnx.load("model.onnx")

// Inferência (zero-copy, compiled code)
var input := [1.0, 2.0, 3.0]
var output := model.infer(input)

// Batch inference
var batch := [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]
var predictions := model.batch_infer(batch)  // Parallelized

// GPU support (futuro)
var gpu_model := onnx.load("model.onnx", device="cuda:0")
```

**Características:**
- ✅ **Zero-copy inference:** Dados passados diretamente via ponteiros
- ✅ **Multi-threading:** Batch inference paralelo automático
- ✅ **CPU optimizations:** AVX-512, ARM NEON
- ✅ **Type-safe:** Input/output shapes validados em compile-time

**Performance esperada:**
- Inferência single: 10-100x mais rápido que Python
- Batch inference: Near-linear scaling com threads

**Por que é diferencial:**
- Python é gargalo para inferência real-time
- Perfeito para edge computing
- Complementa embeddings nativos

---

### 15.4. Type-Safe Tensor Operations

**Motivação:**
- Expandir Matrix para Tensors N-dimensionais
- Type safety em compile-time (evitar shape mismatches)
- Essencial para Deep Learning pipelines

**Sintaxe Proposta:**

```brix
// Dimensões checadas em compile-time
var image := Tensor[28, 28, 3]  // Height, Width, Channels
var batch := Tensor[32, 28, 28, 3]  // Batch de 32 imagens

// Operações verificadas em tempo de compilação
var conv := batch.conv2d(kernel)  // Type error se dimensões incompatíveis

// Broadcasting automático (NumPy-style)
var normalized := (batch - mean) / std  // Broadcasting aplicado corretamente

// Reshape com type checking
var flattened := batch.reshape([32, 2352])  // 28*28*3 = 2352

// Error de compilação se shape inválido
// var invalid := batch.reshape([32, 1000])  // ❌ Error: Shape mismatch
```

**Características:**
- ✅ **Compile-time shape checking:** Zero runtime errors de shape mismatch
- ✅ **Automatic broadcasting:** Como NumPy, mas type-safe
- ✅ **SIMD-optimized:** Mesma performance de Matrix existente
- ✅ **Interop com Matrix:** Tensors são extensão de Matrix

**Performance esperada:**
- Mesma performance de Matrix (BLAS/LAPACK)
- Compile-time checking = zero overhead

**Por que é diferencial:**
- Python/NumPy: runtime errors frequentes
- TensorFlow/PyTorch: verbose, dynamic typing
- Brix: type-safe, compile-time validation

---

### 15.5. Built-in Prompt Engineering (Inovador!)

**Motivação:**
- LLMs dominam desenvolvimento de apps
- Prompt engineering é skill crítica
- Prompts são code, merecem type safety

**Sintaxe Proposta:**

```brix
// Templates type-safe para LLMs
template UserQuery {
    system: String,
    context: String[],  // Array de strings
    question: String,

    function render() -> String {
        return f"""
        System: {self.system}

        Context:
        {self.context.join("\n\n")}

        Question: {self.question}
        """
    }
}

// Uso type-safe
var prompt := UserQuery{
    system: "You are a helpful assistant",
    context: retrieved_docs,
    question: user_input
}

// Validação em compile-time
var rendered := prompt.render()

// LLM call (futuro)
var response := llm.generate(rendered, max_tokens=500)
```

**Características:**
- ✅ **Type-safe templates:** Compile-time validation de fields
- ✅ **Modular prompts:** Composição de templates
- ✅ **Versioning:** Prompts como código (Git, diff, review)
- ✅ **Testing:** Unit tests para prompt rendering

**Performance esperada:**
- Compile-time template validation
- Zero overhead vs string concatenation

**Por que é diferencial:**
- Prompts são code, merecem tooling
- Type safety evita erros de runtime
- Modular, testável, versionável

---

### 15.6. Recomendação: Combo Killer 🎯

**Se tivesse que escolher um diferencial killer para v2.0:**

1. **Embedding/Vector como tipo nativo com operações otimizadas (SIMD)**
2. **Vector Database integration no mesmo nível de SQL**
3. **Performance brutal (Fortran-level) para operações vetoriais**

**Por que isso seria revolucionário:**

✅ **Timing perfeito:** RAG e vector search explodiram em 2024-2025
✅ **Gap real:** Python é lento para isso, Rust é verbose demais
✅ **Sinérgico com Data Engineering:** Dados → Embeddings → Vector DB → Analytics
✅ **Aproveita arquitetura existente:** Matrix, BLAS/LAPACK, forte em numérico
✅ **Diferencial único:** Nenhuma linguagem tem isso nativo

**Marketing tagline:**
> "A linguagem nativa para RAG e Data Engineering"
> "Write embeddings like Python, execute like Fortran, scale like Go"

---

### Roadmap de Implementação (v2.0+)

**Phase 1: Embedding Type (v2.0):**
- `Embedding[DIM]` como novo tipo primitivo
- Operadores `@` (cosine), `<->` (euclidean), `<=>` (dot product)
- SIMD optimization (AVX-512, ARM NEON)
- Batch operations básicas

**Phase 2: Vector DB Integration (v2.1):**
- Bindings para Pinecone, Weaviate, Milvus
- Query syntax nativa
- Type-safe schemas
- Streaming results

**Phase 3: ONNX Runtime (v2.2):**
- Zero-copy inference
- Batch processing paralelo
- GPU support (CUDA, Metal)

**Phase 4: Advanced Features (v2.3+):**
- Type-safe Tensors
- Prompt engineering templates
- LLM integrations (OpenAI, Anthropic, local models)

---

### Performance Targets (Benchmarks futuros)

**Embedding Operations:**
- Cosine similarity (1M pairs): < 100ms (vs Python ~1-2s)
- Batch nearest neighbor (10k queries, 1M corpus): < 1s (vs Python ~10-30s)

**Vector DB:**
- Query latency: Network-bound + <5ms overhead (vs Python +50-100ms)
- Upsert throughput: 10,000+ vectors/sec (vs Python ~1,000/sec)

**ONNX Inference:**
- Single inference: <1ms (vs Python ~10-50ms)
- Batch inference (1000 samples): <100ms (vs Python ~1-5s)

---

### Conclusão

Essas features transformariam Brix em **THE language for AI-powered Data Engineering**:

- ✅ Zero-overhead native performance
- ✅ Type safety em toda pipeline
- ✅ Sinérgico com features existentes (Matrix, BLAS, SQL)
- ✅ Timing perfeito com boom de RAG/LLMs
- ✅ Diferencial competitivo único no mercado

**Status:** Planejado para v2.0+ (após v1.5+ - todas dependências de tipo atendidas)

**Prioridade:** Alta - Alinhado com tendências de mercado e filosofia da linguagem
