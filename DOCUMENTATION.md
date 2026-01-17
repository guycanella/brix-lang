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

### Modelo de Memória: ARC (Automatic Reference Counting)

Optamos por **ARC** em vez de Garbage Collection (GC) ou Gerenciamento Manual.

- **Motivo:** Garante performance determinística (sem pausas aleatórias do "lixeiro") e segurança de memória.
- **Funcionamento:** O compilador insere incrementos/decrementos de contadores de referência automaticamente. Quando a referência chega a zero, a memória é liberada imediatamente.
- **Otimização:** Loops críticos de processamento de dados (hot paths) não sofrem penalidade, pois a checagem ocorre fora do loop.

### Passagem de Parâmetros

Sistema híbrido focado em performance e segurança.

- **Tipos Primitivos (int, float, bool):** Passagem por **Valor (Cópia)**.
  - _Custo:_ Irrisório (registradores de CPU).
- **Tipos Complexos (Arrays, Structs):** Passagem por **Referência Imutável (View)**.
  - _Padrão:_ A função recebe um ponteiro para os dados originais (custo zero de cópia), mas não pode alterá-los.
  - _Mutabilidade:_ Para alterar os dados originais, o parâmetro deve ser explicitamente marcado (ex: `fn process(mut dados: [int])`).

## 10. Status do Desenvolvimento

### O que já foi construído?

1. **Arquitetura de Workspace:**

- Separação clara em crates: `lexer`, `parser`, `codegen` (LLVM).
- Gerenciamento de dependências otimizado no `Cargo.toml` raiz.

2. **Lexer (Tokenizador):**

- Implementado com `Logos`.
- Suporte a comentários (`//`), operadores matemáticos completos (incluindo `**` e `%`), bitwise (`&`, `|`, `^`) e blocos (`{`, `}`).

3. **Parser (Análise Sintática):**

- Implementado com `Chumsky`.
- **Precedência de Operadores:** Hierarquia correta (Átomo -> Potência -> Multiplicação -> Soma -> Bitwise -> Comparação).
- **Estruturas:** Declarações, Atribuições, Blocos de Escopo, If/Else e Arrays.

4. **Codegen (LLVM Backend):**

- **Engine:** LLVM 18 via `inkwell`.
- **Memória:** Sistema de Tabela de Símbolos (`HashMap`) para alocação de variáveis na Stack (`alloca`, `store`, `load`).
- **Fluxo de Controle:** Implementação completa de `If / Else` com Basic Blocks e Conditional Branching.
- **Arrays:** Suporte a criação de Arrays literais e acesso via índice (`x[0]`) usando `GetElementPtr` (GEP).
- **Otimização:** Constant Folding automático (o LLVM pré-calcula constantes matemáticas).

### Próximos passos

1. **Loops:** Implementar `while` and `for` (essencial para Brix ser Turing complete)
2. **Executável Real:** Transformar o LLVM IR (`.ll`) em um binário executável (`.o` -> Linked -> Executável final)
3. **Tipagem de Floats:** Expandir o Codegen (atualmente apenas inteiros) para suportar operações com ponto flutuante (`f64`)
4. **CLI:** Melhorar a interface de linha de comando para aceitar arquivos (`brix run main.bx`)

### Onde vamos começar?

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
