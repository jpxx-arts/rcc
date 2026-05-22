//! Command-line driver for the rcc compiler frontend.
//!
//! Usage: `rcc <file.ling>`
//!
//! Pipeline: preprocessor → lexical analyzer → syntactic analyzer.
//! On success prints "código está sintaticamente correto" and dumps the
//! symbol table populated during lexing. On error prints the diagnostic
//! with line and column.

use std::process::ExitCode;

use rcc::lexical_analyzer::{self, SymbolTable};
use rcc::preprocessor::{self, PreprocessError};
use rcc::syntatic_analyzer::{self, ParseError};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: {} <source-file>", args[0]);
        return ExitCode::from(2);
    };

    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{path}': {e}");
            return ExitCode::from(2);
        }
    };

    match compile(&source) {
        Ok(symbol_table) => {
            println!("código está sintaticamente correto");
            print_symbol_table(&symbol_table);
            ExitCode::SUCCESS
        }
        Err(CompileError::Preprocess(err)) => {
            match err {
                PreprocessError::UnclosedBlockComment { line } => {
                    eprintln!("erro no pré-processamento (linha {line}): comentário de bloco não fechado");
                }
            }
            ExitCode::FAILURE
        }
        Err(CompileError::Parse(err)) => {
            eprintln!(
                "erro sintático (linha {}, coluna {}): {}",
                err.line, err.column, err.msg
            );
            ExitCode::FAILURE
        }
    }
}

enum CompileError {
    Preprocess(PreprocessError),
    Parse(ParseError),
}

impl From<PreprocessError> for CompileError {
    fn from(e: PreprocessError) -> Self {
        CompileError::Preprocess(e)
    }
}

impl From<ParseError> for CompileError {
    fn from(e: ParseError) -> Self {
        CompileError::Parse(e)
    }
}

fn compile(source: &str) -> Result<SymbolTable, CompileError> {
    let preprocessed = preprocessor::preprocess(source)?;
    let (tokens, symbol_table) = lexical_analyzer::get_tokens(&preprocessed);
    syntatic_analyzer::parse(&tokens)?;
    Ok(symbol_table)
}

fn print_symbol_table(table: &SymbolTable) {
    println!("\ntabela de símbolos:");
    if table.registers.is_empty() {
        println!("  (vazia)");
        return;
    }
    println!(
        "  {:>4}  {:<24}  {:<10}  {:<8}  {:>6}  {:>6}",
        "#", "lexeme", "kind", "type", "line", "col"
    );
    for (idx, entry) in table.registers.iter().enumerate() {
        println!(
            "  {:>4}  {:<24}  {:<10?}  {:<8}  {:>6}  {:>6}",
            idx,
            entry.lexeme,
            entry.kind,
            entry.type_info.as_deref().unwrap_or("-"),
            entry.first_line,
            entry.first_column,
        );
    }
}
