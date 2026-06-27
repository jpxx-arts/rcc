# rcc

Compiler frontend for a simplified variant of Java (MiniJava), written in
Rust. Implements a self-contained lexical analyzer, a recursive-descent
syntactic analyzer that builds an AST and a scoped symbol table, and a
static semantic analyzer.

Coursework for DIM0164 - Compilers (UFRN, 2026.1), Trabalho 2.

## Prerequisites

- Rust 1.93 or newer (uses `std::sync::LazyLock` and edition 2024).
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
pipeline: lexical → syntactic (+ symbol table) → semantic analysis.

```
cargo run -- path/to/file.ling
```

Provided examples:

```
cargo run -- specs/prog-factorial.ling
cargo run -- specs/prog-bubblesort.ling
cargo run -- specs/prog-erro-lexico.ling
cargo run -- specs/prog-erro-semantico.ling
```

### Flags

| Flag          | Effect                                                          |
| ------------- | -------------------------------------------------------------- |
| `--tokens`    | Print the token list produced by the lexer.                    |
| `--fail-fast` | Stop at the first lexical error (otherwise report all of them).|
| `--ast`       | Print the abstract syntax tree.                                |
| `--symbols`   | Print the symbol table after syntactic analysis.               |
| `--suggest`   | Show correction hints for lexical/syntactic errors.            |

Flags may be combined, e.g.:

```
cargo run -- --tokens --symbols --ast specs/prog-factorial.ling
```

### Output and exit codes

On success: `code is syntactically and semantically correct`.

On failure, diagnostics go to standard error with exact `line, column`:

- `lexical error (line N, column C): ...`
- `syntactic error (line N, column C): expected X, got Y`
- `semantic error (line N, column C): ...`

Exit codes: `0` on success, `1` on a lexical/syntactic/semantic error,
`2` on usage or file I/O error.

## Tests

Full suite:

```
cargo test
```

A single phase:

```
cargo test --test lexical_analyzer
cargo test --test syntatic_analyzer
cargo test --test semantic_analyzer
```

## Project structure

```
.
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs                  # CLI driver and flag handling
│   ├── lexical_analyzer/mod.rs  # Self-contained scanner (comments, errors)
│   ├── syntatic_analyzer/mod.rs # Recursive-descent parser (builds AST)
│   ├── ast/mod.rs               # AST node types + pretty-printer
│   ├── symbol_table/mod.rs      # Scoped symbol table (--symbols)
│   └── semantic_analyzer/mod.rs # Type / class / inheritance checks
├── tests/
│   ├── lexical_analyzer.rs
│   ├── syntatic_analyzer.rs
│   └── semantic_analyzer.rs
└── specs/
    ├── gramatica-prof.md          # Official grammar (source of truth)
    ├── prog-factorial.ling        # Valid sample program
    ├── prog-bubblesort.ling       # Valid sample program
    ├── prog-erro-lexico.ling      # Sample with lexical errors
    └── prog-erro-semantico.ling   # Sample with semantic errors
```

## Compilation pipeline

```
file.ling
   │
   ▼
lexical_analyzer::tokenize
   │  (skip comments/whitespace, longest-match, EOF,
   │   line/column tracking, interned literal table)
   ▼
syntatic_analyzer::parse
   │  (recursive descent per gramatica-prof.md;
   │   builds the AST and the scoped symbol table)
   ▼
semantic_analyzer::analyze
   │  (operator typing, arrays, .length, assignment
   │   compatibility, classes, inheritance, empty-class rule)
   ▼
(AST, SymbolTable, diagnostics)
```

## Language notes

The grammar in `specs/gramatica-prof.md` is the source of truth. A few
points worth highlighting:

- `if`/`while` bodies require braces (`{ ... }`); `else` is optional.
- A command list (`L_com`) must contain at least one command, so `main`
  and every method body need at least one statement before the closing
  `}` / `return`. Empty bodies are a syntax error.
- The only relational operator is `<`.
- Identifiers follow the C/Java convention (`_` allowed anywhere).
- A class with no fields **and** no methods is a semantic error.
