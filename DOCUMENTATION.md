# Brix Language (Design Document v1.0)

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

// Futuro (v0.8+): Para Textos
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

### Pattern Matching (Influência: Elixir/Rust)

Substitui `switch/case` complexos. Permite desestruturação.

```
when response {
    { status: 200, body: b } -> print("Sucesso: " + b),
    { status: 404 }          -> print("Não encontrado"),
    { status: s } if s > 500 -> print("Erro de servidor"),
    _                        -> print("Erro desconhecido")
}
```

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

### Retornos Múltiplos (Influência: Go)

Funções podem retornar múltiplos valores, facilitando o padrão "resultado, erro".

```
function divide(a: f64, b: f64) -> (f64, error) {
    if b == 0.0 {
        return 0.0, error("Divisão por zero")
    }
    return a / b, nil
}

// Uso
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

### 📊 Progresso Geral: v0.6 Completo (70% MVP Completo)

---

## ✅ IMPLEMENTADO (v0.1 - v0.3)

### 1. Arquitetura do Compilador

- ✅ **Workspace Cargo:** Separação em crates (`lexer`, `parser`, `codegen`)
- ✅ **Lexer (Logos):** Tokenização completa com comentários, operadores e literais
- ✅ **Parser (Chumsky):** Parser combinator com precedência de operadores correta
- ✅ **Codegen (Inkwell/LLVM 18):** Geração de LLVM IR e compilação nativa
- ✅ **Runtime C:** Biblioteca com funções de Matrix e String

### 2. Sistema de Tipos

- ✅ **Tipos Primitivos:** `int` (i64), `float` (f64), `bool` (i1→i64), `string` (struct), `matrix` (struct f64*), `intmatrix` (struct i64*), `void`
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

### 6. Funções Built-in

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

**Data Structures:**
- ✅ **matrix:** Construtor de matriz vazia (`matrix(rows, cols)`)
- ✅ **read_csv:** Lê arquivo CSV como matriz (via runtime C)

### 7. Memória e Performance

- ✅ **Tabela de Símbolos:** HashMap com `(PointerValue, BrixType)` para cada variável
- ✅ **Stack Allocation:** Variáveis alocadas via `alloca` no entry block
- ✅ **Heap (Runtime C):** Matrizes e Strings alocadas dinamicamente
- ✅ **Constant Folding:** LLVM otimiza constantes automaticamente (ex: `2 + 3` → `5`)

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

### 🔧 **v0.5 - Funções de Usuário**

**Core:**

- [ ] **Declaração de Funções:** `function soma(a: int, b: int) -> int { return a + b }`
- [ ] **Chamada de Funções:** `var resultado := soma(10, 20)`
- [ ] **Return Statement:** `return valor`
- [ ] **Funções Void:** Funções sem retorno
- [ ] **Escopo Local:** Variáveis dentro de funções (shadow variables externas)

**Avançado (v0.5.1):**

- [ ] **Retornos Múltiplos (Go Style):** `function divide(a, b) -> (float, error)`
- [ ] **Argumentos Opcionais:** `function greet(name: string = "World")`
- [ ] **Funções Variádicas:** `function sum(nums: ...int)`

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

**Motivação:** Brix é voltado para Engenharia, Física e Ciência de Dados. Precisamos de um sistema de módulos limpo e funções matemáticas performáticas que não reinventem a roda.

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

#### Funções Matemáticas Disponíveis

**Trigonométricas (via math.h):**
```brix
import math
math.sin(x), math.cos(x), math.tan(x)       // Funções trigonométricas
math.asin(x), math.acos(x), math.atan(x)    // Inversas trigonométricas
math.atan2(y, x)                             // Arco tangente de y/x (4 quadrantes)
math.sinh(x), math.cosh(x), math.tanh(x)    // Hiperbólicas
```

**Exponenciais e Logaritmos (via math.h):**
```brix
import math
math.exp(x)      // e^x
math.log(x)      // Logaritmo natural (base e)
math.log10(x)    // Logaritmo base 10
math.log2(x)     // Logaritmo base 2
```

**Raízes e Potências (via math.h):**
```brix
import math
math.sqrt(x)     // Raiz quadrada
math.cbrt(x)     // Raiz cúbica
math.pow(x, y)   // x elevado a y (alternativa ao operador **)
```

**Arredondamento (via math.h):**
```brix
import math
math.floor(x)    // Arredonda para baixo
math.ceil(x)     // Arredonda para cima
math.round(x)    // Arredonda para o inteiro mais próximo
math.trunc(x)    // Trunca parte decimal
```

**Valor Absoluto (via math.h):**
```brix
import math
math.abs(x)      // Valor absoluto (int ou float)
math.fabs(x)     // Valor absoluto float (equivalente)
```

**Álgebra Linear (via BLAS/LAPACK):**
```brix
import math

// Operações de matriz
math.det(A)       // Determinante (LAPACK dgetrf + diagonal product)
math.tr(A)        // Traço (soma da diagonal)
math.inv(A)       // Inversa de matriz (LAPACK dgetri)
math.transpose(A) // Transposta

// Autovalores e autovetores
math.eigvals(A)   // Autovalores (LAPACK dgeev)
math.eigvecs(A)   // Autovetores (LAPACK dgeev)

// Decomposições
math.lu(A)        // Decomposição LU
math.qr(A)        // Decomposição QR
math.svd(A)       // Singular Value Decomposition
```

**Estatística (implementação custom ou GSL):**
```brix
import math
math.sum(arr)     // Soma de elementos
math.mean(arr)    // Média aritmética
math.median(arr)  // Mediana
math.std(arr)     // Desvio padrão
math.var(arr)     // Variância
math.min(a, b, ...)  // Mínimo de N valores
math.max(a, b, ...)  // Máximo de N valores
```

#### Números Complexos (Planejado para v0.8+)

**Motivação:** Física, Engenharia Elétrica, Processamento de Sinais, Análise de Fourier.

**Sintaxe proposta:**
```brix
// Literal complexo usando 'im' (imaginary unit)
var z := 1 + 2im
var w := 3.5 - 1.2im

// Funções via import math
import math
var r := math.real(z)      // Parte real
var i := math.imag(z)      // Parte imaginária
var conj := math.conj(z)   // Conjugado
var mag := math.abs(z)     // Magnitude
var phase := math.angle(z) // Fase

// Aritmética nativa
var soma := z + w          // Operadores suportam complex
var produto := z * w
```

**Decisão de Implementação:**
- Tipo nativo `BrixType::Complex` com struct LLVM { f64 real, f64 imag }
- Operadores aritméticos suportam complex numbers
- Funções complexas disponíveis via `import math`
- Implementação usando C complex.h (C99) quando disponível
- Performance: SIMD-friendly (2 floats = 16 bytes, cabe em registradores)

**Prioridade:** Após sistema de imports estar consolidado (v0.8+)

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

### 🎭 **v0.10 - Pattern Matching e Programação Funcional**

#### Pattern Matching

**Substituir switch/case complexos:**

- [ ] **Match Básico:**
  ```brix
  when response {
      { status: 200 } -> print("OK"),
      { status: 404 } -> print("Not Found"),
      _ -> print("Other")
  }
  ```
- [ ] **Guards (Condições):** `{ status: s } if s > 500 -> ...`
- [ ] **Desestruturação:** Extrair campos de structs no match

#### Programação Funcional

**Iteradores:**

- [ ] **map:** `nums.map(x -> x * 2)`
- [ ] **filter:** `nums.filter(x -> x > 10)`
- [ ] **reduce:** `nums.reduce(0, (acc, x) -> acc + x)`
- [ ] **Lazy Evaluation:** Não processar até consumir resultado

**List Comprehension:**

- [ ] **Básico:** `[x * 2 for x in nums]`
- [ ] **Com Filtro:** `[x for x in nums if x > 10]`
- [ ] **Matrix Comprehension:** `[[i + j for j in 1:n] for i in 1:m]`

**Pipeline Operator (`|>`):**

- [ ] **Encadeamento Funcional:**
  ```brix
  dados |> filter(x -> x > 0) |> map(x -> x * 2) |> sum()
  ```

---

### 📚 **v1.0 - Standard Library (Stdlib)**

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

### 🚀 **v1.1 - Concorrência e Paralelismo**

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
v0.5 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Funções de usuário, return
v0.6 ████████████████████ 100% ✅ IntMatrix type system, format specifiers
v0.7 ░░░░░░░░░░░░░░░░░░░░   0% 🎯 Import system, math library (C bindings)
v0.8 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Complex numbers, multi-file support
v0.9 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Functions, structs, pattern matching
v1.0 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Standard Library completa
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

### ✅ O que já temos (v0.6 COMPLETO):

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

### 🎯 Próximo Passo: v0.7 - Sistema de Imports e Biblioteca Matemática

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

### Alternativas Futuras (v0.8+):

**v0.8 - Números Complexos:**
- Sintaxe: `z := 1 + 2im`
- Funções: `math.real(z)`, `math.imag(z)`, `math.conj(z)`, `math.abs(z)`
- Implementação usando C complex.h

**v0.9 - Funções de Usuário:**
- Definição: `fn nome(params) -> tipo { body }`
- Return values, múltiplos retornos Go-style
- Closures, recursão

**Qualidade (qualquer versão):**
- Testes de integração automatizados
- Mensagens de erro melhores (Ariadne)
- Otimizações LLVM (-O2, -O3)

### 📊 Estatísticas do Projeto:

- **Linhas de Código (Rust):** ~3700 linhas
- **Linhas de Código (C Runtime):** ~125 linhas
- **Arquivos de Teste (.bx):** 15 (types, for, logic, chain, string, arrays, csv, bitwise, ternary, negation, increment, fstring, print, conversion, format)
- **Features Implementadas:** ~55 (v0.6 completo)
- **Features Planejadas:** ~120+
- **Versão Atual:** v0.6 (70% MVP)
- **Progresso MVP:** 62%
- **Versão Atual:** v0.4+ (Operadores Avançados + Type System) ✅ COMPLETO

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
