# 🎯 PRÓXIMO PASSO - v0.7: Import System + Math Library

**Data:** 26/01/2026
**Status:** Planejamento completo, pronto para implementação

---

## 📋 O QUE IMPLEMENTAR

### **1. Import System**
- Import com namespace: `import math`
- Import com alias: `import math as m`
- Flat symbol table com prefixos (`math.sin`, `m.sin`)

### **2. Math Library - 35 itens**

**21 Funções math.h (declaração direta LLVM):**
- Trigonometria (7): sin, cos, tan, asin, acos, atan, atan2
- Hiperbólicas (3): sinh, cosh, tanh
- Exp/Log (4): exp, log, log10, log2
- Raízes (2): sqrt, cbrt
- Arredondamento (3): floor, ceil, round
- Utilidades (5): abs, fmod, hypot, min, max

**5 Funções Estatísticas (wrappers em runtime.c):**
- sum, mean, median, std, var

**3 Funções Álgebra Linear (wrappers LAPACK em runtime.c):**
- det (via dgetrf)
- inv (via dgetri)
- tr (transpose - custom C)

**6 Constantes Matemáticas:**
- pi = 3.14159265358979323846...
- e = 2.71828182845904523536...
- tau = 6.28318530717958647692... (2π)
- phi = 1.61803398874989484820... (golden ratio)
- sqrt2 = 1.41421356237309504880...
- ln2 = 0.69314718055994530942...

---

## ⏳ ADIADO PARA FUTURO

**v0.8+ (Requer Complex):**
- eigvals(A) - autovalores podem ser complexos
- eigvecs(A) - muito complexo
- Decomposições (LU, QR, SVD)

**v0.9+ (Requer Sistema de Unidades):**
- Constantes físicas (c_light, h_planck, G_grav, etc.)

**v0.7.1+ (Baixa prioridade):**
- Selective imports: `from math import sin, cos`

---

## 🛠️ IMPLEMENTAÇÃO - OVERVIEW

### **Fase 1: Lexer + Parser (Token::Import)**
- Adicionar Token::Import ao lexer
- Parser reconhece `import module` e `import module as alias`
- AST: `Stmt::Import { module: String, alias: Option<String> }`

### **Fase 2: Symbol Table (Flat Namespace)**
- Quando vê `import math`, registra funções como `"math.sin"`, `"math.cos"`, etc.
- Quando vê `import math as m`, registra como `"m.sin"`, `"m.cos"`, etc.
- Usar HashMap flat: `variables.insert("math.sin", function_ptr)`

### **Fase 3: Codegen - Math.h Functions (Declaração Direta)**
- Gerar external declarations LLVM para funções math.h
- Exemplo: `declare double @sin(double) external`
- Não precisa de wrappers em runtime.c

### **Fase 4: Runtime.c - Stats + LAPACK Wrappers**
- Implementar wrappers para: sum, mean, median, std, var
- Implementar wrappers LAPACK para: det, inv
- Implementar transpose custom em C
- Exportar como: `brix_sum`, `brix_mean`, `brix_det`, etc.

### **Fase 5: Constantes Matemáticas**
- Registrar constantes como valores imutáveis no namespace
- `math.pi`, `math.e`, `math.tau`, `math.phi`, `math.sqrt2`, `math.ln2`

### **Fase 6: Type Checking**
- Auto-convert Int→Float em funções math
- Exemplo: `math.sin(5)` → converte 5 para 5.0 automaticamente

### **Fase 7: Linking**
- Adicionar `-lm -llapack -lblas` ao comando de linking em src/main.rs
- Sempre adicionar (simplifica)

### **Fase 8: Testes**
- Teste básico de todas as 29 funções
- Teste de física (movimento projectil, etc.)
- Teste de constantes

---

## 📝 DECISÕES TÉCNICAS

| Decisão | Escolha |
|---------|---------|
| Import syntax | Namespace + Alias (A + B) |
| Math.h functions | Declaração direta LLVM (sem wrappers) |
| LAPACK functions | Wrappers em runtime.c (complexidade) |
| Symbol table | Flat com prefixos (simples) |
| Type checking | Auto Int→Float |
| Linking | Sempre `-lm -llapack -lblas` |
| Constantes | 50+ dígitos de precisão |

---

## ✅ CHECKLIST DE IMPLEMENTAÇÃO

- [ ] **Lexer**: Adicionar Token::Import
- [ ] **Parser**: Reconhecer import statements
- [ ] **AST**: Adicionar Stmt::Import
- [ ] **Codegen**: External declarations para math.h (21 funções)
- [ ] **Runtime.c**: Wrappers stats (5 funções)
- [ ] **Runtime.c**: Wrappers LAPACK (2 funções: det, inv)
- [ ] **Runtime.c**: Transpose custom (1 função)
- [ ] **Codegen**: Registrar constantes matemáticas (6 constantes)
- [ ] **Codegen**: Type checking Int→Float
- [ ] **Main.rs**: Adicionar `-lm -llapack -lblas` ao linking
- [ ] **Testes**: math_basic_test.bx
- [ ] **Testes**: math_physics_test.bx
- [ ] **Documentação**: Atualizar CLAUDE.md e DOCUMENTATION.md

---

## 🚀 COMEÇAR AMANHÃ!

**Primeira tarefa:** Adicionar Token::Import ao lexer

**Arquivo:** `crates/lexer/src/token.rs`

Boa sorte! 💪
