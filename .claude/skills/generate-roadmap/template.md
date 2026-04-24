# Template de Roadmap Brix

Use exatamente esta estrutura (em português) ao gerar um novo roadmap:

```markdown
# Brix vX.Y — Roadmap de Implementação

**Status:** Planejado (próximo após vX.Y-1)

**Tema:** [Descrição curta do foco]

**Ordem de Implementação:** [Feature A → Feature B → Feature C ←→ Feature D]

---

## Contexto: O que vX.Y-1 entregou

| Feature | Status | Testes |
|---------|--------|--------|
| [Feature anterior] | ✅ Completa | +X unit, +Y integration |
| **Total acumulado vX.Y-1** | | **U unit + I integration + T Test Library = TOTAL** |

---

## Visão Geral vX.Y

N grupos de features organizados por dependência:

| Grupo | Feature | Impacto | Risco |
|-------|---------|---------|-------|
| **A** | [Nome] | Alto | Baixo |
| **B** | [Nome] | Médio | Médio |

---

## Grupo A — [Nome]

### Motivação

[Por que é importante. O que desbloqueia.]

### Sintaxe

\`\`\`brix
[Código mostrando uso da feature]
\`\`\`

### Estrutura C em `runtime.c`

\`\`\`c
typedef struct {
    long ref_count;
    // campos
} BrixNome;

// Assinaturas
\`\`\`

### Arquivos e mudanças

**`crates/codegen/src/types.rs`** — [mudança]
**`runtime.c`** — [mudança]
**`crates/codegen/src/lib.rs`** — [mudança]

### Testes

- +N codegen unit tests (`test_nome_a`, `test_nome_b`)
- +M integration tests: `NNN_nome`, `NNN+1_nome2`
- +K Test Library tests em `arquivo.test.bx`

---

[Repetir Grupo B, C, D...]

---

## Resumo por Arquivo

| Arquivo | A | B | C | D |
|---------|---|---|---|---|
| `runtime.c` | ✎ | ✎ | — | — |
| `crates/codegen/src/lib.rs` | ✎ | ✎ | ✎ | ✎ |

---

## Metas de Teste

| Grupo | Unit Tests | Integration Tests | Test Library |
|-------|-----------|-------------------|--------------|
| A | +N | +M | +K |
| **Total vX.Y** | **+NN** | **+MM** | **+KK** |

**Baseline pós-vX.Y:** ~U unit + ~I integration + ~T Test Library = ~TOTAL testes

---

## Ordem Recomendada de Execução

\`\`\`
A (Nome)
    ↓
B (Nome)   ←→   C (Nome)   [paralelos, independentes]
    ↓
D (Nome)
\`\`\`

## Verificação por Grupo

\`\`\`bash
cargo build
cargo test -p lexer && cargo test -p parser && cargo test -p codegen
cargo test --test integration_test -- --test-threads=1
cargo run -- test
\`\`\`

---

## Fora do Escopo vX.Y (planejado vX.Y+1)

- [Features adiadas]
```
