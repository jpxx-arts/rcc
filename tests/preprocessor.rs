use rcc::preprocessor;

mod remove_comments {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(preprocessor::remove_comments(""), "");
    }

    #[test]
    fn only_code() {
        let src = "int x = 1;";
        assert_eq!(preprocessor::remove_comments(src), src);
    }

    #[test]
    fn only_block_comment() {
        assert_eq!(preprocessor::remove_comments("/* hello */"), "");
    }

    #[test]
    fn only_line_comment() {
        assert_eq!(preprocessor::remove_comments("// hello"), "");
    }

    #[test]
    fn empty_block_comment() {
        assert_eq!(preprocessor::remove_comments("/**/"), "");
    }

    #[test]
    fn block_comment_with_newlines() {
        assert_eq!(preprocessor::remove_comments("/* a\nb\nc */"), "");
    }

    #[test]
    fn line_comment_without_trailing_newline() {
        assert_eq!(preprocessor::remove_comments("// fim"), "");
    }

    #[test]
    fn line_marker_inside_block_is_text() {
        assert_eq!(preprocessor::remove_comments("/* // ainda bloco */"), "");
    }

    #[test]
    fn block_marker_inside_line_is_text() {
        let src = "// /* texto na linha\nresto";
        assert_eq!(preprocessor::remove_comments(src), "\nresto");
    }

    #[test]
    fn multiple_block_comments_dont_merge() {
        let src = "a /* b */ c /* d */ e";
        assert_eq!(preprocessor::remove_comments(src), "a  c  e");
    }

    #[test]
    fn block_comment_between_code() {
        let src = "int /* tipo */ x;";
        assert_eq!(preprocessor::remove_comments(src), "int  x;");
    }

    #[test]
    fn line_comment_at_end_of_code_line() {
        let src = "int x; // anotação\nint y;";
        assert_eq!(preprocessor::remove_comments(src), "int x; \nint y;");
    }

    // Caracterização do comportamento atual: o regex não casa um /* sem */
    // correspondente, então o input volta inalterado. Quando você implementar
    // a detecção do erro, este teste deve mudar para esperar
    // Err(PreprocessError::UnclosedBlockComment).
    #[test]
    fn unclosed_block_comment_currently_unchanged() {
        let src = "codigo /* sem fim";
        assert_eq!(preprocessor::remove_comments(src), src);
    }
}

mod end_to_end {
    use super::*;

    #[test]
    fn bubblesort_matches_expected() {
        let src = std::fs::read_to_string("specs/prog-bubblesort.ling")
            .expect("fixture should exist");
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
        let src = std::fs::read_to_string("specs/prog-factorial.ling")
            .expect("fixture should exist");
        let expected = std::fs::read_to_string("specs/prog-factorial.expected")
            .expect("fixture should exist");
        let actual = preprocessor::preprocess(&src).expect("preprocess should succeed");
        assert_eq!(
            actual.trim_end_matches('\n'),
            expected.trim_end_matches('\n')
        );
    }
}
