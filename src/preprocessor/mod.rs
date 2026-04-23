use regex::Regex;

#[derive(Debug)]
pub enum PreprocessError {
    UnclosedBlockComment { line: usize },
}

pub fn remove_comments(src: &str) -> String {
    /*
     * the x flag is used to allow white spaces in regex to turn the expression more clear
     * the s flag is used to . matches to \n
     * the m flag is used to indicate $ as end of line
     */
    let re = Regex::new(r"(?x)/\*(?s:.*?)\*/ | //.*(?m:$)").expect("Incorrect regex pattern");
    let output = re.replace_all(src, "");

    output.into()
}

pub fn preprocess(src: &str) -> Result<String, PreprocessError> {
    let output: String;

    output = remove_comments(src);

    Ok(output)
}
