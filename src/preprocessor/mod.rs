use regex::Regex;

pub fn remove_comments(src: &str) -> String {
    /*
     * the x flag is used to allow white spaces in regex to turn the expression more clear
     * the s flag is used to . matches to \n
     * the m flag is used to indicate $ as end of line
     */
    let re = Regex::new(r"(?x)/\*(?s:.*?)\*/ | //.*(?m:$)").expect("Incorrect regex pattern");
    let output: String = re.replace_all(src, "").into();

    output
}

pub fn remove_unnecessary_white_spaces(src: &str) -> String {
    // first we define the meaningfully white space pattern
    let re = Regex::new(r"(?<end_char>[[:word:]])[[:space:]]+(?<start_char>[[:word:]])")
        .expect("Incorrect regex pattern");

    // \x1A (26) is a control code from ascii table for [SUBSTITUTE]
    let substitute = '\x1A';
    let preserve_chars = format!("$end_char{substitute}$start_char");

    let mut output = String::from(src);
    loop {
        let next: String = re.replace_all(&output, &preserve_chars).into();

        if output == next {
            break;
        }

        output = next;
    }

    let re = Regex::new("[[:space:]]+").expect("Incorrect regex pattern");
    let output: String = re.replace_all(&output, "").into();

    let output = output.replace(substitute, " ");

    output
}

pub fn preprocess(src: &str) -> String {
    let output = remove_comments(src);
    let output = remove_unnecessary_white_spaces(&output);

    output
}
