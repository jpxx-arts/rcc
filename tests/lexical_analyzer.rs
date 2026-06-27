//! Integration tests for the self-contained lexical analyzer.

use rcc::lexical_analyzer::{self, Token, TokenClass};

/// Returns all tokens except the trailing EOF.
fn tokens_without_eof(input: &str) -> Vec<Token> {
    let mut result = lexical_analyzer::tokenize(input, false);
    assert!(
        result.errors.is_empty(),
        "unexpected lexical errors: {:?}",
        result.errors
    );
    if let Some(last) = result.tokens.last()
        && last.class == TokenClass::Eof
    {
        result.tokens.pop();
    }
    result.tokens
}

mod identifiers {
    use super::*;

    fn assert_id(input: &str, expected: &str) {
        let tokens = tokens_without_eof(input);
        assert_eq!(tokens[0].class, TokenClass::Id);
        assert_eq!(tokens[0].lexeme, expected);
    }

    #[test]
    fn single_letter() {
        assert_id("x", "x");
    }

    #[test]
    fn multi_char() {
        assert_id("foo", "foo");
    }

    #[test]
    fn with_digits() {
        assert_id("foo42", "foo42");
    }

    #[test]
    fn with_underscore() {
        assert_id("num_aux", "num_aux");
    }

    #[test]
    fn uppercase_first() {
        assert_id("FooBar", "FooBar");
    }
}

mod numbers {
    use super::*;

    fn assert_number(input: &str, expected: &str) {
        let tokens = tokens_without_eof(input);
        assert_eq!(tokens[0].class, TokenClass::Number);
        assert_eq!(tokens[0].lexeme, expected);
    }

    #[test]
    fn single_digit() {
        assert_number("0", "0");
    }

    #[test]
    fn multi_digit() {
        assert_number("123456789", "123456789");
    }
}

mod keywords {
    use super::*;

    fn assert_keyword(input: &str) {
        let tokens = tokens_without_eof(input);
        assert_eq!(tokens[0].class, TokenClass::Keyword);
        assert_eq!(tokens[0].lexeme, input);
    }

    #[test]
    fn all_keywords_recognized() {
        for kw in lexical_analyzer::KEYWORDS_LIST {
            assert_keyword(kw);
        }
    }
}

mod operations_and_delimiters {
    use super::*;

    fn assert_op(input: &str) {
        let tokens = tokens_without_eof(input);
        assert_eq!(tokens[0].class, TokenClass::Operation);
        assert_eq!(tokens[0].lexeme, input);
    }

    fn assert_delim(input: &str) {
        let tokens = tokens_without_eof(input);
        assert_eq!(tokens[0].class, TokenClass::Delimiter);
        assert_eq!(tokens[0].lexeme, input);
    }

    #[test]
    fn operators() {
        for op in ["+", "-", "*", "!", "<", "&&"] {
            assert_op(op);
        }
    }

    #[test]
    fn delimiters() {
        for d in ["{", "}", "(", ")", "[", "]", ";", ",", ".", "="] {
            assert_delim(d);
        }
    }
}

mod longest_match {
    use super::*;

    #[test]
    fn identifier_prefixed_by_keyword() {
        for input in ["class1", "intx", "ifa"] {
            let tokens = tokens_without_eof(input);
            assert_eq!(tokens[0].class, TokenClass::Id);
            assert_eq!(tokens[0].lexeme, input);
        }
    }
}

mod comments {
    use super::*;

    #[test]
    fn line_comment_skipped() {
        let tokens = tokens_without_eof("x // this is ignored\ny");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].lexeme, "x");
        assert_eq!(tokens[1].lexeme, "y");
    }

    #[test]
    fn block_comment_skipped() {
        let tokens = tokens_without_eof("a /* ignored\nmultiline */ b");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].lexeme, "a");
        assert_eq!(tokens[1].lexeme, "b");
    }

    #[test]
    fn block_comment_preserves_line_count() {
        // `b` sits on line 3 after a two-line block comment.
        let tokens = tokens_without_eof("a /* one\ntwo */\nb");
        assert_eq!(tokens[1].lexeme, "b");
        assert_eq!(tokens[1].line, 3);
    }

    #[test]
    fn unclosed_block_comment_is_error() {
        let result = lexical_analyzer::tokenize("a /* never closed", false);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].msg.contains("unclosed"));
    }
}

mod errors {
    use super::*;

    #[test]
    fn unrecognized_character() {
        let result = lexical_analyzer::tokenize("x = 1 @ 2", false);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].line, 1);
        assert_eq!(result.errors[0].column, 7);
    }

    #[test]
    fn collects_all_errors_without_fail_fast() {
        let result = lexical_analyzer::tokenize("@ # $", false);
        assert_eq!(result.errors.len(), 3);
    }

    #[test]
    fn fail_fast_stops_at_first_error() {
        let result = lexical_analyzer::tokenize("@ # $", true);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn malformed_number_identifier() {
        let result = lexical_analyzer::tokenize("12abc", false);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].msg.contains("malformed"));
    }
}

mod interning {
    use super::*;

    #[test]
    fn same_id_shares_pointer() {
        let result = lexical_analyzer::tokenize("x x", false);
        let p0 = result.tokens[0].symbol.unwrap();
        let p1 = result.tokens[1].symbol.unwrap();
        assert_eq!(p0, p1);
        assert_eq!(result.symbols.registers.len(), 1);
    }

    #[test]
    fn distinct_ids_distinct_pointers() {
        let result = lexical_analyzer::tokenize("a b", false);
        assert_ne!(
            result.tokens[0].symbol.unwrap(),
            result.tokens[1].symbol.unwrap()
        );
        assert_eq!(result.symbols.registers.len(), 2);
    }
}

mod line_column {
    use super::*;

    #[test]
    fn first_token_at_1_1() {
        let tokens = tokens_without_eof("foo");
        assert_eq!((tokens[0].line, tokens[0].column), (1, 1));
    }

    #[test]
    fn newline_increments_line_resets_column() {
        let tokens = tokens_without_eof("foo\nbar");
        assert_eq!(tokens[1].line, 2);
        assert_eq!(tokens[1].column, 1);
    }
}

mod eof {
    use super::*;

    #[test]
    fn eof_emitted_at_end() {
        let result = lexical_analyzer::tokenize("x", false);
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(result.tokens[1].class, TokenClass::Eof);
    }

    #[test]
    fn empty_input_yields_only_eof() {
        let result = lexical_analyzer::tokenize("", false);
        assert_eq!(result.tokens.len(), 1);
        assert_eq!(result.tokens[0].class, TokenClass::Eof);
    }
}
