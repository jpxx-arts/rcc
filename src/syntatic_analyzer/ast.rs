//! Tipos de nó da Árvore Sintática Abstrata (AST).
//!
//! Cada não-terminal "significativo" da gramática transformada vira um nó
//! aqui. Não-terminais auxiliares introduzidos por fatoração à esquerda
//! (`ExpRest`, `NewRest`, `ExpAndRest`, etc.) NÃO geram nós próprios — eles
//! existem só pra estruturar o parsing e seu resultado é absorvido pelo nó
//! pai correspondente.
//!
//! A AST é o artefato natural para alimentar a próxima fase do compilador
//! (analisador semântico).

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    IntArray,
    Boolean,
    Class(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub return_type: Type,
    pub name: String,
    pub params: Vec<VarDecl>,
    pub vars: Vec<VarDecl>,
    pub body: Vec<Stmt>,
    pub return_expr: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub extends: Option<String>,
    pub vars: Vec<VarDecl>,
    pub methods: Vec<MethodDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MainClass {
    pub name: String,
    pub args_name: String,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub main: MainClass,
    pub classes: Vec<ClassDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Block(Vec<Stmt>),
    If(Expr, Box<Stmt>, Box<Stmt>),
    While(Expr, Box<Stmt>),
    Println(Expr),
    /// `lhs = expr;`
    Assign(String, Expr),
    /// `lhs[index] = expr;`
    ArrayAssign(String, Expr, Expr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    And, // &&
    Lt,  // <
    Gt,  // >
    Add, // +
    Sub, // -
    Mul, // *
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(String),
    Id(String),
    Bool(bool),
    This,
    /// `new ClassName()`
    NewObject(String),
    /// `new int[size]`
    NewIntArray(Box<Expr>),
    /// `!expr`
    Not(Box<Expr>),
    /// `lhs op rhs`
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// `array[index]`
    Index(Box<Expr>, Box<Expr>),
    /// `target.length`
    Length(Box<Expr>),
    /// `target.method(args)`
    Call(Box<Expr>, String, Vec<Expr>),
}
