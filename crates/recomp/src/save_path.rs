//! Resolve legacy saves without renaming, deleting or copying the user's files.
use std::path::{Path, PathBuf};

fn preference() -> Option<PathBuf> {
    dirs::data_local_dir().map(|p| p.join("FFTARecomp").join("shared-save-path.txt"))
}

pub fn remember(save: &Path) {
    let result = (|| -> std::io::Result<()> {
        let Some(file) = preference() else { return Ok(()); };
        std::fs::create_dir_all(file.parent().unwrap())?;
        std::fs::write(file, std::path::absolute(save)?.to_string_lossy().as_bytes())
    })();
    if let Err(e) = result { eprintln!("save: cannot remember shared save: {e}"); }
}

fn candidates(rom: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = rom.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let mut saves = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if path.extension().is_some_and(|s|s.eq_ignore_ascii_case("sav")){saves.push(path);}
            else if path.file_name().and_then(|s|s.to_str()).is_some_and(|s|s.to_ascii_lowercase().ends_with(".sav.legacy-backup")){
                saves.push(path.with_file_name(path.file_name().unwrap().to_string_lossy().trim_end_matches(".legacy-backup")));
            }
        }
    }
    saves.sort();
    saves.dedup();
    Ok(saves)
}

pub fn resolve(rom: &Path) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("FFTA_SAVE_PATH") {
        return Ok(PathBuf::from(path));
    }
    if let Some(saved) = preference().and_then(|p| std::fs::read_to_string(p).ok()) {
        let saved = PathBuf::from(saved);
        if saved.is_file() || PathBuf::from(format!("{}.legacy-backup",saved.display())).is_file() { return Ok(saved); }
    }
    let expected = rom.with_extension("sav");
    if expected.is_file() || PathBuf::from(format!("{}.legacy-backup",expected.display())).is_file() { return Ok(expected); }
    let saves = candidates(rom)?;
    match saves.as_slice() {
        [] => Ok(expected),
        [only] => {
            eprintln!("save: using {} (independent of ROM name)", only.display());
            Ok(only.clone())
        }
        _ => {
            // Other games may share this directory. Never guess which file to overwrite.
            let selected = rfd::FileDialog::new()
                .set_title("Choose your FFTA save - Cancel to start a new save")
                .add_filter("Game Boy Advance save", &["sav"])
                .set_directory(rom.parent().unwrap_or(Path::new(".")))
                .pick_file();
            Ok(selected.unwrap_or(expected))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn archived_original_still_resolves_to_same_logical_save_path(){
        let dir=std::env::temp_dir().join(format!("ffta-archive-path-{}",std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let rom=dir.join("Europe.gba");let old=dir.join("my-progress.sav");
        std::fs::write(dir.join("my-progress.sav.legacy-backup"),b"preserved").unwrap();
        assert_eq!(super::candidates(&rom).unwrap(),vec![old]);
        std::fs::remove_file(dir.join("my-progress.sav.legacy-backup")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn arbitrary_names_and_multiple_saves_are_preserved() {
        let dir = std::env::temp_dir().join(format!("ffta-save-resolve-{}",std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let rom = dir.join("Europe.gba");
        assert!(super::candidates(&rom).unwrap().is_empty());
        let save = dir.join("my progress.SAV");
        std::fs::write(&save,b"test progress").unwrap();
        assert_eq!(super::candidates(&rom).unwrap(), vec![save.clone()]);
        let other = dir.join("another.sav");
        std::fs::write(&other,b"other progress").unwrap();
        assert_eq!(super::candidates(&rom).unwrap().len(),2);
        assert_eq!(std::fs::read(&save).unwrap(),b"test progress");
        assert_eq!(std::fs::read(&other).unwrap(),b"other progress");
        std::fs::remove_file(save).unwrap();
        std::fs::remove_file(other).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
