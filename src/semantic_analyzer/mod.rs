//! Static semantic analysis over the AST produced by the parser.
//!
//! Implements the rules from `specs/trabalho2.md` §4: operator typing, array
//! indexing and `.length`, assignment compatibility, class instantiation,
//! single inheritance with dynamic-dispatch override checking, and the rule
//! that an entirely empty class is a semantic error.
//!
//! All errors are collected (analysis does not stop at the first one) and
//! returned sorted by source position.

use std::collections::HashMap;

use crate::ast::{ClassDecl, Exp, ExpKind, MainClass, MethodDecl, Program, Stmt, Type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub line: usize,
    pub column: usize,
    pub msg: String,
}

#[derive(Debug, Clone)]
struct MethodSig {
    params: Vec<Type>,
    ret: Type,
}

#[derive(Debug)]
struct ClassInfo {
    parent: Option<String>,
    fields: HashMap<String, Type>,
    methods: HashMap<String, MethodSig>,
}

/// Entry point. Returns every semantic error found, sorted by position.
pub fn analyze(program: &Program) -> Vec<SemanticError> {
    let mut errors = Vec::new();
    let classes = build_class_table(program, &mut errors);

    check_class_rules(program, &classes, &mut errors);

    // Type-check the main class body (static context: no `this`, no fields).
    {
        let mut checker = Checker {
            classes: &classes,
            current_class: None,
            env: HashMap::new(),
            errors: &mut errors,
        };
        checker.check_main(&program.main);
    }

    // Type-check every user class method.
    for class in &program.classes {
        for method in &class.methods {
            let env = build_method_env(&classes, class, method);
            let mut checker = Checker {
                classes: &classes,
                current_class: Some(class.name.clone()),
                env,
                errors: &mut errors,
            };
            checker.check_method(method);
        }
    }

    errors.sort_by_key(|e| (e.line, e.column));
    errors
}

fn build_class_table(
    program: &Program,
    errors: &mut Vec<SemanticError>,
) -> HashMap<String, ClassInfo> {
    let mut classes: HashMap<String, ClassInfo> = HashMap::new();

    // Register the main class so it can be referenced as a type.
    classes.insert(
        program.main.name.clone(),
        ClassInfo {
            parent: None,
            fields: HashMap::new(),
            methods: HashMap::new(),
        },
    );

    for class in &program.classes {
        if classes.contains_key(&class.name) {
            errors.push(SemanticError {
                line: class.line,
                column: class.column,
                msg: format!("class '{}' is declared more than once", class.name),
            });
            continue;
        }

        let mut fields = HashMap::new();
        for f in &class.fields {
            if fields.insert(f.name.clone(), f.ty.clone()).is_some() {
                errors.push(SemanticError {
                    line: f.line,
                    column: f.column,
                    msg: format!("field '{}' is declared more than once", f.name),
                });
            }
        }

        let mut methods = HashMap::new();
        for m in &class.methods {
            let sig = MethodSig {
                params: m.params.iter().map(|p| p.ty.clone()).collect(),
                ret: m.ret_type.clone(),
            };
            if methods.insert(m.name.clone(), sig).is_some() {
                errors.push(SemanticError {
                    line: m.line,
                    column: m.column,
                    msg: format!("method '{}' is declared more than once", m.name),
                });
            }
        }

        classes.insert(
            class.name.clone(),
            ClassInfo {
                parent: class.parent.clone(),
                fields,
                methods,
            },
        );
    }

    classes
}

/// Validates class-level rules: empty classes, inheritance targets and cycles,
/// type references, and override-signature consistency.
fn check_class_rules(
    program: &Program,
    classes: &HashMap<String, ClassInfo>,
    errors: &mut Vec<SemanticError>,
) {
    for class in &program.classes {
        // Empty class (no fields and no methods) is a semantic error.
        if class.fields.is_empty() && class.methods.is_empty() {
            errors.push(SemanticError {
                line: class.line,
                column: class.column,
                msg: format!(
                    "class '{}' is empty: a class must declare at least one field or method",
                    class.name
                ),
            });
        }

        // `extends` target must exist and must not introduce a cycle.
        if let Some(parent) = &class.parent {
            if !classes.contains_key(parent) {
                errors.push(SemanticError {
                    line: class.line,
                    column: class.column,
                    msg: format!("class '{}' extends unknown class '{}'", class.name, parent),
                });
            } else if has_cycle(&class.name, classes) {
                errors.push(SemanticError {
                    line: class.line,
                    column: class.column,
                    msg: format!("inheritance cycle involving class '{}'", class.name),
                });
            }
        }

        // Field types must reference declared classes.
        for f in &class.fields {
            check_type_exists(&f.ty, f.line, f.column, classes, errors);
        }

        // Method signature types and override consistency.
        for m in &class.methods {
            check_type_exists(&m.ret_type, m.line, m.column, classes, errors);
            for p in &m.params {
                check_type_exists(&p.ty, p.line, p.column, classes, errors);
            }
            for l in &m.locals {
                check_type_exists(&l.ty, l.line, l.column, classes, errors);
            }
            check_override(class, m, classes, errors);
        }
    }
}

/// If `method` overrides one inherited from an ancestor, their signatures must
/// match (same parameter types and return type) for sound dynamic dispatch.
fn check_override(
    class: &ClassDecl,
    method: &MethodDecl,
    classes: &HashMap<String, ClassInfo>,
    errors: &mut Vec<SemanticError>,
) {
    let mut ancestor = class.parent.clone();
    let mut guard = 0;
    while let Some(name) = ancestor {
        guard += 1;
        if guard > 1000 {
            break; // cycle; reported elsewhere
        }
        let Some(info) = classes.get(&name) else {
            break;
        };
        if let Some(parent_sig) = info.methods.get(&method.name) {
            let same_params = parent_sig.params.len() == method.params.len()
                && parent_sig
                    .params
                    .iter()
                    .zip(&method.params)
                    .all(|(a, b)| *a == b.ty);
            if !same_params || parent_sig.ret != method.ret_type {
                errors.push(SemanticError {
                    line: method.line,
                    column: method.column,
                    msg: format!(
                        "method '{}' overrides '{}::{}' with an incompatible signature",
                        method.name, name, method.name
                    ),
                });
            }
            return;
        }
        ancestor = info.parent.clone();
    }
}

fn has_cycle(start: &str, classes: &HashMap<String, ClassInfo>) -> bool {
    let mut current = Some(start.to_string());
    let mut steps = 0;
    while let Some(name) = current {
        steps += 1;
        if steps > classes.len() + 1 {
            return true;
        }
        let Some(info) = classes.get(&name) else {
            return false;
        };
        match &info.parent {
            Some(p) if p == start => return true,
            Some(p) => current = Some(p.clone()),
            None => return false,
        }
    }
    false
}

fn check_type_exists(
    ty: &Type,
    line: usize,
    column: usize,
    classes: &HashMap<String, ClassInfo>,
    errors: &mut Vec<SemanticError>,
) {
    if let Type::Class(name) = ty
        && !classes.contains_key(name)
    {
        errors.push(SemanticError {
            line,
            column,
            msg: format!("unknown type '{name}'"),
        });
    }
}

/// Build the variable environment visible inside a method: inherited and own
/// fields, then parameters, then locals (later bindings shadow earlier ones).
fn build_method_env(
    classes: &HashMap<String, ClassInfo>,
    class: &ClassDecl,
    method: &MethodDecl,
) -> HashMap<String, Type> {
    let mut env = HashMap::new();

    // Collect fields from the inheritance chain, ancestors first so that a
    // class's own fields override inherited ones of the same name.
    let mut chain = Vec::new();
    let mut current = Some(class.name.clone());
    let mut guard = 0;
    while let Some(name) = current {
        guard += 1;
        if guard > classes.len() + 1 {
            break;
        }
        chain.push(name.clone());
        current = classes.get(&name).and_then(|c| c.parent.clone());
    }
    for class_name in chain.iter().rev() {
        if let Some(info) = classes.get(class_name) {
            for (fname, fty) in &info.fields {
                env.insert(fname.clone(), fty.clone());
            }
        }
    }

    for p in &method.params {
        env.insert(p.name.clone(), p.ty.clone());
    }
    for l in &method.locals {
        env.insert(l.name.clone(), l.ty.clone());
    }
    env
}

struct Checker<'a> {
    classes: &'a HashMap<String, ClassInfo>,
    current_class: Option<String>,
    env: HashMap<String, Type>,
    errors: &'a mut Vec<SemanticError>,
}

impl Checker<'_> {
    fn err(&mut self, line: usize, column: usize, msg: impl Into<String>) {
        self.errors.push(SemanticError {
            line,
            column,
            msg: msg.into(),
        });
    }

    fn check_main(&mut self, main: &MainClass) {
        for s in &main.body {
            self.check_stmt(s);
        }
    }

    fn check_method(&mut self, method: &MethodDecl) {
        for s in &method.body {
            self.check_stmt(s);
        }
        if let Some(ret_ty) = self.type_of(&method.ret_expr)
            && !self.assignable(&method.ret_type, &ret_ty)
        {
            self.err(
                method.ret_expr.line,
                method.ret_expr.column,
                format!(
                    "return type mismatch: method returns '{}' but expression has type '{}'",
                    method.ret_type.display(),
                    ret_ty.display()
                ),
            );
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign {
                name,
                value,
                line,
                column,
            } => {
                let target = self.env.get(name).cloned();
                let value_ty = self.type_of(value);
                match (target, value_ty) {
                    (Some(t), Some(v)) => {
                        if !self.assignable(&t, &v) {
                            self.err(
                                *line,
                                *column,
                                format!(
                                    "cannot assign value of type '{}' to '{}' of type '{}'",
                                    v.display(),
                                    name,
                                    t.display()
                                ),
                            );
                        }
                    }
                    (None, _) => {
                        self.err(*line, *column, format!("undefined variable '{name}'"));
                    }
                    _ => {}
                }
            }
            Stmt::ArrayAssign {
                name,
                index,
                value,
                line,
                column,
            } => {
                match self.env.get(name).cloned() {
                    Some(Type::IntArray) => {}
                    Some(other) => self.err(
                        *line,
                        *column,
                        format!(
                            "cannot index '{}' of type '{}': not an int array",
                            name,
                            other.display()
                        ),
                    ),
                    None => self.err(*line, *column, format!("undefined variable '{name}'")),
                }
                self.expect(index, &Type::Int, "array index");
                self.expect(value, &Type::Int, "array element");
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                self.expect(cond, &Type::Boolean, "if condition");
                for s in then_body {
                    self.check_stmt(s);
                }
                if let Some(eb) = else_body {
                    for s in eb {
                        self.check_stmt(s);
                    }
                }
            }
            Stmt::While { cond, body, .. } => {
                self.expect(cond, &Type::Boolean, "while condition");
                for s in body {
                    self.check_stmt(s);
                }
            }
            Stmt::Println { value, .. } => {
                self.expect(value, &Type::Int, "println argument");
            }
        }
    }

    /// Type-check `exp` and require it to have type `expected`.
    fn expect(&mut self, exp: &Exp, expected: &Type, ctx: &str) {
        if let Some(actual) = self.type_of(exp)
            && actual != *expected
        {
            self.err(
                exp.line,
                exp.column,
                format!(
                    "{ctx} must be '{}', found '{}'",
                    expected.display(),
                    actual.display()
                ),
            );
        }
    }

    /// Infer the type of an expression. Returns `None` when a sub-error already
    /// makes the type unknown (to avoid cascading diagnostics).
    fn type_of(&mut self, exp: &Exp) -> Option<Type> {
        match &exp.kind {
            ExpKind::Num(_) => Some(Type::Int),
            ExpKind::True | ExpKind::False => Some(Type::Boolean),
            ExpKind::This => match &self.current_class {
                Some(c) => Some(Type::Class(c.clone())),
                None => {
                    self.err(exp.line, exp.column, "'this' cannot be used in static main");
                    None
                }
            },
            ExpKind::Id(name) => match self.env.get(name) {
                Some(t) => Some(t.clone()),
                None => {
                    self.err(exp.line, exp.column, format!("undefined variable '{name}'"));
                    None
                }
            },
            ExpKind::Add(a, b) | ExpKind::Sub(a, b) | ExpKind::Mul(a, b) => {
                self.expect(a, &Type::Int, "arithmetic operand");
                self.expect(b, &Type::Int, "arithmetic operand");
                Some(Type::Int)
            }
            ExpKind::Less(a, b) => {
                self.expect(a, &Type::Int, "'<' operand");
                self.expect(b, &Type::Int, "'<' operand");
                Some(Type::Boolean)
            }
            ExpKind::And(a, b) => {
                self.expect(a, &Type::Boolean, "'&&' operand");
                self.expect(b, &Type::Boolean, "'&&' operand");
                Some(Type::Boolean)
            }
            ExpKind::Not(a) => {
                self.expect(a, &Type::Boolean, "'!' operand");
                Some(Type::Boolean)
            }
            ExpKind::Index { array, index } => {
                if let Some(at) = self.type_of(array)
                    && at != Type::IntArray
                {
                    self.err(
                        array.line,
                        array.column,
                        format!("cannot index type '{}': not an int array", at.display()),
                    );
                }
                self.expect(index, &Type::Int, "array index");
                Some(Type::Int)
            }
            ExpKind::Length(a) => {
                if let Some(at) = self.type_of(a)
                    && at != Type::IntArray
                {
                    self.err(
                        a.line,
                        a.column,
                        format!("'.length' requires an int array, found '{}'", at.display()),
                    );
                }
                Some(Type::Int)
            }
            ExpKind::NewArray(size) => {
                self.expect(size, &Type::Int, "array size");
                Some(Type::IntArray)
            }
            ExpKind::NewObject(name) => {
                if self.classes.contains_key(name) {
                    Some(Type::Class(name.clone()))
                } else {
                    self.err(
                        exp.line,
                        exp.column,
                        format!("cannot instantiate unknown class '{name}'"),
                    );
                    None
                }
            }
            ExpKind::Call {
                receiver,
                method,
                args,
            } => self.type_of_call(exp, receiver, method, args),
        }
    }

    fn type_of_call(
        &mut self,
        exp: &Exp,
        receiver: &Exp,
        method: &str,
        args: &[Exp],
    ) -> Option<Type> {
        let recv_ty = self.type_of(receiver)?;
        let Type::Class(class_name) = recv_ty else {
            self.err(
                receiver.line,
                receiver.column,
                format!(
                    "method call requires a class instance, found '{}'",
                    recv_ty.display()
                ),
            );
            return None;
        };

        let Some(sig) = self.lookup_method(&class_name, method) else {
            self.err(
                exp.line,
                exp.column,
                format!("class '{class_name}' has no method '{method}'"),
            );
            // Still type-check the arguments to surface errors inside them.
            for a in args {
                self.type_of(a);
            }
            return None;
        };

        let params = sig.params.clone();
        let ret = sig.ret.clone();

        if params.len() != args.len() {
            self.err(
                exp.line,
                exp.column,
                format!(
                    "method '{method}' expects {} argument(s), found {}",
                    params.len(),
                    args.len()
                ),
            );
        }
        for (arg, param_ty) in args.iter().zip(params.iter()) {
            if let Some(arg_ty) = self.type_of(arg)
                && !self.assignable(param_ty, &arg_ty)
            {
                self.err(
                    arg.line,
                    arg.column,
                    format!(
                        "argument type '{}' is not compatible with parameter type '{}'",
                        arg_ty.display(),
                        param_ty.display()
                    ),
                );
            }
        }
        // Type-check any extra args beyond the declared parameters.
        for arg in args.iter().skip(params.len()) {
            self.type_of(arg);
        }

        Some(ret)
    }

    /// Resolve `method` in `class_name` or any ancestor.
    fn lookup_method(&self, class_name: &str, method: &str) -> Option<&MethodSig> {
        let mut current = Some(class_name.to_string());
        let mut guard = 0;
        while let Some(name) = current {
            guard += 1;
            if guard > self.classes.len() + 1 {
                return None;
            }
            let info = self.classes.get(&name)?;
            if let Some(sig) = info.methods.get(method) {
                return Some(sig);
            }
            current = info.parent.clone();
        }
        None
    }

    /// Assignment/argument compatibility: identical types, or assigning a
    /// subclass instance to a superclass-typed target (dynamic dispatch).
    fn assignable(&self, target: &Type, value: &Type) -> bool {
        if target == value {
            return true;
        }
        match (target, value) {
            (Type::Class(sup), Type::Class(sub)) => self.is_subclass(sub, sup),
            _ => false,
        }
    }

    /// True if `sub` is `sup` or transitively extends it.
    fn is_subclass(&self, sub: &str, sup: &str) -> bool {
        let mut current = Some(sub.to_string());
        let mut guard = 0;
        while let Some(name) = current {
            guard += 1;
            if guard > self.classes.len() + 1 {
                return false;
            }
            if name == sup {
                return true;
            }
            current = self.classes.get(&name).and_then(|c| c.parent.clone());
        }
        false
    }
}
