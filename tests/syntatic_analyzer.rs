//! Integration tests for the recursive-descent parser.
//!
//! Each test feeds a string source through the full pipeline
//! (preprocessor → lexer → parser) and asserts Ok/Err.

use rcc::lexical_analyzer;
use rcc::preprocessor;
use rcc::syntatic_analyzer::{self, BinOp, Expr, ParseError, Program, Stmt};

fn parse_source(src: &str) -> Result<Program, ParseError> {
    let preprocessed = preprocessor::preprocess(src).expect("preprocess should succeed");
    let (tokens, symbol_table) = lexical_analyzer::get_tokens(&preprocessed);
    syntatic_analyzer::parse(&tokens, &symbol_table)
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

    fn parse_in_main(body: &str) -> Result<Program, ParseError> {
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

    fn parse_with_exp(exp: &str) -> Result<Program, ParseError> {
        let src = format!(
            "class Main {{ public static void main(String[] a) {{ x = {exp}; }} }}"
        );
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
        let src = "class Main { public static void main(String[] a) { if true x = 1; else x = 2; } }";
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

mod precedence {
    use super::*;

    /// Extracts the first stmt body of `main`, asserts it's `Assign(_, expr)`,
    /// returns `expr`.
    fn first_assign_rhs(src: &str) -> Expr {
        let prog = parse_source(src).expect("should parse");
        match &prog.main.body[0] {
            Stmt::Assign(_, expr) => expr.clone(),
            other => panic!("expected Assign, got {:?}", other),
        }
    }

    fn wrap_main(body: &str) -> String {
        format!("class M {{ public static void main(String[] a) {{ x = {body}; }} }}")
    }

    /// `a + b * c` → `Add(a, Mul(b, c))`
    #[test]
    fn mul_binds_tighter_than_add() {
        let expr = first_assign_rhs(&wrap_main("a + b * c"));
        match expr {
            Expr::Binary(BinOp::Add, lhs, rhs) => {
                assert_eq!(*lhs, Expr::Id("a".into()));
                match *rhs {
                    Expr::Binary(BinOp::Mul, l, r) => {
                        assert_eq!(*l, Expr::Id("b".into()));
                        assert_eq!(*r, Expr::Id("c".into()));
                    }
                    other => panic!("expected Mul on rhs, got {:?}", other),
                }
            }
            other => panic!("expected Add at root, got {:?}", other),
        }
    }

    /// `a * b + c` → `Add(Mul(a, b), c)`
    #[test]
    fn mul_binds_tighter_than_add_reversed() {
        let expr = first_assign_rhs(&wrap_main("a * b + c"));
        match expr {
            Expr::Binary(BinOp::Add, lhs, rhs) => {
                assert_eq!(*rhs, Expr::Id("c".into()));
                match *lhs {
                    Expr::Binary(BinOp::Mul, l, r) => {
                        assert_eq!(*l, Expr::Id("a".into()));
                        assert_eq!(*r, Expr::Id("b".into()));
                    }
                    other => panic!("expected Mul on lhs, got {:?}", other),
                }
            }
            other => panic!("expected Add at root, got {:?}", other),
        }
    }

    /// `a + b - c` → `Sub(Add(a, b), c)` (left-associative)
    #[test]
    fn add_and_sub_are_left_associative() {
        let expr = first_assign_rhs(&wrap_main("a + b - c"));
        match expr {
            Expr::Binary(BinOp::Sub, lhs, rhs) => {
                assert_eq!(*rhs, Expr::Id("c".into()));
                match *lhs {
                    Expr::Binary(BinOp::Add, _, _) => {} // ok
                    other => panic!("expected Add on lhs, got {:?}", other),
                }
            }
            other => panic!("expected Sub at root, got {:?}", other),
        }
    }

    /// `a < b && c > d` → `And(Lt(a, b), Gt(c, d))`
    #[test]
    fn and_binds_looser_than_relational() {
        let expr = first_assign_rhs(&wrap_main("a < b && c > d"));
        match expr {
            Expr::Binary(BinOp::And, lhs, rhs) => {
                assert!(matches!(*lhs, Expr::Binary(BinOp::Lt, _, _)));
                assert!(matches!(*rhs, Expr::Binary(BinOp::Gt, _, _)));
            }
            other => panic!("expected And at root, got {:?}", other),
        }
    }

    /// `!a && b` → `And(Not(a), b)`
    #[test]
    fn unary_not_binds_tighter_than_and() {
        let expr = first_assign_rhs(&wrap_main("!a && b"));
        match expr {
            Expr::Binary(BinOp::And, lhs, rhs) => {
                assert!(matches!(*lhs, Expr::Not(_)));
                assert_eq!(*rhs, Expr::Id("b".into()));
            }
            other => panic!("expected And at root, got {:?}", other),
        }
    }

    /// Parens override precedence: `(a + b) * c` → `Mul(Add(a, b), c)`
    #[test]
    fn parens_override_precedence() {
        let expr = first_assign_rhs(&wrap_main("(a + b) * c"));
        match expr {
            Expr::Binary(BinOp::Mul, lhs, rhs) => {
                assert!(matches!(*lhs, Expr::Binary(BinOp::Add, _, _)));
                assert_eq!(*rhs, Expr::Id("c".into()));
            }
            other => panic!("expected Mul at root, got {:?}", other),
        }
    }
}

mod ast_shape {
    use super::*;
    use rcc::syntatic_analyzer::{ClassDecl, Type};

    #[test]
    fn main_class_is_captured() {
        let src = "class M { public static void main(String[] argv) { x = 1; } }";
        let prog = parse_source(src).unwrap();
        assert_eq!(prog.main.name, "M");
        assert_eq!(prog.main.args_name, "argv");
        assert_eq!(prog.main.body.len(), 1);
    }

    #[test]
    fn class_decl_extends_is_captured() {
        let src = "class M { public static void main(String[] a) { } }
                   class Child extends Parent { }";
        let prog = parse_source(src).unwrap();
        let child: &ClassDecl = &prog.classes[0];
        assert_eq!(child.name, "Child");
        assert_eq!(child.extends.as_deref(), Some("Parent"));
    }

    #[test]
    fn types_are_captured() {
        let src = "class M { public static void main(String[] a) { } }
                   class F {
                       int x;
                       boolean b;
                       int[] arr;
                       Foo obj;
                       public int get() { return 0; }
                   }";
        let prog = parse_source(src).unwrap();
        let f = &prog.classes[0];
        assert_eq!(f.vars[0].ty, Type::Int);
        assert_eq!(f.vars[1].ty, Type::Boolean);
        assert_eq!(f.vars[2].ty, Type::IntArray);
        assert_eq!(f.vars[3].ty, Type::Class("Foo".to_string()));
    }

    #[test]
    fn method_call_captures_name_and_args() {
        let src =
            "class M { public static void main(String[] a) { x = this.foo(1, 2, 3); } }";
        let prog = parse_source(src).unwrap();
        match &prog.main.body[0] {
            Stmt::Assign(_, Expr::Call(target, name, args)) => {
                assert_eq!(**target, Expr::This);
                assert_eq!(name, "foo");
                assert_eq!(args.len(), 3);
                assert_eq!(args[0], Expr::Number("1".into()));
            }
            other => panic!("expected Assign with Call, got {:?}", other),
        }
    }
}
