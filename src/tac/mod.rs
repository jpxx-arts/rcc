//! Three-address code (3AC) intermediate representation and generator.
//!
//! Implements the backend required by `specs/trabalho3.md`:
//!
//! * [`Instr`] — one 3AC instruction: an operation code plus up to three
//!   references ([`SymRef`]) into the scoped [`SymbolTable`] (result and two
//!   operands). Instructions are chained in plain `Vec<Instr>` lists, with
//!   [`concat`] and [`render`] as the list helpers.
//! * Temporaries and labels are inserted into the symbol table by
//!   [`SymbolTable::new_temp`] and [`SymbolTable::new_label`].
//! * [`generate`] — walks the AST bottom-up: children are generated first,
//!   their code fragments are concatenated, and the instructions for the
//!   current node are appended, returning the complete fragment for the node.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::ast::{ClassDecl, Exp, ExpKind, MethodDecl, Program, Stmt};
use crate::symbol_table::{SymRef, SymbolTable};

/// Operation code of a 3AC instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    /// `res = op1`
    Move,
    /// `res = op1 + op2`
    Add,
    /// `res = op1 - op2`
    Sub,
    /// `res = op1 * op2`
    Mul,
    /// `res = op1 < op2`
    Less,
    /// `res = op1 && op2`
    And,
    /// `res = ! op1`
    Not,
    /// `op1:` — bind the label to this point in the code
    Label,
    /// `goto op1`
    Jump,
    /// `ifFalse op1 goto op2`
    IfZ,
    /// `param op1` — push one call argument (receiver first)
    Param,
    /// `res = call op1` — invoke method `op1` with the pushed params
    Call,
    /// `return op1`
    Ret,
    /// `print op1`
    Print,
    /// `res = op1[op2]`
    ArrayLoad,
    /// `res[op1] = op2`
    ArrayStore,
    /// `res = new int[op1]`
    NewArray,
    /// `res = new op1` — instantiate class `op1`
    NewObject,
    /// `res = length op1`
    Length,
    /// `begin op1` — method (or main) prologue
    Begin,
    /// `end op1` — method (or main) epilogue
    End,
}

/// One three-address instruction: an opcode and up to three symbol-table
/// references (a result address and two operand addresses).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instr {
    pub op: Opcode,
    pub res: Option<SymRef>,
    pub op1: Option<SymRef>,
    pub op2: Option<SymRef>,
}

impl Instr {
    pub fn new(op: Opcode, res: Option<SymRef>, op1: Option<SymRef>, op2: Option<SymRef>) -> Self {
        Instr { op, res, op1, op2 }
    }
}

/// A chainable list of 3AC instructions.
pub type InstrList = Vec<Instr>;

/// Concatenate two instruction lists, returning `a ++ b`.
pub fn concat(mut a: InstrList, b: InstrList) -> InstrList {
    a.extend(b);
    a
}

/// Render an instruction list as readable three-address code, resolving the
/// symbol references against `symbols`.
pub fn render(code: &[Instr], symbols: &SymbolTable) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "three-address code:");
    for instr in code {
        let _ = writeln!(out, "{}", render_instr(instr, symbols));
    }
    out
}

fn render_instr(instr: &Instr, symbols: &SymbolTable) -> String {
    let name = |sym: Option<SymRef>| sym.map(|s| symbols.name_of(s).to_string()).unwrap_or_default();
    let res = name(instr.res);
    let op1 = name(instr.op1);
    let op2 = name(instr.op2);
    match instr.op {
        Opcode::Move => format!("  {res} = {op1}"),
        Opcode::Add => format!("  {res} = {op1} + {op2}"),
        Opcode::Sub => format!("  {res} = {op1} - {op2}"),
        Opcode::Mul => format!("  {res} = {op1} * {op2}"),
        Opcode::Less => format!("  {res} = {op1} < {op2}"),
        Opcode::And => format!("  {res} = {op1} && {op2}"),
        Opcode::Not => format!("  {res} = ! {op1}"),
        Opcode::Label => format!("{op1}:"),
        Opcode::Jump => format!("  goto {op1}"),
        Opcode::IfZ => format!("  ifFalse {op1} goto {op2}"),
        Opcode::Param => format!("  param {op1}"),
        Opcode::Call => format!("  {res} = call {}", instr.op1.map(|s| symbols.qualified_name(s)).unwrap_or_default()),
        Opcode::Ret => format!("  return {op1}"),
        Opcode::Print => format!("  print {op1}"),
        Opcode::ArrayLoad => format!("  {res} = {op1}[{op2}]"),
        Opcode::ArrayStore => format!("  {res}[{op1}] = {op2}"),
        Opcode::NewArray => format!("  {res} = new int[{op1}]"),
        Opcode::NewObject => format!("  {res} = new {op1}"),
        Opcode::Length => format!("  {res} = length {op1}"),
        Opcode::Begin => format!("begin {}", instr.op1.map(|s| symbols.qualified_name(s)).unwrap_or_default()),
        Opcode::End => format!("end {}\n", instr.op1.map(|s| symbols.qualified_name(s)).unwrap_or_default()),
    }
}

/// Generate the complete 3AC for `program`, inserting the temporaries, labels
/// and interned literals it needs into `symbols` (the same table populated by
/// the parser, so every operand of every instruction is a table entry).
pub fn generate(program: &Program, symbols: &mut SymbolTable) -> InstrList {
    let class_map: HashMap<&str, &ClassDecl> = program
        .classes
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let mut generator = Generator {
        symbols,
        class_map,
        env: HashMap::new(),
        scope: String::new(),
        current_class: None,
    };
    generator.gen_program(program)
}

struct Generator<'a> {
    symbols: &'a mut SymbolTable,
    class_map: HashMap<&'a str, &'a ClassDecl>,
    /// Variables visible in the method being generated: fields (own and
    /// inherited), parameters and locals, each mapped to its table entry.
    env: HashMap<String, SymRef>,
    /// Scope temporaries and labels are created under.
    scope: String,
    /// Class of the method being generated (`None` inside `main`); used to
    /// resolve `this.m(...)` calls to the method's table entry.
    current_class: Option<String>,
}

impl Generator<'_> {
    fn gen_program(&mut self, program: &Program) -> InstrList {
        let main_scope = format!("global::{}::main", program.main.name);
        let main_sym = self
            .symbols
            .lookup("global", &program.main.name)
            .unwrap_or_else(|| self.symbols.intern_const(&program.main.name, "-"));

        self.scope = main_scope;
        self.env = HashMap::new();
        self.current_class = None;

        let mut code = vec![Instr::new(Opcode::Begin, None, Some(main_sym), None)];
        for stmt in &program.main.body {
            code = concat(code, self.gen_stmt(stmt));
        }
        code.push(Instr::new(Opcode::End, None, Some(main_sym), None));

        for class in &program.classes {
            for method in &class.methods {
                code = concat(code, self.gen_method(class, method));
            }
        }
        code
    }

    fn gen_method(&mut self, class: &ClassDecl, method: &MethodDecl) -> InstrList {
        self.scope = format!("global::{}::{}", class.name, method.name);
        self.env = self.build_method_env(class, method);
        self.current_class = Some(class.name.clone());

        let method_sym = self
            .symbols
            .lookup(&format!("global::{}", class.name), &method.name)
            .unwrap_or_else(|| self.symbols.intern_const(&method.name, "-"));

        let mut code = vec![Instr::new(Opcode::Begin, None, Some(method_sym), None)];
        for stmt in &method.body {
            code = concat(code, self.gen_stmt(stmt));
        }
        let (ret_code, ret_sym) = self.gen_exp(&method.ret_expr);
        code = concat(code, ret_code);
        code.push(Instr::new(Opcode::Ret, None, Some(ret_sym), None));
        code.push(Instr::new(Opcode::End, None, Some(method_sym), None));
        code
    }

    /// Map every variable visible inside `method` to its symbol-table entry:
    /// inherited fields first (so own declarations shadow them), then the
    /// class's own fields, then parameters and locals.
    fn build_method_env(&mut self, class: &ClassDecl, method: &MethodDecl) -> HashMap<String, SymRef> {
        let mut env = HashMap::new();

        let mut chain = Vec::new();
        let mut current = Some(class.name.as_str());
        let mut guard = 0;
        while let Some(name) = current {
            guard += 1;
            if guard > self.class_map.len() + 1 {
                break; // inheritance cycle; already rejected by semantics
            }
            chain.push(name);
            current = self
                .class_map
                .get(name)
                .and_then(|c| c.parent.as_deref());
        }
        for class_name in chain.iter().rev() {
            let scope = format!("global::{class_name}");
            if let Some(decl) = self.class_map.get(class_name) {
                for field in &decl.fields {
                    if let Some(sym) = self.symbols.lookup(&scope, &field.name) {
                        env.insert(field.name.clone(), sym);
                    }
                }
            }
        }

        let method_scope = format!("global::{}::{}", class.name, method.name);
        for var in method.params.iter().chain(&method.locals) {
            if let Some(sym) = self.symbols.lookup(&method_scope, &var.name) {
                env.insert(var.name.clone(), sym);
            }
        }
        env
    }

    /// Resolve a variable use to its table entry. Semantic analysis has
    /// already validated the program, so a miss only happens on undeclared
    /// names in unchecked runs; interning keeps generation total.
    fn var(&mut self, name: &str) -> SymRef {
        if let Some(&sym) = self.env.get(name) {
            return sym;
        }
        if let Some(sym) = self.symbols.resolve(&self.scope, name) {
            return sym;
        }
        self.symbols.intern_const(name, "-")
    }

    fn temp(&mut self, ty: &str) -> SymRef {
        self.symbols.new_temp(&self.scope, ty)
    }

    fn label(&mut self) -> SymRef {
        self.symbols.new_label(&self.scope)
    }

    // ---------- statements ----------

    fn gen_stmt(&mut self, stmt: &Stmt) -> InstrList {
        match stmt {
            Stmt::Assign { name, value, .. } => {
                let (code, value_sym) = self.gen_exp(value);
                let target = self.var(name);
                concat(
                    code,
                    vec![Instr::new(Opcode::Move, Some(target), Some(value_sym), None)],
                )
            }
            Stmt::ArrayAssign {
                name, index, value, ..
            } => {
                let (index_code, index_sym) = self.gen_exp(index);
                let (value_code, value_sym) = self.gen_exp(value);
                let array = self.var(name);
                let mut code = concat(index_code, value_code);
                code.push(Instr::new(
                    Opcode::ArrayStore,
                    Some(array),
                    Some(index_sym),
                    Some(value_sym),
                ));
                code
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => self.gen_if(cond, then_body, else_body.as_deref()),
            Stmt::While { cond, body, .. } => self.gen_while(cond, body),
            Stmt::Println { value, .. } => {
                let (mut code, value_sym) = self.gen_exp(value);
                code.push(Instr::new(Opcode::Print, None, Some(value_sym), None));
                code
            }
        }
    }

    // ifFalse cond goto L_else ; then ; goto L_end ; L_else: ; else ; L_end:
    fn gen_if(&mut self, cond: &Exp, then_body: &[Stmt], else_body: Option<&[Stmt]>) -> InstrList {
        let (mut code, cond_sym) = self.gen_exp(cond);
        match else_body {
            Some(else_body) => {
                let l_else = self.label();
                let l_end = self.label();
                code.push(Instr::new(Opcode::IfZ, None, Some(cond_sym), Some(l_else)));
                for s in then_body {
                    code = concat(code, self.gen_stmt(s));
                }
                code.push(Instr::new(Opcode::Jump, None, Some(l_end), None));
                code.push(Instr::new(Opcode::Label, None, Some(l_else), None));
                for s in else_body {
                    code = concat(code, self.gen_stmt(s));
                }
                code.push(Instr::new(Opcode::Label, None, Some(l_end), None));
            }
            None => {
                let l_end = self.label();
                code.push(Instr::new(Opcode::IfZ, None, Some(cond_sym), Some(l_end)));
                for s in then_body {
                    code = concat(code, self.gen_stmt(s));
                }
                code.push(Instr::new(Opcode::Label, None, Some(l_end), None));
            }
        }
        code
    }

    // L_begin: ; ifFalse cond goto L_end ; body ; goto L_begin ; L_end:
    fn gen_while(&mut self, cond: &Exp, body: &[Stmt]) -> InstrList {
        let l_begin = self.label();
        let l_end = self.label();
        let mut code = vec![Instr::new(Opcode::Label, None, Some(l_begin), None)];
        let (cond_code, cond_sym) = self.gen_exp(cond);
        code = concat(code, cond_code);
        code.push(Instr::new(Opcode::IfZ, None, Some(cond_sym), Some(l_end)));
        for s in body {
            code = concat(code, self.gen_stmt(s));
        }
        code.push(Instr::new(Opcode::Jump, None, Some(l_begin), None));
        code.push(Instr::new(Opcode::Label, None, Some(l_end), None));
        code
    }

    // ---------- expressions ----------

    /// Generate code for `exp`, returning the instruction fragment and the
    /// symbol holding the expression's value.
    fn gen_exp(&mut self, exp: &Exp) -> (InstrList, SymRef) {
        match &exp.kind {
            ExpKind::Num(n) => (Vec::new(), self.symbols.intern_const(n, "int")),
            ExpKind::True => (Vec::new(), self.symbols.intern_const("true", "boolean")),
            ExpKind::False => (Vec::new(), self.symbols.intern_const("false", "boolean")),
            ExpKind::This => (Vec::new(), self.symbols.intern_const("this", "-")),
            ExpKind::Id(name) => (Vec::new(), self.var(name)),
            ExpKind::Add(a, b) => self.gen_binary(Opcode::Add, a, b, "int"),
            ExpKind::Sub(a, b) => self.gen_binary(Opcode::Sub, a, b, "int"),
            ExpKind::Mul(a, b) => self.gen_binary(Opcode::Mul, a, b, "int"),
            ExpKind::Less(a, b) => self.gen_binary(Opcode::Less, a, b, "boolean"),
            ExpKind::And(a, b) => self.gen_binary(Opcode::And, a, b, "boolean"),
            ExpKind::Not(a) => {
                let (code, a_sym) = self.gen_exp(a);
                let res = self.temp("boolean");
                (
                    concat(
                        code,
                        vec![Instr::new(Opcode::Not, Some(res), Some(a_sym), None)],
                    ),
                    res,
                )
            }
            ExpKind::Index { array, index } => {
                let (array_code, array_sym) = self.gen_exp(array);
                let (index_code, index_sym) = self.gen_exp(index);
                let res = self.temp("int");
                let mut code = concat(array_code, index_code);
                code.push(Instr::new(
                    Opcode::ArrayLoad,
                    Some(res),
                    Some(array_sym),
                    Some(index_sym),
                ));
                (code, res)
            }
            ExpKind::Length(a) => {
                let (mut code, a_sym) = self.gen_exp(a);
                let res = self.temp("int");
                code.push(Instr::new(Opcode::Length, Some(res), Some(a_sym), None));
                (code, res)
            }
            ExpKind::NewArray(size) => {
                let (mut code, size_sym) = self.gen_exp(size);
                let res = self.temp("int[]");
                code.push(Instr::new(Opcode::NewArray, Some(res), Some(size_sym), None));
                (code, res)
            }
            ExpKind::NewObject(name) => {
                let class_sym = self
                    .symbols
                    .lookup("global", name)
                    .unwrap_or_else(|| self.symbols.intern_const(name, "-"));
                let res = self.temp(name);
                (
                    vec![Instr::new(Opcode::NewObject, Some(res), Some(class_sym), None)],
                    res,
                )
            }
            ExpKind::Call {
                receiver,
                method,
                args,
            } => self.gen_call(receiver, method, args),
        }
    }

    fn gen_binary(&mut self, op: Opcode, a: &Exp, b: &Exp, ty: &str) -> (InstrList, SymRef) {
        let (a_code, a_sym) = self.gen_exp(a);
        let (b_code, b_sym) = self.gen_exp(b);
        let res = self.temp(ty);
        let mut code = concat(a_code, b_code);
        code.push(Instr::new(op, Some(res), Some(a_sym), Some(b_sym)));
        (code, res)
    }

    /// Evaluate receiver and arguments, pass them via `param` (receiver
    /// first, supporting dynamic dispatch on it), then `call`.
    fn gen_call(&mut self, receiver: &Exp, method: &str, args: &[Exp]) -> (InstrList, SymRef) {
        let (mut code, recv_sym) = self.gen_exp(receiver);
        let mut arg_syms = vec![recv_sym];
        for arg in args {
            let (arg_code, arg_sym) = self.gen_exp(arg);
            code = concat(code, arg_code);
            arg_syms.push(arg_sym);
        }
        for sym in arg_syms {
            code.push(Instr::new(Opcode::Param, None, Some(sym), None));
        }
        let method_sym = self.resolve_method(receiver, method);
        let res = self.temp("-");
        code.push(Instr::new(Opcode::Call, Some(res), Some(method_sym), None));
        (code, res)
    }

    /// Best-effort static resolution of the called method's table entry. When
    /// the receiver's class is only known at runtime (dynamic dispatch), fall
    /// back to the interned method name — the receiver was already passed as
    /// the first `param`.
    fn resolve_method(&mut self, receiver: &Exp, method: &str) -> SymRef {
        let static_class = match &receiver.kind {
            ExpKind::NewObject(class_name) => Some(class_name.as_str()),
            ExpKind::This => self.current_class.as_deref(),
            _ => None,
        };
        if let Some(class_name) = static_class {
            let mut current = Some(class_name);
            let mut guard = 0;
            while let Some(name) = current {
                guard += 1;
                if guard > self.class_map.len() + 1 {
                    break;
                }
                if let Some(sym) = self.symbols.lookup(&format!("global::{name}"), method) {
                    return sym;
                }
                current = self.class_map.get(name).and_then(|c| c.parent.as_deref());
            }
        }
        self.symbols.intern_const(method, "method")
    }
}
