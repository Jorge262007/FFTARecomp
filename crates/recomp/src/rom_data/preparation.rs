//! Managed resource storage. Immutable generations and a small atomic commit
//! file keep interrupted preparation separate from the last published dataset.
use super::*;
use std::{fs::TryLockError, time::Duration};

const ACTIVE: &str = "current";
const GENERATION_PREFIX: &str = "generation-";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Checking,
    Waiting,
    Repairing,
    Extracting,
    Validating,
    Publishing,
    Ready,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Checking => "Checking game data",
            Self::Waiting => "Waiting for game data preparation",
            Self::Repairing => "Repairing game data",
            Self::Extracting => "Preparing game data",
            Self::Validating => "Verifying game data",
            Self::Publishing => "Finishing game data preparation",
            Self::Ready => "Game data ready",
        }
    }

    pub fn percent(self) -> u8 {
        match self {
            Self::Checking | Self::Waiting | Self::Repairing => 0,
            Self::Extracting => 20,
            Self::Validating => 70,
            Self::Publishing => 90,
            Self::Ready => 100,
        }
    }
}

pub struct Prepared {
    pub directory: PathBuf,
    pub dimensions: ObjDimensions,
    pub action: &'static str,
}

pub fn console_progress(status: bool) -> impl FnMut(Stage) -> Result<(), String> {
    let mut previous = None;
    move |stage| {
        if previous != Some(stage) {
            eprintln!("resources: {}", stage.label());
            if status {
                println!("STATUS building {} {}", stage.percent(), stage.label());
            }
            previous = Some(stage);
        }
        Ok(())
    }
}

fn resolve_root(
    explicit: Option<PathBuf>,
    portable: Option<PathBuf>,
    installed: Option<PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(root) = explicit {
        return Ok(root);
    }
    if let Some(root) = portable {
        return Ok(root.join("Data").join("resources"));
    }
    installed
        .map(|root| root.join("FFTARecomp").join("resources"))
        .ok_or_else(|| "Cannot locate application storage for game data".into())
}

pub fn storage_root() -> Result<PathBuf, String> {
    resolve_root(
        std::env::var_os("FFTA_RESOURCES_DIR").map(PathBuf::from),
        input_config::storage::portable_root(),
        dirs::data_local_dir(),
    )
}

fn revision_directory(root: &Path, source: &Extraction) -> Result<PathBuf, String> {
    let hash = &source.manifest.source_rom_sha256;
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Invalid source identity for resource storage".into());
    }
    Ok(root
        .join(hash)
        .join(format!("v{}", source.manifest.schema))
        .join(format!(
            "e{}-d{}",
            source.manifest.extractor_revision, source.manifest.decoder_revision
        )))
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir()) {
                Ok(())
            } else {
                Err(format!(
                    "Game data location is not a regular directory: {}",
                    path.display()
                ))
            }
        }
        Err(e) => Err(format!(
            "Cannot create game data directory {}: {e}",
            path.display()
        )),
    }
}

fn active_dataset(revision: &Path, source: &Extraction) -> Result<Prepared, String> {
    let bytes = read_bounded(&revision.join(ACTIVE), 80)?;
    if bytes.len() > 80 {
        return Err("Game data generation name is too long".into());
    }
    let name = std::str::from_utf8(&bytes)
        .map_err(|_| "Invalid game data generation name")?
        .trim_end_matches('\n');
    if !name.starts_with(GENERATION_PREFIX)
        || name.len() <= GENERATION_PREFIX.len()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid game data generation name".into());
    }
    let directory = revision.join(name);
    let dimensions = verify(&directory, source)?;
    Ok(Prepared {
        directory,
        dimensions,
        action: "reused",
    })
}

fn sync_directory(path: &Path) -> Result<(), String> {
    // Windows has no portable std API for flushing directory metadata. File
    // contents are synced on both targets, with atomic replacement below.
    #[cfg(unix)]
    fs::File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|e| format!("Cannot sync game data directory {}: {e}", path.display()))?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub fn prepare(
    root: &Path,
    source: &mut Extraction,
    notify: &mut impl FnMut(Stage) -> Result<(), String>,
) -> Result<Prepared, String> {
    prepare_with_writer(root, source, notify, &mut write_synced)
}

fn prepare_with_writer(
    root: &Path,
    source: &mut Extraction,
    notify: &mut impl FnMut(Stage) -> Result<(), String>,
    write: &mut impl FnMut(&Path, &[u8]) -> Result<(), String>,
) -> Result<Prepared, String> {
    notify(Stage::Checking)?;
    let revision = revision_directory(root, source)?;
    // Every managed path component must be an actual directory. An explicit
    // resource root may have normal system symlinks among its ancestors.
    fs::create_dir_all(root)
        .map_err(|e| format!("Cannot create game data root {}: {e}", root.display()))?;
    if !fs::symlink_metadata(root).is_ok_and(|m| m.file_type().is_dir()) {
        return Err("Game data root must be a regular directory".into());
    }
    let schema = revision.parent().unwrap();
    let image = schema.parent().unwrap();
    ensure_directory(image)?;
    ensure_directory(schema)?;
    ensure_directory(&revision)?;
    if let Ok(ready) = active_dataset(&revision, source) {
        notify(Stage::Ready)?;
        return Ok(ready);
    }

    let lock_path = revision.join("prepare.lock");
    match fs::symlink_metadata(&lock_path) {
        Ok(m) if !m.file_type().is_file() => {
            return Err("Game data preparation lock is not a regular file".into())
        }
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("Cannot inspect game data lock: {e}"))
        }
        _ => {}
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| format!("Cannot open game data preparation lock: {e}"))?;
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) => {
                notify(Stage::Waiting)?;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(TryLockError::Error(e)) => {
                return Err(format!("Cannot lock game data preparation: {e}"))
            }
        }
    }
    // The lock is released by the OS on normal return, cancellation or process
    // death. Never remove the lock file: replacing its inode breaks exclusion.
    let invalid_reason = match active_dataset(&revision, source) {
        Ok(ready) => {
            notify(Stage::Ready)?;
            return Ok(ready);
        }
        Err(error) => error,
    };
    let repairing = fs::symlink_metadata(revision.join(ACTIVE)).is_ok();
    if repairing {
        notify(Stage::Repairing)?;
        eprintln!("resources: regenerating invalid game data ({invalid_reason})");
    }
    notify(Stage::Extracting)?;
    let generation = tempfile::Builder::new()
        .prefix(GENERATION_PREFIX)
        .tempdir_in(&revision)
        .map_err(|e| format!("Cannot stage game data: {e}"))?;
    write_dataset(generation.path(), source, write)?;
    notify(Stage::Validating)?;
    let dimensions = verify(generation.path(), source)?;
    sync_directory(generation.path())?;
    notify(Stage::Publishing)?;
    let name = generation
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .ok_or("Invalid generated directory name")?
        .to_owned();
    // A killed process can leave an unreferenced generation. Only the validated
    // current file selects a dataset; partial/orphan generations are ignored.
    let directory = generation.keep();
    let mut pointer = tempfile::NamedTempFile::new_in(&revision)
        .map_err(|e| format!("Cannot stage game data selection: {e}"))?;
    pointer
        .write_all(format!("{name}\n").as_bytes())
        .and_then(|_| pointer.as_file().sync_all())
        .map_err(|e| format!("Cannot finish game data selection: {e}"))?;
    sync_directory(&revision)?;
    pointer
        .persist(revision.join(ACTIVE))
        .map_err(|e| format!("Cannot publish game data selection: {}", e.error))?;
    sync_directory(&revision)?;
    notify(Stage::Ready)?;
    Ok(Prepared {
        directory,
        dimensions,
        action: if repairing { "repaired" } else { "created" },
    })
}

/// The caller opts in until the direct-consumer/equivalence gates are complete.
/// This runs after source verification and before any cartridge save writes.
pub fn prepare_startup(rom: &[u8], status: bool) -> Result<Prepared, String> {
    let mut source = Extraction::from_rom(rom)?;
    let root = storage_root()?;
    let prepared = prepare(&root, &mut source, &mut console_progress(status))?;
    eprintln!(
        "resources: {} {} from {} ({} dimensions, {} background descriptors, {} base palettes verified)",
        prepared.action,
        source.manifest.coverage.join(","),
        prepared.directory.display(),
        prepared.dimensions.entries.len(),
        source.background.as_ref().map_or(0, |bg| bg.entries.len()),
        source.palettes.as_ref().map_or(0, |catalog| catalog.decoded.entries.len())
    );
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Extraction {
        let mut source = super::super::tests::fixture();
        source.manifest.source_rom_sha256 = "0".repeat(64);
        source
    }

    fn generation_count(dir: &Path) -> usize {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .is_some_and(|s| s.starts_with(GENERATION_PREFIX))
            })
            .count()
    }

    #[test]
    fn first_preparation_reuses_verified_files_and_repairs_corruption() {
        let temp = tempfile::tempdir().unwrap();
        let mut source = fixture();
        let first = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(first.action, "created");
        assert_eq!(first.dimensions, source.dimensions);
        let revision = revision_directory(temp.path(), &source).unwrap();
        let pointer = fs::read(revision.join(ACTIVE)).unwrap();
        let data = fs::read(first.directory.join(OBJ_DIMENSIONS_FILE)).unwrap();
        let mut stages = Vec::new();
        let reused = prepare_with_writer(
            temp.path(),
            &mut source,
            &mut |stage| {
                stages.push(stage);
                Ok(())
            },
            &mut |_, _| panic!("reuse must not extract"),
        )
        .unwrap();
        assert_eq!(stages, [Stage::Checking, Stage::Ready]);
        assert_eq!(reused.directory, first.directory);
        assert_eq!(reused.action, "reused");
        assert_eq!(generation_count(&revision), 1);
        assert_eq!(pointer, fs::read(revision.join(ACTIVE)).unwrap());
        fs::write(first.directory.join(OBJ_DIMENSIONS_FILE), b"corrupt").unwrap();
        let repaired = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(repaired.action, "repaired");
        assert_ne!(repaired.directory, first.directory);
        assert_eq!(
            fs::read(repaired.directory.join(OBJ_DIMENSIONS_FILE)).unwrap(),
            data
        );
        assert_eq!(
            fs::read(first.directory.join(OBJ_DIMENSIONS_FILE)).unwrap(),
            b"corrupt"
        );
        fs::remove_file(repaired.directory.join(MANIFEST_FILE)).unwrap();
        let repaired_missing = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(repaired_missing.action, "repaired");
        verify(&repaired_missing.directory, &source).unwrap();
    }

    #[test]
    fn background_repair_and_partial_write_keep_committed_generation() {
        let temp = tempfile::tempdir().unwrap();
        let original = fixture();
        let mut source = fixture();
        let first = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        let revision = revision_directory(temp.path(), &source).unwrap();
        let pointer = fs::read(revision.join(ACTIVE)).unwrap();
        let bg = fs::read(first.directory.join(BG_DESCRIPTORS_FILE)).unwrap();
        let mut updated = fixture();
        updated.manifest.resources[1].format = "synthetic-updated-bg".into();
        let error = prepare_with_writer(
            temp.path(),
            &mut updated,
            &mut |_| Ok(()),
            &mut |path, bytes| {
                if path.file_name().unwrap() == BG_DESCRIPTORS_FILE {
                    fs::write(path, b"partial-background").unwrap();
                    return Err("simulated background storage failure".into());
                }
                write_synced(path, bytes)
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("background storage failure"));
        assert_eq!(fs::read(revision.join(ACTIVE)).unwrap(), pointer);
        assert_eq!(
            fs::read(first.directory.join(BG_DESCRIPTORS_FILE)).unwrap(),
            bg
        );
        verify(&first.directory, &original).unwrap();
        fs::remove_file(first.directory.join(BG_DESCRIPTORS_FILE)).unwrap();
        let repaired = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(repaired.action, "repaired");
        assert_ne!(repaired.directory, first.directory);
        assert_eq!(
            fs::read(repaired.directory.join(BG_DESCRIPTORS_FILE)).unwrap(),
            bg
        );
        assert!(!first.directory.join(BG_DESCRIPTORS_FILE).exists());
    }

    #[test]
    fn palette_partial_write_and_missing_repair_keep_prior_generation() {
        let temp = tempfile::tempdir().unwrap();
        let mut source = fixture();
        let first = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        let revision = revision_directory(temp.path(), &source).unwrap();
        let pointer = fs::read(revision.join(ACTIVE)).unwrap();
        let palette = fs::read(first.directory.join(BG_BASE_PALETTES_FILE)).unwrap();
        let mut updated = fixture();
        updated.manifest.resources[2].format = "synthetic-updated-palette".into();
        let error = prepare_with_writer(
            temp.path(),
            &mut updated,
            &mut |_| Ok(()),
            &mut |path, bytes| {
                if path.file_name().unwrap() == BG_BASE_PALETTES_FILE {
                    fs::write(path, b"partial-palette").unwrap();
                    return Err("simulated palette storage failure".into());
                }
                write_synced(path, bytes)
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("palette storage failure"));
        assert_eq!(fs::read(revision.join(ACTIVE)).unwrap(), pointer);
        assert_eq!(
            fs::read(first.directory.join(BG_BASE_PALETTES_FILE)).unwrap(),
            palette
        );
        verify(&first.directory, &source).unwrap();
        fs::remove_file(first.directory.join(BG_BASE_PALETTES_FILE)).unwrap();
        let repaired = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(repaired.action, "repaired");
        assert_ne!(repaired.directory, first.directory);
        assert!(!first.directory.join(BG_BASE_PALETTES_FILE).exists());
        assert_eq!(
            fs::read(repaired.directory.join(BG_BASE_PALETTES_FILE)).unwrap(),
            palette
        );
        verify(&repaired.directory, &source).unwrap();
    }

    #[test]
    fn cancellation_and_write_failure_preserve_published_generation() {
        let temp = tempfile::tempdir().unwrap();
        let mut source = fixture();
        let first = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        let revision = revision_directory(temp.path(), &source).unwrap();
        let pointer = fs::read(revision.join(ACTIVE)).unwrap();
        // Simulate a decoder expecting an updated contract at the same path.
        // Its old generation must survive every failure before publication.
        source.manifest.resources[0].format = "synthetic-updated-format".into();
        for stop in [
            Stage::Checking,
            Stage::Extracting,
            Stage::Validating,
            Stage::Publishing,
        ] {
            let result = prepare(temp.path(), &mut source, &mut |stage| {
                if stage == stop {
                    Err("Preparation cancelled".into())
                } else {
                    Ok(())
                }
            });
            assert!(result.err().unwrap().contains("cancelled"));
            assert_eq!(fs::read(revision.join(ACTIVE)).unwrap(), pointer);
            assert_eq!(generation_count(&revision), 1);
            assert!(first.directory.is_dir());
        }
        let mut writes = 0;
        let error = prepare_with_writer(
            temp.path(),
            &mut source,
            &mut |_| Ok(()),
            &mut |path, bytes| {
                writes += 1;
                if writes == 2 {
                    Err("Simulated storage full while writing manifest".into())
                } else {
                    write_synced(path, bytes)
                }
            },
        )
        .err()
        .unwrap();
        assert!(error.contains("storage full"));
        assert_eq!(fs::read(revision.join(ACTIVE)).unwrap(), pointer);
        assert_eq!(generation_count(&revision), 1);
        let original = fixture();
        verify(&first.directory, &original).unwrap();
    }

    #[test]
    fn region_and_decoder_versions_have_separate_generations() {
        let temp = tempfile::tempdir().unwrap();
        let mut source = fixture();
        let old = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        source.manifest.decoder_revision += 1;
        let revised = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(revised.action, "created");
        assert_ne!(old.directory, revised.directory);
        source.manifest.source_rom_sha256 = "1".repeat(64);
        let other = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        assert_ne!(other.directory, revised.directory);
        assert!(old.directory.exists());
        assert!(revised.directory.exists());
        source.manifest.source_rom_sha256 = "../escape".into();
        assert!(prepare(temp.path(), &mut source, &mut |_| Ok(())).is_err());
    }

    #[test]
    fn incomplete_generations_and_bad_current_paths_are_not_reused() {
        let temp = tempfile::tempdir().unwrap();
        let mut source = fixture();
        let first = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        let revision = revision_directory(temp.path(), &source).unwrap();
        let partial = revision.join("generation-interrupted");
        fs::create_dir(&partial).unwrap();
        fs::write(partial.join(OBJ_DIMENSIONS_FILE), b"partial").unwrap();
        fs::write(revision.join(ACTIVE), b"generation-interrupted\n").unwrap();
        let repaired = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        assert_ne!(repaired.directory, partial);
        assert_eq!(repaired.action, "repaired");
        verify(&first.directory, &source).unwrap();
        for name in [
            "../outside",
            "/absolute",
            "generation-/escape",
            "generation-..",
            "generation-x\ntrailer",
            "",
        ] {
            fs::write(revision.join(ACTIVE), name).unwrap();
            assert!(active_dataset(&revision, &source).is_err());
        }
        let repaired = prepare(temp.path(), &mut source, &mut |_| Ok(())).unwrap();
        verify(&repaired.directory, &source).unwrap();
    }

    #[test]
    fn simultaneous_preparation_publishes_once_and_lock_wait_is_cancellable() {
        let temp = tempfile::tempdir().unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let root = temp.path().to_owned();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    prepare(&root, &mut fixture(), &mut |_| Ok(())).unwrap()
                })
            })
            .collect();
        let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| r.action == "created").count(), 1);
        assert!(results.iter().all(|r| r.directory == results[0].directory));
        let mut source = fixture();
        let revision = revision_directory(temp.path(), &source).unwrap();
        assert_eq!(generation_count(&revision), 1);
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(revision.join("prepare.lock"))
            .unwrap();
        lock.lock().unwrap();
        fs::remove_file(revision.join(ACTIVE)).unwrap();
        let error = prepare(temp.path(), &mut source, &mut |stage| {
            if stage == Stage::Waiting {
                Err("Cancelled while waiting".into())
            } else {
                Ok(())
            }
        })
        .err()
        .unwrap();
        assert!(error.contains("Cancelled"));
        drop(lock);
        assert_eq!(
            prepare(temp.path(), &mut source, &mut |_| Ok(()))
                .unwrap()
                .action,
            "created"
        );
    }

    #[test]
    fn portable_relocation_installed_override_and_private_data_separation() {
        let temp = tempfile::tempdir().unwrap();
        let portable = temp.path().join("portable folder");
        let installed = temp.path().join("installed");
        fs::create_dir(&portable).unwrap();
        let root = resolve_root(None, Some(portable.clone()), Some(installed.clone())).unwrap();
        assert_eq!(root, portable.join("Data/resources"));
        let mut source = fixture();
        let prepared = prepare(&root, &mut source, &mut |_| Ok(())).unwrap();
        let relative = prepared
            .directory
            .strip_prefix(&portable)
            .unwrap()
            .to_owned();
        for name in ["mods/custom.txt", "saves/ffta.sav", "cache/native"] {
            let path = portable.join("Data").join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"user data").unwrap();
        }
        let moved = temp.path().join("moved folder");
        fs::rename(&portable, &moved).unwrap();
        let moved_root = resolve_root(None, Some(moved.clone()), Some(installed.clone())).unwrap();
        let ready = prepare_with_writer(&moved_root, &mut source, &mut |_| Ok(()), &mut |_, _| {
            panic!("move must reuse")
        })
        .unwrap();
        assert_eq!(ready.directory, moved.join(relative));
        fs::remove_file(ready.directory.join(OBJ_DIMENSIONS_FILE)).unwrap();
        prepare(&moved_root, &mut source, &mut |_| Ok(())).unwrap();
        for name in ["mods/custom.txt", "saves/ffta.sav", "cache/native"] {
            assert_eq!(
                fs::read(moved.join("Data").join(name)).unwrap(),
                b"user data"
            );
        }
        assert!(!installed.exists());
        assert_eq!(
            resolve_root(None, None, Some(installed.clone())).unwrap(),
            installed.join("FFTARecomp/resources")
        );
        let explicit = temp.path().join("explicit");
        assert_eq!(
            resolve_root(Some(explicit.clone()), Some(moved), None).unwrap(),
            explicit
        );
        assert!(resolve_root(None, None, None).is_err());
    }

    #[test]
    fn filesystem_errors_are_loud_and_do_not_touch_other_roots() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("blocked");
        fs::write(&root, b"keep").unwrap();
        assert!(prepare(&root, &mut fixture(), &mut |_| Ok(())).is_err());
        assert_eq!(fs::read(&root).unwrap(), b"keep");
        let mut source = fixture();
        let other = temp.path().join("other");
        let ready = prepare(&other, &mut source, &mut |_| Ok(())).unwrap();
        let revision = revision_directory(&other, &source).unwrap();
        fs::remove_file(revision.join(ACTIVE)).unwrap();
        fs::create_dir(revision.join(ACTIVE)).unwrap();
        assert!(prepare(&other, &mut source, &mut |_| Ok(())).is_err());
        verify(&ready.directory, &source).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn managed_symlinks_cannot_redirect_preparation() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(&outside, &alias).unwrap();
        assert!(prepare(&alias, &mut fixture(), &mut |_| Ok(())).is_err());
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("0".repeat(64))).unwrap();
        assert!(prepare(&root, &mut fixture(), &mut |_| Ok(())).is_err());
        assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
    }

    #[test]
    fn killed_preparation_processes_preserve_commit_and_release_lock() {
        use std::{
            process::{Command, Stdio},
            time::Instant,
        };
        for point in ["partial-file", "before-commit", "after-commit"] {
            let temp = tempfile::tempdir().unwrap();
            let original = fixture();
            let first = prepare(temp.path(), &mut fixture(), &mut |_| Ok(())).unwrap();
            let revision = revision_directory(temp.path(), &original).unwrap();
            let pointer = fs::read(revision.join(ACTIVE)).unwrap();
            let signal = temp.path().join("child-ready");
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "rom_data::preparation::tests::interruption_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("FFTA_PREPARATION_CHILD_ROOT", temp.path())
                .env("FFTA_PREPARATION_CHILD_POINT", point)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(15);
            let reached = loop {
                if signal.is_file() {
                    break true;
                }
                if child.try_wait().unwrap().is_some() || Instant::now() > deadline {
                    break false;
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            let _ = child.kill();
            child.wait().unwrap();
            assert!(reached, "child failed to reach {point}");
            verify(&first.directory, &original).unwrap();
            let mut updated = fixture();
            updated.manifest.resources[0].format = "synthetic-updated-format".into();
            let ready = if point == "after-commit" {
                assert_ne!(pointer, fs::read(revision.join(ACTIVE)).unwrap());
                let ready =
                    prepare_with_writer(temp.path(), &mut updated, &mut |_| Ok(()), &mut |_, _| {
                        panic!("committed generation must be reused")
                    })
                    .unwrap();
                assert_eq!(ready.action, "reused");
                ready
            } else {
                assert_eq!(pointer, fs::read(revision.join(ACTIVE)).unwrap());
                // This must reacquire the dead process's lock and ignore any
                // partly written or complete-but-unpublished generation.
                let ready = prepare(temp.path(), &mut updated, &mut |_| Ok(())).unwrap();
                assert_eq!(ready.action, "repaired");
                ready
            };
            verify(&ready.directory, &updated).unwrap();
        }
    }

    #[test]
    #[ignore = "subprocess helper, invoked only by killed_preparation_processes_preserve_commit_and_release_lock"]
    fn interruption_child() {
        let root = PathBuf::from(
            std::env::var_os("FFTA_PREPARATION_CHILD_ROOT").expect("subprocess root"),
        );
        let point = std::env::var("FFTA_PREPARATION_CHILD_POINT").expect("subprocess point");
        let pause = || {
            fs::write(root.join("child-ready"), b"ready").unwrap();
            loop {
                std::thread::park();
            }
        };
        let mut source = fixture();
        source.manifest.resources[0].format = "synthetic-updated-format".into();
        let mut first_write = true;
        prepare_with_writer(
            &root,
            &mut source,
            &mut |stage| {
                if (point == "before-commit" && stage == Stage::Publishing)
                    || (point == "after-commit" && stage == Stage::Ready)
                {
                    pause();
                }
                Ok(())
            },
            &mut |path, bytes| {
                write_synced(path, bytes)?;
                if first_write && point == "partial-file" {
                    pause();
                }
                first_write = false;
                Ok(())
            },
        )
        .unwrap();
        panic!("child should have been killed at a preparation boundary");
    }
}
