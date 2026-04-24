---
name: generate-roadmap
description: "Gera roadmap no template português do projeto Brix. Segue exatamente a estrutura dos ROADMAPs existentes: Contexto, Visão Geral, Grupos A/B/C com Motivação/Sintaxe/Implementação/Testes."
argument-hint: "[versao - tema, ex: v2.3 - GPU Computing]"
allowed-tools: Read Write Grep Glob Bash
model: opus
effort: high
user-invocable: true
---

# Gerar roadmap Brix

**Input:** $ARGUMENTS

## 1. Coletar contexto

- Leia o ROADMAP da versão anterior para "Contexto: O que vX.Y-1 entregou"
- Leia CLAUDE.md para baseline de testes atual
- Se o usuário não especificou features, pergunte quais devem entrar

## 2. Organizar em Grupos (A, B, C...)

Para cada grupo: **Impacto** (Alto/Médio/Baixo), **Risco** (Alto/Médio/Baixo), dependências, o que desbloqueia.

## 3. Gerar usando o template

Consulte [template.md](template.md) para o formato exato em português. Respeitar todas as seções:
- Header com Status/Tema/Ordem
- Contexto da versão anterior (tabela)
- Visão Geral (tabela)
- Cada Grupo com: Motivação, Sintaxe, Estrutura C, Arquivos e mudanças, Testes
- Resumo por Arquivo (tabela)
- Metas de Teste (tabela com totais acumulados)
- Ordem Recomendada (diagrama ASCII com → e ←→)
- Verificação por Grupo (comandos bash)
- Fora do Escopo

## 4. Salvar

Salvar como `ROADMAP_V{versao}.md` (ex: v2.3 → `ROADMAP_V2.3.md`) na raiz do projeto.

## 5. Revisão

Apresentar para o usuário destacando: total de features, testes estimados, dependências, riscos.
