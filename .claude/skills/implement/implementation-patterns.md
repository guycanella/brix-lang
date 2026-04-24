# Padrões de Implementação — Referência

## runtime.c

Toda struct heap-allocated começa com `long ref_count`. Seções marcadas com:
```c
// ==========================================
// SECTION X: NOME (vX.Y)
// ==========================================
```

Prefixos por tipo: `matrix_*`, `intmatrix_*`, `str_*`, `brix_*`, `atom_*`, `complex_*`.

Padrão mínimo para tipo novo:
```c
TypeName* typename_new(...) {
    TypeName* t = brix_malloc(sizeof(TypeName));
    t->ref_count = 1;
    return t;
}
void* typename_retain(TypeName* t) { if (t) t->ref_count++; return t; }
void typename_release(TypeName* t) { if (t && --t->ref_count <= 0) { /* free internals */ brix_free(t); } }
```

## types.rs — BrixType enum

Adicionar variant ao `pub enum BrixType { ... }`. Atualizar `Display` impl.

## builtins/*.rs — Declarações externas

```rust
let fn_type = context.f64_type().fn_type(&[context.f64_type().into()], false);
module.add_function("brix_fn_name", fn_type, Some(Linkage::External));
```

Módulos: `math.rs`, `stats.rs`, `linalg.rs`, `string.rs`, `io.rs`, `matrix.rs`, `test.rs`.

## lib.rs — Dispatch de built-in functions

Bloco de dispatch em `compile_expr()` (~linha 8480–9020):
```rust
if fn_name == "nome" {
    if args.len() != N { return Err(CodegenError::InvalidOperation { ... }); }
    let (val, typ) = self.compile_expr(&args[0])?;
    // type check...
    let fn_val = self.module.get_function("brix_fn_name").unwrap();
    let call = self.builder.build_call(fn_val, &[val.into()], "result")?;
    return Ok((call.try_as_basic_value().left().unwrap(), BrixType::ReturnType));
}
```

## lib.rs — Iterator methods

Adicionar match arm em `compile_iterator_method()` (~linha 12871):
```rust
"method" => self.compile_array_method(target_val, &brix_type),
```
E registrar no guard `matches!(field.as_str(), "map" | "filter" | ... | "method")` (~linha 7392).

## lib.rs — String methods

Adicionar em `compile_string_method()` (~linha 13648).

## lib.rs — Test matchers

Adicionar em `compile_test_matcher()` (~linha 15923). Usar flag `is_negated` para `.not.`.

## lib.rs — Constructors (zeros, ones, etc.)

Dispatch: `if fn_name == "nome" { let val = self.compile_nome(args)?; return Ok((val, BrixType::Tipo)); }`
Método helper: `fn compile_nome(&mut self, args: &[Expr]) -> CodegenResult<BasicValueEnum<'ctx>> { ... }`

## Parser — Nova sintaxe

1. Token variant em `lexer/src/token.rs` (se necessário)
2. AST variant em `parser/src/ast.rs`
3. Parser rule em `parser/src/parser.rs`
4. Soft keywords: `just(Token::Identifier("keyword".to_string()))`

## Coerções

`self.coerce_to_f64(val, &brix_type)` — quando float args podem receber int literals.
`intmatrix_to_matrix()` — IntMatrix op Float promove a Matrix.
