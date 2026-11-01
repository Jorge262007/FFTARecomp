//! Build-time discovery and validation. JSON is never parsed during gameplay.
use serde::{de, Deserialize, Deserializer};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    path::Path,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub code: String,
    pub name: String,
    #[serde(deserialize_with = "unique_messages")]
    pub messages: BTreeMap<String, String>,
}

fn unique_messages<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    struct Messages;
    impl<'de> de::Visitor<'de> for Messages {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("an object of unique message keys and string translations")
        }
        fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut messages = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if messages.insert(key.clone(), value).is_some() {
                    return Err(de::Error::custom(format!("duplicate message key {key:?}")));
                }
            }
            Ok(messages)
        }
    }
    deserializer.deserialize_map(Messages)
}

pub fn valid_code(code: &str) -> bool {
    let mut parts = code.split('-');
    let first = parts.next().unwrap_or("");
    (2..=3).contains(&first.len())
        && first.bytes().all(|c| c.is_ascii_lowercase())
        && parts.all(|part| {
            (2..=8).contains(&part.len())
                && part
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

pub fn placeholders(text: &str) -> Result<BTreeSet<&str>, String> {
    let mut result = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find(['{', '}']) {
        if rest.as_bytes()[start] == b'}' {
            return Err("unmatched closing brace".into());
        }
        rest = &rest[start + 1..];
        let end = rest.find('}').ok_or("unmatched opening brace")?;
        let name = &rest[..end];
        if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            return Err(format!("invalid placeholder {{{name}}}"));
        }
        result.insert(name);
        rest = &rest[end + 1..];
    }
    Ok(result)
}

pub fn load(directory: &Path) -> Result<Vec<Catalog>, String> {
    let mut catalogs = Vec::new();
    let mut names = BTreeSet::new();
    let mut paths: Vec<_> = fs::read_dir(directory)
        .map_err(|e| e.to_string())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    paths.sort();
    for path in paths
        .iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
    {
        let validate = || -> Result<Catalog, String> {
            let bytes = fs::read(path).map_err(|e| e.to_string())?;
            let catalog: Catalog = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if !valid_code(&catalog.code) {
                return Err("code must be lowercase, e.g. fr or pt-br".into());
            }
            if path.file_stem().and_then(|s| s.to_str()) != Some(catalog.code.as_str()) {
                return Err("filename must match code".into());
            }
            if catalog.name.trim().is_empty() || catalog.name.chars().any(char::is_control) {
                return Err(
                    "name must be a nonempty display name without control characters".into(),
                );
            }
            for (key, value) in &catalog.messages {
                if key.trim().is_empty() || value.trim().is_empty() {
                    return Err(format!("empty message key or translation: {key:?}"));
                }
                placeholders(value).map_err(|error| format!("message {key:?}: {error}"))?;
            }
            Ok(catalog)
        };
        let catalog = validate().map_err(|error| format!("{}: {error}", path.display()))?;
        if !names.insert(catalog.name.clone()) {
            return Err(format!(
                "{}: duplicate display name {:?}",
                path.display(),
                catalog.name
            ));
        }
        catalogs.push(catalog);
    }
    catalogs.sort_by_key(|catalog| (catalog.code != "en", catalog.code.clone()));
    let english = catalogs
        .first()
        .filter(|catalog| catalog.code == "en")
        .ok_or("en.json is required")?;
    if english.messages.is_empty() {
        return Err("en.json must contain the base messages".into());
    }
    for catalog in &catalogs {
        for (key, value) in &catalog.messages {
            let base = english
                .messages
                .get(key)
                .ok_or_else(|| format!("{}.json: unknown message key {key:?}", catalog.code))?;
            if placeholders(base)? != placeholders(value)? {
                return Err(format!(
                    "{}.json: placeholders differ from English for {key:?}",
                    catalog.code
                ));
            }
        }
    }
    Ok(catalogs)
}

pub fn generate(catalogs: &[Catalog]) -> String {
    let mut output = format!(
        "impl Language {{ pub const ALL: [Self; {}] = [",
        catalogs.len()
    );
    for index in 0..catalogs.len() {
        output.push_str(&format!("Self({index}),"));
    }
    output.push_str("]; }\nstatic CATALOGS: &[Catalog] = &[\n");
    for catalog in catalogs {
        output.push_str(&format!(
            "Catalog {{ code: {:?}, name: {:?}, messages: &[\n",
            catalog.code, catalog.name
        ));
        for (key, value) in &catalog.messages {
            output.push_str(&format!("({key:?}, {value:?}),\n"));
        }
        output.push_str("] },\n");
    }
    output.push_str("];\n");
    output
}
