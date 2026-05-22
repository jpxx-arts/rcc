use rcc::preprocessor::{self, PreprocessError};

mod remove_comments {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(preprocessor::remove_comments("").unwrap(), "");
    }

    #[test]
    fn only_code() {
        let src = "int x = 1;";
        assert_eq!(preprocessor::remove_comments(src).unwrap(), src);
    }

    #[test]
    fn only_block_comment() {
        assert_eq!(preprocessor::remove_comments("/* hello */").unwrap(), "");
    }

    #[test]
    fn only_line_comment() {
        assert_eq!(preprocessor::remove_comments("// hello").unwrap(), "");
    }

    #[test]
    fn empty_block_comment() {
        assert_eq!(preprocessor::remove_comments("/**/").unwrap(), "");
    }

    #[test]
    fn block_comment_preserves_newlines() {
        // multi-line block comments now leave behind their newline count so
        // downstream phases can keep accurate line numbers.
        assert_eq!(
            preprocessor::remove_comments("/* a\nb\nc */").unwrap(),
            "\n\n"
        );
    }

    #[test]
    fn line_comment_without_trailing_newline() {
        assert_eq!(preprocessor::remove_comments("// fim").unwrap(), "");
    }

    #[test]
    fn line_marker_inside_block_is_text() {
        assert_eq!(
            preprocessor::remove_comments("/* // ainda bloco */").unwrap(),
            ""
        );
    }

    #[test]
    fn block_marker_inside_line_is_text() {
        let src = "// /* texto na linha\nresto";
        assert_eq!(preprocessor::remove_comments(src).unwrap(), "\nresto");
    }

    #[test]
    fn multiple_block_comments_dont_merge() {
        let src = "a /* b */ c /* d */ e";
        assert_eq!(preprocessor::remove_comments(src).unwrap(), "a  c  e");
    }

    #[test]
    fn block_comment_between_code() {
        let src = "int /* tipo */ x;";
        assert_eq!(preprocessor::remove_comments(src).unwrap(), "int  x;");
    }

    #[test]
    fn line_comment_at_end_of_code_line() {
        let src = "int x; // anotação\nint y;";
        assert_eq!(
            preprocessor::remove_comments(src).unwrap(),
            "int x; \nint y;"
        );
    }

    #[test]
    fn unclosed_block_comment_reports_line() {
        let src = "codigo\nmais codigo\n/* sem fim";
        let err = preprocessor::remove_comments(src).unwrap_err();
        assert_eq!(err, PreprocessError::UnclosedBlockComment { line: 3 });
    }

    #[test]
    fn unclosed_block_comment_on_first_line() {
        let src = "/* sem fim";
        let err = preprocessor::remove_comments(src).unwrap_err();
        assert_eq!(err, PreprocessError::UnclosedBlockComment { line: 1 });
    }
}

mod remove_unnecessary_white_spaces {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(preprocessor::remove_unnecessary_white_spaces(""), "");
    }

    #[test]
    fn only_whitespace_keeps_newlines() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("   \t\n  "),
            "\n"
        );
    }

    #[test]
    fn single_token_unchanged() {
        assert_eq!(preprocessor::remove_unnecessary_white_spaces("abc"), "abc");
    }

    #[test]
    fn two_words_single_space_preserved() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("int x"),
            "int x"
        );
    }

    #[test]
    fn two_words_many_spaces_collapsed() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("int     x"),
            "int x"
        );
    }

    #[test]
    fn newline_between_words_preserved() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("int\nx"),
            "int\nx"
        );
    }

    #[test]
    fn tab_between_words_becomes_space() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("int\tx"),
            "int x"
        );
    }

    #[test]
    fn word_punct_no_space() {
        assert_eq!(preprocessor::remove_unnecessary_white_spaces("a;"), "a;");
    }

    #[test]
    fn word_ws_punct_drops_ws() {
        assert_eq!(preprocessor::remove_unnecessary_white_spaces("a ;"), "a;");
    }

    #[test]
    fn punct_word_no_space() {
        assert_eq!(preprocessor::remove_unnecessary_white_spaces(";a"), ";a");
    }

    #[test]
    fn punct_ws_word_drops_ws() {
        assert_eq!(preprocessor::remove_unnecessary_white_spaces("; a"), ";a");
    }

    #[test]
    fn punct_punct_no_space() {
        assert_eq!(preprocessor::remove_unnecessary_white_spaces("+;"), "+;");
    }

    #[test]
    fn punct_ws_punct_drops_ws() {
        assert_eq!(preprocessor::remove_unnecessary_white_spaces("+ ;"), "+;");
    }

    #[test]
    fn leading_whitespace_removed() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("   abc"),
            "abc"
        );
    }

    #[test]
    fn trailing_whitespace_removed() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("abc   "),
            "abc"
        );
    }

    #[test]
    fn assignment_with_spaces_minified() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("int x = 1;"),
            "int x=1;"
        );
    }

    #[test]
    fn numeric_literal_treated_as_word() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("return 42 ;"),
            "return 42;"
        );
    }

    #[test]
    fn underscore_is_word_char() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("_a   _b"),
            "_a _b"
        );
    }

    #[test]
    fn chain_three_short_words() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("a b c"),
            "a b c"
        );
    }

    #[test]
    fn chain_four_short_words() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("a b c d"),
            "a b c d"
        );
    }

    #[test]
    fn chain_xyz() {
        assert_eq!(
            preprocessor::remove_unnecessary_white_spaces("x y z"),
            "x y z"
        );
    }
}

mod end_to_end {
    use super::*;

    #[test]
    fn bubblesort_matches_expected() {
        let src =
            std::fs::read_to_string("specs/prog-bubblesort.ling").expect("fixture should exist");
        let expected = std::fs::read_to_string("specs/prog-bubblesort.expected")
            .expect("fixture should exist");
        let actual = preprocessor::preprocess(&src).expect("preprocess should succeed");
        assert_eq!(
            actual.trim_end_matches('\n'),
            expected.trim_end_matches('\n')
        );
    }

    #[test]
    fn factorial_matches_expected() {
        let src =
            std::fs::read_to_string("specs/prog-factorial.ling").expect("fixture should exist");
        let expected =
            std::fs::read_to_string("specs/prog-factorial.expected").expect("fixture should exist");
        let actual = preprocessor::preprocess(&src).expect("preprocess should succeed");
        assert_eq!(
            actual.trim_end_matches('\n'),
            expected.trim_end_matches('\n')
        );
    }
}
