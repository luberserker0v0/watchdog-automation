use regex_lite::Regex;

pub(super) struct OcrTextMatcher {
    pattern: Regex,
    whitespace_tolerant_pattern: Option<Regex>,
}

impl OcrTextMatcher {
    pub(super) fn new(pattern: &str) -> Result<Self, regex_lite::Error> {
        let whitespace_tolerant = make_literal_whitespace_optional(pattern);
        Ok(Self {
            pattern: Regex::new(pattern)?,
            whitespace_tolerant_pattern: (whitespace_tolerant != pattern)
                .then(|| Regex::new(&whitespace_tolerant))
                .transpose()?,
        })
    }

    pub(super) fn is_match(&self, text: &str) -> bool {
        self.pattern.is_match(text)
            || self
                .whitespace_tolerant_pattern
                .as_ref()
                .is_some_and(|pattern| pattern.is_match(text))
    }
}

/// OCR models may omit a visible word gap from the decoded text. Make only
/// literal whitespace in a user pattern optional; explicit regex constructs
/// such as `\s+` and spaces inside character classes retain their semantics.
pub(super) fn make_literal_whitespace_optional(pattern: &str) -> String {
    let mut result = String::with_capacity(pattern.len());
    let mut chars = pattern.chars().peekable();
    let mut in_character_class = false;

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            result.push(ch);
            if let Some(escaped) = chars.next() {
                result.push(escaped);
            }
            continue;
        }

        match ch {
            '[' => {
                in_character_class = true;
                result.push(ch);
            }
            ']' if in_character_class => {
                in_character_class = false;
                result.push(ch);
            }
            _ if ch.is_whitespace() && !in_character_class => {
                while chars.peek().is_some_and(|next| next.is_whitespace()) {
                    chars.next();
                }
                result.push_str(r"\s*");
            }
            _ => result.push(ch),
        }
    }

    result
}
