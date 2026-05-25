//! Parser descendente recursivo para a gramática transformada em
//! `specs/gramatica-transformada.md`.
//!
//! Constrói uma AST (`ast::Program`) que pode ser consumida por fases
//! subsequentes (análise semântica, geração de código). Erros sintáticos
//! são reportados com linha, coluna e descrição do que se esperava vs.
//! o que se encontrou.

pub mod ast;
pub use ast::*;

use crate::lexical_analyzer::{SymbolTable, Token, TokenAttribute, TokenClass};

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub msg: String,
}

pub struct Parser<'a> {
    tokens: &'a [Token],
    symbol_table: &'a SymbolTable,
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], symbol_table: &'a SymbolTable) -> Self {
        Parser {
            tokens,
            symbol_table,
            cursor: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let prog = self.parse_prog()?;
        if self.peek().token_name != TokenClass::EOF {
            return Err(self.error("expected end of input".to_string()));
        }
        Ok(prog)
    }

    // ---------- helpers ----------

    fn peek(&self) -> &Token {
        &self.tokens[self.cursor.min(self.tokens.len() - 1)]
    }

    fn peek_at(&self, offset: usize) -> &Token {
        let idx = (self.cursor + offset).min(self.tokens.len() - 1);
        &self.tokens[idx]
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.cursor];
        if self.cursor < self.tokens.len() - 1 {
            self.cursor += 1;
        }
        tok
    }

    fn is_keyword(token: &Token, kw: &str) -> bool {
        matches!(&token.token_name, TokenClass::KEYWORD(s) if s == kw)
    }

    fn is_delim_str(token: &Token, lex: &str) -> bool {
        matches!(&token.attribute_value, TokenAttribute::Itself(s)
            if token.token_name == TokenClass::DELIMITER && s == lex)
    }

    fn is_op_str(token: &Token, lex: &str) -> bool {
        matches!(&token.attribute_value, TokenAttribute::Itself(s)
            if token.token_name == TokenClass::OPERATION && s == lex)
    }

    fn is_id(token: &Token) -> bool {
        token.token_name == TokenClass::ID
    }

    fn is_number(token: &Token) -> bool {
        token.token_name == TokenClass::NUMBER
    }

    /// Lexeme of the current token (looked up in the symbol table when
    /// attribute is a Pointer).
    fn lexeme_of(&self, token: &Token) -> String {
        match &token.attribute_value {
            TokenAttribute::Pointer(idx) => self.symbol_table.registers[*idx].lexeme.clone(),
            TokenAttribute::Itself(s) => s.clone(),
            TokenAttribute::Null => match &token.token_name {
                TokenClass::KEYWORD(s) => s.clone(),
                _ => String::new(),
            },
        }
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), ParseError> {
        if Self::is_keyword(self.peek(), kw) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected keyword '{kw}'")))
        }
    }

    fn expect_delim(&mut self, lex: &str) -> Result<(), ParseError> {
        if Self::is_delim_str(self.peek(), lex) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected '{lex}'")))
        }
    }

    fn expect_id(&mut self) -> Result<String, ParseError> {
        if Self::is_id(self.peek()) {
            let name = self.lexeme_of(self.peek());
            self.advance();
            Ok(name)
        } else {
            Err(self.error("expected identifier".to_string()))
        }
    }

    fn error(&self, msg: String) -> ParseError {
        let tok = self.peek();
        let got = describe_token(tok);
        ParseError {
            line: tok.line,
            column: tok.column,
            msg: format!("{msg}, got {got}"),
        }
    }

    // ---------- productions ----------

    // Prog → MainC DefCl
    fn parse_prog(&mut self) -> Result<Program, ParseError> {
        let main = self.parse_main_c()?;
        let classes = self.parse_def_cl()?;
        Ok(Program { main, classes })
    }

    // MainC → 'class' Id '{' 'public' 'static' 'void' 'main'
    //         '(' 'String' '[' ']' Id ')' '{' Cmds '}' '}'
    fn parse_main_c(&mut self) -> Result<MainClass, ParseError> {
        self.expect_keyword("class")?;
        let name = self.expect_id()?;
        self.expect_delim("{")?;
        self.expect_keyword("public")?;
        self.expect_keyword("static")?;
        self.expect_keyword("void")?;
        self.expect_keyword("main")?;
        self.expect_delim("(")?;
        self.expect_keyword("String")?;
        self.expect_delim("[")?;
        self.expect_delim("]")?;
        let args_name = self.expect_id()?;
        self.expect_delim(")")?;
        self.expect_delim("{")?;
        let body = self.parse_cmds()?;
        self.expect_delim("}")?;
        self.expect_delim("}")?;
        Ok(MainClass {
            name,
            args_name,
            body,
        })
    }

    // DefCl → 'class' Id DefClHead '{' DefVar DefMet '}' DefCl | λ
    fn parse_def_cl(&mut self) -> Result<Vec<ClassDecl>, ParseError> {
        let mut classes = Vec::new();
        while Self::is_keyword(self.peek(), "class") {
            self.advance();
            let name = self.expect_id()?;
            let extends = if Self::is_keyword(self.peek(), "extends") {
                self.advance();
                Some(self.expect_id()?)
            } else {
                None
            };
            self.expect_delim("{")?;
            let vars = self.parse_def_var()?;
            let methods = self.parse_def_met()?;
            self.expect_delim("}")?;
            classes.push(ClassDecl {
                name,
                extends,
                vars,
                methods,
            });
        }
        Ok(classes)
    }

    // DefVar → Type Id ';' DefVar | λ
    fn parse_def_var(&mut self) -> Result<Vec<VarDecl>, ParseError> {
        let mut vars = Vec::new();
        while self.at_def_var() {
            let ty = self.parse_type()?;
            let name = self.expect_id()?;
            self.expect_delim(";")?;
            vars.push(VarDecl { ty, name });
        }
        Ok(vars)
    }

    fn at_def_var(&self) -> bool {
        let t = self.peek();
        if Self::is_keyword(t, "int") || Self::is_keyword(t, "boolean") {
            return true;
        }
        if Self::is_id(t) {
            return Self::is_id(self.peek_at(1));
        }
        false
    }

    // DefMet → 'public' Type Id '(' ArgsOpt ')'
    //          '{' DefVar Cmds 'return' Exp ';' '}' DefMet
    //        | λ
    fn parse_def_met(&mut self) -> Result<Vec<MethodDecl>, ParseError> {
        let mut methods = Vec::new();
        while Self::is_keyword(self.peek(), "public") {
            self.advance();
            let return_type = self.parse_type()?;
            let name = self.expect_id()?;
            self.expect_delim("(")?;
            let params = if Self::is_delim_str(self.peek(), ")") {
                Vec::new()
            } else {
                self.parse_args()?
            };
            self.expect_delim(")")?;
            self.expect_delim("{")?;
            let vars = self.parse_def_var()?;
            let body = self.parse_cmds()?;
            self.expect_keyword("return")?;
            let return_expr = self.parse_exp()?;
            self.expect_delim(";")?;
            self.expect_delim("}")?;
            methods.push(MethodDecl {
                return_type,
                name,
                params,
                vars,
                body,
                return_expr,
            });
        }
        Ok(methods)
    }

    // Args → Type Id ArgsRest
    // ArgsRest → ',' Args | λ
    fn parse_args(&mut self) -> Result<Vec<VarDecl>, ParseError> {
        let mut args = Vec::new();
        loop {
            let ty = self.parse_type()?;
            let name = self.expect_id()?;
            args.push(VarDecl { ty, name });
            if Self::is_delim_str(self.peek(), ",") {
                self.advance();
                continue;
            }
            break;
        }
        Ok(args)
    }

    // Type → 'int' TypeIntRest | 'boolean' | Id
    // TypeIntRest → '[' ']' | λ
    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let t = self.peek();
        if Self::is_keyword(t, "int") {
            self.advance();
            if Self::is_delim_str(self.peek(), "[") {
                self.advance();
                self.expect_delim("]")?;
                return Ok(Type::IntArray);
            }
            Ok(Type::Int)
        } else if Self::is_keyword(t, "boolean") {
            self.advance();
            Ok(Type::Boolean)
        } else if Self::is_id(t) {
            let name = self.lexeme_of(t);
            self.advance();
            Ok(Type::Class(name))
        } else {
            Err(self.error("expected type ('int', 'boolean', or identifier)".to_string()))
        }
    }

    // Cmds → Cmd Cmds | λ
    fn parse_cmds(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        while self.at_cmd() {
            stmts.push(self.parse_cmd()?);
        }
        Ok(stmts)
    }

    fn at_cmd(&self) -> bool {
        let t = self.peek();
        Self::is_delim_str(t, "{")
            || Self::is_keyword(t, "if")
            || Self::is_keyword(t, "while")
            || Self::is_keyword(t, "System")
            || (Self::is_id(t)
                && (Self::is_delim_str(self.peek_at(1), "=")
                    || Self::is_delim_str(self.peek_at(1), "[")))
    }

    fn parse_cmd(&mut self) -> Result<Stmt, ParseError> {
        let t = self.peek();

        if Self::is_delim_str(t, "{") {
            self.advance();
            let stmts = self.parse_cmds()?;
            self.expect_delim("}")?;
            return Ok(Stmt::Block(stmts));
        }
        if Self::is_keyword(t, "if") {
            self.advance();
            self.expect_delim("(")?;
            let cond = self.parse_exp()?;
            self.expect_delim(")")?;
            let then_branch = self.parse_cmd()?;
            self.expect_keyword("else")?;
            let else_branch = self.parse_cmd()?;
            return Ok(Stmt::If(
                cond,
                Box::new(then_branch),
                Box::new(else_branch),
            ));
        }
        if Self::is_keyword(t, "while") {
            self.advance();
            self.expect_delim("(")?;
            let cond = self.parse_exp()?;
            self.expect_delim(")")?;
            let body = self.parse_cmd()?;
            return Ok(Stmt::While(cond, Box::new(body)));
        }
        if Self::is_keyword(t, "System") {
            self.advance();
            self.expect_delim(".")?;
            self.expect_keyword("out")?;
            self.expect_delim(".")?;
            self.expect_keyword("println")?;
            self.expect_delim("(")?;
            let expr = self.parse_exp()?;
            self.expect_delim(")")?;
            self.expect_delim(";")?;
            return Ok(Stmt::Println(expr));
        }
        if Self::is_id(t) {
            let name = self.lexeme_of(t);
            self.advance();
            return self.parse_cmd_id_rest(name);
        }
        Err(self.error("expected statement".to_string()))
    }

    fn parse_cmd_id_rest(&mut self, name: String) -> Result<Stmt, ParseError> {
        let t = self.peek();
        if Self::is_delim_str(t, "=") {
            self.advance();
            let expr = self.parse_exp()?;
            self.expect_delim(";")?;
            return Ok(Stmt::Assign(name, expr));
        }
        if Self::is_delim_str(t, "[") {
            self.advance();
            let index = self.parse_exp()?;
            self.expect_delim("]")?;
            self.expect_delim("=")?;
            let value = self.parse_exp()?;
            self.expect_delim(";")?;
            return Ok(Stmt::ArrayAssign(name, index, value));
        }
        Err(self.error("expected '=' or '[' after identifier".to_string()))
    }

    // ----------------------------------------------------------------
    // Expressões estratificadas por precedência. Cada nível associa
    // à esquerda — usamos um `while` iterativo em vez de recursão à
    // direita pra que `a+b+c` parseie como `(a+b)+c`.
    // ----------------------------------------------------------------

    fn parse_exp(&mut self) -> Result<Expr, ParseError> {
        self.parse_exp_and()
    }

    fn parse_exp_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_exp_rel()?;
        while Self::is_op_str(self.peek(), "&&") {
            self.advance();
            let right = self.parse_exp_rel()?;
            left = Expr::Binary(BinOp::And, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_exp_rel(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_exp_add()?;
        loop {
            let op = if Self::is_op_str(self.peek(), "<") {
                BinOp::Lt
            } else if Self::is_op_str(self.peek(), ">") {
                BinOp::Gt
            } else {
                break;
            };
            self.advance();
            let right = self.parse_exp_add()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_exp_add(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_exp_mul()?;
        loop {
            let op = if Self::is_op_str(self.peek(), "+") {
                BinOp::Add
            } else if Self::is_op_str(self.peek(), "-") {
                BinOp::Sub
            } else {
                break;
            };
            self.advance();
            let right = self.parse_exp_mul()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_exp_mul(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_exp_unary()?;
        while Self::is_op_str(self.peek(), "*") {
            self.advance();
            let right = self.parse_exp_unary()?;
            left = Expr::Binary(BinOp::Mul, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_exp_unary(&mut self) -> Result<Expr, ParseError> {
        if Self::is_op_str(self.peek(), "!") {
            self.advance();
            let inner = self.parse_exp_unary()?;
            return Ok(Expr::Not(Box::new(inner)));
        }
        self.parse_exp_postfix()
    }

    fn parse_exp_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_exp_primary()?;
        loop {
            let t = self.peek();
            if Self::is_delim_str(t, "[") {
                self.advance();
                let index = self.parse_exp()?;
                self.expect_delim("]")?;
                expr = Expr::Index(Box::new(expr), Box::new(index));
                continue;
            }
            if Self::is_delim_str(t, ".") {
                self.advance();
                expr = self.parse_dot_rest(expr)?;
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_exp_primary(&mut self) -> Result<Expr, ParseError> {
        let t = self.peek();

        if Self::is_keyword(t, "new") {
            self.advance();
            return self.parse_new_rest();
        }
        if Self::is_delim_str(t, "(") {
            self.advance();
            let inner = self.parse_exp()?;
            self.expect_delim(")")?;
            return Ok(inner);
        }
        if Self::is_keyword(t, "true") {
            self.advance();
            return Ok(Expr::Bool(true));
        }
        if Self::is_keyword(t, "false") {
            self.advance();
            return Ok(Expr::Bool(false));
        }
        if Self::is_keyword(t, "this") {
            self.advance();
            return Ok(Expr::This);
        }
        if Self::is_id(t) {
            let name = self.lexeme_of(t);
            self.advance();
            return Ok(Expr::Id(name));
        }
        if Self::is_number(t) {
            let val = self.lexeme_of(t);
            self.advance();
            return Ok(Expr::Number(val));
        }
        Err(self.error("expected expression".to_string()))
    }

    // NewRest → Id '(' ')' | 'int' '[' Exp ']'
    fn parse_new_rest(&mut self) -> Result<Expr, ParseError> {
        let t = self.peek();
        if Self::is_keyword(t, "int") {
            self.advance();
            self.expect_delim("[")?;
            let size = self.parse_exp()?;
            self.expect_delim("]")?;
            return Ok(Expr::NewIntArray(Box::new(size)));
        }
        if Self::is_id(t) {
            let name = self.lexeme_of(t);
            self.advance();
            self.expect_delim("(")?;
            self.expect_delim(")")?;
            return Ok(Expr::NewObject(name));
        }
        Err(self.error("expected identifier or 'int' after 'new'".to_string()))
    }

    // DotRest → 'length' | Id '(' ListExpOpt ')'
    fn parse_dot_rest(&mut self, target: Expr) -> Result<Expr, ParseError> {
        let t = self.peek();
        if Self::is_keyword(t, "length") {
            self.advance();
            return Ok(Expr::Length(Box::new(target)));
        }
        if Self::is_id(t) {
            let method = self.lexeme_of(t);
            self.advance();
            self.expect_delim("(")?;
            let args = if Self::is_delim_str(self.peek(), ")") {
                Vec::new()
            } else {
                self.parse_list_exp()?
            };
            self.expect_delim(")")?;
            return Ok(Expr::Call(Box::new(target), method, args));
        }
        Err(self.error("expected 'length' or method name after '.'".to_string()))
    }

    fn parse_list_exp(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        loop {
            args.push(self.parse_exp()?);
            if Self::is_delim_str(self.peek(), ",") {
                self.advance();
                continue;
            }
            break;
        }
        Ok(args)
    }
}

fn describe_token(token: &Token) -> String {
    match &token.token_name {
        TokenClass::ID => "identifier".to_string(),
        TokenClass::NUMBER => "number".to_string(),
        TokenClass::KEYWORD(s) => format!("keyword '{s}'"),
        TokenClass::OPERATION => match &token.attribute_value {
            TokenAttribute::Itself(s) => format!("operator '{s}'"),
            _ => "operator".to_string(),
        },
        TokenClass::DELIMITER => match &token.attribute_value {
            TokenAttribute::Itself(s) => format!("'{s}'"),
            _ => "delimiter".to_string(),
        },
        TokenClass::EOF => "end of input".to_string(),
        TokenClass::UNKNOWN => "unknown token".to_string(),
    }
}

/// Convenience function: builds a Parser and runs it.
pub fn parse(tokens: &[Token], symbol_table: &SymbolTable) -> Result<Program, ParseError> {
    Parser::new(tokens, symbol_table).parse()
}
