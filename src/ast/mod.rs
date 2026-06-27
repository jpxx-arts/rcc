//! Abstract syntax tree for the WHILE/MiniJava variant, mirroring the official
//! grammar in `specs/gramatica-prof.md`.
//!
//! Every node carries the source line/column of its leading token so the
//! semantic analyzer can report precise diagnostics. The [`Program::pretty`]
//! method renders the tree for the `--ast` flag.

use std::fmt::Write as _;

/// A primitive or user-defined type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Boolean,
    IntArray,
    /// User-defined class type (`Id`).
    Class(String),
}

impl Type {
    pub fn display(&self) -> String {
        match self {
            Type::Int => "int".to_string(),
            Type::Boolean => "boolean".to_string(),
            Type::IntArray => "int[]".to_string(),
            Type::Class(name) => name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub ty: Type,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub name: String,
    pub ret_type: Type,
    pub params: Vec<VarDecl>,
    pub locals: Vec<VarDecl>,
    pub body: Vec<Stmt>,
    pub ret_expr: Exp,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub parent: Option<String>,
    pub fields: Vec<VarDecl>,
    pub methods: Vec<MethodDecl>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct MainClass {
    pub name: String,
    /// The `String[] args` parameter name.
    pub arg_name: String,
    pub body: Vec<Stmt>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub main: MainClass,
    pub classes: Vec<ClassDecl>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `Id '=' Exp ';'`
    Assign {
        name: String,
        value: Exp,
        line: usize,
        column: usize,
    },
    /// `Id '[' Exp ']' '=' Exp ';'`
    ArrayAssign {
        name: String,
        index: Exp,
        value: Exp,
        line: usize,
        column: usize,
    },
    /// `'if' '(' Exp ')' '{' L_com '}' ('else' '{' L_com '}')?`
    If {
        cond: Exp,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
        line: usize,
        column: usize,
    },
    /// `'while' '(' Exp ')' '{' L_com '}'`
    While {
        cond: Exp,
        body: Vec<Stmt>,
        line: usize,
        column: usize,
    },
    /// `'System' '.' 'out' '.' 'println' '(' Exp ')' ';'`
    Println {
        value: Exp,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone)]
pub struct Exp {
    pub kind: ExpKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub enum ExpKind {
    And(Box<Exp>, Box<Exp>),
    Less(Box<Exp>, Box<Exp>),
    Add(Box<Exp>, Box<Exp>),
    Sub(Box<Exp>, Box<Exp>),
    Mul(Box<Exp>, Box<Exp>),
    Not(Box<Exp>),
    /// `array '[' index ']'`
    Index {
        array: Box<Exp>,
        index: Box<Exp>,
    },
    /// `receiver '.' 'length'`
    Length(Box<Exp>),
    /// `receiver '.' method '(' args ')'`
    Call {
        receiver: Box<Exp>,
        method: String,
        args: Vec<Exp>,
    },
    /// `'new' Id '(' ')'`
    NewObject(String),
    /// `'new' 'int' '[' size ']'`
    NewArray(Box<Exp>),
    Id(String),
    Num(String),
    True,
    False,
    This,
}

// -------------------------------------------------------------------------
// Pretty-printer for the `--ast` flag.
// -------------------------------------------------------------------------

impl Program {
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "Program");
        self.main.pretty(&mut out, 1);
        for c in &self.classes {
            c.pretty(&mut out, 1);
        }
        out
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

impl MainClass {
    fn pretty(&self, out: &mut String, depth: usize) {
        indent(out, depth);
        let _ = writeln!(out, "MainClass {} (args: {})", self.name, self.arg_name);
        for s in &self.body {
            s.pretty(out, depth + 1);
        }
    }
}

impl ClassDecl {
    fn pretty(&self, out: &mut String, depth: usize) {
        indent(out, depth);
        match &self.parent {
            Some(p) => {
                let _ = writeln!(out, "Class {} extends {}", self.name, p);
            }
            None => {
                let _ = writeln!(out, "Class {}", self.name);
            }
        }
        for f in &self.fields {
            indent(out, depth + 1);
            let _ = writeln!(out, "Field {}: {}", f.name, f.ty.display());
        }
        for m in &self.methods {
            m.pretty(out, depth + 1);
        }
    }
}

impl MethodDecl {
    fn pretty(&self, out: &mut String, depth: usize) {
        indent(out, depth);
        let params: Vec<String> = self
            .params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.ty.display()))
            .collect();
        let _ = writeln!(
            out,
            "Method {}({}) -> {}",
            self.name,
            params.join(", "),
            self.ret_type.display()
        );
        for l in &self.locals {
            indent(out, depth + 1);
            let _ = writeln!(out, "Local {}: {}", l.name, l.ty.display());
        }
        for s in &self.body {
            s.pretty(out, depth + 1);
        }
        indent(out, depth + 1);
        let _ = writeln!(out, "Return");
        self.ret_expr.pretty(out, depth + 2);
    }
}

impl Stmt {
    fn pretty(&self, out: &mut String, depth: usize) {
        indent(out, depth);
        match self {
            Stmt::Assign { name, value, .. } => {
                let _ = writeln!(out, "Assign {name}");
                value.pretty(out, depth + 1);
            }
            Stmt::ArrayAssign {
                name, index, value, ..
            } => {
                let _ = writeln!(out, "ArrayAssign {name}");
                indent(out, depth + 1);
                let _ = writeln!(out, "index:");
                index.pretty(out, depth + 2);
                indent(out, depth + 1);
                let _ = writeln!(out, "value:");
                value.pretty(out, depth + 2);
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                let _ = writeln!(out, "If");
                indent(out, depth + 1);
                let _ = writeln!(out, "cond:");
                cond.pretty(out, depth + 2);
                indent(out, depth + 1);
                let _ = writeln!(out, "then:");
                for s in then_body {
                    s.pretty(out, depth + 2);
                }
                if let Some(eb) = else_body {
                    indent(out, depth + 1);
                    let _ = writeln!(out, "else:");
                    for s in eb {
                        s.pretty(out, depth + 2);
                    }
                }
            }
            Stmt::While { cond, body, .. } => {
                let _ = writeln!(out, "While");
                indent(out, depth + 1);
                let _ = writeln!(out, "cond:");
                cond.pretty(out, depth + 2);
                indent(out, depth + 1);
                let _ = writeln!(out, "body:");
                for s in body {
                    s.pretty(out, depth + 2);
                }
            }
            Stmt::Println { value, .. } => {
                let _ = writeln!(out, "Println");
                value.pretty(out, depth + 1);
            }
        }
    }
}

impl Exp {
    fn pretty(&self, out: &mut String, depth: usize) {
        indent(out, depth);
        match &self.kind {
            ExpKind::And(a, b) => {
                let _ = writeln!(out, "And");
                a.pretty(out, depth + 1);
                b.pretty(out, depth + 1);
            }
            ExpKind::Less(a, b) => {
                let _ = writeln!(out, "Less");
                a.pretty(out, depth + 1);
                b.pretty(out, depth + 1);
            }
            ExpKind::Add(a, b) => {
                let _ = writeln!(out, "Add");
                a.pretty(out, depth + 1);
                b.pretty(out, depth + 1);
            }
            ExpKind::Sub(a, b) => {
                let _ = writeln!(out, "Sub");
                a.pretty(out, depth + 1);
                b.pretty(out, depth + 1);
            }
            ExpKind::Mul(a, b) => {
                let _ = writeln!(out, "Mul");
                a.pretty(out, depth + 1);
                b.pretty(out, depth + 1);
            }
            ExpKind::Not(a) => {
                let _ = writeln!(out, "Not");
                a.pretty(out, depth + 1);
            }
            ExpKind::Index { array, index } => {
                let _ = writeln!(out, "Index");
                array.pretty(out, depth + 1);
                index.pretty(out, depth + 1);
            }
            ExpKind::Length(a) => {
                let _ = writeln!(out, "Length");
                a.pretty(out, depth + 1);
            }
            ExpKind::Call {
                receiver,
                method,
                args,
            } => {
                let _ = writeln!(out, "Call .{method}()");
                receiver.pretty(out, depth + 1);
                for a in args {
                    a.pretty(out, depth + 1);
                }
            }
            ExpKind::NewObject(name) => {
                let _ = writeln!(out, "NewObject {name}");
            }
            ExpKind::NewArray(size) => {
                let _ = writeln!(out, "NewArray int[]");
                size.pretty(out, depth + 1);
            }
            ExpKind::Id(name) => {
                let _ = writeln!(out, "Id {name}");
            }
            ExpKind::Num(n) => {
                let _ = writeln!(out, "Num {n}");
            }
            ExpKind::True => {
                let _ = writeln!(out, "True");
            }
            ExpKind::False => {
                let _ = writeln!(out, "False");
            }
            ExpKind::This => {
                let _ = writeln!(out, "This");
            }
        }
    }
}
