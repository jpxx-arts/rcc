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

/// Wrap a class body in a program with a trivial (but non-empty) main.
fn program(classes: &str) -> String {
    format!(
        "class Main {{ public static void main(String[] a) {{ System.out.println(0) ; }} }}\n{classes}"
    )
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
                    int q ;
                    q = 0 ;
                    return zzz ;
                }
            }",
        ),
        "undefined variable 'zzz'",
    );
}

// §4.3 of the spec: name resolution order is (1) method parameters,
// (2) own class fields, (3) inherited fields.
#[test]
fn param_shadows_class_field() {
    // `x` is an `int` field but a `boolean` parameter; using it as boolean
    // must pass, proving the parameter wins.
    assert_ok(&program(
        "class C {
            int x ;
            public boolean f(boolean x) {
                boolean ok ;
                ok = x && true ;
                return ok ;
            }
        }",
    ));
}

#[test]
fn param_shadows_class_field_negative_control() {
    // Same shape, but using `x` with the *field's* type must fail — the
    // parameter (boolean) shadows the int field.
    assert_err(
        &program(
            "class C {
                int x ;
                public int f(boolean x) {
                    int y ;
                    y = x + 1 ;
                    return y ;
                }
            }",
        ),
        "arithmetic operand must be 'int'",
    );
}

#[test]
fn own_field_shadows_inherited_field() {
    // `y` is boolean in A but int in B; inside B it must resolve to int.
    assert_ok(&program(
        "class A {
            boolean y ;
        }
        class B extends A {
            int y ;
            public int f() {
                y = y + 1 ;
                return y ;
            }
        }",
    ));
}

// Params and locals share the method scope: duplicates among them are
// errors (Java semantics per §4.4), unlike the legal cross-scope
// shadowing of §4.3 covered above.
#[test]
fn duplicate_parameter_is_error() {
    assert_err(
        &program(
            "class C {
                public int f(int x, boolean x) {
                    int y ;
                    y = 0 ;
                    return y ;
                }
            }",
        ),
        "parameter 'x' is declared more than once",
    );
}

#[test]
fn duplicate_local_is_error() {
    assert_err(
        &program(
            "class C {
                public int f() {
                    int y ;
                    int y ;
                    y = 0 ;
                    return y ;
                }
            }",
        ),
        "local variable 'y' is declared more than once",
    );
}

#[test]
fn local_redeclaring_parameter_is_error() {
    assert_err(
        &program(
            "class C {
                public int f(int x) {
                    boolean x ;
                    x = true ;
                    return 0 ;
                }
            }",
        ),
        "local variable 'x' redeclares a parameter",
    );
}

// §4.2 of the spec: the professor's dynamic-dispatch example — a
// superclass-typed variable holding a subclass instance calls the
// overridden method through the superclass signature.
#[test]
fn dynamic_dispatch_professor_example() {
    assert_ok(&program(
        "class A {
            public int f() { int r ; r = 1 ; return r ; }
        }
        class B extends A {
            public int f() { int r ; r = 2 ; return r ; }
        }
        class Driver {
            public int run() {
                A x ;
                int res ;
                x = new B() ;
                res = x.f() ;
                return res ;
            }
        }",
    ));
}

#[test]
fn inheritance_field_access() {
    // B inherits field `v` from A.
    assert_ok(&program(
        "class A {
            int v ;
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
            public int f() { int z ; z = 0 ; return z ; }
        }
        class B extends A {
            public int g() { int z ; z = 1 ; return z ; }
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
                public int f(int x) { int z ; z = x ; return z ; }
            }
            class B extends A {
                public boolean f(int x) { boolean z ; z = true ; return z ; }
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
                public int f(int x) { int z ; z = x ; return z ; }
                public int g() { int q ; q = 0 ; return this.f() ; }
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
                public int f(int x) { int z ; z = x ; return z ; }
                public int g() { int q ; q = 0 ; return this.f(true) ; }
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
            }",
        ),
        "extends unknown class",
    );
}
