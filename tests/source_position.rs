//! Tests for source-position fidelity through preprocessor + lexer pipeline.
//!
//! The preprocessor strips comments and minifies non-significant whitespace.
//! Downstream phases (lexer, parser) must still report positions in the
//! ORIGINAL source, not in the post-preprocessed text. These tests pin the
//! expected behavior; if the lexer/preprocessor stops carrying enough
//! information to recover original positions, they fail.

use rcc::lexical_analyzer::{self, SymbolTable, Token, TokenClass};
use rcc::preprocessor;
use rcc::syntatic_analyzer::{self, ParseError};

fn run(src: &str) -> (Vec<Token>, SymbolTable) {
    let (preprocessed, map) = preprocessor::preprocess(src).expect("preprocess should succeed");
    lexical_analyzer::get_tokens_with_map(&preprocessed, Some(&map))
}

fn parse(src: &str) -> Result<(), ParseError> {
    let (preprocessed, map) = preprocessor::preprocess(src).expect("preprocess should succeed");
    let (tokens, _) = lexical_analyzer::get_tokens_with_map(&preprocessed, Some(&map));
    syntatic_analyzer::parse(&tokens)
}

fn find_keyword<'a>(tokens: &'a [Token], kw: &str) -> &'a Token {
    tokens
        .iter()
        .find(|t| matches!(&t.token_name, TokenClass::KEYWORD(s) if s == kw))
        .unwrap_or_else(|| panic!("keyword `{kw}` not found"))
}

mod line_is_preserved {
    use super::*;

    #[test]
    fn token_on_third_line() {
        let src = "\n\nint x;";
        let (tokens, _) = run(src);
        assert_eq!(find_keyword(&tokens, "int").line, 3);
    }

    #[test]
    fn token_after_multi_line_block_comment() {
        // Block comment spans 3 source lines; `int` is on line 4.
        let src = "/*\n line 2\n line 3\n*/\nint x;";
        let (tokens, _) = run(src);
        assert_eq!(find_keyword(&tokens, "int").line, 5);
    }

    #[test]
    fn symbol_first_line_matches_original() {
        let src = "\n\nclass Foo { public static void main(String[] a) { } }";
        let (_tokens, sym) = run(src);
        let foo = sym
            .registers
            .iter()
            .find(|e| e.lexeme == "Foo")
            .expect("Foo in symbol table");
        assert_eq!(foo.first_line, 3);
    }
}

mod column_reflects_original {
    use super::*;

    #[test]
    fn indented_keyword_reports_original_column() {
        // 4-space indent: `int` starts at original column 5.
        let src = "    int x;";
        let (tokens, _) = run(src);
        assert_eq!(find_keyword(&tokens, "int").column, 5);
    }

    #[test]
    fn token_after_collapsed_internal_spaces_reports_original_column() {
        // `int    x;` -> internal spaces collapse to one, but `x` still lives
        // at original column 8.
        let src = "int    x;";
        let (tokens, _) = run(src);
        let x = tokens
            .iter()
            .find(|t| t.token_name == TokenClass::ID)
            .expect("identifier present");
        assert_eq!(x.column, 8);
    }

    #[test]
    fn symbol_first_column_matches_original() {
        // `Foo` starts at original column 11.
        let src = "    class Foo { public static void main(String[] a) { } }";
        let (_tokens, sym) = run(src);
        let foo = sym
            .registers
            .iter()
            .find(|e| e.lexeme == "Foo")
            .expect("Foo in symbol table");
        assert_eq!(foo.first_column, 11);
    }
}

mod parse_error_position {
    use super::*;

    #[test]
    fn missing_semicolon_reports_line_of_offending_token() {
        // Missing `;` after `1`; the parser errors at `}` on line 2.
        let src = "class Main { public static void main(String[] a) { x = 1\n} }";
        let err = parse(src).unwrap_err();
        assert_eq!(err.line, 2);
    }

    #[test]
    fn missing_semicolon_reports_original_column() {
        // `}` is at original column 1 of line 2.
        let src = "class Main { public static void main(String[] a) { x = 1\n} }";
        let err = parse(src).unwrap_err();
        assert_eq!(err.column, 1);
    }

    #[test]
    fn indented_error_reports_original_column() {
        // `    }` — `}` is at original col 5 of line 2.
        let src = "class Main { public static void main(String[] a) { x = 1\n    } }";
        let err = parse(src).unwrap_err();
        assert_eq!(err.column, 5);
    }
}
