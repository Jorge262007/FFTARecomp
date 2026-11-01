#[cfg_attr(windows, allow(dead_code))]
#[path = "../language_catalog.rs"]
mod catalog;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ffta-language-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, content: &str) {
        fs::write(self.0.join(name), content).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const ENGLISH: &str =
    r#"{"code":"en","name":"English","messages":{"Hello":"Hello","Saved {slot}":"Saved {slot}"}}"#;

#[test]
fn discovery_is_ordered_with_english_first_and_missing_keys_allowed() {
    let fixture = Fixture::new();
    fixture.write("en.json", ENGLISH);
    fixture.write(
        "aa.json",
        r#"{"code":"aa","name":"Àlpha","messages":{"Hello":"Ciao"}}"#,
    );
    let result = catalog::load(&fixture.0).unwrap();
    assert_eq!(
        result.iter().map(|c| c.code.as_str()).collect::<Vec<_>>(),
        ["en", "aa"]
    );
    assert_eq!(result[1].messages.len(), 1);
}

#[test]
fn invalid_json_metadata_duplicate_keys_and_placeholders_report_the_file() {
    for invalid in [
        r#"{"code":"fr","name":"French","messages":{"Hello":"Un","Hello":"Deux"}}"#,
        r#"{"code":"fr","name":"French","messages":{"Hello":4}}"#,
        r#"{"code":"fr","name":"French","messages":{"Unknown":"Inconnu"}}"#,
        r#"{"code":"fr","name":"French","messages":{"Saved {slot}":"Sauvé {other}"}}"#,
        r#"{"code":"fr","name":"French","messages":{"Hello":"Bad {"}}"#,
        r#"{"code":"fr","name":"French","messages":{"Hello":""}}"#,
        r#"{"code":"fr","name":"","messages":{}}"#,
        r#"{"code":"de","name":"French","messages":{}}"#,
        r#"{"code":"fr","name":"French","messages":{},"typo":true}"#,
        r#"{"code":"fr","name":"English","messages":{}}"#,
        "not JSON",
    ] {
        let fixture = Fixture::new();
        fixture.write("en.json", ENGLISH);
        fixture.write("fr.json", invalid);
        let error = catalog::load(&fixture.0).unwrap_err();
        assert!(error.contains("fr.json"), "{error}");
    }
}

#[test]
fn english_is_required_and_regional_codes_are_validated() {
    let fixture = Fixture::new();
    assert!(catalog::load(&fixture.0).unwrap_err().contains("en.json"));
    for code in ["en", "it", "fr", "pt-br", "zh-hant"] {
        assert!(catalog::valid_code(code));
    }
    for code in ["", "../fr", "FR", "english", "en-", "x", "en-!", "fr\n"] {
        assert!(!catalog::valid_code(code));
    }
}

// Use the real Language implementation with a generated third-language fixture.
// This proves addition/registration/fallback without adding a test locale to releases.
#[cfg(not(windows))]
#[test]
fn adding_one_json_compiles_into_a_selectable_locale_without_rust_edits() {
    let fixture = Fixture::new();
    fixture.write("en.json", ENGLISH);
    fixture.write(
        "zz.json",
        r#"{"code":"zz","name":"Zèta","messages":{"Hello":"Salve"}}"#,
    );
    fs::write(
        fixture.0.join("languages.rs"),
        catalog::generate(&catalog::load(&fixture.0).unwrap()),
    )
    .unwrap();
    let language_source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/language.rs");
    fixture.write(
        "main.rs",
        &format!(
            r#"
        #[path = {language_source:?}] mod language;
        use language::Language;
        fn main() {{
            assert_eq!(Language::ALL.len(), 2);
            assert_eq!(Language::parse("ZZ").token(), "zz");
            assert_eq!(Language::parse("zz").name(), "Zèta");
            assert_eq!(Language::parse("zz").text("Hello"), "Salve");
            assert_eq!(Language::parse("zz").text("Saved {{slot}}"), "Saved {{slot}}");
            assert_eq!(Language::parse("unknown").token(), "en");
        }}
    "#
        ),
    );
    let executable = fixture.0.join("registry-test");
    let compilation = std::process::Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "registry_test"])
        .arg(fixture.0.join("main.rs"))
        .arg("-o")
        .arg(&executable)
        .env("OUT_DIR", &fixture.0)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "{}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    assert!(std::process::Command::new(executable)
        .status()
        .unwrap()
        .success());
}
