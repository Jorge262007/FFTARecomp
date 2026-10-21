//! Two automatic startup copies of the selected cartridge save, kept as raw .sav files.
use gba_core::Machine;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
const MAX_BYTES: usize = 128 * 1024;

pub struct Backups {
    source: PathBuf,
    directory: PathBuf,
}
struct Entry {
    generation: u64,
    digest: String,
    path: PathBuf,
}
pub struct Choice {
    pub generation: Option<u64>,
    pub status: String,
}

impl Backups {
    pub fn new(save: &Path) -> Self {
        let parent = save
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        Self {
            source: save.to_owned(),
            directory: parent
                .join("SaveBackups")
                .join(save.file_name().unwrap_or_default()),
        }
    }

    fn entries(&self) -> Result<Vec<Entry>, String> {
        let items = match std::fs::read_dir(&self.directory) {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.to_string()),
        };
        let mut entries = Vec::new();
        for item in items {
            let item = item.map_err(|e| e.to_string())?;
            if !item.file_type().map_err(|e| e.to_string())?.is_file() {
                continue;
            }
            let name = item.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(stem) = name
                .strip_prefix("backup-")
                .and_then(|n| n.strip_suffix(".sav"))
            else {
                continue;
            };
            let Some((number, digest)) = stem.split_once('-') else {
                continue;
            };
            if number.len() != 20
                || digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                continue;
            }
            let Ok(generation) = number.parse::<u64>() else {
                continue;
            };
            entries.push(Entry {
                generation,
                digest: digest.into(),
                path: item.path(),
            });
        }
        entries.sort_by(|a, b| b.generation.cmp(&a.generation).then(b.path.cmp(&a.path)));
        Ok(entries)
    }

    pub fn has_copies(&self) -> bool {
        self.entries().is_ok_and(|entries| !entries.is_empty())
    }

    /// Capture exactly the bytes that will be loaded at startup, before gameplay or autosave.
    pub fn capture_startup(&self) -> Result<Option<Vec<u8>>, String> {
        let bytes = match read(&self.source) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("Cannot read cartridge save for backup: {e}")),
        };
        let generation = self
            .entries()?
            .first()
            .map_or(Some(1), |e| e.generation.checked_add(1))
            .ok_or("Backup sequence exhausted")?;
        let digest = hash(&bytes);
        std::fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
        let path = self
            .directory
            .join(format!("backup-{generation:020}-{digest}.sav"));
        atomic_write(&path, &bytes, false)?;
        // Publish the complete new copy first. Failed writes never delete old backups.
        for old in self.entries()?.into_iter().skip(2) {
            std::fs::remove_file(&old.path)
                .map_err(|e| format!("Cannot retire old backup: {e}"))?;
        }
        Ok(Some(bytes))
    }

    fn verified(&self, entry: &Entry, expected: usize) -> Result<Vec<u8>, String> {
        let bytes = read(&entry.path).map_err(|e| e.to_string())?;
        if bytes.len() != expected || hash(&bytes) != entry.digest {
            return Err("Backup is damaged or has an incompatible size".into());
        }
        Ok(bytes)
    }

    pub fn choices(&self, expected: usize) -> [Choice; 2] {
        match self.entries() {
            Err(error) => std::array::from_fn(|_| Choice {
                generation: None,
                status: format!("Unavailable: {error}"),
            }),
            Ok(entries) => std::array::from_fn(|i| match entries.get(i) {
                None => Choice {
                    generation: None,
                    status: "Empty".into(),
                },
                Some(e) => match self.verified(e, expected) {
                    Ok(_) => Choice {
                        generation: Some(e.generation),
                        status: "Ready".into(),
                    },
                    Err(_) => Choice {
                        generation: None,
                        status: "Damaged / incompatible".into(),
                    },
                },
            }),
        }
    }

    /// Prepare a clean boot before touching the current file or machine. Restore does not rotate copies.
    pub fn restore(
        &self,
        generation: u64,
        machine: &mut Machine,
        boot: &[u8],
    ) -> Result<(), String> {
        let expected = machine
            .bus
            .save_data()
            .ok_or("This cartridge has no save memory")?
            .len();
        let entries = self.entries()?;
        let matches: Vec<_> = entries
            .iter()
            .take(2)
            .filter(|e| e.generation == generation)
            .collect();
        if matches.len() != 1 {
            return Err("The selected backup is no longer available".into());
        }
        let bytes = self.verified(matches[0], expected)?;
        let bios = machine.bus.real_bios.then_some(machine.bus.bios.as_slice());
        let mut fresh = super::make_machine(machine.bus.rom.clone(), bios);
        fresh.bus.set_rom_resources(machine.bus.rom_resources().cloned())?;
        fresh.load_state(boot)?;
        fresh.bus.load_save_data(&bytes);
        atomic_write(&self.source, &bytes, true)?;
        *machine = fresh;
        Ok(())
    }
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn read(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Save exceeds 128 KiB",
        ));
    }
    Ok(bytes)
}
fn atomic_write(path: &Path, bytes: &[u8], replace: bool) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    if replace {
        file.persist(path)
    } else {
        file.persist_noclobber(path)
    }
    .map_err(|e| e.error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn machine() -> Machine {
        let mut rom = vec![0; 512];
        rom[256..265].copy_from_slice(b"FLASH_V12");
        super::super::make_machine(rom, None)
    }

    #[test]
    fn startup_keeps_two_exact_raw_copies_and_never_changes_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("progress.sav");
        let backups = Backups::new(&path);
        assert!(backups.capture_startup().unwrap().is_none());
        assert!(!backups.directory.exists());
        for value in [1, 2, 3] {
            let bytes = vec![value; 65536];
            std::fs::write(&path, &bytes).unwrap();
            assert_eq!(backups.capture_startup().unwrap().unwrap(), bytes);
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
        let entries = backups.entries().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(read(&entries[0].path).unwrap(), vec![3; 65536]);
        assert_eq!(read(&entries[1].path).unwrap(), vec![2; 65536]);
        assert_eq!(std::fs::read_dir(&backups.directory).unwrap().count(), 2);
        let other = Backups::new(&dir.path().join("other.sav"));
        assert_ne!(backups.directory, other.directory);
    }

    #[test]
    fn restore_reboots_machine_and_survives_autosave_without_rotating_copies() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sav");
        let backups = Backups::new(&path);
        let mut m = machine();
        let boot = m.save_state().unwrap();
        for value in [1, 2] {
            std::fs::write(&path, vec![value; 65536]).unwrap();
            backups.capture_startup().unwrap();
        }
        let choices = backups.choices(65536);
        let older = choices[1].generation.unwrap();
        m.cpu.regs[4] = 999;
        m.bus.ewram[10] = 99;
        m.bus.tick(12345);
        m.bus.load_save_data(&vec![9; 65536]);
        backups.restore(older, &mut m, &boot).unwrap();
        assert_eq!(m.cpu.regs[4], 0);
        assert_eq!(m.bus.ewram[10], 0);
        assert_eq!(m.bus.clock, 0);
        assert_eq!(m.bus.save_data().unwrap(), vec![1; 65536]);
        assert_eq!(std::fs::read(&path).unwrap(), vec![1; 65536]);
        super::super::write_sav(path.to_str().unwrap(), m.bus.save_data().unwrap()).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), vec![1; 65536]);
        assert_eq!(backups.choices(65536)[1].generation, Some(older));
        assert_eq!(backups.entries().unwrap().len(), 2);
    }

    #[test]
    fn bad_or_missing_backup_and_boot_leave_file_and_machine_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sav");
        let backups = Backups::new(&path);
        let mut m = machine();
        let boot = m.save_state().unwrap();
        let original = m.save_state().unwrap();
        std::fs::write(&path, vec![7; 65536]).unwrap();
        backups.capture_startup().unwrap();
        let entry = backups.entries().unwrap().remove(0);
        let valid = read(&entry.path).unwrap();
        std::fs::write(&entry.path, vec![8; 65536]).unwrap();
        assert!(backups.choices(65536)[0].generation.is_none());
        assert!(backups.restore(entry.generation, &mut m, &boot).is_err());
        assert_eq!(m.save_state().unwrap(), original);
        assert_eq!(std::fs::read(&path).unwrap(), valid);
        std::fs::write(&entry.path, &valid).unwrap();
        assert!(backups
            .restore(entry.generation, &mut m, b"bad boot")
            .is_err());
        assert_eq!(m.save_state().unwrap(), original);
        assert_eq!(std::fs::read(&path).unwrap(), valid);
        std::fs::remove_file(&entry.path).unwrap();
        assert!(backups.restore(entry.generation, &mut m, &boot).is_err());
        assert_eq!(m.save_state().unwrap(), original);
    }

    #[test]
    fn restore_handles_missing_primary_and_failed_publish_without_mutating_machine() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sav");
        let backups = Backups::new(&path);
        let mut m = machine();
        let boot = m.save_state().unwrap();
        std::fs::write(&path, vec![7; 65536]).unwrap();
        backups.capture_startup().unwrap();
        let id = backups.choices(65536)[0].generation.unwrap();
        m.cpu.regs[4] = 99;
        let current = m.save_state().unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap(); // Existing directory blocks atomic publication.
        assert!(backups.restore(id, &mut m, &boot).is_err());
        assert!(path.is_dir());
        assert_eq!(m.save_state().unwrap(), current);
        assert_eq!(backups.entries().unwrap().len(), 1);
        std::fs::remove_dir(&path).unwrap(); // Primary save is now missing.
        backups.restore(id, &mut m, &boot).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), vec![7; 65536]);
        assert_eq!(m.cpu.regs[4], 0);
        assert_eq!(m.bus.save_data().unwrap(), vec![7; 65536]);
    }

    #[test]
    fn failed_capture_does_not_prune_existing_copies_or_unrelated_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sav");
        let backups = Backups::new(&path);
        for value in [1, 2] {
            std::fs::write(&path, vec![value; 65536]).unwrap();
            backups.capture_startup().unwrap();
        }
        let names: Vec<_> = backups
            .entries()
            .unwrap()
            .into_iter()
            .map(|e| e.path)
            .collect();
        let unrelated = backups.directory.join("personal.sav");
        std::fs::write(&unrelated, b"keep").unwrap();
        std::fs::write(&path, vec![0; MAX_BYTES + 1]).unwrap();
        assert!(backups.capture_startup().is_err());
        assert!(names.iter().all(|p| p.exists()));
        assert_eq!(std::fs::read(unrelated).unwrap(), b"keep");
    }
}
