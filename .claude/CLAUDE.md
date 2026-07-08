# CLAUDE.md (.claude/)

Este arquivo documenta o **sistema de IA** deste repositório: os agents e skills disponíveis, como eles se encadeiam, e as convenções de código Rust/C que qualquer agent deve seguir ao editar o compilador Brix.

Para arquitetura do compilador (pipeline, tipos, ARC, etc.), veja o `CLAUDE.md` na raiz do projeto. Este arquivo é complementar — foca em **como trabalhar** neste repo, não **o que o compilador faz**.

## Regras de operação

- **Nunca rode `git commit`.** Quem commita é sempre o usuário. Ao final de uma implementação, apresente a mensagem de commit sugerida (seguindo o padrão `Phase N completed` / `update documentation` já usado no histórico) e pare — o usuário decide se e quando commitar.
- **Sempre limpe os binários de teste ao final de uma implementação.** Rodar a suíte de integração (`cargo test --test integration_test`) compila um binário nativo por teste (ex.: `01_hello_world`, `40_string_split_join`, `153_split_basic`) e os deixa soltos na raiz do repo — não há `.gitignore` para eles. Antes de reportar a tarefa como concluída, delete esses artefatos (ex.: `git clean -n` para conferir o que seria removido, depois remover apenas os binários extensionless que batem com nomes de teste — nunca use `git clean -fd` sem antes checar `git status`/`-n`, para não apagar trabalho do usuário por engano).

## Agents disponíveis (`.claude/agents/`)

| Agent | Domínio | Model | Edita? |
|-------|---------|-------|--------|
| `parser-dev` | `lexer/src/token.rs`, `parser/src/ast.rs`, `parser.rs`, `closure_analysis.rs` | sonnet | Sim |
| `codegen-dev` | `codegen/src/lib.rs`, `stmt.rs`, `expr.rs`, `types.rs`, `builtins/*.rs` | opus | Sim |
| `runtime-dev` | `runtime.c` (structs C, ARC, funções) | sonnet | Sim |
| `test-writer` | Testes nas 3 camadas (unit Rust, integration `.bx`, Test Library `.test.bx`) — escreve **e roda** para verificar | sonnet | Sim |
| `reviewer` | Revisão read-only pós-implementação (type system, ARC, LLVM IR, dispatch, testes) | opus | **Não** (read-only) |

Cada agent tem uma seção explícita "O que você NÃO faz" — respeite os limites de domínio ao delegar trabalho. Não peça a `codegen-dev` para editar `runtime.c`, nem a `runtime-dev` para tocar em `lib.rs`.

## Skills disponíveis (`.claude/skills/`)

| Skill | Uso |
|-------|-----|
| `/generate-roadmap` | Gera `ROADMAP_VX.Y.md` para a próxima versão, seguindo o template de 5 partes em português |
| `/implement <versao> <grupo>` | Lê um grupo do roadmap, apresenta plano, implementa na ordem runtime.c → types.rs → builtins → lib.rs → parser → testes |
| `/brix-test` | Roda a suíte completa nas 3 camadas |
| `/add-builtin` | Adiciona uma built-in function (dispatch + C + testes) |
| `/add-type` | Adiciona um novo `BrixType` (checklist completo: get_llvm_type, infer_type, ARC, etc.) |
| `/add-test-matcher` | Adiciona um matcher Jest-style ao Test Library |
| `/phase-done` | Marca uma fase/grupo como completa, atualiza contadores de teste |
| `/update-docs` | Atualiza o `CLAUDE.md` raiz (line counts, status, roadmap) |

### Fluxo recomendado para uma feature nova

```
/generate-roadmap (se não existir ainda)
  → /implement <versao> <grupo>   (delega para runtime-dev / codegen-dev / parser-dev conforme a camada)
  → test-writer                    (garante cobertura nas 3 camadas)
  → reviewer                       (audita antes de comitar)
  → /phase-done                    (atualiza contadores)
  → /update-docs                   (atualiza CLAUDE.md raiz)
```

Isso espelha o padrão de commits já usado no histórico: `Phase N: <descrição> - completed` seguido de um commit `update documentation`.

## Convenções de código Rust

Baseado no estado real do código (não aspiracional):

### Error handling
- Toda função de codegen retorna `CodegenResult<T>` = `Result<T, CodegenError>` (`crates/codegen/src/error.rs`). **Nunca** deixe uma função de compilação retornar `Option` ou entrar em pânico silenciosamente — propague com `?`.
- `CodegenError` tem 6 variantes (`LLVMError`, `TypeError`, `UndefinedSymbol`, `InvalidOperation`, `MissingValue`, `General`) — use a mais específica, não `General` como padrão.
- Erros de builder do inkwell (`build_int_mul`, `build_gep`, etc. retornam `Result`) devem ser mapeados com `.map_err(|_| CodegenError::LLVMError { .. })`, nunca com `.unwrap()`.

### `.unwrap()` — quando é aceitável
`.unwrap()` aparece hoje só em pontos específicos e **intencionais**, não espalhados por conveniência:
- Chamadas de API do inkwell que são infalíveis no contexto de uso (`get_nth_param(i)` quando `i` já foi validado contra a assinatura da função; `size_of()` em tipos LLVM conhecidos)
- Lookups em `self.variables` / `var_field_map` **depois** que a chave já foi confirmada existente por um `contains_key` ou lógica de controle anterior
- Arquivos de teste (`tests/*.rs`, `#[test]` functions) — aceitável, já que uma falha ali deve mesmo abortar o teste

**Não é aceitável**: `.unwrap()` em qualquer caminho alcançável a partir de input do usuário (parsing de `.bx`, valores literais, chamadas de função com args variáveis). Se o valor pode vir de um programa Brix malformado, use `CodegenResult` + `?`.

### `panic!()`
Usado hoje só para invariantes internas do compilador que **nunca** deveriam ser alcançáveis a partir de código Brix válido — ex.: `panic!("Optional type should have been converted to Union")` em `lib.rs`, porque a desugaring de `T?` para `Union(T, Nil)` acontece antes e é garantida pelo parser. Se você adicionar um `panic!`, comente por que o caminho é realmente inatingível — senão é um bug disfarçado de invariante.

### `unsafe`
Só aparece em `lib.rs` / `stmt.rs`, sempre em blocos `unsafe { self.builder.build_gep(...) }` — exigido pela API do inkwell para GEP bruto, não por manipulação manual de ponteiros. Não introduza `unsafe` fora desse padrão sem justificar em comentário.

### Naming
- Métodos de compilação: prefixo `compile_*` (ex.: `compile_iterator_method`, `compile_ones`) — 40+ já seguem esse padrão em `lib.rs`
- Funções C por tipo: `matrix_*`, `intmatrix_*`, `str_*`/`brix_str_*`, `complex_*`, `atom_*`, `brix_*` (utilitários globais), `test_*`
- Testes Rust: `test_{feature}_{variante}` (ex.: `test_string_matrix_type`)
- Testes de integração: `NNN_feature_name.bx` + `.expected`, numeração sequencial — nunca reutilize um número

### Workspace
`Cargo.toml` na raiz define o workspace (`crates/codegen`, `crates/lexer`, `crates/parser`) mais o binário principal em `src/main.rs`. Dependências chave: `inkwell` (LLVM 18 bindings), `logos` (lexer), `chumsky 0.9.3` (parser), `ariadne` (error reporting), `clap` (CLI).

Não há `rustfmt.toml` nem `clippy.toml` customizados — usar defaults do `rustfmt`/`clippy` stable.

## Coisas a evitar

- **Não** adicionar `.unwrap()`/`.expect()` em caminho alcançável por input do usuário — sempre `CodegenResult` + `?`
- **Não** editar `lib.rs` sem passar pelo checklist de dispatch (guard `matches!()` em ~linha 7392 + match arm no local certo) — feature "funciona só às vezes" costuma ser esse checklist incompleto
- **Não** esquecer ARC ao adicionar um tipo heap-allocated: todo struct C precisa de `ref_count` como primeiro campo, `_retain()`/`_release()` idempotentes, e os pontos em `lib.rs` (`is_ref_counted()`, `insert_retain()`, `insert_release()`)
- **Não** rodar integration tests em paralelo — todos compilam no mesmo diretório; sempre `--test-threads=1`
- **Não** pular o clean build (`rm -f runtime.o output.o program && cargo clean`) ao investigar erros de linking antes de assumir que é um bug real
- **Não** misturar mudança de feature com "update documentation" no mesmo commit — o histórico separa os dois (`Phase N completed` → commit próprio `update documentation`)
- **Não** deixar contadores de teste no `CLAUDE.md` raiz dessincronizados da suíte real — rode `/brix-test` antes de `/update-docs`
- **Não** commitar — sempre entregar a mensagem de commit para o usuário rodar (ver "Regras de operação" acima)
- **Não** terminar uma tarefa sem limpar os binários de teste soltos na raiz (ver "Regras de operação" acima)
