---
name: add-test-matcher
description: "Adicionar novo matcher à Test Library Jest-style do Brix. Cobre: função C no runtime, declaração em builtins/test.rs, dispatch em compile_test_matcher(), e testes."
argument-hint: "[matcher, ex: toStartWith(prefix: string)]"
allowed-tools: Read Edit Write Grep Glob Bash
model: sonnet
effort: medium
user-invocable: true
---

# Adicionar test matcher

**Matcher:** $ARGUMENTS

## 1. Análise

- **Nome**: como aparece (`test.expect(x).matcherName(args)`)
- **Tem `.not`?**: versão negada
- **Tipos aceitos**: quais tipos `expect()` pode receber
- **Args**: parâmetros do matcher
- **Compile-time?** (ex: `toHaveProperty` resolve campo de struct)
- **Runtime?** (ex: `toBe` compara valores)

## 2. Funções C em runtime.c

Na seção TEST FRAMEWORK:

```c
void test_expect_matcher_name(TipoRecebido received, TipoArg arg) {
    test_assert_count++;
    int passed = /* lógica */;
    if (passed) {
        test_pass_count++;
        if (test_verbose) printf("    \033[32m✓\033[0m description\n");
    } else {
        test_fail_count++;
        printf("    \033[31m✗\033[0m description\n");
        printf("      Expected: ...\n      Received: ...\n");
    }
}
// versão not (se aplicável)
void test_expect_not_matcher_name(...) { /* lógica invertida */ }
```

## 3. Declarar em builtins/test.rs

`module.add_function("test_expect_matcher_name", fn_type, Some(Linkage::External));`

## 4. Dispatch em lib.rs

Em `compile_test_matcher()` (~linha 15923):
```rust
"matcherName" => {
    let (arg_val, arg_type) = self.compile_expr(&matcher_args[0])?;
    let fn_name = if is_negated { "test_expect_not_matcher_name" } else { "test_expect_matcher_name" };
    let fn_val = self.module.get_function(fn_name).unwrap();
    self.builder.build_call(fn_val, &[received_val.into(), arg_val.into()], "")?;
}
```

## 5. Testes

Integration test + Test Library test exercitando o matcher (positivo e `.not.`).

Matchers existentes para referência: `toBe`, `not.toBe`, `toEqual`, `toBeCloseTo`, `toBeTruthy`, `toBeFalsy`, `toBeGreaterThan`, `toBeLessThan`, `toBeGreaterThanOrEqual`, `toBeLessThanOrEqual`, `toContain`, `toHaveLength`, `toBeNil`, `not.toBeNil`.
