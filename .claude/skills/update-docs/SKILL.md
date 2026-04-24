---
name: update-docs
description: "Coleta métricas atuais do código (wc -l, test counts, file counts) e atualiza CLAUDE.md automaticamente com os valores reais."
allowed-tools: Read Edit Grep Glob Bash
model: sonnet
effort: medium
user-invocable: true
---

# Atualizar CLAUDE.md com estado atual

## 1. Coletar métricas

**Linhas dos arquivos principais:**
```bash
wc -l crates/codegen/src/lib.rs crates/codegen/src/stmt.rs crates/codegen/src/expr.rs crates/parser/src/parser.rs runtime.c src/main.rs
```

**Contagem de testes:**
```bash
cargo test -p lexer 2>&1 | grep "test result"
cargo test -p parser 2>&1 | grep "test result"
cargo test -p codegen 2>&1 | grep "test result"
cargo test --test integration_test -- --test-threads=1 2>&1 | grep "test result"
```

**Test Library e integration:**
```bash
ls tests/brix/*.test.bx | wc -l
cargo run -- test 2>&1 | tail -3
ls tests/integration/success/*.bx | wc -l
```

## 2. Atualizar CLAUDE.md

Pontos de atualização (ordem de aparição):
1. `lib.rs` → `(~XX,XXX lines)`
2. `stmt.rs` → `(~X,XXX lines)`
3. `runtime.c` → `(~X,XXX lines)`
4. `parser.rs` → `(~X,XXX lines)`
5. Test Library file count → `XX files, XXX tests`
6. Test baseline → `X,XXX unit + XXX integration + XXX Test Library`
7. BrixType variants count → `XX core types`

## 3. Reportar diff

```
Atualizações:
- lib.rs: ANTES → DEPOIS (+N)
- Unit tests: ANTES → DEPOIS (+N)
- ...
```
