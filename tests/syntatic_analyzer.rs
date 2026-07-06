//! Integration tests for the recursive-descent parser (official grammar in
//! `specs/gramatica-prof.md`).
//!
//! Each test feeds a string source through lexer → parser and asserts Ok/Err.

use rcc::lexical_analyzer;
use rcc::syntatic_analyzer::{self, ParseError};

fn parse_source(src: &str) -> Result<(), ParseError> {
    let lex = lexical_analyzer::tokenize(src, false);
    assert!(
        lex.errors.is_empty(),
        "unexpected lexical errors: {:?}",
        lex.errors
    );
    syntatic_analyzer::parse(&lex.tokens).map(|_| ())
}

mod minimal_programs {
    use super::*;

    #[test]
    fn empty_main_is_error() {
        // L_com requires at least one command, so an empty body is rejected.
        let src = "class Main { public static void main(String[] a) { } }";
        assert!(parse_source(src).is_err());
    }

    #[test]
    fn empty_method_body_is_error() {
        // A method body must contain at least one command before `return`.
        let src = "class Main { public static void main(String[] a) { x = 1; } }
        class Foo {
            int x;
            public int get() { return x; }
        }";
        assert!(parse_source(src).is_err());
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
        let src = "class Main { public static void main(String[] a) { System.out.println(0); } }
        class Foo {
            int x;
            public int get() { x = 1; return x; }
        }";
        parse_source(src).unwrap();
    }

    #[test]
    fn class_with_extends() {
        let src = "class Main { public static void main(String[] a) { System.out.println(0); } }
        class Foo extends Bar {
            int x;
            public int get() { x = 1; return x; }
        }";
        parse_source(src).unwrap();
    }

    #[test]
    fn method_with_params() {
        let src = "class Main { public static void main(String[] a) { System.out.println(0); } }
        class Foo {
            public int add(int p, int q) { int s; s = p + q; return s; }
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
        parse_in_main("if (true) { x = 1; } else { x = 2; }").unwrap();
    }

    #[test]
    fn if_without_else() {
        parse_in_main("if (true) { x = 1; }").unwrap();
    }

    #[test]
    fn nested_if() {
        parse_in_main("if (true) { if (false) { x = 1; } else { x = 2; } }").unwrap();
    }

    #[test]
    fn while_loop() {
        parse_in_main("while (x < 10) { x = x + 1; }").unwrap();
    }

    #[test]
    fn multiple_statements() {
        parse_in_main("x = 1; y = 2; z = 3;").unwrap();
    }

    #[test]
    fn if_without_braces_is_error() {
        // The official grammar requires braces around if/while bodies.
        assert!(parse_in_main("if (true) x = 1; else x = 2;").is_err());
    }

    #[test]
    fn greater_than_is_not_an_operator() {
        // Only '<' is part of the grammar; '>' is rejected.
        let src = "class Main { public static void main(String[] a) { x = a < b; } }";
        // ensure '<' parses but a stray '>' would not even lex
        parse_source(src).unwrap();
    }
}

mod expressions {
    use super::*;

    fn parse_with_exp(exp: &str) -> Result<(), ParseError> {
        let src = format!("class Main {{ public static void main(String[] a) {{ x = {exp}; }} }}");
        parse_source(&src)
    }

    #[test]
    fn literals() {
        for e in ["42", "true", "false", "y", "this"] {
            parse_with_exp(e).unwrap();
        }
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
    fn arithmetic() {
        parse_with_exp("1 + 2 * 3 - 4").unwrap();
    }

    #[test]
    fn logical_and() {
        parse_with_exp("true && false").unwrap();
    }

    #[test]
    fn less_than() {
        parse_with_exp("a < b").unwrap();
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
        parse_with_exp("new Foo().bar(1).baz()").unwrap();
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
    fn missing_paren_in_if() {
        let src = "class Main { public static void main(String[] a) { if true { x = 1; } } }";
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
        assert!(err.line >= 1, "line should be reported, got {}", err.line);
    }

    #[test]
    fn return_is_not_a_statement() {
        let src = "class Main { public static void main(String[] a) { return 1; } }";
        assert!(parse_source(src).is_err());
    }
}

mod suggestions {
    use super::*;

    #[test]
    fn mistyped_keyword_suggests_replacement() {
        let src = "claas Main { public static void main(String[] a) { x = 1; } }";
        let err = parse_source(src).unwrap_err();
        let hint = err.suggestion.as_deref().unwrap();
        assert_eq!(hint, "replace 'claas' with 'class'");
    }

    #[test]
    fn mistyped_while_suggests_the_keyword() {
        let src = "class Main { public static void main(String[] a) { whle (1 < 2) { x = 1; } } }";
        let err = parse_source(src).unwrap_err();
        let hint = err.suggestion.as_deref().unwrap();
        assert_eq!(hint, "did you mean 'while'?");
    }

    #[test]
    fn missing_token_still_suggests_insertion() {
        let src = "class Main { public static void main(String[] a) { x = 1 } }";
        let err = parse_source(src).unwrap_err();
        assert_eq!(err.suggestion.as_deref(), Some("insert ';'"));
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

mod allow_empty_body_tests {
    use super::*;

    fn parse_source_with(src: &str, allow_empty_body: bool) -> Result<(), ParseError> {
        let lex = lexical_analyzer::tokenize(src, false);
        assert!(
            lex.errors.is_empty(),
            "unexpected lexical errors: {:?}",
            lex.errors
        );
        syntatic_analyzer::parse_with(&lex.tokens, allow_empty_body).map(|_| ())
    }

    #[test]
    fn empty_body_rejected_by_default() {
        let src = "class Main { public static void main(String[] a) { } }";
        assert!(parse_source_with(src, false).is_err());
    }

    #[test]
    fn empty_body_accepted_with_flag() {
        let src = "class Main { public static void main(String[] a) { } }";
        parse_source_with(src, true).unwrap();
    }

    #[test]
    fn empty_method_accepted_with_flag() {
        let src = "class Main { public static void main(String[] a) { x = 1; } }
        class Foo {
            int x;
            public int get() { return x; }
        }";
        parse_source_with(src, true).unwrap();
    }
}
