use regex::{Captures, Regex};
use std::sync::LazyLock;

const SUBSTITUTE: char = '\x1A';
const PRESERVE_CHARS_TEMPLATE: &str = "$end_char\x1A$start_char";

static COMMENT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?x)/\*(?s:.*?)\*/ | //.*(?m:$)")
        .expect("Preprocessor - Comment Regex Failed")
});

static MEANINGFUL_WS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?<end_char>[[:word:]])[[:space:]--\n]+(?<start_char>[[:word:]])")
        .expect("Preprocessor - Meaningful Whitespace Regex Failed")
});

static NON_NEWLINE_WS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[[:space:]--\n]+")
        .expect("Preprocessor - Non-newline Whitespace Regex Failed")
});

#[derive(Debug, PartialEq)]
pub enum PreprocessError {
    UnclosedBlockComment { line: usize },
}

pub fn remove_comments(src: &str) -> Result<String, PreprocessError> {
    // Replace each comment with its newline-equivalent. Block comments keep
    // their internal `\n` count so downstream line tracking matches the
    // original source. Line comments don't span newlines, so they get empty.
    let output: String = COMMENT_REGEX
        .replace_all(src, |caps: &Captures| {
            let matched = caps.get(0).unwrap().as_str();
            if matched.starts_with("/*") {
                "\n".repeat(matched.matches('\n').count())
            } else {
                String::new()
            }
        })
        .into();

    // After removing all closed comments, any remaining `/*` is unclosed.
    if let Some(pos) = output.find("/*") {
        let line = output[..pos].matches('\n').count() + 1;
        return Err(PreprocessError::UnclosedBlockComment { line });
    }

    Ok(output)
}

pub fn remove_unnecessary_white_spaces(src: &str) -> String {
    let mut output = String::from(src);
    loop {
        let next: String = MEANINGFUL_WS_REGEX
            .replace_all(&output, PRESERVE_CHARS_TEMPLATE)
            .into();
        if output == next {
            break;
        }
        output = next;
    }

    let output: String = NON_NEWLINE_WS_REGEX.replace_all(&output, "").into();
    output.replace(SUBSTITUTE, " ")
}

pub fn preprocess(src: &str) -> Result<String, PreprocessError> {
    let output = remove_comments(src)?;
    Ok(remove_unnecessary_white_spaces(&output))
}
