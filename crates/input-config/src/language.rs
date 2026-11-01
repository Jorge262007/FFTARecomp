//! Languages discovered from languages/*.json at build time.
//! Only stable codes are persisted; generated indices remain an implementation detail.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Language(usize);

impl std::fmt::Debug for Language {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("Language")
            .field(&self.token())
            .finish()
    }
}

struct Catalog {
    code: &'static str,
    name: &'static str,
    messages: &'static [(&'static str, &'static str)],
}

include!(concat!(env!("OUT_DIR"), "/languages.rs"));

impl Language {
    pub fn parse(token: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|language| language.token().eq_ignore_ascii_case(token.trim()))
            .unwrap_or_default()
    }

    pub fn token(self) -> &'static str {
        CATALOGS[self.0].code
    }

    /// Names come directly from the JSON file, in the language's own spelling.
    pub fn name(self) -> &'static str {
        CATALOGS[self.0].name
    }

    pub fn messages(self) -> &'static [(&'static str, &'static str)] {
        CATALOGS[self.0].messages
    }

    fn translation(self, key: &str) -> Option<&'static str> {
        let messages = self.messages();
        messages
            .binary_search_by_key(&key, |(key, _)| *key)
            .ok()
            .map(|index| messages[index].1)
    }

    /// Missing translations use English; unknown source keys remain literal.
    pub fn text(self, key: &str) -> &str {
        self.translation(key)
            .or_else(|| Self::default().translation(key))
            .unwrap_or(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvConfig;

    #[test]
    fn legacy_unknown_and_missing_language_default_to_english() {
        for config in [
            "",
            "audio.volume = 0.4",
            "menu.language = ??",
            "menu.language = ",
        ] {
            assert_eq!(AvConfig::parse(config).menu_language, Language::default());
        }
        assert_eq!(
            AvConfig::parse("menu.language = IT # user choice").menu_language,
            Language::parse("it")
        );
    }

    #[test]
    fn both_languages_survive_config_roundtrip_with_other_preferences() {
        for language in Language::ALL {
            let config = AvConfig {
                menu_language: language,
                audio_volume: 0.4,
                video_fullscreen: true,
                ..AvConfig::default()
            };
            let serialized = config.serialize();
            assert!(serialized.contains(&format!("menu.language = {}\n", language.token())));
            let loaded = AvConfig::parse(&serialized);
            assert_eq!(loaded.menu_language, language);
            assert_eq!(loaded.audio_volume, 0.4);
            assert!(loaded.video_fullscreen);
        }
    }
}
