use regex::Regex;
use std::sync::LazyLock;

const KEYWORDS: &str = "^(class|public|extends|else|int|static|void|main|String|return|boolean|if|while|System|out|println|length|new|true|false|this)$";
const OPERATIONS: &str = r"^(\&\&|>|\+|\-|\*|\!)$";
const DELIMITERS: &str = r"^(\{|\}|\(|\)|\[|\]|;|,|\.|=)$";

static ID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?x)^ [[:alpha:]] ([[:digit:]] | [[:alpha:]])* _? $")
        .expect("Lexical Analyzer - Id Regex Failed")
});
static NUMBER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("^[[:digit:]]+$").expect("Lexical Analyzer - Number Regex Failed"));
static KEYWORD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!("{KEYWORDS}")).expect("Lexical Analyzer - Keyword Regex Failed")
});
static OPERATION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!("{OPERATIONS}")).expect("Lexical Analyzer - Operations Regex Failed")
});
static DELIMITER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!("{DELIMITERS}")).expect("Lexical Analyzer - Delimiters Regex Failed")
});

#[derive(Debug, Clone)]
pub struct Token {
    pub token_name: TokenClass,
    pub attribute_value: TokenAttribute,
}

#[derive(Debug, Clone)]
pub enum TokenAttribute {
    POINTER { pointer: usize },
    ITSELF(String),
    NULL,
}

#[derive(Debug)]
pub struct SymbolTableEntry {
    pub lexeme: String,
}

#[derive(Debug)]
pub struct SymbolTable {
    pub registers: Vec<SymbolTableEntry>,
}

impl SymbolTable {
    fn new() -> Self {
        SymbolTable {
            registers: Vec::new(),
        }
    }

    fn push(&mut self, entry: SymbolTableEntry) -> usize {
        self.registers.push(entry);
        let loc = self.registers.len() - 1;

        loc
    }
}

#[derive(Debug)]
pub struct ErrorUnrecognizedLexeme {
    line: usize,
    msg: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenClass {
    ID,
    NUMBER,
    KEYWORD(String),
    OPERATION,
    DELIMITER,
    UNKNOWN,
}

pub fn get_tokens(src: &str) -> (Vec<Token>, SymbolTable) {
    let mut tokens = Vec::new();
    let mut symbol_table = SymbolTable::new();

    let mut line_count = 0;

    let mut previous_location = 0;
    while previous_location < src.len() {
        match get_token(
            &src[previous_location..src.len()],
            &mut symbol_table,
            &mut line_count,
        ) {
            Ok((token, lexeme_len)) => {
                tokens.push(token);
                previous_location += lexeme_len;
            }
            Err(err) => {
                eprintln!("Lexical error at line {}: {}", err.line, err.msg);
                return (tokens, symbol_table);
            }
        };
    }

    (tokens, symbol_table)
}

pub fn get_token(
    src: &str,
    symbol_table: &mut SymbolTable,
    line_count: &mut usize,
) -> Result<(Token, usize), ErrorUnrecognizedLexeme> {
    let mut lexeme = String::new();
    let mut consumed: usize = 0;

    let mut token_class = TokenClass::UNKNOWN;
    for c in src.chars() {
        consumed += c.len_utf8();

        match c {
            '\n' => {
                *line_count += 1;
                if lexeme.is_empty() {
                    continue;
                }
                return match build_token(&lexeme, &token_class, symbol_table, *line_count) {
                    Ok(token) => Ok((token, consumed)),
                    Err(err) => Err(err),
                };
            }
            ' ' => {
                if lexeme.is_empty() {
                    continue;
                }
                return match build_token(&lexeme, &token_class, symbol_table, *line_count) {
                    Ok(token) => Ok((token, consumed)),
                    Err(err) => Err(err),
                };
            }
            _ => {}
        }

        lexeme.push(c);

        if NUMBER_REGEX.is_match(&lexeme) {
            token_class = TokenClass::NUMBER;
        } else if KEYWORD_REGEX.is_match(&lexeme) {
            token_class = TokenClass::KEYWORD(lexeme.to_string());
        } else if ID_REGEX.is_match(&lexeme) {
            token_class = TokenClass::ID;
        } else if DELIMITER_REGEX.is_match(&lexeme) {
            token_class = TokenClass::DELIMITER;
        } else if OPERATION_REGEX.is_match(&lexeme) {
            token_class = TokenClass::OPERATION;
        } else if lexeme == "&" {
            continue;
        } else {
            lexeme.pop();
            consumed -= c.len_utf8();
            return match build_token(&lexeme, &token_class, symbol_table, *line_count) {
                Ok(token) => Ok((token, consumed)),
                Err(err) => Err(err),
            };
        }
    }

    return match build_token(&lexeme, &token_class, symbol_table, *line_count) {
        Ok(token) => Ok((token, consumed)),
        Err(err) => Err(err),
    };
}

fn build_token(
    lexeme: &str,
    token_class: &TokenClass,
    symbol_table: &mut SymbolTable,
    line_count: usize,
) -> Result<Token, ErrorUnrecognizedLexeme> {
    let lexeme_name: String = lexeme.to_string();
    match token_class {
        TokenClass::ID => {
            let mut token = Token {
                token_name: TokenClass::ID,
                attribute_value: TokenAttribute::NULL,
            };
            token.attribute_value = TokenAttribute::POINTER {
                pointer: symbol_table.push(SymbolTableEntry {
                    lexeme: lexeme_name,
                }),
            };
            return Ok(token);
        }
        TokenClass::NUMBER => {
            let mut token = Token {
                token_name: TokenClass::NUMBER,
                attribute_value: TokenAttribute::NULL,
            };

            token.attribute_value = TokenAttribute::POINTER {
                pointer: symbol_table.push(SymbolTableEntry {
                    lexeme: lexeme_name,
                }),
            };

            return Ok(token);
        }
        TokenClass::KEYWORD(lexeme_name) => {
            let token = Token {
                token_name: TokenClass::KEYWORD(lexeme_name.clone()),
                attribute_value: TokenAttribute::NULL,
            };

            symbol_table.push(SymbolTableEntry {
                lexeme: lexeme_name.clone(),
            });

            return Ok(token);
        }
        TokenClass::OPERATION => {
            let token = Token {
                token_name: TokenClass::OPERATION,
                attribute_value: TokenAttribute::ITSELF(lexeme_name.clone()),
            };

            symbol_table.push(SymbolTableEntry {
                lexeme: lexeme_name,
            });

            return Ok(token);
        }
        TokenClass::DELIMITER => {
            let token = Token {
                token_name: TokenClass::DELIMITER,
                attribute_value: TokenAttribute::ITSELF(lexeme_name.clone()),
            };

            symbol_table.push(SymbolTableEntry {
                lexeme: lexeme_name,
            });

            return Ok(token);
        }
        TokenClass::UNKNOWN => {
            return Err(ErrorUnrecognizedLexeme {
                line: line_count,
                msg: format!("Unknown lexeme: {lexeme}"),
            });
        }
    }
}
