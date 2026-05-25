# rcc

Compilador (frontend) para uma variante simplificada de Java (MiniJava),
escrito em Rust. Implementa pré-processador, analisador léxico e analisador
sintático.

Trabalho 1 da disciplina DIM0164 — Compiladores (UFRN, 2026.1).

## Pré-requisitos

- Rust 1.93
- `cargo` no PATH.

Verificação:

```
$ rustc --version
$ cargo --version
```

## Build

```
cargo build
```

Release com otimizações:

```
cargo build --release
```

O binário fica em `target/debug/rcc` ou `target/release/rcc`.

## Executar

O programa lê um arquivo `.ling` (sintaxe MiniJava) e processa a pipeline
completa.

```
cargo run -- caminho/para/arquivo.ling
```

Exemplos fornecidos:

```
cargo run -- specs/prog-factorial.ling
cargo run -- specs/prog-bubblesort.ling
```

A saída padrão contém, em ordem:

1. `código está sintaticamente correto`
2. Tabela de símbolos (índice, lexema, kind, type, line, col)

Em caso de erro, a saída de erro padrão contém:

- `erro no pré-processamento (linha N): comentário de bloco não fechado`
- `erro sintático (linha N, coluna C): expected X, got Y`
- `Lexical error at line N, column C: Unknown lexeme: 'X'. Did you mean: 'Y'?`

O processo termina com código 0 em sucesso, 1 em erro sintático/léxico,
2 em erro de uso ou leitura de arquivo.

## Testes

Suíte completa (137 testes de integração):

```
cargo test
```

Suíte de uma fase específica:

```
cargo test --test preprocessor
cargo test --test lexical_analyzer
cargo test --test syntatic_analyzer
```

Cobertura por arquivo:

| Suíte | Testes |
|-------|-------:|
| `tests/preprocessor.rs` | 37 |
| `tests/lexical_analyzer.rs` | 52 |
| `tests/syntatic_analyzer.rs` | 48 |

## Estrutura do projeto

```
.
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs                  # CLI driver
│   ├── preprocessor/mod.rs
│   ├── lexical_analyzer/mod.rs
│   └── syntatic_analyzer/
│       ├── mod.rs               # Parser recursivo descendente
├── tests/
│   ├── preprocessor.rs
│   ├── lexical_analyzer.rs
│   └── syntatic_analyzer.rs
├── specs/
│   ├── gramatica.md             # Gramática original da disciplina
│   ├── gramatica-transformada.md  # Sem recursão à esquerda, fatorada
│   ├── prog-bubblesort.ling     # Programa de teste
│   ├── prog-factorial.ling      # Programa de teste
│   ├── prog-bubblesort.expected # Saída esperada do pré-processador
│   └── prog-factorial.expected  # Saída esperada do pré-processador
└── docs/
    ├── relatorio-tecnico.tex    # Relatório técnico (compila com pdflatex)
    ├── teoria-fase-1.md         # Notas de estudo: léxico
    ├── teoria-fase-2.md         # Notas de estudo: sintático
    └── tasks.md                 # Pendências e melhorias futuras
```

## Pipeline de compilação

```
arquivo.ling
   │
   ▼
preprocessor::preprocess
   │  (remove comentários, normaliza whitespace, preserva newlines)
   ▼
lexical_analyzer::get_tokens
   │  (longest-match, EOF, suggestions, symbol table com interning)
   ▼
syntatic_analyzer::parse
   │  (descendente recursivo)
   ▼
(Program, SymbolTable)
```

## Recompilar o relatório técnico

```
cd docs
pdflatex relatorio-tecnico.tex
pdflatex relatorio-tecnico.tex   # segunda passagem para referências
```

Requer TeX Live com os pacotes `amsmath`, `listings`, `xcolor`, `hyperref`
e `geometry` (todos padrão).

## Limitações conhecidas

- A gramática original (`specs/gramatica.md`) restringe `_` à última
  posição de um identificador via `Word -> '_'` terminal. O lexer
  relaxa para o padrão C/Java (`_` em qualquer posição) para aceitar
  os arquivos de teste fornecidos (`num_aux`, `aux01`, etc.).
- O operador `<` foi adicionado à linguagem (não consta na gramática
  original mas é usado pelos arquivos de teste).
- A gramática original admite apenas um único `Cmd` por corpo;
  introduzido `Cmds` para sequências, necessário para os testes.
- `DotRest` restringe a chamada de método a identificador (em vez de
  qualquer expressão arbitrária como a gramática original permitiria).

Detalhes em `specs/gramatica-transformada.md`.
