# rcc

Compiler frontend for a simplified variant of Java (MiniJava), written in
Rust. Implements a preprocessor, a lexical analyzer, and a syntactic
analyzer.

Coursework for DIM0164 - Compilers (UFRN, 2026.1).

## Prerequisites

- Rust 1.93 or newer (uses `std::sync::LazyLock`).
- `cargo` in `PATH`.

Check with:

```
$ rustc --version
$ cargo --version
```

## Build

```
cargo build
```

Optimized release build:

```
cargo build --release
```

The binary is at `target/debug/rcc` or `target/release/rcc`.

## Run

The program reads a `.ling` file (MiniJava syntax) and runs the full
pipeline.

```
cargo run -- path/to/file.ling
```

Provided examples:

```
cargo run -- specs/prog-factorial.ling
cargo run -- specs/prog-bubblesort.ling
```

Standard output, in order:

1. `code is syntactically correct`
2. The symbol table (index, lexeme, kind, type, line, col).

Standard error, on failure:

- `preprocessing error (line N): unclosed block comment`
- `syntactic error (line N, column C): expected X, got Y`
- `Lexical error at line N, column C: Unknown lexeme: 'X'. Did you mean: 'Y'?`

Exit codes: `0` on success, `1` on lexical/syntactic error, `2` on usage
or file I/O error.

## Tests

Full suite (integration tests):

```
cargo test
```

A single phase:

```
cargo test --test preprocessor
cargo test --test lexical_analyzer
cargo test --test syntatic_analyzer
```

## Project structure

```
.
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs                  # CLI driver
│   ├── preprocessor/mod.rs
│   ├── lexical_analyzer/mod.rs
│   └── syntatic_analyzer/mod.rs # Recursive-descent parser
├── tests/
│   ├── preprocessor.rs
│   ├── lexical_analyzer.rs
│   └── syntatic_analyzer.rs
├── specs/
│   ├── gramatica.md                # Original course grammar
│   ├── gramatica-transformada.md   # Left-recursion removed, factored
│   ├── prog-bubblesort.ling        # Test program
│   ├── prog-factorial.ling         # Test program
│   ├── prog-bubblesort.expected    # Expected preprocessor output
│   └── prog-factorial.expected     # Expected preprocessor output
└── docs/
    ├── relatorio-tecnico.tex    # Technical report (compile with pdflatex)
    ├── teoria-fase-1.md         # Study notes: lexical
    ├── teoria-fase-2.md         # Study notes: syntactic
    └── tasks.md                 # Pending work and future improvements
```

## Compilation pipeline

```
file.ling
   │
   ▼
preprocessor::preprocess
   │  (strip comments, normalize whitespace, preserve newlines)
   ▼
lexical_analyzer::get_tokens
   │  (longest-match, EOF, suggestions, symbol table with interning)
   ▼
syntatic_analyzer::parse
   │  (recursive descent)
   ▼
(SymbolTable, syntactic correctness)
```

## Rebuilding the technical report

```
cd docs
pdflatex relatorio-tecnico.tex
pdflatex relatorio-tecnico.tex   # second pass for cross-references
```

Requires TeX Live with the standard packages `amsmath`, `listings`,
`xcolor`, `hyperref`, and `geometry`.

## Known deviations from the original grammar

- `specs/gramatica.md` restricts `_` to the trailing position of an
  identifier via the terminal production `Word -> '_'`. The lexer
  relaxes this to the C/Java convention (`_` anywhere) to accept the
  provided test programs (`num_aux`, `aux01`, etc.).
- The `<` operator was added (not in the original grammar, but used by
  the test programs).
- The original grammar admits only a single `Cmd` per body; a `Cmds`
  non-terminal was introduced for sequences, required by the test
  programs.
- `DotRest` restricts the method-call form to `Id ( args )` instead of
  any `Exp ( args )` (semantically sane, mirrors C/Java).

Details in `specs/gramatica-transformada.md`.
