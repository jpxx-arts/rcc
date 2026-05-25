#[derive(Debug, PartialEq)]
pub enum PreprocessError {
    UnclosedBlockComment { line: usize },
}

pub struct SourceMap {
    pub mapping: Vec<(usize, usize)>,
}

impl SourceMap {
    pub fn new() -> Self {
        SourceMap {
            mapping: Vec::new(),
        }
    }

    pub fn get(&self, offset: usize) -> (usize, usize) {
        if offset < self.mapping.len() {
            self.mapping[offset]
        } else if let Some(&last) = self.mapping.last() {
            last
        } else {
            (1, 1)
        }
    }
}

pub fn preprocess(src: &str) -> Result<(String, SourceMap), PreprocessError> {
    let (output, map) = remove_comments(src)?;
    let (output, map) = remove_unnecessary_white_spaces(&output, map);
    Ok((output, map))
}

pub fn remove_comments(src: &str) -> Result<(String, SourceMap), PreprocessError> {
    let mut output = String::new();
    let mut mapping = Vec::new();
    let mut line = 1;
    let mut column = 1;
    let mut char_indices = src.char_indices().peekable();

    while let Some((_, c)) = char_indices.next() {
        if c == '/' && char_indices.peek().map(|p| p.1) == Some('*') {
            char_indices.next(); // consume '*'
            let start_line = line;
            let mut closed = false;
            let mut current_column = column + 2;
            while let Some((_, inner_c)) = char_indices.next() {
                if inner_c == '\n' {
                    output.push('\n');
                    mapping.push((line, current_column));
                    line += 1;
                    current_column = 1;
                } else if inner_c == '*' && char_indices.peek().map(|p| p.1) == Some('/') {
                    char_indices.next(); // consume '/'
                    current_column += 2;
                    closed = true;
                    break;
                } else {
                    current_column += 1;
                }
            }
            if !closed {
                return Err(PreprocessError::UnclosedBlockComment { line: start_line });
            }
            column = current_column;
        } else if c == '/' && char_indices.peek().map(|p| p.1) == Some('/') {
            char_indices.next(); // consume '/'
            let mut current_column = column + 2;
            while let Some((_, inner_c)) = char_indices.next() {
                if inner_c == '\n' {
                    output.push('\n');
                    mapping.push((line, current_column));
                    line += 1;
                    current_column = 1;
                    break;
                }
                current_column += 1;
            }
            column = current_column;
        } else {
            output.push(c);
            let pos = (line, column);
            for _ in 0..c.len_utf8() {
                mapping.push(pos);
            }
            if c == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
    }
    Ok((output, SourceMap { mapping }))
}

pub fn remove_unnecessary_white_spaces(src: &str, incoming_map: SourceMap) -> (String, SourceMap) {
    let mut output = String::new();
    let mut mapping = Vec::new();

    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '_';

    let mut i = 0;
    while i < chars.len() {
        let (byte_offset, c) = chars[i];

        if c.is_whitespace() && c != '\n' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].1.is_whitespace() && chars[j].1 != '\n' {
                j += 1;
            }

            let prev_word = i > 0 && is_word(chars[i - 1].1);
            let next_word = j < chars.len() && is_word(chars[j].1);

            if prev_word && next_word {
                output.push(' ');
                mapping.push(incoming_map.get(byte_offset));
            }
            i = j;
        } else {
            output.push(c);
            let pos = incoming_map.get(byte_offset);
            for _ in 0..c.len_utf8() {
                mapping.push(pos);
            }
            i += 1;
        }
    }

    (output, SourceMap { mapping })
}
