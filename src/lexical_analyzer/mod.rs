//! Self-contained lexical analyzer for the WHILE/MiniJava variant.
//!
//! Unlike the previous version, this lexer no longer relies on a preprocessor:
//! it skips whitespace and comments (`// ...` line comments and `/* ... */`
//! block comments) on its own and reports the exact line/column of any
//! malformed token.
//!
//! `tokenize` returns the token stream, the list of lexical errors collected
//! along the way, and the interned literal table (identifiers and numbers).
//! In `fail_fast` mode scanning stops at the first lexical error.

use std::collections::HashMap;

/// Reserved words of the language. Used both to classify keyword tokens and to
/// power Levenshtein-based suggestions for malformed identifiers.
pub const KEYWORDS_LIST: &[&str] = &[
    "class", "public", "extends", "else", "int", "static", "void", "main", "String", "return",
    "boolean", "if", "while", "System", "out", "println", "length", "new", "true", "false", "this",
];

/// Lexical class of a token. The concrete lexeme lives in [`Token::lexeme`];
/// keyword/operator/delimiter matching is done by comparing that string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenClass {
    Id,
    Number,
    Keyword,
    Operation,
    Delimiter,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub class: TokenClass,
    /// Raw text of the token (`"if"`, `"&&"`, `"42"`, `"foo"`). EOF is empty.
    pub lexeme: String,
    /// Index into the interned [`SymbolTable`] for `Id`/`Number`; `None` otherwise.
    pub symbol: Option<usize>,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn is_keyword(&self, kw: &str) -> bool {
        self.class == TokenClass::Keyword && self.lexeme == kw
    }
    pub fn is_op(&self, op: &str) -> bool {
        self.class == TokenClass::Operation && self.lexeme == op
    }
    pub fn is_delim(&self, d: &str) -> bool {
        self.class == TokenClass::Delimiter && self.lexeme == d
    }
    pub fn is_id(&self) -> bool {
        self.class == TokenClass::Id
    }
    pub fn is_number(&self) -> bool {
        self.class == TokenClass::Number
    }
    pub fn is_eof(&self) -> bool {
        self.class == TokenClass::Eof
    }

    /// Human-friendly description used in diagnostics.
    pub fn describe(&self) -> String {
        match self.class {
            TokenClass::Id => format!("identifier '{}'", self.lexeme),
            TokenClass::Number => format!("number '{}'", self.lexeme),
            TokenClass::Keyword => format!("keyword '{}'", self.lexeme),
            TokenClass::Operation => format!("operator '{}'", self.lexeme),
            TokenClass::Delimiter => format!("'{}'", self.lexeme),
            TokenClass::Eof => "end of input".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub line: usize,
    pub column: usize,
    pub msg: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Number,
}

/// One entry in the interned literal table.
#[derive(Debug, Clone)]
pub struct SymbolTableEntry {
    pub lexeme: String,
    pub kind: SymbolKind,
    pub first_line: usize,
    pub first_column: usize,
}

/// Interned table of identifier and number literals seen by the lexer. This is
/// the classic compiler "string table"; the richer, scoped symbol table built
/// during parsing lives in [`crate::symbol_table`].
#[derive(Debug, Default)]
pub struct SymbolTable {
    pub registers: Vec<SymbolTableEntry>,
    intern_map: HashMap<String, usize>,
}

impl SymbolTable {
    fn new() -> Self {
        SymbolTable::default()
    }

    fn intern(&mut self, lexeme: &str, kind: SymbolKind, line: usize, column: usize) -> usize {
        if let Some(&idx) = self.intern_map.get(lexeme) {
            return idx;
        }
        let idx = self.registers.len();
        self.registers.push(SymbolTableEntry {
            lexeme: lexeme.to_string(),
            kind,
            first_line: line,
            first_column: column,
        });
        self.intern_map.insert(lexeme.to_string(), idx);
        idx
    }
}

/// Outcome of a lexical pass: the token stream (always terminated by `Eof`),
/// every lexical error found (empty on success), and the interned table.
pub struct LexResult {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
    pub symbols: SymbolTable,
}

struct Scanner {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Scanner {
    fn new(src: &str) -> Self {
        Scanner {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    /// Advance one character, maintaining line/column counters.
    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }
}

/// Tokenize `src`. When `fail_fast` is true, scanning stops at the first
/// lexical error; otherwise it recovers and reports every error.
pub fn tokenize(src: &str, fail_fast: bool) -> LexResult {
    let mut sc = Scanner::new(src);
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut symbols = SymbolTable::new();

    'outer: loop {
        // Skip whitespace and comments between tokens.
        loop {
            match sc.peek() {
                Some(c) if c.is_whitespace() => {
                    sc.bump();
                }
                Some('/') if sc.peek2() == Some('/') => {
                    // Line comment: consume up to (not including) the newline.
                    while let Some(c) = sc.peek() {
                        if c == '\n' {
                            break;
                        }
                        sc.bump();
                    }
                }
                Some('/') if sc.peek2() == Some('*') => {
                    let start_line = sc.line;
                    let start_col = sc.column;
                    sc.bump(); // '/'
                    sc.bump(); // '*'
                    let mut closed = false;
                    while let Some(c) = sc.bump() {
                        if c == '*' && sc.peek() == Some('/') {
                            sc.bump(); // '/'
                            closed = true;
                            break;
                        }
                    }
                    if !closed {
                        errors.push(LexError {
                            line: start_line,
                            column: start_col,
                            msg: "unclosed block comment".to_string(),
                        });
                        if fail_fast {
                            break 'outer;
                        }
                    }
                }
                _ => break,
            }
        }

        let Some(c) = sc.peek() else { break };
        let start_line = sc.line;
        let start_column = sc.column;

        if c.is_alphabetic() {
            let lexeme = scan_word(&mut sc);
            let class = if KEYWORDS_LIST.contains(&lexeme.as_str()) {
                TokenClass::Keyword
            } else {
                TokenClass::Id
            };
            let symbol = if class == TokenClass::Id {
                Some(symbols.intern(&lexeme, SymbolKind::Variable, start_line, start_column))
            } else {
                None
            };
            tokens.push(Token {
                class,
                lexeme,
                symbol,
                line: start_line,
                column: start_column,
            });
            continue;
        }

        if c.is_ascii_digit() {
            let lexeme = scan_number(&mut sc);
            // A number immediately followed by a letter is a malformed token
            // (e.g. `12ab`). Report it but consume the trailing word so we
            // don't spam an error per character.
            if matches!(sc.peek(), Some(ch) if ch.is_alphabetic() || ch == '_') {
                let bad_tail = scan_word(&mut sc);
                errors.push(LexError {
                    line: start_line,
                    column: start_column,
                    msg: format!("malformed number/identifier '{lexeme}{bad_tail}'"),
                });
                if fail_fast {
                    break;
                }
                continue;
            }
            let symbol = symbols.intern(&lexeme, SymbolKind::Number, start_line, start_column);
            tokens.push(Token {
                class: TokenClass::Number,
                lexeme,
                symbol: Some(symbol),
                line: start_line,
                column: start_column,
            });
            continue;
        }

        // Operators and delimiters.
        if let Some((lexeme, class)) = scan_symbolic(&mut sc) {
            tokens.push(Token {
                class,
                lexeme,
                symbol: None,
                line: start_line,
                column: start_column,
            });
            continue;
        }

        // Nothing matched: malformed lexeme. Consume the single offending char.
        let bad = sc.bump().unwrap();
        errors.push(LexError {
            line: start_line,
            column: start_column,
            msg: format!("unrecognized character '{bad}'"),
        });
        if fail_fast {
            break;
        }
    }

    tokens.push(Token {
        class: TokenClass::Eof,
        lexeme: String::new(),
        symbol: None,
        line: sc.line,
        column: sc.column,
    });

    LexResult {
        tokens,
        errors,
        symbols,
    }
}

/// Identifier/word: `[A-Za-z][A-Za-z0-9_]*`.
fn scan_word(sc: &mut Scanner) -> String {
    let mut s = String::new();
    while let Some(c) = sc.peek() {
        if c.is_alphanumeric() || c == '_' {
            s.push(c);
            sc.bump();
        } else {
            break;
        }
    }
    s
}

/// Number literal: one or more digits.
fn scan_number(sc: &mut Scanner) -> String {
    let mut s = String::new();
    while let Some(c) = sc.peek() {
        if c.is_ascii_digit() {
            s.push(c);
            sc.bump();
        } else {
            break;
        }
    }
    s
}

/// Operator or delimiter starting at the cursor. Returns `None` if the current
/// character begins no valid symbolic token (so the caller can report it).
fn scan_symbolic(sc: &mut Scanner) -> Option<(String, TokenClass)> {
    let c = sc.peek()?;
    match c {
        '&' => {
            if sc.peek2() == Some('&') {
                sc.bump();
                sc.bump();
                Some(("&&".to_string(), TokenClass::Operation))
            } else {
                // Lone '&' is not a valid token; let the caller report it.
                None
            }
        }
        '<' | '+' | '-' | '*' | '!' => {
            sc.bump();
            Some((c.to_string(), TokenClass::Operation))
        }
        '{' | '}' | '(' | ')' | '[' | ']' | ';' | ',' | '.' | '=' => {
            sc.bump();
            Some((c.to_string(), TokenClass::Delimiter))
        }
        _ => None,
    }
}

/// If `lexeme` is within edit distance ≤ 2 of a reserved keyword, return that
/// keyword. Used to enrich diagnostics with a "did you mean" suggestion.
pub fn suggest_keyword(lexeme: &str) -> Option<String> {
    if lexeme.is_empty() {
        return None;
    }
    KEYWORDS_LIST
        .iter()
        .map(|kw| (*kw, strsim::levenshtein(lexeme, kw)))
        .filter(|(_, d)| *d <= 2)
        .min_by_key(|(_, d)| *d)
        .map(|(kw, _)| kw.to_string())
}
