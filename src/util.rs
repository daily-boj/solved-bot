use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

pub fn escape_markdown(s: &str) -> Cow<str> {
    static REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"[_\*\[\]\(\)~`>#+-=\|\{\}\.!]"#).unwrap());
    REGEX.replace_all(s, "\\$0")
}
