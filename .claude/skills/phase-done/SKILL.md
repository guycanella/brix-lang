---
name: phase-done
description: "Finaliza uma fase de implementação: roda testes, atualiza CLAUDE.md (line counts, test counts, feature status), e prepara commits seguindo o padrão do projeto."
argument-hint: "[descricao da fase]"
allowed-tools: Read Edit Grep Glob Bash
model: sonnet
effort: high
user-invocable: true
---

# Completar fase de implementação

**Fase:** $ARGUMENTS

## 1. Todos os testes devem passar

```bash
cargo build
cargo test -p lexer -p parser -p codegen
cargo test --test integration_test -- --test-threads=1
cargo run -- test
```

Se algum falhar, **PARE** e reporte. Não finalize com testes quebrando.

## 2. Coletar métricas atuais

Em paralelo:

```bash
wc -l crates/codegen/src/lib.rs crates/codegen/src/stmt.rs crates/codegen/src/expr.rs crates/parser/src/parser.rs runtime.c src/main.rs
```

```bash
cargo test -p lexer 2>&1 | grep "test result"
cargo test -p parser 2>&1 | grep "test result"
cargo test -p codegen 2>&1 | grep "test result"
cargo test --test integration_test -- --test-threads=1 2>&1 | grep "test result"
```

```bash
ls tests/brix/*.test.bx | wc -l
cargo run -- test 2>&1 | tail -5
```

## 3. Atualizar CLAUDE.md

Editar as seguintes seções com valores reais:

**Workspace Structure** — line counts:
- `lib.rs`: `(~XX,XXX lines)`
- `stmt.rs`: `(~X,XXX lines)`
- `runtime.c`: `(~X,XXX lines)`
- `parser.rs`: `(~X,XXX lines)`

**Status & Limitations** — adicionar feature no padrão:
```
- Feature (Fase X): Descrição. Implementado em `arquivo.rs` + `runtime.c`. Integration tests NNN–MMM; +N codegen unit tests; +M Test Library tests.
```

**Current test baseline** — atualizar totais.

## 4. Apresentar resumo e aguardar aprovação para commit

Mostrar: diff resumido + mensagem de commit sugerida.

Padrão de commits do projeto (observado no git log):
```
git commit -m "Phase X completed"       # código
git commit -m "update documentation"    # CLAUDE.md separado
```

Só commitar após aprovação explícita.
