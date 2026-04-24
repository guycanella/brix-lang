---
name: implement
description: "Implementa um grupo/fase de um ROADMAP Brix. Lê o roadmap, apresenta plano, aguarda aprovação, implementa na ordem correta (runtime.c → types.rs → builtins → lib.rs → parser → testes), e verifica."
argument-hint: "[versao] [grupo]"
arguments: [versao, grupo]
allowed-tools: Read Edit Write Grep Glob Bash
model: opus
effort: high
user-invocable: true
---

# Implementar feature do roadmap Brix

**Versão:** $versao | **Grupo:** $grupo

## 1. Ler o Roadmap

Leia `ROADMAP_V$versao.md` (sem o ponto, ex: v1.7 → ROADMAP_V1.7.md). Localize o **Grupo $grupo** e extraia:
- Motivação
- Sintaxe (exemplos de uso)
- Funções C (assinaturas para `runtime.c`)
- Arquivos e mudanças (quais arquivos e onde)
- Testes (quantos e quais tipos)

## 2. Apresentar plano e aguardar aprovação

Antes de escrever qualquer código, apresente um plano com:
- Arquivos a modificar e em que ordem
- Funções C a adicionar
- Tipos novos (se houver)
- Pontos de dispatch em lib.rs
- Testes a criar

**Aguarde aprovação explícita do usuário antes de codificar.**

## 3. Ordem de implementação

Sempre seguir esta ordem — camada mais baixa primeiro:

```
1. runtime.c          → Structs C + new/retain/release + funções
2. types.rs           → Novo BrixType (se houver)
3. builtins/*.rs      → Declarações externas
4. lib.rs             → Dispatch + compile_* methods
5. parser (se necessário) → ast.rs → parser.rs
6. stmt.rs (se necessário) → Compilação de statements
```

Consulte [implementation-patterns.md](implementation-patterns.md) para os padrões exatos de cada camada.

## 4. Implementar testes nas 3 camadas

**Codegen unit tests** em `crates/codegen/src/tests/`:
- Usar `Expr::dummy()` e `Stmt::dummy()` para construir AST em testes

**Integration tests** em `tests/integration/success/`:
- Numerar sequencialmente: verificar último com `ls tests/integration/success/ | sort -n | tail -1`
- Criar `.bx` + `.expected` + adicionar `#[test]` em `tests/integration_test.rs`

**Test Library** em `tests/brix/*.test.bx`:
```brix
import test
test.describe("Feature", () -> {
    test.it("behavior", () -> {
        test.expect(actual).toBe(expected)
    })
})
```

## 5. Verificar

```bash
cargo build
cargo test -p lexer -p parser -p codegen
cargo test --test integration_test -- --test-threads=1
cargo run -- test
```

Se algo falhar, investigar e corrigir antes de reportar.

## 6. Resumo final

Reportar: arquivos modificados, testes adicionados (unit + integration + Test Library), linhas adicionadas, TODOs pendentes.
