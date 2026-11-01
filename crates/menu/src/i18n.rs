//! Menu-owned copy only. Configuration tokens, bindings, command syntax, paths,
//! imported content and diagnostic payloads keep their original values.
use input_config::Language;

pub(super) struct Locale(pub Language);
impl Locale {
    pub fn tr<'a>(&self, key: &'a str) -> &'a str {
        text(self.0, key)
    }
}

/// Resolve a stable source key. Unknown keys fall back to their source text.
pub fn text(language: Language, key: &str) -> &str {
    language.text(key)
}

/// Substitute named placeholders in one pass, so values containing braces or
/// other placeholder names remain verbatim and translations may reorder them.
pub fn format(language: Language, key: &str, args: &[(&str, String)]) -> String {
    let mut rest = text(language, key);
    let mut result = String::new();
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find('}') else { break };
        let name = &rest[1..end];
        if let Some((_, value)) = args.iter().find(|(key, _)| *key == name) {
            result.push_str(value);
        } else {
            result.push_str(&rest[..=end]);
        }
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    result
}

impl super::Menu {
    pub fn language(&self) -> Language {
        self.language.get()
    }

    pub(super) fn tr<'a>(&self, key: &'a str) -> &'a str {
        text(self.language(), key)
    }

    pub(super) fn tr_format(&self, key: &str, args: &[(&str, String)]) -> String {
        format(self.language(), key, args)
    }
}

#[cfg(test)]
mod tests;
