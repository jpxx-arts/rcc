//! Integration tests for the recursive-descent parser.
//!
//! Each test feeds a string source through the full pipeline
//! (preprocessor → lexer → parser) and asserts Ok/Err.

use rcc::lexical_analyzer;
use rcc::preprocessor;
use rcc::syntatic_analyzer::{self, ParseError};

fn parse_source(src: &str) -> Result<(), ParseError> {
    let (preprocessed, map) = preprocessor::preprocess(src).expect("preprocess should succeed");
    let (tokens, _symbol_table) = lexical_analyzer::get_tokens_with_map(&preprocessed, Some(&map));
    syntatic_analyzer::parse(&tokens)
}

mod minimal_programs {
    use super::*;

    #[test]
    fn empty_main_only() {
        let src = "class Main { public static void main(String[] a) { } }";
        parse_source(src).unwrap();
    }

    #[test]
    fn main_with_println() {
        let src = "class Main { public static void main(String[] a) {
            System.out.println(1);
        } }";
        parse_source(src).unwrap();
    }

    #[test]
    fn class_with_var_and_method() {
        let src = "class Main { public static void main(String[] a) { } }
        class Foo {
            int x;
            public int get() { return x; }
        }";
        parse_source(src).unwrap();
    }

    #[test]
    fn class_with_extends() {
        let src = "class Main { public static void main(String[] a) { } }
        class Foo extends Bar {
            int x;
            public int get() { return x; }
        }";
        parse_source(src).unwrap();
    }
}

mod statements {
    use super::*;

    fn parse_in_main(body: &str) -> Result<(), ParseError> {
        let src = format!("class Main {{ public static void main(String[] a) {{ {body} }} }}");
        parse_source(&src)
    }

    #[test]
    fn assignment() {
        parse_in_main("x = 1;").unwrap();
    }

    #[test]
    fn array_assignment() {
        parse_in_main("a[0] = 1;").unwrap();
    }

    #[test]
    fn if_else() {
        parse_in_main("if (true) x = 1; else x = 2;").unwrap();
    }

    #[test]
    fn nested_if() {
        parse_in_main("if (true) if (false) x = 1; else x = 2; else x = 3;").unwrap();
    }

    #[test]
    fn while_loop() {
        parse_in_main("while (x > 0) x = x - 1;").unwrap();
    }

    #[test]
    fn nested_block() {
        parse_in_main("{ x = 1; y = 2; }").unwrap();
    }

    #[test]
    fn multiple_statements() {
        parse_in_main("x = 1; y = 2; z = 3;").unwrap();
    }

    #[test]
    fn empty_block() {
        parse_in_main("{ }").unwrap();
    }
}

mod expressions {
    use super::*;

    fn parse_with_exp(exp: &str) -> Result<(), ParseError> {
        let src = format!("class Main {{ public static void main(String[] a) {{ x = {exp}; }} }}");
        parse_source(&src)
    }

    #[test]
    fn literal_number() {
        parse_with_exp("42").unwrap();
    }

    #[test]
    fn literal_true() {
        parse_with_exp("true").unwrap();
    }

    #[test]
    fn literal_false() {
        parse_with_exp("false").unwrap();
    }

    #[test]
    fn identifier() {
        parse_with_exp("y").unwrap();
    }

    #[test]
    fn this_keyword() {
        parse_with_exp("this").unwrap();
    }

    #[test]
    fn paren_expr() {
        parse_with_exp("(1 + 2)").unwrap();
    }

    #[test]
    fn not_expr() {
        parse_with_exp("!true").unwrap();
    }

    #[test]
    fn binary_add() {
        parse_with_exp("1 + 2").unwrap();
    }

    #[test]
    fn binary_mul() {
        parse_with_exp("1 * 2").unwrap();
    }

    #[test]
    fn binary_and() {
        parse_with_exp("true && false").unwrap();
    }

    #[test]
    fn binary_gt() {
        parse_with_exp("a > b").unwrap();
    }

    #[test]
    fn array_index() {
        parse_with_exp("arr[i]").unwrap();
    }

    #[test]
    fn length_access() {
        parse_with_exp("arr.length").unwrap();
    }

    #[test]
    fn method_call_no_args() {
        parse_with_exp("this.foo()").unwrap();
    }

    #[test]
    fn method_call_with_args() {
        parse_with_exp("this.foo(1, 2, 3)").unwrap();
    }

    #[test]
    fn new_object() {
        parse_with_exp("new Foo()").unwrap();
    }

    #[test]
    fn new_int_array() {
        parse_with_exp("new int[10]").unwrap();
    }

    #[test]
    fn complex_chain() {
        // .baz alone isn't a DotRest (only `length` or `Id ( args )`)
        parse_with_exp("new Foo().bar(1).baz()").unwrap();
    }

    #[test]
    fn less_than() {
        parse_with_exp("a < b").unwrap();
    }
}

mod errors {
    use super::*;

    #[test]
    fn missing_semicolon() {
        let src = "class Main { public static void main(String[] a) { x = 1 } }";
        let err = parse_source(src).unwrap_err();
        assert!(err.msg.contains("';'") || err.msg.contains("expected"));
    }

    #[test]
    fn missing_paren() {
        let src =
            "class Main { public static void main(String[] a) { if true x = 1; else x = 2; } }";
        assert!(parse_source(src).is_err());
    }

    #[test]
    fn extra_token() {
        let src = "class Main { public static void main(String[] a) { x = 1; } } extra";
        assert!(parse_source(src).is_err());
    }

    #[test]
    fn error_carries_line_column() {
        let src = "class Main { public static void main(String[] a) { x = 1\n} }";
        let err = parse_source(src).unwrap_err();
        // After preprocessor minifies, the `\n` is preserved. The missing
        // semicolon shows up at line 2 (the `}` token).
        assert!(err.line >= 1, "line should be reported, got {}", err.line);
    }

    #[test]
    fn unexpected_keyword() {
        let src = "class Main { public static void main(String[] a) { return 1; } }";
        assert!(parse_source(src).is_err()); // 'return' is not a Cmd at top
    }
}

mod e2e_fixtures {
    use super::*;

    #[test]
    fn bubblesort_parses() {
        let src =
            std::fs::read_to_string("specs/prog-bubblesort.ling").expect("fixture should exist");
        parse_source(&src).expect("bubblesort should parse");
    }

    #[test]
    fn factorial_parses() {
        let src =
            std::fs::read_to_string("specs/prog-factorial.ling").expect("fixture should exist");
        parse_source(&src).expect("factorial should parse");
    }
}
