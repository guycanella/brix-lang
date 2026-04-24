---
name: brix-test
description: "Roda todas as suítes de teste do Brix (unit, integration, Test Library) e reporta resultado estruturado com tabela comparativa ao baseline do CLAUDE.md."
argument-hint: "[unit|integration|brix|pattern]"
allowed-tools: Read Grep Bash
model: sonnet
effort: medium
user-invocable: true
---

# Rodar e reportar testes Brix

**Filtro:** $ARGUMENTS (vazio = tudo, ou: `unit`, `integration`, `brix`, `<pattern>`)

## 1. Build

```bash
cargo build 2>&1
```
Se falhar, pare e reporte. Não faz sentido testar com build quebrado.

## 2. Rodar suítes

**Unit tests (3 crates) — rodar em paralelo:**
```bash
cargo test -p lexer 2>&1
cargo test -p parser 2>&1
cargo test -p codegen 2>&1
```

**Integration tests (sequencial!):**
```bash
cargo test --test integration_test -- --test-threads=1 2>&1
```

**Test Library:**
```bash
cargo run -- test 2>&1
```

Se `$ARGUMENTS` foi fornecido, rodar apenas a suíte correspondente.

## 3. Relatório

Apresentar tabela:

```
| Suíte            | Total | Passed | Failed |
|------------------|-------|--------|--------|
| Lexer (unit)     |       |        |        |
| Parser (unit)    |       |        |        |
| Codegen (unit)   |       |        |        |
| Integration      |       |        |        |
| Test Library     |       |        |        |
| **Total**        |       |        |        |
```

## 4. Comparar com baseline

Ler CLAUDE.md seção "Current test baseline" e comparar. Reportar divergências.

## 5. Se houver falhas

Para cada falha: nome do teste, output de erro, causa provável. NÃO corrija — apenas reporte.
