# Brix Language (Design Document v1.0)

> ⚠️ **Status do Projeto (Fev 2026):** O compilador Brix está em desenvolvimento ativo (v1.2.1). Core funcional com sistema de error handling robusto - 1001/1001 testes passando (100%). Ariadne integration completa para parser e codegen, com mensagens de erro lindas e contextuais para o usuário final.

## Status Atual (Fevereiro 2026)

### ✅ **Funcionalidades Implementadas (v1.0-v1.2):**
- Compilação completa `.bx` → binário nativo via LLVM
- 14 tipos core (Int, Float, String, Matrix, IntMatrix, Complex, ComplexMatrix, Atom, Nil, Error, etc.)
- Operadores completos (aritméticos, lógicos, bitwise, power operator `**`)
- Funções definidas pelo usuário com múltiplos retornos
- Pattern matching com guards
- List comprehensions
- Import system (zero-overhead)
- 38 funções matemáticas (math module)
- Integração LAPACK (eigvals, eigvecs)
- Atoms estilo Elixir (`:ok`, `:error`)
- F-strings com format specifiers
- Ariadne error reporting (parser)

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
  - ✅ **1001/1001 testes passando** (Lexer: 292, Parser: 150, Codegen: 559)
  - ✅ **Phase E COMPLETE!** 🎉

### 🔮 **Planejado (v1.3+):**
- Generics
- Structs com métodos
- Result<T,E> type
- Closures
- Concurrency (goroutines-style)

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

##### b) Funções zeros() e izeros()

Para clareza semântica entre Engenharia (Floats) e Matemática Discreta (Ints):

```brix
// Matrizes Float (f64) - padrão para engenharia/matemática
var m1 := zeros(5)        // Array 1D de 5 floats
var m2 := zeros(3, 4)     // Matriz 3x4 de floats

// Matrizes Int (i64) - para dados discretos/índices
var i1 := izeros(5)       // Array 1D de 5 ints
var i2 := izeros(3, 4)    // Matriz 3x4 de ints
```

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

### Loops (Híbrido C/Go/Java)

```
// Clássico
for (var i = 0; i < 10; i++) { ... }

// Iterator (Range based)
for (num: numbers) { ... }

// Go Style (Index + Value)
for i, val := range numbers { ... }
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

**Futuro (v1.0+):**
- [ ] **Error Type:** `function divide(a, b) -> (float, error)` (requer null safety)
- [ ] **Funções Variádicas:** `function sum(nums: ...int)`
- [ ] **Closures:** `var fn := (x: int) -> int { return x * 2 }`
- [ ] **First-class functions:** Passar funções como parâmetros

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
- [ ] Closures and lambda functions ⏸️ **Adiado para v1.2**
- [ ] First-class functions ⏸️ **Adiado para v1.2**
- [ ] User-defined modules ⏸️ **Adiado para v1.2**

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

### ⏸️ **v1.2 - Closures e Funções Avançadas** (ADIADO - Após Testes)

**NOTA:** Esta versão foi adiada para priorizar infraestrutura de testes.

#### Closures e Lambda Functions (planejado)

- [ ] **Closures básicas:** `var double := (x) -> x * 2`
- [ ] **Capture de variáveis:** Acesso a variáveis do escopo externo
- [ ] **First-class functions:** Passar funções como argumentos
- [ ] **Higher-order functions:** Funções que retornam funções

#### User-Defined Modules (planejado)

- [ ] **Sintaxe de módulo:** `module mymod { ... }`
- [ ] **Export/import:** `export function foo()`, `import mymod`
- [ ] **Multi-file compilation**

---

### 🔧 **v1.3 - Programação Funcional Avançada** (ADIADO)

**Iteradores:**

- [ ] **map:** `nums.map(x -> x * 2)`
- [ ] **filter:** `nums.filter(x -> x > 10)`
- [ ] **reduce:** `nums.reduce(0, (acc, x) -> acc + x)`
- [ ] **Lazy Evaluation:** Não processar até consumir resultado

**List Comprehension Avançada:**

- [x] **Básico:** `[x * 2 for x in nums]` ✅ **v0.9 IMPLEMENTADO**
- [x] **Com Filtro:** `[x for x in nums if x > 10]` ✅ **v0.9 IMPLEMENTADO**
- [x] **Nested Loops:** `[x * y for x in a for y in b]` ✅ **v0.9 IMPLEMENTADO**
- [x] **Com Destructuring:** `[x + y for x, y in zip(a, b)]` ✅ **v0.9 IMPLEMENTADO**
- [ ] **Matrix Comprehension 2D:** `[[i + j for j in 1:n] for i in 1:m]`

**Pipeline Operator (`|>`):**

- [ ] **Encadeamento Funcional:**
  ```brix
  dados |> filter(x -> x > 0) |> map(x -> x * 2) |> sum()
  ```

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

### 🚀 **v1.3 - Concorrência e Paralelismo**

**Paralelismo de Dados:**

- [ ] **par for:** Distribui iterações entre threads automaticamente
- [ ] **par map:** Map paralelo sobre arrays
- [ ] **Threads Nativas:** `spawn { ... }` (estilo Go)

**I/O Assíncrono:**

- [ ] **Non-blocking I/O:** Para servidores HTTP de alta performance
- [ ] **async/await:** Modelo de programação assíncrona (opcional)

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
TESTES ██░░░░░░░░░░░░░░░░  10% 🚧 Testing Infrastructure (~1,520 tests) ← EM ANDAMENTO
v1.2 ░░░░░░░░░░░░░░░░░░░░   0% ⏸️ Closures, modules (ADIADO - Após testes)
v1.3 ░░░░░░░░░░░░░░░░░░░░   0% ⏸️ Generics, Result<T,E>, Structs (ADIADO)
v1.4 ░░░░░░░░░░░░░░░░░░░░   0% ⏸️ Concurrency, stdlib, optimizations (ADIADO)
```

**Legenda:**
- ✅ Completo
- 🚧 Em desenvolvimento
- 📋 Planejado
- 🎯 Meta principal

---

## 12. Diferenciais Competitivos (The "Killer Features")

Para destacar o Brix no cenário atual, a linguagem adota três pilares de inovação que resolvem dores latentes de Engenharia de Dados e Backend.

### 12.1. Pipeline First (`|>`)

Inspirado em Elixir e F#, mas focado em processamento de dados massivos. O operador pipe transforma código aninhado complexo em um fluxo linear de leitura natural.

- **Conceito:** O resultado da expressão à esquerda é passado como o _primeiro argumento_ da função à direita.
- **Paralelismo Implícito:** O compilador é capaz de otimizar cadeias de pipes, injetando paralelismo automaticamente em operações como `map` ou `filter` (via `par`).

```rust
// O "Jeito Brix" de processar dados
"vendas_2024.csv"
    |> io::read_csv()               // Carrega
    |> par map(x -> x.total * 1.1)  // Ajusta preços (em todas as threads)
    |> filter(x -> x.total > 100)   // Filtra relevantes
    |> json::serialize()            // Transforma
    |> http::post("api/vendas")     // Envia
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

**v1.1 - Closures & Modules:**
- Closures: `var fn := (x: int) -> int { return x * 2 }`
- First-class functions: Passar funções como parâmetros
- User-defined modules: `module mymod { ... }`

**v1.2 - Generics & Concurrency:**
- Generics: `function map<T, U>(arr: [T], fn: T -> U) -> [U]`
- Concurrency: `spawn`, `par for`, `par map`
- Channels para comunicação entre threads

**Qualidade (qualquer versão):**
- Testes de integração automatizados
- Mensagens de erro melhores (Ariadne)
- Otimizações LLVM (-O2, -O3)

### 📊 Estatísticas do Projeto:

- **Linhas de Código (Rust):** ~5600 linhas (compiler core + atoms + type checkers + string functions)
- **Linhas de Código (C Runtime):** ~1200 linhas (math + matrix + complex + LAPACK + error handling + atoms + string functions)
- **Arquivos de Teste (.bx):** 49+ (core + math + functions + pattern matching + complex + nil/error + atoms + type checking + strings)
- **Tipos Implementados:** 14 (Int, Float, String, Matrix, IntMatrix, Complex, ComplexMatrix, FloatPtr, Void, Tuple, Nil, Error, Atom)
- **Built-in Functions:** 60+ (I/O, type system, type checking, conversions, math, stats, linalg, complex, string operations)
- **Features Implementadas:** ~118 (v1.1 100% completo ✅)
- **Features v1.1:** Lexer fix + 10 type checkers + 7 string functions + atoms + escape sequences = 18 features
- **Features Planejadas v1.2+:** ~150+
- **Versão Atual:** v1.2.1 🚧 **EM PROGRESSO (06/02/2026)**
- **Versão Anterior:** v1.2 ✅ **COMPLETO (05/02/2026)**
- **Progresso MVP:** 99.9%
- **Próxima Versão:** v1.3 (generics, structs, closures)
- **Última Atualização:** 06/02/2026

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

**Status:** Planejado para v2.0+ (após v1.3 - Generics, Structs, Closures)

**Prioridade:** Alta - Alinhado com tendências de mercado e filosofia da linguagem
