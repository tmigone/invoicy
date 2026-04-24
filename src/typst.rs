/// Escape special characters for Typst string literals
pub fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Convert an Option<String> to a Typst value (quoted string or "none")
pub fn option_string(opt: &Option<String>) -> String {
    opt.as_ref()
        .map(|s| format!("\"{}\"", escape_string(s)))
        .unwrap_or_else(|| "none".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_string_plain() {
        assert_eq!(escape_string("hello"), "hello");
    }

    #[test]
    fn escape_string_quotes() {
        assert_eq!(escape_string(r#"say "hi""#), r#"say \"hi\""#);
    }

    #[test]
    fn escape_string_backslash() {
        assert_eq!(escape_string(r"path\to\file"), r"path\\to\\file");
    }

    #[test]
    fn option_string_none() {
        assert_eq!(option_string(&None), "none");
    }

    #[test]
    fn option_string_some() {
        assert_eq!(option_string(&Some("value".into())), r#""value""#);
    }

    #[test]
    fn option_string_some_with_quotes() {
        assert_eq!(option_string(&Some(r#"say "hi""#.into())), r#""say \"hi\"""#);
    }
}
