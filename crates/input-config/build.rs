mod language_catalog;

fn main() {
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let directory = root.join("languages");
    // Watching the directory also discovers newly added or removed languages.
    println!("cargo:rerun-if-changed={}", directory.display());
    println!("cargo:rerun-if-changed=language_catalog.rs");
    let catalogs = language_catalog::load(&directory)
        .unwrap_or_else(|error| panic!("Invalid menu language: {error}"));
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(
        output.join("languages.rs"),
        language_catalog::generate(&catalogs),
    )
    .unwrap();
}
