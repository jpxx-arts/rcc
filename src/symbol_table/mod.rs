//! Scoped symbol table populated during syntactic analysis.
//!
//! Unlike the lexer's interned literal table, this records the *semantic*
//! symbols of the program — classes, fields, methods, parameters and locals —
//! together with their declared type and the scope they belong to. It is
//! printed by the `--symbols` flag.
//!
//! For the intermediate-code phase (Trabalho 3) the table also acts as the
//! address space of the three-address code: every 3AC operand is an index
//! into `entries`. A hash index keyed by `(scope, name)` gives O(1) lookup,
//! and [`SymbolTable::new_temp`] / [`SymbolTable::new_label`] insert the
//! compiler-generated temporaries (`t0`, `t1`, ...) and labels (`L0`, `L1`,
//! ...) used by the code generator. Literals referenced by instructions are
//! interned via [`SymbolTable::intern_const`].

use std::collections::HashMap;
use std::fmt::Write as _;

/// Index of an entry in the symbol table; how 3AC instructions refer to their
/// operands and results.
pub type SymRef = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Class,
    Field,
    Method,
    Param,
    Local,
    MainArg,
    /// Compiler-generated temporary holding an intermediate 3AC result.
    Temp,
    /// Compiler-generated jump target for 3AC control flow.
    Label,
    /// Interned literal (`42`, `true`, `false`, `this`) referenced by 3AC.
    Const,
}

impl Category {
    fn label(&self) -> &'static str {
        match self {
            Category::Class => "class",
            Category::Field => "field",
            Category::Method => "method",
            Category::Param => "param",
            Category::Local => "local",
            Category::MainArg => "main-arg",
            Category::Temp => "temp",
            Category::Label => "label",
            Category::Const => "const",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// Fully-qualified scope, e.g. `global`, `global::BBS`, `global::BBS::Sort`.
    pub scope: String,
    pub name: String,
    pub category: Category,
    /// Type display: declared type for variables, return type for methods,
    /// parent for classes (`-` when none / not applicable).
    pub ty: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    pub entries: Vec<Entry>,
    /// Hash index `(scope, name) -> entry`, used by lookups and interning.
    index: HashMap<(String, String), usize>,
    temp_count: usize,
    label_count: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable::default()
    }

    pub fn add(
        &mut self,
        scope: impl Into<String>,
        name: impl Into<String>,
        category: Category,
        ty: impl Into<String>,
        line: usize,
        column: usize,
    ) -> SymRef {
        let scope = scope.into();
        let name = name.into();
        let idx = self.entries.len();
        self.index.insert((scope.clone(), name.clone()), idx);
        self.entries.push(Entry {
            scope,
            name,
            category,
            ty: ty.into(),
            line,
            column,
        });
        idx
    }

    /// Exact lookup of `name` declared directly in `scope`.
    pub fn lookup(&self, scope: &str, name: &str) -> Option<SymRef> {
        self.index
            .get(&(scope.to_string(), name.to_string()))
            .copied()
    }

    /// Resolve `name` starting at `scope` and walking outward through the
    /// enclosing scopes (`global::C::m` → `global::C` → `global`).
    pub fn resolve(&self, scope: &str, name: &str) -> Option<SymRef> {
        let mut parts: Vec<&str> = scope.split("::").collect();
        while !parts.is_empty() {
            if let Some(idx) = self.lookup(&parts.join("::"), name) {
                return Some(idx);
            }
            parts.pop();
        }
        None
    }

    /// Insert a fresh compiler temporary (`t0`, `t1`, ...) into `scope` and
    /// return its reference. Used by the 3AC generator to hold intermediate
    /// results.
    pub fn new_temp(&mut self, scope: &str, ty: &str) -> SymRef {
        let name = format!("t{}", self.temp_count);
        self.temp_count += 1;
        self.add(scope, name, Category::Temp, ty, 0, 0)
    }

    /// Insert a fresh label (`L0`, `L1`, ...) into `scope` and return its
    /// reference. Used by the 3AC generator as a jump/control-flow target.
    pub fn new_label(&mut self, scope: &str) -> SymRef {
        let name = format!("L{}", self.label_count);
        self.label_count += 1;
        self.add(scope, name, Category::Label, "-", 0, 0)
    }

    /// Intern a literal so 3AC instructions can reference it as an operand.
    /// Repeated literals share a single entry.
    pub fn intern_const(&mut self, lexeme: &str, ty: &str) -> SymRef {
        if let Some(idx) = self.lookup("const", lexeme) {
            return idx;
        }
        self.add("const", lexeme, Category::Const, ty, 0, 0)
    }

    /// Display name of an entry (used when rendering 3AC instructions).
    pub fn name_of(&self, sym: SymRef) -> &str {
        &self.entries[sym].name
    }

    /// Scope-qualified display name, e.g. `Fac.ComputeFac` for the entry
    /// `global::Fac :: ComputeFac`. Falls back to the bare name at top level.
    pub fn qualified_name(&self, sym: SymRef) -> String {
        let e = &self.entries[sym];
        match e.scope.strip_prefix("global::") {
            Some(inner) if !inner.is_empty() => {
                format!("{}.{}", inner.replace("::", "."), e.name)
            }
            _ => e.name.clone(),
        }
    }

    /// Render the table as an aligned plain-text listing.
    pub fn render(&self) -> String {
        let mut out = String::new();
        if self.entries.is_empty() {
            out.push_str("symbol table: (empty)\n");
            return out;
        }
        let _ = writeln!(out, "symbol table:");
        let _ = writeln!(
            out,
            "  {:<28}  {:<16}  {:<10}  {:<10}  {:>5}  {:>5}",
            "scope", "name", "category", "type", "line", "col"
        );
        for e in &self.entries {
            let (line, col) = if e.line == 0 {
                ("-".to_string(), "-".to_string())
            } else {
                (e.line.to_string(), e.column.to_string())
            };
            let _ = writeln!(
                out,
                "  {:<28}  {:<16}  {:<10}  {:<10}  {:>5}  {:>5}",
                e.scope,
                e.name,
                e.category.label(),
                e.ty,
                line,
                col
            );
        }
        out
    }
}
