---
name: add-type
description: "Checklist completo para adicionar um novo BrixType ao compilador. Cobre: struct C com ARC, LLVM type, enum, retain/release, println, inferência, indexação, iteração, e testes."
argument-hint: "[nome do tipo e descrição]"
allowed-tools: Read Edit Write Grep Glob Bash
model: opus
effort: high
user-invocable: true
---

# Adicionar novo BrixType

**Tipo:** $ARGUMENTS

## 1. Análise

- **Nome Brix**: como aparece no código fonte
- **Nome enum**: variant no `BrixType` Rust
- **Ref-counted?**: heap-allocated com ARC
- **Genérico?**: monomorphizado (`Vector<int>` vs `Vector<float>`)
- **Layout C**: campos do struct
- **Layout LLVM**: tipos correspondentes

## 2. Struct C em runtime.c

Nova seção com:
```c
// ==========================================
// SECTION X: TYPENAME (vX.Y)
// ==========================================
typedef struct { long ref_count; /* campos */ } BrixTypeName;
BrixTypeName* typename_new(...);
void* typename_retain(BrixTypeName*);
void typename_release(BrixTypeName*);
BrixString* typename_to_string(BrixTypeName*);
```

## 3. BrixType enum em types.rs

Adicionar variant + `Display` impl.

## 4. Registrar em lib.rs — TODOS estes pontos

Checklist obrigatório:
- [ ] `get_llvm_type()` / `brix_type_to_llvm()` → tipo LLVM
- [ ] `get_typename_type()` → helper que retorna LLVM struct type
- [ ] `infer_type()` → inferência de tipo
- [ ] `are_types_compatible()` → regras de compatibilidade
- [ ] `is_ref_counted()` → `true` se ref-counted
- [ ] `insert_retain()` / `insert_release()` → ARC
- [ ] `value_to_string()` → println funciona
- [ ] `cast_value()` → conversões (se houver)
- [ ] `compile_index()` → indexação (se indexável)
- [ ] `compile_field_access()` → `.len` etc (se tiver campos)
- [ ] `compile_for_stmt()` → `for x in value` (se iterável)

## 5. Declarar em builtins

Criar ou adicionar ao módulo correspondente em `builtins/`. Registrar em `builtins/mod.rs`.

## 6. Testes mínimos

- Unit: `test_typename_creation`, `test_typename_println`, `test_typename_refcount`
- Integration: programa que cria, usa, imprime
- Test Library: `typeof()` check + operações básicas
