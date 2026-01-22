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

### 📊 Progresso Geral: v0.3 → v0.4 (53% MVP Completo)

---

## ✅ IMPLEMENTADO (v0.1 - v0.3)

### 1. Arquitetura do Compilador

- ✅ **Workspace Cargo:** Separação em crates (`lexer`, `parser`, `codegen`)
- ✅ **Lexer (Logos):** Tokenização completa com comentários, operadores e literais
- ✅ **Parser (Chumsky):** Parser combinator com precedência de operadores correta
- ✅ **Codegen (Inkwell/LLVM 18):** Geração de LLVM IR e compilação nativa
- ✅ **Runtime C:** Biblioteca com funções de Matrix e String

### 2. Sistema de Tipos

- ✅ **Tipos Primitivos:** `int` (i64), `float` (f64), `bool` (i1→i64), `string` (struct), `matrix` (struct), `void`
- ✅ **Inferência de Tipos:** `var x := 10` detecta automaticamente o tipo
- ✅ **Tipagem Explícita:** `var x: float = 10`
- ✅ **Casting Automático:**
  - `var x: int = 99.9` → trunca para 99 (float→int)
  - `var y: float = 50` → promove para 50.0 (int→float)
  - Promoção automática em operações mistas (int + float → float)
- ✅ **Introspecção:** `typeof(x)` retorna string do tipo em compile-time

### 3. Estruturas de Dados

- ✅ **Arrays Literais:** `var v := [10, 20, 30]` (implementado como Matrix 1xN)
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

- ✅ **printf:** Saída formatada estilo C (`printf("x: %d", x)`)
- ✅ **scanf/input:** Entrada tipada (`input("int")`, `input("float")`, `input("string")`)
- ✅ **typeof:** Retorna tipo como string
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

### 🎯 **v0.4 - Operadores e Expressões Avançadas** (Em Andamento)

**Prioridade Alta:**

- [ ] **Increment/Decrement:** `x++`, `x--`, `++x`, `--x`
- [x] **Bitwise Operators:** `&`, `|`, `^` ✅ **IMPLEMENTADO**
- [x] **Operador Ternário:** `cond ? true_val : false_val` ✅ **IMPLEMENTADO**
- [x] **Negação Lógica:** `!condition` ou `not condition` ✅ **IMPLEMENTADO**
- [ ] **Elvis Operator:** `val ?: default` (para null coalescing futuro)
- [ ] **Operador de Potência para Floats:** Atualmente `**` só funciona para int

**Açúcar Sintático:**

- [ ] **String Interpolation:** `f"Valor: {x}"` ou `"Valor: ${x}"`

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

### 📦 **v0.6 - Arrays Avançados e Slicing**

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

### 🗂️ **v0.7 - Structs e Tipos Customizados**

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

### 🎭 **v0.8 - Pattern Matching**

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

---

### 🔁 **v0.9 - Programação Funcional**

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
v0.4 █████████░░░░░░░░░░░  45% 🚧 Bitwise + Ternário + Negação (3/7 features)
v0.5 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Funções de usuário, return
v0.6 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Slicing, broadcasting
v0.7 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Structs, tipos customizados
v0.8 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Pattern matching
v0.9 ░░░░░░░░░░░░░░░░░░░░   0% 📋 Programação funcional
v1.0 ░░░░░░░░░░░░░░░░░░░░   0% 🎯 Standard Library completa
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

### ✅ O que já temos (v0.3 → v0.4):

1. **Compilador funcional completo:** Lexer → Parser → Codegen → Binário nativo
2. **Sistema de tipos robusto:** 6 tipos primitivos com casting automático inteligente
3. **Operadores matemáticos completos:** Incluindo potência, módulo, chained comparison
4. **Operadores bitwise:** `&`, `|`, `^` (apenas para inteiros)
5. **Operadores unários:** `!`, `not` (negação lógica), `-` (negação aritmética)
6. **Operador ternário:** `cond ? true_val : false_val` com promoção automática de tipos
7. **Controle de fluxo:** If/Else, While, For (range e iteração)
8. **Matrizes e Arrays:** Com indexação 2D e field access
9. **Strings:** Com concatenação, comparação e introspection
10. **Runtime C:** Funções de matriz e string otimizadas
11. **typeof():** Introspecção de tipos em compile-time

### 🎯 Próximos Passos Imediatos (v0.4):

**Prioridade 1:**

1. **String Interpolation:** `f"Valor: {x}"` via transformação do parser
2. **Increment/Decrement:** `x++`, `--x`, etc

**Prioridade 2:**

3. **Elvis Operator:** `val ?: default`
4. **Operador de Potência para Floats:** Atualmente `**` só funciona para int
5. **Testes de Integração:** Suite de testes automatizados para todas as features

**Prioridade 3 (Semana 3):**

8. **Mensagens de Erro Melhores:** Error reporting com Ariadne (já é dependência)
9. **Otimizações LLVM:** Habilitar `-O2` e `-O3` via flag CLI
10. **Documentação:** README completo com exemplos

### 📊 Estatísticas do Projeto:

- **Linhas de Código (Rust):** ~2700 linhas
- **Linhas de Código (C Runtime):** ~125 linhas
- **Arquivos de Teste (.bx):** 10 (types, for, logic, chain, string, arrays, csv, bitwise, ternary, negation)
- **Features Implementadas:** ~40
- **Features Planejadas:** ~120+
- **Progresso MVP:** 53%

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
