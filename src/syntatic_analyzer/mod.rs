//! Recursive-descent parser for the transformed grammar in
//! `specs/gramatica-transformada.md`.
//!
//! The parser consumes the token stream produced by `lexical_analyzer` and
//! reports `Ok(())` if the program is syntactically valid, or
//! `Err(ParseError)` with line/column when it isn't. No AST is produced —
//! the assignment requires only a yes/no answer plus the symbol table that
//! the lexer already populated.

use crate::lexical_analyzer::{Token, TokenAttribute, TokenClass};

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub column: usize,
    pub msg: String,
}

pub struct Parser<'a> {
    tokens: &'a [Token],
    cursor: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Parser { tokens, cursor: 0 }
    }

    pub fn parse(&mut self) -> Result<(), ParseError> {
        self.parse_prog()
    }

    // ---------- helpers ----------

    fn peek(&self) -> &Token {
        // The token list always ends with EOF, so cursor is always in range.
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

    fn expect_id(&mut self) -> Result<(), ParseError> {
        if Self::is_id(self.peek()) {
            self.advance();
            Ok(())
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
    fn parse_prog(&mut self) -> Result<(), ParseError> {
        self.parse_main_c()?;
        self.parse_def_cl()?;
        if self.peek().token_name != TokenClass::EOF {
            return Err(self.error("expected end of input".to_string()));
        }
        Ok(())
    }

    // MainC → 'class' Id '{' 'public' 'static' 'void' 'main'
    //         '(' 'String' '[' ']' Id ')' '{' Cmds '}' '}'
    fn parse_main_c(&mut self) -> Result<(), ParseError> {
        self.expect_keyword("class")?;
        self.expect_id()?;
        self.expect_delim("{")?;
        self.expect_keyword("public")?;
        self.expect_keyword("static")?;
        self.expect_keyword("void")?;
        self.expect_keyword("main")?;
        self.expect_delim("(")?;
        self.expect_keyword("String")?;
        self.expect_delim("[")?;
        self.expect_delim("]")?;
        self.expect_id()?;
        self.expect_delim(")")?;
        self.expect_delim("{")?;
        self.parse_cmds()?;
        self.expect_delim("}")?;
        self.expect_delim("}")?;
        Ok(())
    }

    // DefCl → 'class' Id DefClHead '{' DefVar DefMet '}' DefCl | λ
    // DefClHead → 'extends' Id | λ
    fn parse_def_cl(&mut self) -> Result<(), ParseError> {
        while Self::is_keyword(self.peek(), "class") {
            self.advance();
            self.expect_id()?;
            if Self::is_keyword(self.peek(), "extends") {
                self.advance();
                self.expect_id()?;
            }
            self.expect_delim("{")?;
            self.parse_def_var()?;
            self.parse_def_met()?;
            self.expect_delim("}")?;
        }
        Ok(())
    }

    // DefVar → Type Id ';' DefVar | λ
    fn parse_def_var(&mut self) -> Result<(), ParseError> {
        while self.at_def_var() {
            self.parse_type()?;
            self.expect_id()?;
            self.expect_delim(";")?;
        }
        Ok(())
    }

    /// Returns true when the next tokens unambiguously start a `Type Id ;`
    /// declaration (DefVar) rather than a Cmd. Disambiguation uses lookahead-2
    /// because a custom-type declaration `MyClass x;` has `Id Id` shape.
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
    fn parse_def_met(&mut self) -> Result<(), ParseError> {
        while Self::is_keyword(self.peek(), "public") {
            self.advance();
            self.parse_type()?;
            self.expect_id()?;
            self.expect_delim("(")?;
            if !Self::is_delim_str(self.peek(), ")") {
                self.parse_args()?;
            }
            self.expect_delim(")")?;
            self.expect_delim("{")?;
            self.parse_def_var()?;
            self.parse_cmds()?;
            self.expect_keyword("return")?;
            self.parse_exp()?;
            self.expect_delim(";")?;
            self.expect_delim("}")?;
        }
        Ok(())
    }

    // Args → Type Id ArgsRest
    // ArgsRest → ',' Args | λ
    fn parse_args(&mut self) -> Result<(), ParseError> {
        loop {
            self.parse_type()?;
            self.expect_id()?;
            if Self::is_delim_str(self.peek(), ",") {
                self.advance();
                continue;
            }
            break;
        }
        Ok(())
    }

    // Type → 'int' TypeIntRest | 'boolean' | Id
    // TypeIntRest → '[' ']' | λ
    fn parse_type(&mut self) -> Result<(), ParseError> {
        let t = self.peek();
        if Self::is_keyword(t, "int") {
            self.advance();
            if Self::is_delim_str(self.peek(), "[") {
                self.advance();
                self.expect_delim("]")?;
            }
            Ok(())
        } else if Self::is_keyword(t, "boolean") {
            self.advance();
            Ok(())
        } else if Self::is_id(t) {
            self.advance();
            Ok(())
        } else {
            Err(self.error("expected type ('int', 'boolean', or identifier)".to_string()))
        }
    }

    // Cmds → Cmd Cmds | λ
    fn parse_cmds(&mut self) -> Result<(), ParseError> {
        while self.at_cmd() {
            self.parse_cmd()?;
        }
        Ok(())
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

    // Cmd → '{' Cmds '}'
    //     | 'if' '(' Exp ')' Cmd 'else' Cmd
    //     | 'while' '(' Exp ')' Cmd
    //     | 'System' '.' 'out' '.' 'println' '(' Exp ')' ';'
    //     | Id CmdIdRest
    fn parse_cmd(&mut self) -> Result<(), ParseError> {
        let t = self.peek();

        if Self::is_delim_str(t, "{") {
            self.advance();
            self.parse_cmds()?;
            self.expect_delim("}")?;
            return Ok(());
        }
        if Self::is_keyword(t, "if") {
            self.advance();
            self.expect_delim("(")?;
            self.parse_exp()?;
            self.expect_delim(")")?;
            self.parse_cmd()?;
            self.expect_keyword("else")?;
            self.parse_cmd()?;
            return Ok(());
        }
        if Self::is_keyword(t, "while") {
            self.advance();
            self.expect_delim("(")?;
            self.parse_exp()?;
            self.expect_delim(")")?;
            self.parse_cmd()?;
            return Ok(());
        }
        if Self::is_keyword(t, "System") {
            self.advance();
            self.expect_delim(".")?;
            self.expect_keyword("out")?;
            self.expect_delim(".")?;
            self.expect_keyword("println")?;
            self.expect_delim("(")?;
            self.parse_exp()?;
            self.expect_delim(")")?;
            self.expect_delim(";")?;
            return Ok(());
        }
        if Self::is_id(t) {
            self.advance(); // consume the Id
            return self.parse_cmd_id_rest();
        }
        Err(self.error("expected statement".to_string()))
    }

    // CmdIdRest → '=' Exp ';' | '[' Exp ']' '=' Exp ';'
    fn parse_cmd_id_rest(&mut self) -> Result<(), ParseError> {
        let t = self.peek();
        if Self::is_delim_str(t, "=") {
            self.advance();
            self.parse_exp()?;
            self.expect_delim(";")?;
            return Ok(());
        }
        if Self::is_delim_str(t, "[") {
            self.advance();
            self.parse_exp()?;
            self.expect_delim("]")?;
            self.expect_delim("=")?;
            self.parse_exp()?;
            self.expect_delim(";")?;
            return Ok(());
        }
        Err(self.error("expected '=' or '[' after identifier".to_string()))
    }

    // Exp → ExpBase ExpRest
    fn parse_exp(&mut self) -> Result<(), ParseError> {
        self.parse_exp_base()?;
        self.parse_exp_rest()
    }

    // ExpBase → 'new' NewRest | '!' Exp | '(' Exp ')'
    //         | 'true' | 'false' | 'this' | Id | Number
    fn parse_exp_base(&mut self) -> Result<(), ParseError> {
        let t = self.peek();

        if Self::is_keyword(t, "new") {
            self.advance();
            return self.parse_new_rest();
        }
        if Self::is_op_str(t, "!") {
            self.advance();
            return self.parse_exp();
        }
        if Self::is_delim_str(t, "(") {
            self.advance();
            self.parse_exp()?;
            self.expect_delim(")")?;
            return Ok(());
        }
        if Self::is_keyword(t, "true")
            || Self::is_keyword(t, "false")
            || Self::is_keyword(t, "this")
        {
            self.advance();
            return Ok(());
        }
        if Self::is_id(t) || Self::is_number(t) {
            self.advance();
            return Ok(());
        }
        Err(self.error("expected expression".to_string()))
    }

    // NewRest → Id '(' ')' | 'int' '[' Exp ']'
    fn parse_new_rest(&mut self) -> Result<(), ParseError> {
        let t = self.peek();
        if Self::is_keyword(t, "int") {
            self.advance();
            self.expect_delim("[")?;
            self.parse_exp()?;
            self.expect_delim("]")?;
            return Ok(());
        }
        if Self::is_id(t) {
            self.advance();
            self.expect_delim("(")?;
            self.expect_delim(")")?;
            return Ok(());
        }
        Err(self.error("expected identifier or 'int' after 'new'".to_string()))
    }

    // ExpRest → '&&' Exp ExpRest | '>' Exp ExpRest | '+' Exp ExpRest
    //         | '-' Exp ExpRest | '*' Exp ExpRest
    //         | '[' Exp ']' ExpRest
    //         | '.' DotRest ExpRest
    //         | λ
    fn parse_exp_rest(&mut self) -> Result<(), ParseError> {
        loop {
            let t = self.peek();
            if Self::is_op_str(t, "&&")
                || Self::is_op_str(t, "<")
                || Self::is_op_str(t, ">")
                || Self::is_op_str(t, "+")
                || Self::is_op_str(t, "-")
                || Self::is_op_str(t, "*")
            {
                self.advance();
                self.parse_exp()?;
                continue;
            }
            if Self::is_delim_str(t, "[") {
                self.advance();
                self.parse_exp()?;
                self.expect_delim("]")?;
                continue;
            }
            if Self::is_delim_str(t, ".") {
                self.advance();
                self.parse_dot_rest()?;
                continue;
            }
            break;
        }
        Ok(())
    }

    // DotRest → 'length' | Id '(' ListExpOpt ')'
    fn parse_dot_rest(&mut self) -> Result<(), ParseError> {
        let t = self.peek();
        if Self::is_keyword(t, "length") {
            self.advance();
            return Ok(());
        }
        if Self::is_id(t) {
            self.advance();
            self.expect_delim("(")?;
            if !Self::is_delim_str(self.peek(), ")") {
                self.parse_list_exp()?;
            }
            self.expect_delim(")")?;
            return Ok(());
        }
        Err(self.error("expected 'length' or method name after '.'".to_string()))
    }

    // ListExpOpt → Exp ListExpRest | λ
    // ListExpRest → ',' Exp ListExpRest | λ
    fn parse_list_exp(&mut self) -> Result<(), ParseError> {
        loop {
            self.parse_exp()?;
            if Self::is_delim_str(self.peek(), ",") {
                self.advance();
                continue;
            }
            break;
        }
        Ok(())
    }
}

fn describe_token(token: &Token) -> String {
    match &token.token_name {
        TokenClass::ID => match &token.attribute_value {
            TokenAttribute::Pointer(_) => "identifier".to_string(),
            _ => "identifier".to_string(),
        },
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

/// Convenience: parse a token slice and return Ok/Err.
pub fn parse(tokens: &[Token]) -> Result<(), ParseError> {
    Parser::new(tokens).parse()
}
