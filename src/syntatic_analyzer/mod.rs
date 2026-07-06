//! Recursive-descent parser for the official grammar in
//! `specs/gramatica-prof.md` (with operator precedence, no left recursion).
//!
//! The parser consumes the token stream from `lexical_analyzer`, validates the
//! syntax, builds an [`ast::Program`], and populates the scoped
//! [`SymbolTable`] as declarations are recognized. On a syntax error it returns
//! a [`ParseError`] with the offending line/column and (when applicable) a
//! correction suggestion.

use crate::ast::{ClassDecl, Exp, ExpKind, MainClass, MethodDecl, Program, Stmt, Type, VarDecl};
use crate::lexical_analyzer::{Token, suggest_keyword};
use crate::symbol_table::{Category, SymbolTable};

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub msg: String,
    /// Optional fix hint surfaced by the `--suggest` flag.
    pub suggestion: Option<String>,
}

pub struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
    symbols: SymbolTable,
    /// Current scope path, joined with `::` (e.g. `["global", "BBS", "Sort"]`).
    scope: Vec<String>,
    /// When true, `L_com` is treated as nullable (zero or more commands),
    /// relaxing the official grammar so empty method/if/while/else bodies are
    /// accepted. Default (`false`) keeps the strict `L_com -> Com L'_com`.
    allow_empty_body: bool,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Parser {
            tokens,
            cursor: 0,
            symbols: SymbolTable::new(),
            scope: vec!["global".to_string()],
            allow_empty_body: false,
        }
    }

    /// Builder: opt into the lax, nullable-`L_com` mode.
    pub fn allow_empty_body(mut self, yes: bool) -> Self {
        self.allow_empty_body = yes;
        self
    }

    /// Parse the whole program, returning the AST and the populated symbol
    /// table on success.
    pub fn parse(mut self) -> Result<(Program, SymbolTable), ParseError> {
        let program = self.parse_prog()?;
        Ok((program, self.symbols))
    }

    // ---------- scope helpers ----------

    fn scope_str(&self) -> String {
        self.scope.join("::")
    }

    fn push_scope(&mut self, name: &str) {
        self.scope.push(name.to_string());
    }

    fn pop_scope(&mut self) {
        self.scope.pop();
    }

    // ---------- token helpers ----------

    fn peek(&self) -> &Token {
        &self.tokens[self.cursor.min(self.tokens.len() - 1)]
    }

    fn peek_at(&self, offset: usize) -> &Token {
        let idx = (self.cursor + offset).min(self.tokens.len() - 1);
        &self.tokens[idx]
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.cursor].clone();
        if self.cursor < self.tokens.len() - 1 {
            self.cursor += 1;
        }
        tok
    }

    fn error(&self, msg: impl Into<String>) -> ParseError {
        let tok = self.peek();
        ParseError {
            line: tok.line,
            column: tok.column,
            msg: format!("{}, got {}", msg.into(), tok.describe()),
            suggestion: None,
        }
    }

    fn error_expecting(&self, what: &str, hint: String) -> ParseError {
        let tok = self.peek();
        ParseError {
            line: tok.line,
            column: tok.column,
            msg: format!("expected {what}, got {}", tok.describe()),
            suggestion: Some(hint),
        }
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<Token, ParseError> {
        if self.peek().is_keyword(kw) {
            Ok(self.advance())
        } else {
            // A nearby identifier is most likely the keyword mistyped
            // (`claas` -> `class`); suggest the replacement instead of an
            // insertion.
            let tok = self.peek();
            let hint = if tok.is_id() && suggest_keyword(&tok.lexeme).as_deref() == Some(kw) {
                format!("replace '{}' with '{kw}'", tok.lexeme)
            } else {
                format!("insert '{kw}'")
            };
            Err(self.error_expecting(&format!("keyword '{kw}'"), hint))
        }
    }

    fn expect_delim(&mut self, d: &str) -> Result<Token, ParseError> {
        if self.peek().is_delim(d) {
            Ok(self.advance())
        } else {
            Err(self.error_expecting(&format!("'{d}'"), format!("insert '{d}'")))
        }
    }

    fn expect_id(&mut self) -> Result<Token, ParseError> {
        if self.peek().is_id() {
            Ok(self.advance())
        } else {
            Err(self.error_expecting("identifier", "insert an identifier".to_string()))
        }
    }

    // ---------- productions ----------

    // Prog -> Main_C Def_C
    fn parse_prog(&mut self) -> Result<Program, ParseError> {
        let main = self.parse_main_c()?;
        let classes = self.parse_def_c()?;
        if !self.peek().is_eof() {
            return Err(self.error("expected end of input"));
        }
        Ok(Program { main, classes })
    }

    // Main_C -> 'class' Id '{' 'public' 'static' 'void' 'main'
    //           '(' 'String' '[' ']' Id ')' '{' L_com '}' '}'
    fn parse_main_c(&mut self) -> Result<MainClass, ParseError> {
        let class_tok = self.expect_keyword("class")?;
        let name_tok = self.expect_id()?;
        let name = name_tok.lexeme.clone();
        self.symbols.add(
            "global",
            &name,
            Category::Class,
            "-",
            name_tok.line,
            name_tok.column,
        );

        self.expect_delim("{")?;
        self.expect_keyword("public")?;
        self.expect_keyword("static")?;
        self.expect_keyword("void")?;
        self.expect_keyword("main")?;
        self.expect_delim("(")?;
        self.expect_keyword("String")?;
        self.expect_delim("[")?;
        self.expect_delim("]")?;
        let arg_tok = self.expect_id()?;
        let arg_name = arg_tok.lexeme.clone();
        self.expect_delim(")")?;
        self.expect_delim("{")?;

        self.push_scope(&name);
        self.push_scope("main");
        self.symbols.add(
            self.scope_str(),
            &arg_name,
            Category::MainArg,
            "String[]",
            arg_tok.line,
            arg_tok.column,
        );
        let body = self.parse_l_com()?;
        self.pop_scope();
        self.pop_scope();

        self.expect_delim("}")?;
        self.expect_delim("}")?;

        Ok(MainClass {
            name,
            arg_name,
            body,
            line: class_tok.line,
            column: class_tok.column,
        })
    }

    // Def_C -> 'class' Id Def'_C | λ
    // Def'_C -> '{' Def_V Def_M '}' Def_C
    //        | 'extends' Id '{' Def_V Def_M '}' Def_C
    fn parse_def_c(&mut self) -> Result<Vec<ClassDecl>, ParseError> {
        let mut classes = Vec::new();
        while self.peek().is_keyword("class") {
            let class_tok = self.advance();
            let name_tok = self.expect_id()?;
            let name = name_tok.lexeme.clone();

            let parent = if self.peek().is_keyword("extends") {
                self.advance();
                let parent_tok = self.expect_id()?;
                Some(parent_tok.lexeme.clone())
            } else {
                None
            };

            self.symbols.add(
                "global",
                &name,
                Category::Class,
                parent.as_deref().unwrap_or("-"),
                name_tok.line,
                name_tok.column,
            );

            self.expect_delim("{")?;
            self.push_scope(&name);
            let fields = self.parse_def_v(Category::Field)?;
            let methods = self.parse_def_m()?;
            self.pop_scope();
            self.expect_delim("}")?;

            classes.push(ClassDecl {
                name,
                parent,
                fields,
                methods,
                line: class_tok.line,
                column: class_tok.column,
            });
        }
        Ok(classes)
    }

    // Def_V -> Type Id ';' Def_V | λ
    fn parse_def_v(&mut self, category: Category) -> Result<Vec<VarDecl>, ParseError> {
        let mut vars = Vec::new();
        while self.at_def_var() {
            let ty = self.parse_type()?;
            let id_tok = self.expect_id()?;
            let name = id_tok.lexeme.clone();
            self.expect_delim(";")?;
            self.symbols.add(
                self.scope_str(),
                &name,
                category,
                ty.display(),
                id_tok.line,
                id_tok.column,
            );
            vars.push(VarDecl {
                name,
                ty,
                line: id_tok.line,
                column: id_tok.column,
            });
        }
        Ok(vars)
    }

    /// True when the next tokens start a `Type Id ;` declaration rather than a
    /// command. Uses lookahead-2 because a class-typed declaration `Foo x;` has
    /// the same `Id Id` shape that a command never has.
    fn at_def_var(&self) -> bool {
        let t = self.peek();
        if t.is_keyword("int") || t.is_keyword("boolean") {
            return true;
        }
        if t.is_id() {
            return self.peek_at(1).is_id();
        }
        false
    }

    // Def_M -> 'public' Type Id '(' Def'_M | λ
    // Def'_M -> Args ')' '{' Def_V L_com 'return' Exp ';' '}' Def_M
    //        | ')' '{' Def_V L_com 'return' Exp ';' '}' Def_M
    fn parse_def_m(&mut self) -> Result<Vec<MethodDecl>, ParseError> {
        let mut methods = Vec::new();
        while self.peek().is_keyword("public") {
            let pub_tok = self.advance();
            let ret_type = self.parse_type()?;
            let name_tok = self.expect_id()?;
            let name = name_tok.lexeme.clone();
            self.symbols.add(
                self.scope_str(),
                &name,
                Category::Method,
                ret_type.display(),
                name_tok.line,
                name_tok.column,
            );

            self.expect_delim("(")?;
            self.push_scope(&name);
            let params = if self.peek().is_delim(")") {
                Vec::new()
            } else {
                self.parse_args()?
            };
            self.expect_delim(")")?;
            self.expect_delim("{")?;
            let locals = self.parse_def_v(Category::Local)?;
            let body = self.parse_l_com()?;
            self.expect_keyword("return")?;
            let ret_expr = self.parse_exp()?;
            self.expect_delim(";")?;
            self.expect_delim("}")?;
            self.pop_scope();

            methods.push(MethodDecl {
                name,
                ret_type,
                params,
                locals,
                body,
                ret_expr,
                line: pub_tok.line,
                column: pub_tok.column,
            });
        }
        Ok(methods)
    }

    // Args -> Type Id Args'    Args' -> ',' Type Id Args' | λ
    fn parse_args(&mut self) -> Result<Vec<VarDecl>, ParseError> {
        let mut params = Vec::new();
        loop {
            let ty = self.parse_type()?;
            let id_tok = self.expect_id()?;
            let name = id_tok.lexeme.clone();
            self.symbols.add(
                self.scope_str(),
                &name,
                Category::Param,
                ty.display(),
                id_tok.line,
                id_tok.column,
            );
            params.push(VarDecl {
                name,
                ty,
                line: id_tok.line,
                column: id_tok.column,
            });
            if self.peek().is_delim(",") {
                self.advance();
                continue;
            }
            break;
        }
        Ok(params)
    }

    // Type -> 'int' Type' | 'boolean' | Id      Type' -> '[' ']' | λ
    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let t = self.peek();
        if t.is_keyword("int") {
            self.advance();
            if self.peek().is_delim("[") {
                self.advance();
                self.expect_delim("]")?;
                Ok(Type::IntArray)
            } else {
                Ok(Type::Int)
            }
        } else if t.is_keyword("boolean") {
            self.advance();
            Ok(Type::Boolean)
        } else if t.is_id() {
            let tok = self.advance();
            Ok(Type::Class(tok.lexeme))
        } else {
            Err(self.error("expected type ('int', 'boolean', or class name)"))
        }
    }

    // L_com -> Com L'_com   (at least one command, per the grammar)
    // L'_com -> Com L'_com | λ
    fn parse_l_com(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        // Strict mode: the grammar requires at least one command, so we parse a
        // mandatory first `Com` (`parse_cmd` reports a precise "expected
        // statement" error when the body is empty). In `allow_empty_body` mode
        // we skip that requirement, making `L_com` nullable.
        if !self.allow_empty_body {
            stmts.push(self.parse_cmd()?);
        }
        while self.at_cmd() {
            stmts.push(self.parse_cmd()?);
        }
        Ok(stmts)
    }

    fn at_cmd(&self) -> bool {
        let t = self.peek();
        t.is_keyword("if")
            || t.is_keyword("while")
            || t.is_keyword("System")
            || (t.is_id() && (self.peek_at(1).is_delim("=") || self.peek_at(1).is_delim("[")))
    }

    // Com -> Id Com_Ass
    //      | 'if' '(' Exp ')' '{' L_com '}' I
    //      | 'while' '(' Exp ')' '{' L_com '}'
    //      | 'System' '.' 'out' '.' 'println' '(' Exp ')' ';'
    fn parse_cmd(&mut self) -> Result<Stmt, ParseError> {
        let t = self.peek().clone();

        if t.is_keyword("if") {
            self.advance();
            self.expect_delim("(")?;
            let cond = self.parse_exp()?;
            self.expect_delim(")")?;
            self.expect_delim("{")?;
            let then_body = self.parse_l_com()?;
            self.expect_delim("}")?;
            // I -> 'else' '{' L_com '}' | λ
            let else_body = if self.peek().is_keyword("else") {
                self.advance();
                self.expect_delim("{")?;
                let eb = self.parse_l_com()?;
                self.expect_delim("}")?;
                Some(eb)
            } else {
                None
            };
            return Ok(Stmt::If {
                cond,
                then_body,
                else_body,
                line: t.line,
                column: t.column,
            });
        }

        if t.is_keyword("while") {
            self.advance();
            self.expect_delim("(")?;
            let cond = self.parse_exp()?;
            self.expect_delim(")")?;
            self.expect_delim("{")?;
            let body = self.parse_l_com()?;
            self.expect_delim("}")?;
            return Ok(Stmt::While {
                cond,
                body,
                line: t.line,
                column: t.column,
            });
        }

        if t.is_keyword("System") {
            self.advance();
            self.expect_delim(".")?;
            self.expect_keyword("out")?;
            self.expect_delim(".")?;
            self.expect_keyword("println")?;
            self.expect_delim("(")?;
            let value = self.parse_exp()?;
            self.expect_delim(")")?;
            self.expect_delim(";")?;
            return Ok(Stmt::Println {
                value,
                line: t.line,
                column: t.column,
            });
        }

        if t.is_id() {
            let id_tok = self.advance();
            let name = id_tok.lexeme.clone();
            return self.parse_com_ass(name, id_tok.line, id_tok.column);
        }

        Err(self.error("expected statement"))
    }

    // Com_Ass -> '=' Exp ';' | '[' Exp ']' '=' Exp ';'
    fn parse_com_ass(
        &mut self,
        name: String,
        line: usize,
        column: usize,
    ) -> Result<Stmt, ParseError> {
        if self.peek().is_delim("=") {
            self.advance();
            let value = self.parse_exp()?;
            self.expect_delim(";")?;
            return Ok(Stmt::Assign {
                name,
                value,
                line,
                column,
            });
        }
        if self.peek().is_delim("[") {
            self.advance();
            let index = self.parse_exp()?;
            self.expect_delim("]")?;
            self.expect_delim("=")?;
            let value = self.parse_exp()?;
            self.expect_delim(";")?;
            return Ok(Stmt::ArrayAssign {
                name,
                index,
                value,
                line,
                column,
            });
        }
        // `whle (...)`, `fi (...)` and the like reach this point as an
        // identifier that starts no valid command; point at the keyword the
        // user probably meant.
        let hint = match suggest_keyword(&name) {
            Some(kw) if matches!(kw.as_str(), "if" | "while" | "System") => {
                format!("did you mean '{kw}'?")
            }
            _ => "insert '=' or '['".to_string(),
        };
        Err(self.error_expecting("'=' or '[' after identifier", hint))
    }

    // ----------------------------------------------------------------
    // Expressions, lowest to highest precedence (all left-associative):
    //   Exp -> And_exp -> Rel_exp -> Add_exp -> Mul_exp -> Un_exp
    //       -> Psf_exp -> Pri_exp
    // ----------------------------------------------------------------

    fn parse_exp(&mut self) -> Result<Exp, ParseError> {
        self.parse_exp_and()
    }

    fn parse_exp_and(&mut self) -> Result<Exp, ParseError> {
        let mut left = self.parse_exp_rel()?;
        while self.peek().is_op("&&") {
            let op = self.advance();
            let right = self.parse_exp_rel()?;
            left = Exp {
                kind: ExpKind::And(Box::new(left), Box::new(right)),
                line: op.line,
                column: op.column,
            };
        }
        Ok(left)
    }

    fn parse_exp_rel(&mut self) -> Result<Exp, ParseError> {
        let mut left = self.parse_exp_add()?;
        while self.peek().is_op("<") {
            let op = self.advance();
            let right = self.parse_exp_add()?;
            left = Exp {
                kind: ExpKind::Less(Box::new(left), Box::new(right)),
                line: op.line,
                column: op.column,
            };
        }
        Ok(left)
    }

    fn parse_exp_add(&mut self) -> Result<Exp, ParseError> {
        let mut left = self.parse_exp_mul()?;
        loop {
            let t = self.peek();
            let is_plus = t.is_op("+");
            let is_minus = t.is_op("-");
            if !is_plus && !is_minus {
                break;
            }
            let op = self.advance();
            let right = self.parse_exp_mul()?;
            let kind = if is_plus {
                ExpKind::Add(Box::new(left), Box::new(right))
            } else {
                ExpKind::Sub(Box::new(left), Box::new(right))
            };
            left = Exp {
                kind,
                line: op.line,
                column: op.column,
            };
        }
        Ok(left)
    }

    fn parse_exp_mul(&mut self) -> Result<Exp, ParseError> {
        let mut left = self.parse_exp_unary()?;
        while self.peek().is_op("*") {
            let op = self.advance();
            let right = self.parse_exp_unary()?;
            left = Exp {
                kind: ExpKind::Mul(Box::new(left), Box::new(right)),
                line: op.line,
                column: op.column,
            };
        }
        Ok(left)
    }

    fn parse_exp_unary(&mut self) -> Result<Exp, ParseError> {
        if self.peek().is_op("!") {
            let op = self.advance();
            let inner = self.parse_exp_unary()?;
            return Ok(Exp {
                kind: ExpKind::Not(Box::new(inner)),
                line: op.line,
                column: op.column,
            });
        }
        self.parse_exp_postfix()
    }

    // Psf_exp -> Pri_exp Psf'_exp
    // Psf'_exp -> '[' Exp ']' Psf'_exp | '.' 'length' Psf'_exp
    //          | '.' Id '(' L_exp ')' Psf'_exp | λ
    fn parse_exp_postfix(&mut self) -> Result<Exp, ParseError> {
        let mut e = self.parse_exp_primary()?;
        loop {
            let t = self.peek().clone();
            if t.is_delim("[") {
                self.advance();
                let index = self.parse_exp()?;
                self.expect_delim("]")?;
                e = Exp {
                    kind: ExpKind::Index {
                        array: Box::new(e),
                        index: Box::new(index),
                    },
                    line: t.line,
                    column: t.column,
                };
                continue;
            }
            if t.is_delim(".") {
                self.advance();
                if self.peek().is_keyword("length") {
                    self.advance();
                    e = Exp {
                        kind: ExpKind::Length(Box::new(e)),
                        line: t.line,
                        column: t.column,
                    };
                    continue;
                }
                let method_tok = self.expect_id()?;
                let method = method_tok.lexeme.clone();
                self.expect_delim("(")?;
                let args = if self.peek().is_delim(")") {
                    Vec::new()
                } else {
                    self.parse_list_exp()?
                };
                self.expect_delim(")")?;
                e = Exp {
                    kind: ExpKind::Call {
                        receiver: Box::new(e),
                        method,
                        args,
                    },
                    line: t.line,
                    column: t.column,
                };
                continue;
            }
            break;
        }
        Ok(e)
    }

    // Pri_exp -> '(' Exp ')' | 'true' | 'false' | Id | Number | 'this'
    //         | 'new' Id '(' ')' | 'new' 'int' '[' Exp ']'
    fn parse_exp_primary(&mut self) -> Result<Exp, ParseError> {
        let t = self.peek().clone();

        if t.is_delim("(") {
            self.advance();
            let inner = self.parse_exp()?;
            self.expect_delim(")")?;
            return Ok(inner);
        }
        if t.is_keyword("true") {
            self.advance();
            return Ok(Exp {
                kind: ExpKind::True,
                line: t.line,
                column: t.column,
            });
        }
        if t.is_keyword("false") {
            self.advance();
            return Ok(Exp {
                kind: ExpKind::False,
                line: t.line,
                column: t.column,
            });
        }
        if t.is_keyword("this") {
            self.advance();
            return Ok(Exp {
                kind: ExpKind::This,
                line: t.line,
                column: t.column,
            });
        }
        if t.is_keyword("new") {
            self.advance();
            return self.parse_new_rest(t.line, t.column);
        }
        if t.is_id() {
            self.advance();
            return Ok(Exp {
                kind: ExpKind::Id(t.lexeme),
                line: t.line,
                column: t.column,
            });
        }
        if t.is_number() {
            self.advance();
            return Ok(Exp {
                kind: ExpKind::Num(t.lexeme),
                line: t.line,
                column: t.column,
            });
        }
        Err(self.error("expected expression"))
    }

    // after 'new': Id '(' ')' | 'int' '[' Exp ']'
    fn parse_new_rest(&mut self, line: usize, column: usize) -> Result<Exp, ParseError> {
        if self.peek().is_keyword("int") {
            self.advance();
            self.expect_delim("[")?;
            let size = self.parse_exp()?;
            self.expect_delim("]")?;
            return Ok(Exp {
                kind: ExpKind::NewArray(Box::new(size)),
                line,
                column,
            });
        }
        if self.peek().is_id() {
            let name_tok = self.advance();
            self.expect_delim("(")?;
            self.expect_delim(")")?;
            return Ok(Exp {
                kind: ExpKind::NewObject(name_tok.lexeme),
                line,
                column,
            });
        }
        Err(self.error("expected class name or 'int' after 'new'"))
    }

    // L_exp -> Exp L'_exp | λ      L'_exp -> ',' Exp L'_exp | λ
    fn parse_list_exp(&mut self) -> Result<Vec<Exp>, ParseError> {
        let mut exps = Vec::new();
        loop {
            exps.push(self.parse_exp()?);
            if self.peek().is_delim(",") {
                self.advance();
                continue;
            }
            break;
        }
        Ok(exps)
    }
}

/// Convenience: parse a token slice into an AST and symbol table, using the
/// strict official grammar (`L_com` requires at least one command).
pub fn parse(tokens: &[Token]) -> Result<(Program, SymbolTable), ParseError> {
    Parser::new(tokens).parse()
}

/// Like [`parse`], but when `allow_empty_body` is true `L_com` is treated as
/// nullable, so empty method/if/while/else bodies are accepted. With
/// `allow_empty_body == false` the behavior is identical to [`parse`].
pub fn parse_with(
    tokens: &[Token],
    allow_empty_body: bool,
) -> Result<(Program, SymbolTable), ParseError> {
    Parser::new(tokens)
        .allow_empty_body(allow_empty_body)
        .parse()
}
