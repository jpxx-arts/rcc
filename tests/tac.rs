//! Integration tests for the three-address code (3AC) generator.

use rcc::lexical_analyzer;
use rcc::semantic_analyzer;
use rcc::symbol_table::SymbolTable;
use rcc::syntatic_analyzer;
use rcc::tac::{self, Instr, Opcode};

/// Compile `src` through the full frontend and generate its 3AC.
fn compile(src: &str) -> (Vec<Instr>, SymbolTable) {
    let lex = lexical_analyzer::tokenize(src, false);
    assert!(lex.errors.is_empty(), "lexical errors: {:?}", lex.errors);
    let (program, mut symbols) = syntatic_analyzer::parse(&lex.tokens).expect("should parse");
    let sem_errors = semantic_analyzer::analyze(&program);
    assert!(sem_errors.is_empty(), "semantic errors: {sem_errors:?}");
    let code = tac::generate(&program, &mut symbols);
    (code, symbols)
}

fn rendered(src: &str) -> String {
    let (code, symbols) = compile(src);
    tac::render(&code, &symbols)
}

fn opcodes(code: &[Instr]) -> Vec<Opcode> {
    code.iter().map(|i| i.op).collect()
}

/// Wrap statements in a minimal main (the grammar allows no declarations
/// there, only commands).
fn main_with(body: &str) -> String {
    format!("class Main {{ public static void main(String[] a) {{ {body} }} }}")
}

/// Wrap declarations + statements in a method body (`int` locals allowed),
/// driven by a trivial main.
fn method_with(decls_and_body: &str) -> String {
    format!(
        "class Main {{ public static void main(String[] a) {{ System.out.println(new C().go()) ; }} }}
         class C {{ public int go() {{ {decls_and_body} return 0 ; }} }}"
    )
}

mod instruction_lists {
    use super::*;

    #[test]
    fn concat_joins_in_order() {
        let a = vec![Instr::new(Opcode::Print, None, Some(0), None)];
        let b = vec![Instr::new(Opcode::Ret, None, Some(1), None)];
        let joined = tac::concat(a, b);
        assert_eq!(opcodes(&joined), vec![Opcode::Print, Opcode::Ret]);
    }

    #[test]
    fn program_is_wrapped_in_begin_end() {
        let (code, _) = compile(&main_with("System.out.println(1) ;"));
        assert_eq!(code.first().map(|i| i.op), Some(Opcode::Begin));
        assert_eq!(code.last().map(|i| i.op), Some(Opcode::End));
    }
}

mod expressions {
    use super::*;

    #[test]
    fn arithmetic_uses_temporaries() {
        let out = rendered(&main_with("System.out.println(1 + 2 * 3) ;"));
        assert!(out.contains("t0 = 2 * 3"), "got:\n{out}");
        assert!(out.contains("t1 = 1 + t0"), "got:\n{out}");
        assert!(out.contains("print t1"), "got:\n{out}");
    }

    #[test]
    fn every_binary_operator_lowers_to_its_opcode() {
        let src = "class Main { public static void main(String[] a) {
            if (1 < 2 && !false) { System.out.println(1 + 2 - 3 * 4) ; }
        } }";
        let (code, _) = compile(src);
        let ops = opcodes(&code);
        for expected in [
            Opcode::Less,
            Opcode::Not,
            Opcode::And,
            Opcode::Add,
            Opcode::Sub,
            Opcode::Mul,
        ] {
            assert!(ops.contains(&expected), "missing {expected:?} in {ops:?}");
        }
    }

    // §4.5 of the spec: left-to-right evaluation with precedence — the 3AC
    // for `1 - 2 + 3` must compute the subtraction first and feed it into
    // the addition (result 2, never 1 - (2 + 3)).
    #[test]
    fn evaluation_is_left_to_right() {
        let out = rendered(&main_with("System.out.println(1 - 2 + 3) ;"));
        assert!(out.contains("t0 = 1 - 2"), "got:\n{out}");
        assert!(out.contains("t1 = t0 + 3"), "got:\n{out}");
    }

    #[test]
    fn temporaries_are_unique() {
        let (code, symbols) = compile(&main_with(
            "System.out.println(1 + 2) ; System.out.println(3 + 4) ;",
        ));
        let temps: Vec<&str> = code
            .iter()
            .filter(|i| i.op == Opcode::Add)
            .map(|i| symbols.name_of(i.res.expect("add has result")))
            .collect();
        assert_eq!(temps.len(), 2);
        assert_ne!(temps[0], temps[1]);
    }

    #[test]
    fn array_operations() {
        let out = rendered(&method_with(
            "int[] v ; int x ;
             v = new int[10] ;
             v[0] = 5 ;
             x = v[0] + v.length ;
             System.out.println(x) ;",
        ));
        assert!(out.contains("= new int[10]"), "got:\n{out}");
        assert!(out.contains("v[0] = 5"), "got:\n{out}");
        assert!(out.contains("= v[0]"), "got:\n{out}");
        assert!(out.contains("= length v"), "got:\n{out}");
    }
}

mod control_flow {
    use super::*;

    #[test]
    fn if_without_else_uses_one_label() {
        let (code, _) = compile(&main_with(
            "if (1 < 2) { System.out.println(1) ; }",
        ));
        let ops = opcodes(&code);
        assert_eq!(ops.iter().filter(|o| **o == Opcode::IfZ).count(), 1);
        assert_eq!(ops.iter().filter(|o| **o == Opcode::Label).count(), 1);
        assert_eq!(ops.iter().filter(|o| **o == Opcode::Jump).count(), 0);
    }

    #[test]
    fn if_else_shape() {
        let out = rendered(&main_with(
            "if (1 < 2) { System.out.println(1) ; } else { System.out.println(2) ; }",
        ));
        assert!(out.contains("ifFalse t0 goto L0"), "got:\n{out}");
        assert!(out.contains("goto L1"), "got:\n{out}");
        assert!(out.contains("L0:"), "got:\n{out}");
        assert!(out.contains("L1:"), "got:\n{out}");
    }

    #[test]
    fn while_loops_back_to_its_condition() {
        let out = rendered(&method_with(
            "int i ; i = 0 ;
             while (i < 10) { i = i + 1 ; }
             System.out.println(i) ;",
        ));
        // L0: cond ; ifFalse ... goto L1 ; body ; goto L0 ; L1:
        assert!(out.contains("L0:"), "got:\n{out}");
        assert!(out.contains("goto L1"), "got:\n{out}");
        assert!(out.contains("goto L0"), "got:\n{out}");
        let l0_def = out.find("L0:").unwrap();
        let jump_back = out.find("goto L0").unwrap();
        assert!(l0_def < jump_back, "loop label must precede the back jump:\n{out}");
    }
}

mod methods_and_calls {
    use super::*;

    const FACTORIAL: &str = "class Factorial {
        public static void main(String[] a) {
            System.out.println(new Fac().ComputeFac(10)) ;
        }
    }
    class Fac {
        public int ComputeFac(int num) {
            int aux ;
            if (num < 1) { aux = 1 ; } else { aux = num * (this.ComputeFac(num - 1)) ; }
            return aux ;
        }
    }";

    #[test]
    fn methods_are_delimited_and_return() {
        let (code, _) = compile(FACTORIAL);
        let ops = opcodes(&code);
        // main + ComputeFac
        assert_eq!(ops.iter().filter(|o| **o == Opcode::Begin).count(), 2);
        assert_eq!(ops.iter().filter(|o| **o == Opcode::End).count(), 2);
        assert_eq!(ops.iter().filter(|o| **o == Opcode::Ret).count(), 1);
    }

    #[test]
    fn call_passes_receiver_then_args_as_params() {
        let out = rendered(FACTORIAL);
        assert!(out.contains("t0 = new Fac"), "got:\n{out}");
        assert!(out.contains("param t0"), "got:\n{out}");
        assert!(out.contains("param 10"), "got:\n{out}");
        assert!(out.contains("call Fac.ComputeFac"), "got:\n{out}");
    }

    #[test]
    fn this_call_resolves_to_the_current_class_method() {
        let out = rendered(FACTORIAL);
        assert!(out.contains("param this"), "got:\n{out}");
        // Recursive call through `this` also references Fac.ComputeFac.
        assert_eq!(out.matches("call Fac.ComputeFac").count(), 2, "got:\n{out}");
    }

    #[test]
    fn inherited_method_call_resolves_through_the_parent() {
        let src = "class Main {
            public static void main(String[] a) {
                System.out.println(new B().f(1)) ;
            }
        }
        class A { public int f(int x) { System.out.println(x) ; return x ; } }
        class B extends A { int unused ; }";
        let out = rendered(src);
        assert!(out.contains("call A.f"), "got:\n{out}");
    }
}

mod symbol_table_integration {
    use super::*;

    #[test]
    fn temps_and_labels_are_registered_in_the_table() {
        let (_, symbols) = compile(&method_with(
            "int i ; i = 0 ;
             while (i < 3) { i = i + 1 ; }
             System.out.println(i) ;",
        ));
        let rendered = symbols.render();
        assert!(rendered.contains("t0"), "got:\n{rendered}");
        assert!(rendered.contains("temp"), "got:\n{rendered}");
        assert!(rendered.contains("L0"), "got:\n{rendered}");
        assert!(rendered.contains("label"), "got:\n{rendered}");
    }

    #[test]
    fn literals_are_interned_once() {
        let (code, symbols) = compile(&main_with(
            "System.out.println(7 + 7) ;",
        ));
        let add = code.iter().find(|i| i.op == Opcode::Add).expect("add");
        assert_eq!(add.op1, add.op2, "same literal must share one entry");
        assert_eq!(symbols.name_of(add.op1.unwrap()), "7");
    }

    #[test]
    fn every_operand_is_a_valid_table_reference() {
        let (code, symbols) = compile(
            "class Main { public static void main(String[] a) {
                System.out.println(new C().go(1, 2)) ;
            } }
            class C { public int go(int x, int y) { System.out.println(x) ; return x + y ; } }",
        );
        for instr in &code {
            for sym in [instr.res, instr.op1, instr.op2].into_iter().flatten() {
                assert!(sym < symbols.entries.len(), "dangling SymRef {sym}");
            }
        }
    }
}
