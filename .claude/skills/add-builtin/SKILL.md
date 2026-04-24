---
name: add-builtin
description: "Workflow guiado para adicionar uma nova função built-in ao Brix. Cobre as 3 camadas: runtime.c → builtins → codegen dispatch → testes."
argument-hint: "[assinatura, ex: abs(x: float) -> float]"
allowed-tools: Read Edit Write Grep Glob Bash
model: opus
effort: high
user-invocable: true
---

# Adicionar built-in function

**Assinatura:** $ARGUMENTS

## 1. Análise

Determinar:
- **Nome Brix** (como o usuário chama)
- **Parâmetros**: nome, tipo Brix, tipo C (`int→long`, `float→double`, `string→BrixString*`, `Matrix→Matrix*`, `IntMatrix→IntMatrix*`)
- **Retorno**: tipo Brix → tipo C
- **É método?** (`arr.sort()` vs global `abs(x)`)
- **Categoria**: math / string / matrix / linalg / stats / io / test
- **Precisa de `coerce_to_f64`?** (float que pode receber int)

## 2. Implementar em runtime.c

Localizar seção correta e adicionar função C. Se retorna tipo ref-counted, retornar com `ref_count = 1`.

Seções: `SECTION 3: MATH`, `SECTION 2.1: STRING`, `SECTION 1: MATRIX`, `SECTION 1.5: INTMATRIX`, `SECTION 4: STATISTICS`, `SECTION 5: LINEAR ALGEBRA`.

## 3. Declarar em builtins

No arquivo `crates/codegen/src/builtins/{categoria}.rs`, declarar a função externa com `module.add_function()`.

## 4. Dispatch em lib.rs

**Global** (`abs(x)`): bloco `if fn_name == "nome"` em `compile_expr()` (~linha 8480–9020).
**Iterator method** (`arr.sort()`): match arm em `compile_iterator_method()` (~linha 12871) + guard `matches!()` (~linha 7392).
**String method** (`str.trim()`): match em `compile_string_method()` (~linha 13648).

Consulte [implementation-patterns.md](../implement/implementation-patterns.md) para templates de código.

## 5. Testes (3 camadas)

- **Codegen unit test** em `crates/codegen/src/tests/`
- **Integration test** em `tests/integration/success/NNN_nome.bx` + `.expected` + `#[test]` em `integration_test.rs`
- **Test Library** em `tests/brix/{categoria}.test.bx`

## 6. Verificar

```bash
cargo build && cargo test -p codegen -- test_nome && cargo test --test integration_test -- NNN --test-threads=1
```
