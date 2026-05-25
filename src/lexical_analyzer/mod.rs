use crate::preprocessor::SourceMap;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

const KEYWORDS_PATTERN: &str = "^(class|public|extends|else|int|static|void|main|String|return|boolean|if|while|System|out|println|length|new|true|false|this)$";
const OPERATIONS_PATTERN: &str = r"^(\&\&|<|>|\+|\-|\*|\!)$";
const DELIMITERS_PATTERN: &str = r"^(\{|\}|\(|\)|\[|\]|;|,|\.|=)$";

/// Flat list of reserved words, used both for the keyword regex and for
/// Levenshtein-based suggestion on invalid identifiers.
pub const KEYWORDS_LIST: &[&str] = &[
    "class", "public", "extends", "else", "int", "static", "void", "main", "String", "return",
    "boolean", "if", "while", "System", "out", "println", "length", "new", "true", "false", "this",
];

static ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Identifier: starts with alpha, then any of (alpha|digit|_) zero or more
    // times. The original grammar restricts `_` to the trailing position
    // (Word's terminal `_` alternative), but the test fixtures rely on the
    // permissive C/Java-style form (e.g. `num_aux`, `aux01`).
    Regex::new(r"^[[:alpha:]]([[:digit:]]|[[:alpha:]]|_)*$")
        .expect("Lexical Analyzer - Id Regex Failed")
});
static NUMBER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[[:digit:]]+$").expect("Lexical Analyzer - Number Regex Failed"));
static KEYWORD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(KEYWORDS_PATTERN).expect("Lexical Analyzer - Keyword Regex Failed")
});
static OPERATION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(OPERATIONS_PATTERN).expect("Lexical Analyzer - Operations Regex Failed")
});
static DELIMITER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(DELIMITERS_PATTERN).expect("Lexical Analyzer - Delimiters Regex Failed")
});

#[derive(Debug, Clone, PartialEq)]
pub enum TokenClass {
    ID,
    NUMBER,
    KEYWORD(String),
    OPERATION,
    DELIMITER,
    EOF,
    UNKNOWN,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenAttribute {
    /// Pointer (index) into the SymbolTable. Used by ID and NUMBER.
    Pointer(usize),
    /// The lexeme itself, inlined into the token. Used by OPERATION and DELIMITER.
    Itself(String),
    /// No attribute (KEYWORD carries the lexeme in its class variant; EOF has nothing).
    Null,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_name: TokenClass,
    pub attribute_value: TokenAttribute,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Number,
}

#[derive(Debug, Clone)]
pub struct SymbolTableEntry {
    pub lexeme: String,
    pub kind: SymbolKind,
    /// Filled in by a later phase (semantic analyzer). Lexer leaves None.
    pub type_info: Option<String>,
    pub first_line: usize,
    pub first_column: usize,
}

#[derive(Debug)]
pub struct SymbolTable {
    pub registers: Vec<SymbolTableEntry>,
    intern_map: HashMap<String, usize>,
}

impl SymbolTable {
    fn new() -> Self {
        SymbolTable {
            registers: Vec::new(),
            intern_map: HashMap::new(),
        }
    }

    /// Interning: returns the existing index for `lexeme` if present, else
    /// inserts a new entry and returns its index. Two occurrences of the same
    /// lexeme yield the same pointer.
    fn intern(&mut self, lexeme: &str, kind: SymbolKind, line: usize, column: usize) -> usize {
        if let Some(&idx) = self.intern_map.get(lexeme) {
            return idx;
        }
        let idx = self.registers.len();
        self.registers.push(SymbolTableEntry {
            lexeme: lexeme.to_string(),
            kind,
            type_info: None,
            first_line: line,
            first_column: column,
        });
        self.intern_map.insert(lexeme.to_string(), idx);
        idx
    }
}

#[derive(Debug)]
pub struct ErrorUnrecognizedLexeme {
    pub line: usize,
    pub column: usize,
    pub msg: String,
}

#[derive(Debug, Clone)]
enum TokenClassTag {
    Id,
    Number,
    Keyword(String),
    Operation,
    Delimiter,
    Unknown,
}

pub fn get_tokens(src: &str) -> (Vec<Token>, SymbolTable) {
    get_tokens_with_map(src, None)
}

pub fn get_tokens_with_map(src: &str, map: Option<&SourceMap>) -> (Vec<Token>, SymbolTable) {
    let mut tokens = Vec::new();
    let mut symbol_table = SymbolTable::new();

    let mut line = 1usize;
    let mut column = 1usize;

    let mut cursor = 0;
    while cursor < src.len() {
        match get_token(src, cursor, map, &mut symbol_table, &mut line, &mut column) {
            Ok((Some(token), consumed)) => {
                tokens.push(token);
                cursor += consumed;
            }
            Ok((None, consumed)) => {
                // Only whitespace/newlines were skipped; no token produced.
                cursor += consumed;
            }
            Err(err) => {
                eprintln!(
                    "Lexical error at line {}, column {}: {}",
                    err.line, err.column, err.msg
                );
                // Skip the offending char and continue so the parser can see EOF.
                cursor += 1;
            }
        }
    }

    tokens.push(Token {
        token_name: TokenClass::EOF,
        attribute_value: TokenAttribute::Null,
        line,
        column,
    });

    (tokens, symbol_table)
}

/// Consume one token from `src` starting at `cursor`.
///
/// Returns `(Some(token), consumed)` if a token was produced, or
/// `(None, consumed)` if only whitespace/newlines were skipped (no token).
fn get_token(
    full_src: &str,
    cursor: usize,
    map: Option<&SourceMap>,
    symbol_table: &mut SymbolTable,
    line: &mut usize,
    column: &mut usize,
) -> Result<(Option<Token>, usize), ErrorUnrecognizedLexeme> {
    let src = &full_src[cursor..];
    let mut lexeme = String::new();
    let mut consumed: usize = 0;
    let mut token_tag = TokenClassTag::Unknown;

    // Initial position
    if let Some(m) = map {
        let pos = m.get(cursor);
        *line = pos.0;
        *column = pos.1;
    }
    let mut start_line = *line;
    let mut start_column = *column;

    for c in src.chars() {
        let char_len = c.len_utf8();

        match c {
            '\n' => {
                if lexeme.is_empty() {
                    consumed += char_len;
                    if let Some(m) = map {
                        let pos = m.get(cursor + consumed);
                        *line = pos.0;
                        *column = pos.1;
                    } else {
                        *line += 1;
                        *column = 1;
                    }
                    start_line = *line;
                    start_column = *column;
                    continue;
                }
                let token =
                    build_token(&lexeme, &token_tag, symbol_table, start_line, start_column)?;
                return Ok((Some(token), consumed));
            }
            ' ' | '\t' | '\r' => {
                if lexeme.is_empty() {
                    consumed += char_len;
                    if let Some(m) = map {
                        let pos = m.get(cursor + consumed);
                        *line = pos.0;
                        *column = pos.1;
                    } else {
                        *column += 1;
                    }
                    start_line = *line;
                    start_column = *column;
                    continue;
                }
                let token =
                    build_token(&lexeme, &token_tag, symbol_table, start_line, start_column)?;
                return Ok((Some(token), consumed));
            }
            _ => {}
        }

        lexeme.push(c);
        consumed += char_len;

        // Update line/column for the NEXT character
        if let Some(m) = map {
            let pos = m.get(cursor + consumed);
            *line = pos.0;
            *column = pos.1;
        } else {
            *column += 1;
        }

        if NUMBER_REGEX.is_match(&lexeme) {
            token_tag = TokenClassTag::Number;
        } else if KEYWORD_REGEX.is_match(&lexeme) {
            token_tag = TokenClassTag::Keyword(lexeme.clone());
        } else if ID_REGEX.is_match(&lexeme) {
            token_tag = TokenClassTag::Id;
        } else if DELIMITER_REGEX.is_match(&lexeme) {
            token_tag = TokenClassTag::Delimiter;
        } else if OPERATION_REGEX.is_match(&lexeme) {
            token_tag = TokenClassTag::Operation;
        } else if lexeme == "&" {
            // Special case: `&` alone matches nothing, but `&&` does. Keep
            // extending so the next `&` completes the operator.
            continue;
        } else {
            // The current char broke every classification. Roll it back and
            // emit whatever we had before.
            lexeme.pop();
            consumed -= char_len;
            if let Some(m) = map {
                let pos = m.get(cursor + consumed);
                *line = pos.0;
                *column = pos.1;
            } else {
                *column -= 1;
            }
            let token = build_token(&lexeme, &token_tag, symbol_table, start_line, start_column)?;
            return Ok((Some(token), consumed));
        }
    }

    // End of input: emit whatever we accumulated, if anything.
    if matches!(token_tag, TokenClassTag::Unknown) && lexeme.is_empty() {
        return Ok((None, consumed));
    }
    let token = build_token(&lexeme, &token_tag, symbol_table, start_line, start_column)?;
    Ok((Some(token), consumed))
}

fn build_token(
    lexeme: &str,
    tag: &TokenClassTag,
    symbol_table: &mut SymbolTable,
    line: usize,
    column: usize,
) -> Result<Token, ErrorUnrecognizedLexeme> {
    match tag {
        TokenClassTag::Id => {
            let ptr = symbol_table.intern(lexeme, SymbolKind::Variable, line, column);
            Ok(Token {
                token_name: TokenClass::ID,
                attribute_value: TokenAttribute::Pointer(ptr),
                line,
                column,
            })
        }
        TokenClassTag::Number => {
            let ptr = symbol_table.intern(lexeme, SymbolKind::Number, line, column);
            Ok(Token {
                token_name: TokenClass::NUMBER,
                attribute_value: TokenAttribute::Pointer(ptr),
                line,
                column,
            })
        }
        TokenClassTag::Keyword(kw) => Ok(Token {
            token_name: TokenClass::KEYWORD(kw.clone()),
            attribute_value: TokenAttribute::Null,
            line,
            column,
        }),
        TokenClassTag::Operation => Ok(Token {
            token_name: TokenClass::OPERATION,
            attribute_value: TokenAttribute::Itself(lexeme.to_string()),
            line,
            column,
        }),
        TokenClassTag::Delimiter => Ok(Token {
            token_name: TokenClass::DELIMITER,
            attribute_value: TokenAttribute::Itself(lexeme.to_string()),
            line,
            column,
        }),
        TokenClassTag::Unknown => {
            let suggestion = suggest_keyword(lexeme);
            let msg = match suggestion {
                Some(s) => format!("Unknown lexeme: '{lexeme}'. Did you mean: '{s}'?"),
                None => format!("Unknown lexeme: '{lexeme}'"),
            };
            Err(ErrorUnrecognizedLexeme { line, column, msg })
        }
    }
}

/// If `lexeme` is within edit distance ≤ 2 of a reserved keyword, return that
/// keyword as a suggestion. Used when reporting unrecognized identifiers.
fn suggest_keyword(lexeme: &str) -> Option<String> {
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
