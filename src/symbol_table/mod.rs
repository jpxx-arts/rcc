//! Scoped symbol table populated during syntactic analysis.
//!
//! Unlike the lexer's interned literal table, this records the *semantic*
//! symbols of the program — classes, fields, methods, parameters and locals —
//! together with their declared type and the scope they belong to. It is
//! printed by the `--symbols` flag.

use std::fmt::Write as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Class,
    Field,
    Method,
    Param,
    Local,
    MainArg,
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
    ) {
        self.entries.push(Entry {
            scope: scope.into(),
            name: name.into(),
            category,
            ty: ty.into(),
            line,
            column,
        });
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
            let _ = writeln!(
                out,
                "  {:<28}  {:<16}  {:<10}  {:<10}  {:>5}  {:>5}",
                e.scope,
                e.name,
                e.category.label(),
                e.ty,
                e.line,
                e.column
            );
        }
        out
    }
}
