use rcc::lexical_analyzer::{self, SymbolTable, Token, TokenAttribute, TokenClass};

fn lexeme_of(token: &Token, symbol_table: &SymbolTable) -> String {
    match &token.attribute_value {
        TokenAttribute::Pointer(idx) => symbol_table.registers[*idx].lexeme.clone(),
        TokenAttribute::Itself(lex) => lex.clone(),
        TokenAttribute::Null => match &token.token_name {
            TokenClass::KEYWORD(lex) => lex.clone(),
            TokenClass::EOF => String::new(),
            other => panic!("Null attribute on unexpected class: {:?}", other),
        },
    }
}

/// Returns all tokens except the trailing EOF.
fn tokens_without_eof(input: &str) -> (Vec<Token>, SymbolTable) {
    let (mut tokens, symbol_table) = lexical_analyzer::get_tokens(input);
    if let Some(last) = tokens.last() {
        if last.token_name == TokenClass::EOF {
            tokens.pop();
        }
    }
    (tokens, symbol_table)
}

mod identifiers {
    use super::*;

    fn assert_id(input: &str, expected_lexeme: &str) {
        let (tokens, symbol_table) = tokens_without_eof(input);
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), expected_lexeme);
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
    fn with_trailing_underscore() {
        assert_id("a_", "a_");
    }

    #[test]
    fn uppercase_first() {
        assert_id("Foo", "Foo");
    }

    #[test]
    fn mixed_case() {
        assert_id("FooBar", "FooBar");
    }
}

mod numbers {
    use super::*;

    fn assert_number(input: &str, expected_lexeme: &str) {
        let (tokens, symbol_table) = tokens_without_eof(input);
        assert_eq!(tokens[0].token_name, TokenClass::NUMBER);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), expected_lexeme);
    }

    #[test]
    fn single_digit() {
        assert_number("0", "0");
    }

    #[test]
    fn multi_digit() {
        assert_number("42", "42");
    }

    #[test]
    fn long_number() {
        assert_number("123456789", "123456789");
    }
}

mod keywords {
    use super::*;

    fn assert_keyword(input: &str, expected: &str) {
        let (tokens, _) = tokens_without_eof(input);
        assert_eq!(
            tokens[0].token_name,
            TokenClass::KEYWORD(expected.to_string())
        );
    }

    #[test]
    fn class() {
        assert_keyword("class", "class");
    }

    #[test]
    fn int() {
        assert_keyword("int", "int");
    }

    #[test]
    fn if_keyword() {
        assert_keyword("if", "if");
    }

    #[test]
    fn while_keyword() {
        assert_keyword("while", "while");
    }

    #[test]
    fn return_keyword() {
        assert_keyword("return", "return");
    }

    #[test]
    fn true_keyword() {
        assert_keyword("true", "true");
    }

    #[test]
    fn this_keyword() {
        assert_keyword("this", "this");
    }
}

mod operations {
    use super::*;

    fn assert_op(input: &str, expected_lexeme: &str) {
        let (tokens, _) = tokens_without_eof(input);
        assert_eq!(tokens[0].token_name, TokenClass::OPERATION);
        match &tokens[0].attribute_value {
            TokenAttribute::Itself(lex) => assert_eq!(lex, expected_lexeme),
            other => panic!("expected Itself, got {:?}", other),
        }
    }

    #[test]
    fn plus() {
        assert_op("+", "+");
    }

    #[test]
    fn minus() {
        assert_op("-", "-");
    }

    #[test]
    fn star() {
        assert_op("*", "*");
    }

    #[test]
    fn greater_than() {
        assert_op(">", ">");
    }

    #[test]
    fn not() {
        assert_op("!", "!");
    }

    #[test]
    fn and_and() {
        assert_op("&&", "&&");
    }
}

mod delimiters {
    use super::*;

    fn assert_delim(input: &str, expected_lexeme: &str) {
        let (tokens, _) = tokens_without_eof(input);
        assert_eq!(tokens[0].token_name, TokenClass::DELIMITER);
        match &tokens[0].attribute_value {
            TokenAttribute::Itself(lex) => assert_eq!(lex, expected_lexeme),
            other => panic!("expected Itself, got {:?}", other),
        }
    }

    #[test]
    fn left_brace() {
        assert_delim("{", "{");
    }

    #[test]
    fn right_brace() {
        assert_delim("}", "}");
    }

    #[test]
    fn left_paren() {
        assert_delim("(", "(");
    }

    #[test]
    fn right_paren() {
        assert_delim(")", ")");
    }

    #[test]
    fn left_bracket() {
        assert_delim("[", "[");
    }

    #[test]
    fn right_bracket() {
        assert_delim("]", "]");
    }

    #[test]
    fn semicolon() {
        assert_delim(";", ";");
    }

    #[test]
    fn comma() {
        assert_delim(",", ",");
    }

    #[test]
    fn dot() {
        assert_delim(".", ".");
    }

    #[test]
    fn equals() {
        assert_delim("=", "=");
    }
}

mod longest_match_disambiguation {
    use super::*;

    #[test]
    fn id_starting_with_keyword_class() {
        let (tokens, symbol_table) = tokens_without_eof("class1");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "class1");
    }

    #[test]
    fn id_starting_with_keyword_int() {
        let (tokens, symbol_table) = tokens_without_eof("intx");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "intx");
    }

    #[test]
    fn id_starting_with_keyword_if() {
        let (tokens, symbol_table) = tokens_without_eof("ifa");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "ifa");
    }

    #[test]
    fn keyword_alone_is_keyword() {
        let (tokens, _) = tokens_without_eof("int");
        assert_eq!(tokens[0].token_name, TokenClass::KEYWORD("int".to_string()));
    }
}

mod multi_token_sequences {
    use super::*;

    #[test]
    fn keyword_then_id() {
        let (tokens, symbol_table) = tokens_without_eof("int x");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_name, TokenClass::KEYWORD("int".to_string()));
        assert_eq!(tokens[1].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "x");
    }

    #[test]
    fn declaration_with_initializer() {
        let (tokens, symbol_table) = tokens_without_eof("int a=2;");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].token_name, TokenClass::KEYWORD("int".to_string()));
        assert_eq!(tokens[1].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "a");
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "=");
        assert_eq!(tokens[3].token_name, TokenClass::NUMBER);
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), "2");
        assert_eq!(lexeme_of(&tokens[4], &symbol_table), ";");
    }

    #[test]
    fn arithmetic_expression() {
        let (tokens, symbol_table) = tokens_without_eof("a+b*c");
        assert_eq!(tokens.len(), 5);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "a");
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "+");
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "b");
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), "*");
        assert_eq!(lexeme_of(&tokens[4], &symbol_table), "c");
    }

    #[test]
    fn boolean_expression() {
        let (tokens, symbol_table) = tokens_without_eof("a&&b");
        assert_eq!(tokens.len(), 3);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "a");
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "&&");
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "b");
    }

    #[test]
    fn nested_calls() {
        let (tokens, symbol_table) = tokens_without_eof("foo(bar)");
        assert_eq!(tokens.len(), 4);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "foo");
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "(");
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "bar");
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), ")");
    }

    #[test]
    fn class_block() {
        let (tokens, symbol_table) = tokens_without_eof("class Foo{}");
        assert_eq!(tokens.len(), 4);
        assert_eq!(
            tokens[0].token_name,
            TokenClass::KEYWORD("class".to_string())
        );
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "Foo");
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "{");
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), "}");
    }

    #[test]
    fn full_method_call() {
        let (tokens, symbol_table) = tokens_without_eof("System.out.println(x)");
        assert_eq!(tokens.len(), 8);
        assert_eq!(
            tokens[0].token_name,
            TokenClass::KEYWORD("System".to_string())
        );
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), ".");
        assert_eq!(
            tokens[2].token_name,
            TokenClass::KEYWORD("out".to_string())
        );
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), ".");
        assert_eq!(
            tokens[4].token_name,
            TokenClass::KEYWORD("println".to_string())
        );
        assert_eq!(lexeme_of(&tokens[5], &symbol_table), "(");
        assert_eq!(lexeme_of(&tokens[6], &symbol_table), "x");
        assert_eq!(lexeme_of(&tokens[7], &symbol_table), ")");
    }
}

mod whitespace_handling {
    use super::*;

    #[test]
    fn newline_skipped_before_token() {
        let (tokens, symbol_table) = tokens_without_eof("\nfoo");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "foo");
    }

    #[test]
    fn multiple_newlines_skipped() {
        let (tokens, symbol_table) = tokens_without_eof("\n\n\nfoo");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "foo");
    }

    #[test]
    fn newline_between_tokens() {
        let (tokens, _) = tokens_without_eof("int\nx");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_name, TokenClass::KEYWORD("int".to_string()));
        assert_eq!(tokens[1].token_name, TokenClass::ID);
    }
}

mod symbol_table_interning {
    use super::*;

    #[test]
    fn two_uses_of_same_id_share_pointer() {
        // `x x` should produce 2 tokens with the same POINTER value.
        let (tokens, symbol_table) = tokens_without_eof("x x");
        assert_eq!(tokens.len(), 2);
        let ptr0 = match &tokens[0].attribute_value {
            TokenAttribute::Pointer(idx) => *idx,
            other => panic!("expected Pointer, got {:?}", other),
        };
        let ptr1 = match &tokens[1].attribute_value {
            TokenAttribute::Pointer(idx) => *idx,
            other => panic!("expected Pointer, got {:?}", other),
        };
        assert_eq!(ptr0, ptr1, "same lexeme should produce same pointer");
        assert_eq!(symbol_table.registers.len(), 1, "only one entry interned");
    }

    #[test]
    fn distinct_ids_get_distinct_pointers() {
        let (tokens, symbol_table) = tokens_without_eof("a b");
        assert_eq!(tokens.len(), 2);
        let ptr0 = match &tokens[0].attribute_value {
            TokenAttribute::Pointer(idx) => *idx,
            _ => panic!(),
        };
        let ptr1 = match &tokens[1].attribute_value {
            TokenAttribute::Pointer(idx) => *idx,
            _ => panic!(),
        };
        assert_ne!(ptr0, ptr1);
        assert_eq!(symbol_table.registers.len(), 2);
    }
}

mod line_and_column_tracking {
    use super::*;

    #[test]
    fn first_token_starts_at_line_1_column_1() {
        let (tokens, _) = tokens_without_eof("foo");
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);
    }

    #[test]
    fn newline_increments_line_resets_column() {
        let (tokens, _) = tokens_without_eof("foo\nbar");
        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[1].line, 2);
        assert_eq!(tokens[1].column, 1);
    }
}

mod eof_token {
    use super::*;

    #[test]
    fn eof_is_emitted_at_end() {
        let (tokens, _) = lexical_analyzer::get_tokens("x");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1].token_name, TokenClass::EOF);
    }

    #[test]
    fn empty_input_yields_only_eof() {
        let (tokens, _) = lexical_analyzer::get_tokens("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_name, TokenClass::EOF);
    }
}
