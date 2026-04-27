use rcc::lexical_analyzer::{self, SymbolTable, Token, TokenAttribute, TokenClass};

fn lexeme_of(token: &Token, symbol_table: &SymbolTable) -> String {
    match &token.attribute_value {
        TokenAttribute::POINTER { pointer } => symbol_table.registers[*pointer].lexeme.clone(),
        TokenAttribute::ITSELF(lex) => lex.clone(),
        TokenAttribute::NULL => match &token.token_name {
            TokenClass::KEYWORD(lex) => lex.clone(),
            other => panic!("NULL attribute on non-keyword: {:?}", other),
        },
    }
}

mod identifiers {
    use super::*;

    fn assert_id(input: &str, expected_lexeme: &str) {
        let (tokens, symbol_table) = lexical_analyzer::get_tokens(input);
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
        let (tokens, symbol_table) = lexical_analyzer::get_tokens(input);
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
        let (tokens, _) = lexical_analyzer::get_tokens(input);
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
        let (tokens, _) = lexical_analyzer::get_tokens(input);
        assert_eq!(tokens[0].token_name, TokenClass::OPERATION);
        match &tokens[0].attribute_value {
            TokenAttribute::ITSELF(lex) => assert_eq!(lex, expected_lexeme),
            other => panic!("expected ITSELF, got {:?}", other),
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
        let (tokens, _) = lexical_analyzer::get_tokens(input);
        assert_eq!(tokens[0].token_name, TokenClass::DELIMITER);
        match &tokens[0].attribute_value {
            TokenAttribute::ITSELF(lex) => assert_eq!(lex, expected_lexeme),
            other => panic!("expected ITSELF, got {:?}", other),
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
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("class1");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "class1");
    }

    #[test]
    fn id_starting_with_keyword_int() {
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("intx");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "intx");
    }

    #[test]
    fn id_starting_with_keyword_if() {
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("ifa");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "ifa");
    }

    #[test]
    fn keyword_alone_is_keyword() {
        let (tokens, _) = lexical_analyzer::get_tokens("int");
        assert_eq!(tokens[0].token_name, TokenClass::KEYWORD("int".to_string()));
    }
}

mod multi_token_sequences {
    use super::*;

    #[test]
    fn keyword_then_id() {
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("int x");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_name, TokenClass::KEYWORD("int".to_string()));
        assert_eq!(tokens[1].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "x");
    }

    #[test]
    fn declaration_with_initializer() {
        // int a=2; → [KEYWORD(int), ID(a), DELIM(=), NUMBER(2), DELIM(;)]
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("int a=2;");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].token_name, TokenClass::KEYWORD("int".to_string()));
        assert_eq!(tokens[1].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "a");
        assert_eq!(tokens[2].token_name, TokenClass::DELIMITER);
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "=");
        assert_eq!(tokens[3].token_name, TokenClass::NUMBER);
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), "2");
        assert_eq!(tokens[4].token_name, TokenClass::DELIMITER);
        assert_eq!(lexeme_of(&tokens[4], &symbol_table), ";");
    }

    #[test]
    fn arithmetic_expression() {
        // a+b*c → [ID(a), OP(+), ID(b), OP(*), ID(c)]
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("a+b*c");
        assert_eq!(tokens.len(), 5);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "a");
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "+");
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "b");
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), "*");
        assert_eq!(lexeme_of(&tokens[4], &symbol_table), "c");
    }

    #[test]
    fn boolean_expression() {
        // a&&b → [ID(a), OP(&&), ID(b)]
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("a&&b");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "a");
        assert_eq!(tokens[1].token_name, TokenClass::OPERATION);
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "&&");
        assert_eq!(tokens[2].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "b");
    }

    #[test]
    fn nested_calls() {
        // foo(bar) → [ID(foo), DELIM((), ID(bar), DELIM())]
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("foo(bar)");
        assert_eq!(tokens.len(), 4);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "foo");
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "(");
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "bar");
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), ")");
    }

    #[test]
    fn class_block() {
        // class Foo{} → [KEYWORD(class), ID(Foo), DELIM({), DELIM(})]
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("class Foo{}");
        assert_eq!(tokens.len(), 4);
        assert_eq!(
            tokens[0].token_name,
            TokenClass::KEYWORD("class".to_string())
        );
        assert_eq!(tokens[1].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "Foo");
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "{");
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), "}");
    }

    #[test]
    fn array_access() {
        // a[0] → [ID(a), DELIM([), NUMBER(0), DELIM(])]
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("a[0]");
        assert_eq!(tokens.len(), 4);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "a");
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "[");
        assert_eq!(lexeme_of(&tokens[2], &symbol_table), "0");
        assert_eq!(lexeme_of(&tokens[3], &symbol_table), "]");
    }

    #[test]
    fn member_access() {
        // this.length → [KEYWORD(this), DELIM(.), KEYWORD(length)]
        let (tokens, _) = lexical_analyzer::get_tokens("this.length");
        assert_eq!(tokens.len(), 3);
        assert_eq!(
            tokens[0].token_name,
            TokenClass::KEYWORD("this".to_string())
        );
        assert_eq!(tokens[1].token_name, TokenClass::DELIMITER);
        assert_eq!(
            tokens[2].token_name,
            TokenClass::KEYWORD("length".to_string())
        );
    }

    #[test]
    fn unary_not() {
        // !x → [OP(!), ID(x)]
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("!x");
        assert_eq!(tokens.len(), 2);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "!");
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "x");
    }

    #[test]
    fn full_method_call() {
        // System.out.println(x) → 8 tokens
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("System.out.println(x)");
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
        assert_eq!(tokens[6].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[6], &symbol_table), "x");
        assert_eq!(lexeme_of(&tokens[7], &symbol_table), ")");
    }
}

mod whitespace_handling {
    use super::*;

    #[test]
    fn newline_skipped_before_token() {
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("\nfoo");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "foo");
    }

    #[test]
    fn multiple_newlines_skipped() {
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("\n\n\nfoo");
        assert_eq!(tokens[0].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[0], &symbol_table), "foo");
    }

    #[test]
    fn newline_between_tokens() {
        // int\nx → [KEYWORD(int), ID(x)]
        let (tokens, symbol_table) = lexical_analyzer::get_tokens("int\nx");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].token_name, TokenClass::KEYWORD("int".to_string()));
        assert_eq!(tokens[1].token_name, TokenClass::ID);
        assert_eq!(lexeme_of(&tokens[1], &symbol_table), "x");
    }
}
