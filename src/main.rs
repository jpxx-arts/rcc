//! Command-line driver for the rcc compiler frontend.
//!
//! Usage: `rcc [flags] <source-file>`
//!
//! Pipeline: lexical analyzer -> syntactic analyzer (+ symbol table) ->
//! semantic analyzer. The lexer is now self-contained (no preprocessor).
//!
//! Flags:
//!   --tokens            print the token list produced by the lexer
//!   --fail-fast         stop at the first lexical error (otherwise report all)
//!   --ast               print the abstract syntax tree
//!   --symbols           print the symbol table after syntactic analysis
//!   --suggest           show correction hints for lexical/syntactic errors
//!   --allow-empty-body  relax L_com to be nullable (accept empty bodies)

use std::process::ExitCode;

use rcc::lexical_analyzer::{self, LexError, Token, TokenClass};
use rcc::semantic_analyzer;
use rcc::syntatic_analyzer::{self, ParseError};

struct Options {
    path: String,
    tokens: bool,
    fail_fast: bool,
    ast: bool,
    symbols: bool,
    suggest: bool,
    allow_empty_body: bool,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let opts = match parse_args(&args) {
        Ok(o) => o,
        Err(code) => return code,
    };

    let source = match std::fs::read_to_string(&opts.path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {e}", opts.path);
            return ExitCode::from(2);
        }
    };

    run(&source, &opts)
}

fn parse_args(args: &[String]) -> Result<Options, ExitCode> {
    let mut path: Option<String> = None;
    let mut tokens = false;
    let mut fail_fast = false;
    let mut ast = false;
    let mut symbols = false;
    let mut suggest = false;
    let mut allow_empty_body = false;

    for arg in &args[1..] {
        match arg.as_str() {
            "--tokens" => tokens = true,
            "--fail-fast" => fail_fast = true,
            "--ast" => ast = true,
            "--symbols" => symbols = true,
            "--suggest" => suggest = true,
            "--allow-empty-body" => allow_empty_body = true,
            "-h" | "--help" => {
                print_usage(&args[0]);
                return Err(ExitCode::SUCCESS);
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag '{other}'");
                print_usage(&args[0]);
                return Err(ExitCode::from(2));
            }
            other => {
                if path.is_some() {
                    eprintln!("error: multiple source files given");
                    return Err(ExitCode::from(2));
                }
                path = Some(other.to_string());
            }
        }
    }

    match path {
        Some(path) => Ok(Options {
            path,
            tokens,
            fail_fast,
            ast,
            symbols,
            suggest,
            allow_empty_body,
        }),
        None => {
            print_usage(&args[0]);
            Err(ExitCode::from(2))
        }
    }
}

fn print_usage(prog: &str) {
    eprintln!(
        "usage: {prog} [--tokens] [--fail-fast] [--ast] [--symbols] [--suggest] [--allow-empty-body] <source-file>"
    );
}

fn run(source: &str, opts: &Options) -> ExitCode {
    // ---- Lexical analysis ----
    let lex = lexical_analyzer::tokenize(source, opts.fail_fast);

    if opts.tokens {
        print_tokens(&lex.tokens);
    }

    if !lex.errors.is_empty() {
        report_lex_errors(&lex.errors, opts.suggest);
        return ExitCode::FAILURE;
    }

    // ---- Syntactic analysis (+ symbol table) ----
    let (program, symbol_table) =
        match syntatic_analyzer::parse_with(&lex.tokens, opts.allow_empty_body) {
            Ok(result) => result,
            Err(err) => {
                report_parse_error(&err, opts.suggest);
                return ExitCode::FAILURE;
            }
        };

    if opts.symbols {
        print!("\n{}", symbol_table.render());
    }
    if opts.ast {
        print!("\n{}", program.pretty());
    }

    // ---- Semantic analysis ----
    let sem_errors = semantic_analyzer::analyze(&program);
    if !sem_errors.is_empty() {
        eprintln!("\n{} semantic error(s) found:", sem_errors.len());
        for e in &sem_errors {
            eprintln!(
                "  semantic error (line {}, column {}): {}",
                e.line, e.column, e.msg
            );
        }
        return ExitCode::FAILURE;
    }

    println!("\ncode is syntactically and semantically correct");
    ExitCode::SUCCESS
}

fn print_tokens(tokens: &[Token]) {
    println!("tokens:");
    println!(
        "  {:>5}  {:<5}  {:<12}  {:<16}",
        "line", "col", "class", "lexeme"
    );
    for t in tokens {
        let class = match t.class {
            TokenClass::Id => "ID",
            TokenClass::Number => "NUMBER",
            TokenClass::Keyword => "KEYWORD",
            TokenClass::Operation => "OPERATION",
            TokenClass::Delimiter => "DELIMITER",
            TokenClass::Eof => "EOF",
        };
        let lexeme = if t.class == TokenClass::Eof {
            "<eof>"
        } else {
            &t.lexeme
        };
        println!(
            "  {:>5}  {:<5}  {:<12}  {:<16}",
            t.line, t.column, class, lexeme
        );
    }
}

fn report_lex_errors(errors: &[LexError], suggest: bool) {
    eprintln!("{} lexical error(s) found:", errors.len());
    for e in errors {
        eprintln!(
            "  lexical error (line {}, column {}): {}",
            e.line, e.column, e.msg
        );
        if suggest && let Some(hint) = suggest_lexeme_hint(&e.msg) {
            eprintln!("    suggestion: {hint}");
        }
    }
}

/// The lexer already embeds "Did you mean" hints in some messages; for the rest
/// there's no extra structured suggestion to surface.
fn suggest_lexeme_hint(msg: &str) -> Option<String> {
    if msg.contains("Did you mean") {
        None
    } else if msg.contains("unclosed block comment") {
        Some("close the comment with '*/'".to_string())
    } else {
        None
    }
}

fn report_parse_error(err: &ParseError, suggest: bool) {
    eprintln!(
        "syntactic error (line {}, column {}): {}",
        err.line, err.column, err.msg
    );
    if suggest && let Some(hint) = &err.suggestion {
        eprintln!("  suggestion: {hint}");
    }
}
