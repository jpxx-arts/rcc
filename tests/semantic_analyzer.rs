//! Integration tests for the semantic analyzer.

use rcc::lexical_analyzer;
use rcc::semantic_analyzer::{self, SemanticError};
use rcc::syntatic_analyzer;

fn analyze(src: &str) -> Vec<SemanticError> {
    let lex = lexical_analyzer::tokenize(src, false);
    assert!(lex.errors.is_empty(), "lexical errors: {:?}", lex.errors);
    let (program, _) = syntatic_analyzer::parse(&lex.tokens).expect("should parse");
    semantic_analyzer::analyze(&program)
}

fn assert_ok(src: &str) {
    let errors = analyze(src);
    assert!(errors.is_empty(), "expected no errors, got {errors:?}");
}

fn assert_err(src: &str, needle: &str) {
    let errors = analyze(src);
    assert!(
        errors.iter().any(|e| e.msg.contains(needle)),
        "expected an error containing {needle:?}, got {errors:?}"
    );
}

/// Wrap a class body in a program with a trivial main.
fn program(classes: &str) -> String {
    format!("class Main {{ public static void main(String[] a) {{ }} }}\n{classes}")
}

#[test]
fn valid_program() {
    assert_ok(&program(
        "class C {
            int v ;
            public int f(int x) {
                int y ;
                y = x + v ;
                return y ;
            }
        }",
    ));
}

#[test]
fn empty_class_is_error() {
    assert_err(&program("class Empty { }"), "is empty");
}

#[test]
fn arithmetic_requires_int() {
    assert_err(
        &program(
            "class C {
                public int f() {
                    int y ;
                    y = true + 1 ;
                    return y ;
                }
            }",
        ),
        "arithmetic operand must be 'int'",
    );
}

#[test]
fn and_requires_boolean() {
    assert_err(
        &program(
            "class C {
                public int f() {
                    boolean b ;
                    b = 1 && true ;
                    return 0 ;
                }
            }",
        ),
        "'&&' operand must be 'boolean'",
    );
}

#[test]
fn assignment_type_mismatch() {
    assert_err(
        &program(
            "class C {
                public int f() {
                    boolean b ;
                    b = 5 ;
                    return 0 ;
                }
            }",
        ),
        "cannot assign",
    );
}

#[test]
fn length_requires_array() {
    assert_err(
        &program(
            "class C {
                public int f() {
                    int n ;
                    n = n.length ;
                    return n ;
                }
            }",
        ),
        "'.length' requires an int array",
    );
}

#[test]
fn array_index_ok_and_typed() {
    assert_ok(&program(
        "class C {
            int[] arr ;
            public int f() {
                int n ;
                n = arr[0] ;
                return n ;
            }
        }",
    ));
}

#[test]
fn new_object_unknown_class() {
    assert_err(
        &program(
            "class C {
                public int f() {
                    C c ;
                    c = new Nope() ;
                    return 0 ;
                }
            }",
        ),
        "unknown class 'Nope'",
    );
}

#[test]
fn undefined_variable() {
    assert_err(
        &program(
            "class C {
                public int f() {
                    return zzz ;
                }
            }",
        ),
        "undefined variable 'zzz'",
    );
}

#[test]
fn inheritance_field_access() {
    // B inherits field `v` from A.
    assert_ok(&program(
        "class A {
            int v ;
            public int getv() { return v ; }
        }
        class B extends A {
            public int f() {
                int y ;
                y = v + 1 ;
                return y ;
            }
        }",
    ));
}

#[test]
fn subclass_assignable_to_parent() {
    assert_ok(&program(
        "class A {
            public int f() { return 0 ; }
        }
        class B extends A {
            public int g() { return 1 ; }
        }
        class C {
            public int use() {
                A a ;
                a = new B() ;
                return a.f() ;
            }
        }",
    ));
}

#[test]
fn override_signature_mismatch() {
    assert_err(
        &program(
            "class A {
                public int f(int x) { return x ; }
            }
            class B extends A {
                public boolean f(int x) { return true ; }
            }",
        ),
        "incompatible signature",
    );
}

#[test]
fn method_call_wrong_arg_count() {
    assert_err(
        &program(
            "class C {
                public int f(int x) { return x ; }
                public int g() { return this.f() ; }
            }",
        ),
        "expects 1 argument",
    );
}

#[test]
fn method_call_arg_type_mismatch() {
    assert_err(
        &program(
            "class C {
                public int f(int x) { return x ; }
                public int g() { return this.f(true) ; }
            }",
        ),
        "not compatible with parameter type",
    );
}

#[test]
fn extends_unknown_class() {
    assert_err(
        &program(
            "class B extends Ghost {
                int v ;
                public int f() { return v ; }
            }",
        ),
        "extends unknown class",
    );
}
