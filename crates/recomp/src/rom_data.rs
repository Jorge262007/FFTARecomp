//! Source-authenticated extraction, preparation and verified dataset loading.
use gba_core::rom_data::{
    bg_descriptors::Descriptors, bg_palettes::Palettes, BaseFiles, Extraction, Manifest,
    ObjDimensions, BG_BASE_PALETTES_FILE, BG_BASE_PALETTES_ID, BG_DESCRIPTORS_FILE,
    BG_DESCRIPTORS_ID, MANIFEST_FILE, MAX_BG_BASE_PALETTES_FILE_BYTES,
    MAX_BG_DESCRIPTORS_FILE_BYTES, MAX_DATASET_BYTES, MAX_FILE_BYTES, MAX_MANIFEST_FILE_BYTES,
    OBJ_DIMENSIONS_FILE,
};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(test)]
mod alternate_palette_tests;
#[cfg(test)]
mod bg_descriptor_tests;
#[cfg(test)]
mod bg_packet_tests;
#[cfg(test)]
mod bg_palette_tests;
#[cfg(test)]
mod bg_runtime_tests;
#[cfg(test)]
mod compression_tests;
#[cfg(test)]
mod managed_tests;
#[cfg(test)]
mod native_tests;
#[cfg(test)]
mod palette_data_tests;
#[cfg(test)]
mod palette_runtime_tests;
mod preparation;
#[cfg(test)]
mod validation_tests;
pub use preparation::prepare_startup;

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("Cannot inspect {}: {e}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.len() > limit {
        return Err(format!(
            "Resource must be a regular file of at most {limit} bytes: {}",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|f| f.take(limit + 1).read_to_end(&mut bytes))
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if bytes.len() as u64 > limit {
        return Err("Resource grew beyond the file size limit".into());
    }
    Ok(bytes)
}

fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T, String> {
    let value = std::str::from_utf8(bytes).map_err(|_| format!("{label} is not UTF-8"))?;
    toml::from_str(value).map_err(|e| format!("Invalid {label}: {e}"))
}

fn parse_background(bytes: &[u8]) -> Result<Descriptors, String> {
    // Deserialize directly into bounded typed records. A generic TOML/JSON
    // document tree needlessly multiplies temporary memory for this family.
    let result: Descriptors = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid background descriptors: {e}"))?;
    if result.entries.is_empty()
        || result.entries.len() > gba_core::rom_data::bg_descriptors::MAX_RECORDS
    {
        return Err("Invalid background descriptor count".into());
    }
    Ok(result)
}

fn encode_background(background: &Descriptors) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(background)
        .map_err(|e| format!("Cannot encode background descriptors: {e}"))
}

fn parse_palettes(bytes: &[u8]) -> Result<Palettes, String> {
    let palettes: Palettes =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid base palettes: {e}"))?;
    palettes.validate()?;
    Ok(palettes)
}

fn encode_palettes(palettes: &Palettes) -> Result<Vec<u8>, String> {
    palettes.validate()?;
    serde_json::to_vec_pretty(palettes).map_err(|e| format!("Cannot encode base palettes: {e}"))
}

pub fn verify(path: &Path, source: &Extraction) -> Result<ObjDimensions, String> {
    let dataset = read_dataset(path)?;
    source.validate_files(dataset.files())?;
    Ok(dataset.dimensions)
}

struct Dataset {
    manifest: Manifest,
    dimensions: ObjDimensions,
    obj_bytes: Vec<u8>,
    background: Descriptors,
    bg_bytes: Vec<u8>,
    palettes: Palettes,
    palette_bytes: Vec<u8>,
}

impl Dataset {
    fn files(&self) -> BaseFiles<'_> {
        BaseFiles {
            manifest: &self.manifest,
            dimensions: (&self.dimensions, &self.obj_bytes),
            background: Some((&self.background, &self.bg_bytes)),
            palettes: Some((&self.palettes, &self.palette_bytes)),
        }
    }
}

fn read_dataset(path: &Path) -> Result<Dataset, String> {
    if !fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_dir()) {
        return Err(format!(
            "Dataset must be a directory, not a symlink: {}",
            path.display()
        ));
    }
    let manifest_bytes = read_bounded(&path.join(MANIFEST_FILE), MAX_MANIFEST_FILE_BYTES)?;
    let manifest: Manifest = parse(&manifest_bytes, "resource manifest")?;
    // Never follow paths from a manifest. The supported resource set and names
    // are fixed, and each file has its own bound before parsing.
    let obj_bytes = read_bounded(&path.join(OBJ_DIMENSIONS_FILE), MAX_FILE_BYTES)?;
    let bg_bytes = read_bounded(
        &path.join(BG_DESCRIPTORS_FILE),
        MAX_BG_DESCRIPTORS_FILE_BYTES,
    )?;
    let palette_bytes = read_bounded(
        &path.join(BG_BASE_PALETTES_FILE),
        MAX_BG_BASE_PALETTES_FILE_BYTES,
    )?;
    check_dataset_budget(
        manifest_bytes.len(),
        obj_bytes.len(),
        bg_bytes.len(),
        palette_bytes.len(),
    )?;
    let dimensions = parse(&obj_bytes, "OBJ dimensions")?;
    let background = parse_background(&bg_bytes)?;
    let palettes = parse_palettes(&palette_bytes)?;
    Ok(Dataset {
        manifest,
        dimensions,
        obj_bytes,
        background,
        bg_bytes,
        palettes,
        palette_bytes,
    })
}

fn check_dataset_budget(
    manifest: usize,
    obj: usize,
    bg: usize,
    palettes: usize,
) -> Result<(), String> {
    if manifest as u64 > MAX_MANIFEST_FILE_BYTES
        || obj as u64 > MAX_FILE_BYTES
        || bg as u64 > MAX_BG_DESCRIPTORS_FILE_BYTES
        || palettes as u64 > MAX_BG_BASE_PALETTES_FILE_BYTES
    {
        return Err("Resource file exceeds its size budget".into());
    }
    if manifest
        .saturating_add(obj)
        .saturating_add(bg)
        .saturating_add(palettes)
        > MAX_DATASET_BYTES
    {
        return Err("Resource dataset exceeds its total size budget".into());
    }
    Ok(())
}

pub fn attach_from_directory(machine: &mut gba_core::Machine, path: &Path) -> Result<(), String> {
    let dataset = read_dataset(path)?;
    let resources =
        gba_core::rom_data::RuntimeResources::from_base_files(&machine.bus.rom, dataset.files())?;
    let obj_resident = resources.obj_resident_bytes() + 2 * std::mem::size_of::<usize>();
    let bg_resident = resources.background_resident_bytes();
    let palette_resident = resources.palette_resident_bytes();
    let resident = obj_resident + bg_resident + palette_resident;
    if obj_resident > 1024 {
        return Err("Decoded OBJ metadata exceed the session memory budget".into());
    }
    if bg_resident > 64 * 1024 {
        return Err("Decoded BG metadata exceed the session memory budget".into());
    }
    machine.bus.set_rom_resources(Some(resources.clone()))?;
    eprintln!("ROM_DATA runtime=decoded resource={} source={} dataset={} file={} resident_bytes={obj_resident} total_resident_bytes={resident}",
        resources.resource_id(), resources.source_sha256(), resources.dataset_sha256(), resources.file_sha256());
    eprintln!(
        "ROM_DATA runtime=decoded resource={BG_DESCRIPTORS_ID} source={} dataset={} file={} records={} resident_bytes={bg_resident}",
        resources.source_sha256(),resources.dataset_sha256(),
        resources.background_file_sha256().unwrap(),dataset.background.entries.len()
    );
    eprintln!("ROM_DATA runtime=decoded resource={BG_BASE_PALETTES_ID} source={} dataset={} file={} records={} resident_bytes={palette_resident}", resources.source_sha256(), resources.dataset_sha256(), resources.palette_file_sha256().unwrap(), dataset.palettes.entries.len());
    Ok(())
}

pub fn prepare_and_attach(machine: &mut gba_core::Machine, status: bool) -> Result<(), String> {
    let prepared = prepare_startup(&machine.bus.rom, status)?;
    attach_from_directory(machine, &prepared.directory)
}

pub fn report_usage(machine: &gba_core::Machine) {
    if let Some(resources) = machine.bus.rom_resources() {
        let stats = resources.stats();
        eprintln!("ROM_DATA_USE resource={} dataset={} bus_reads={} host_rom_reads={} host_ram_reads={} dynamic_ram_reads={} validation_reads={} bus_source_mask={}",
            resources.resource_id(), resources.dataset_sha256(), stats.bus_reads, stats.host_rom_reads,
            stats.host_ram_reads, stats.dynamic_ram_reads, stats.validation_reads, stats.bus_source_mask);
        if let Some(bg) = resources.background_stats() {
            eprintln!("ROM_DATA_USE resource={BG_DESCRIPTORS_ID} dataset={} bus_reads={} projected_bytes={}",
                resources.dataset_sha256(),bg.bus_reads,bg.projected_bytes);
        }
        if let Some(palette) = resources.palette_stats() {
            eprintln!("ROM_DATA_USE resource={BG_BASE_PALETTES_ID} dataset={} bus_reads={} source_bytes={} lz77_calls={} decoded_bytes={}", resources.dataset_sha256(), palette.bus_reads, palette.source_bytes, palette.lz77_calls, palette.decoded_bytes);
        }
    }
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("Cannot create {}: {e}", path.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("Cannot finish {}: {e}", path.display()))
}

/// Publish only into an absent directory. An existing valid dataset is reused;
/// invalid data produce an explicit error, never a silent ROM fallback.
pub fn extract(path: &Path, source: &mut Extraction) -> Result<&'static str, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            verify(path, source)?;
            return Ok("reused");
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Cannot inspect output directory: {e}")),
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| format!("Cannot create resources parent: {e}"))?;
    let temporary = tempfile::Builder::new()
        .prefix(".ffta-resources-")
        .tempdir_in(parent)
        .map_err(|e| format!("Cannot prepare resource directory: {e}"))?;
    write_dataset(temporary.path(), source, &mut write_synced)?;
    match fs::rename(temporary.path(), path) {
        Ok(()) => Ok("created"),
        // A concurrent writer may have published the same complete dataset.
        Err(_) if verify(path, source).is_ok() => Ok("reused"),
        Err(e) => Err(format!("Cannot publish resource directory: {e}")),
    }
}

fn write_dataset(
    path: &Path,
    source: &mut Extraction,
    write: &mut impl FnMut(&Path, &[u8]) -> Result<(), String>,
) -> Result<(), String> {
    let data = toml::to_string_pretty(&source.dimensions)
        .map_err(|e| format!("Cannot encode dimensions: {e}"))?;
    let background = source
        .background
        .as_ref()
        .ok_or("Missing background descriptor extraction")?;
    let bg_data = encode_background(background)?;
    let palette_data = encode_palettes(
        &source
            .palettes
            .as_ref()
            .ok_or("Missing base palette extraction")?
            .decoded,
    )?;
    source.set_file_hashes(&[data.as_bytes(), &bg_data, &palette_data])?;
    let manifest = toml::to_string_pretty(&source.manifest)
        .map_err(|e| format!("Cannot encode manifest: {e}"))?;
    check_dataset_budget(
        manifest.len(),
        data.len(),
        bg_data.len(),
        palette_data.len(),
    )?;
    write(&path.join(OBJ_DIMENSIONS_FILE), data.as_bytes())?;
    write(&path.join(BG_DESCRIPTORS_FILE), &bg_data)?;
    write(&path.join(BG_BASE_PALETTES_FILE), &palette_data)?;
    write(&path.join(MANIFEST_FILE), manifest.as_bytes())?;
    verify(path, source)?;
    Ok(())
}

pub fn cmd(args: &[String]) -> Result<(), String> {
    let operation = args.first().map(String::as_str);
    if !matches!(
        (operation, args.len()),
        (Some("extract" | "verify"), 3) | (Some("prepare"), 2 | 3)
    ) {
        return Err("usage: recomp rom-data <extract|verify> <rom> <dataset-directory> OR rom-data prepare <rom> [resources-root]".into());
    }
    let rom = fs::read(&args[1]).map_err(|e| format!("Cannot read ROM: {e}"))?;
    let mut source = Extraction::from_rom(&rom)?;
    if operation == Some("prepare") {
        let root = args
            .get(2)
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(preparation::storage_root)?;
        let prepared = preparation::prepare(
            &root,
            &mut source,
            &mut preparation::console_progress(false),
        )?;
        println!("ROM_DATA action={} profile={} source={} schema={} decoded={} runtime=offline directory={}",
            prepared.action, source.manifest.profile, source.manifest.source_rom_sha256,
            source.manifest.schema, source.manifest.coverage.join(","), prepared.directory.display());
        return Ok(());
    }
    let path = Path::new(&args[2]);
    let action = if args[0] == "extract" {
        extract(path, &mut source)?
    } else {
        verify(path, &source)?;
        "verified"
    };
    println!("ROM_DATA action={action} profile={} source={} schema={} decoded={} files=4 runtime=offline",
        source.manifest.profile, source.manifest.source_rom_sha256, source.manifest.schema,
        source.manifest.coverage.join(","));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic structure, deliberately not a copy of a game's dimension table.
    pub(super) fn fixture() -> Extraction {
        use gba_core::rom_data::*;
        let dimensions = ObjDimensions {
            entries: std::array::from_fn(|i| ObjSize {
                width: 8 << (i % 4),
                height: 8 << ((i + 1) % 4),
            }),
        };
        let absent = bg_descriptors::TransferLink {
            control_source: 0,
            data_source: 0,
            destination: 0,
        };
        let background = Descriptors {
            entries: vec![
                bg_descriptors::Descriptor {
                    graphics_source: 0x4000,
                    paired_planes_source: 0x4100,
                    halfword_planes_source: 0x4200,
                    palette_source: 0x4300,
                    auxiliary_patches_source: 0x4400,
                    transfer: absent.clone(),
                    extra_transfers: std::array::from_fn(|_| absent.clone()),
                    variant_source: 0,
                    selectors: [1, 2],
                    alternate_palette_key: 3,
                    trailing_byte: 0,
                };
                bg_descriptors::BASE_RECORDS
            ],
        };
        let bg_hash = gba_core::ffta_rom::digest(&background.source_bytes().unwrap());
        let decoded_palettes = Palettes {
            entries: vec![
                bg_palettes::Palette {
                    banks: vec![[0x8123; 16]; 4],
                    encoded_source: [
                        vec![0, 128, 0, 0],
                        (0..64).flat_map(|_| 0x8123u16.to_le_bytes()).collect()
                    ]
                    .concat(),
                };
                bg_descriptors::BASE_RECORDS
            ],
        };
        let palette_sources: Vec<_> = (0..bg_descriptors::BASE_RECORDS)
            .map(|id| bg_palettes::PaletteSource {
                range: SourceRange {
                    offset: 0x4400,
                    length: 132,
                    sha256: "synthetic-palette-source".into(),
                    role: format!("base-palette/{id}"),
                },
                encoding: bg_palettes::Encoding::Raw,
            })
            .collect();
        let palette_ranges = palette_sources
            .iter()
            .map(|source| source.range.clone())
            .collect();
        Extraction {
            manifest: Manifest {
                schema: SCHEMA,
                extractor_revision: EXTRACTOR_REV,
                decoder_revision: DECODER_REV,
                source_rom_sha256: "synthetic-source".into(),
                source_rom_size: 0x100000,
                profile: "synthetic".into(),
                coverage: vec![
                    OBJ_DIMENSIONS_ID.into(),
                    BG_DESCRIPTORS_ID.into(),
                    BG_BASE_PALETTES_ID.into(),
                ],
                resources: vec![
                    Resource {
                        id: OBJ_DIMENSIONS_ID.into(),
                        path: OBJ_DIMENSIONS_FILE.into(),
                        format: "obj-dimensions-v1".into(),
                        sha256: String::new(),
                        dependencies: vec![],
                        sources: vec![SourceRange {
                            offset: 0,
                            length: 48,
                            sha256: gba_core::ffta_rom::digest(&dimensions.source_bytes()),
                            role: "synthetic".into(),
                        }],
                    },
                    Resource {
                        id: BG_DESCRIPTORS_ID.into(),
                        path: BG_DESCRIPTORS_FILE.into(),
                        format: gba_core::rom_data::BG_DESCRIPTORS_FORMAT.into(),
                        sha256: String::new(),
                        dependencies: vec![],
                        sources: vec![SourceRange {
                            offset: 0x100,
                            length: (bg_descriptors::BASE_RECORDS * bg_descriptors::RECORD_BYTES)
                                as u32,
                            sha256: bg_hash,
                            role: "synthetic-bg".into(),
                        }],
                    },
                    Resource {
                        id: BG_BASE_PALETTES_ID.into(),
                        path: BG_BASE_PALETTES_FILE.into(),
                        format: BG_BASE_PALETTES_FORMAT.into(),
                        sha256: String::new(),
                        dependencies: vec![BG_DESCRIPTORS_ID.into()],
                        sources: palette_ranges,
                    },
                ],
            },
            dimensions,
            background: Some(background),
            palettes: Some(bg_palettes::Catalog {
                decoded: decoded_palettes,
                sources: palette_sources,
            }),
        }
    }

    #[test]
    fn deterministic_publication_reuse_and_relocation() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("first folder/data");
        let b = temp.path().join("second");
        let mut source = fixture();
        assert_eq!(extract(&a, &mut source).unwrap(), "created");
        let before = fs::read(a.join(MANIFEST_FILE)).unwrap();
        assert_eq!(extract(&a, &mut source).unwrap(), "reused");
        assert_eq!(before, fs::read(a.join(MANIFEST_FILE)).unwrap());
        assert_eq!(extract(&b, &mut source).unwrap(), "created");
        for name in [
            MANIFEST_FILE,
            OBJ_DIMENSIONS_FILE,
            BG_DESCRIPTORS_FILE,
            BG_BASE_PALETTES_FILE,
        ] {
            assert_eq!(
                fs::read(a.join(name)).unwrap(),
                fs::read(b.join(name)).unwrap()
            );
        }
        let moved = temp.path().join("moved folder");
        fs::rename(&a, &moved).unwrap();
        assert_eq!(verify(&moved, &source).unwrap(), source.dimensions);
        assert!(!String::from_utf8(before)
            .unwrap()
            .contains(&temp.path().display().to_string()));
    }

    #[test]
    fn corruption_missing_files_and_edits_fail_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let mut source = fixture();
        extract(&path, &mut source).unwrap();
        let original = fs::read(path.join(OBJ_DIMENSIONS_FILE)).unwrap();
        fs::write(path.join(OBJ_DIMENSIONS_FILE), b"broken").unwrap();
        assert!(verify(&path, &source).is_err());
        assert!(extract(&path, &mut source).is_err());
        assert_eq!(fs::read(path.join(OBJ_DIMENSIONS_FILE)).unwrap(), b"broken");
        fs::write(path.join(OBJ_DIMENSIONS_FILE), &original).unwrap();
        let mut changed = source.dimensions.clone();
        changed.entries[0].width = 64;
        let bytes = toml::to_string_pretty(&changed).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[0].sha256 = gba_core::ffta_rom::digest(bytes.as_bytes());
        fs::write(path.join(OBJ_DIMENSIONS_FILE), &bytes).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(&path, &source).is_err());
        fs::remove_file(path.join(OBJ_DIMENSIONS_FILE)).unwrap();
        assert!(verify(&path, &source).is_err());
        fs::write(
            path.join(OBJ_DIMENSIONS_FILE),
            [0; MAX_FILE_BYTES as usize + 1],
        )
        .unwrap();
        assert!(verify(&path, &source).is_err());
    }

    #[test]
    fn strict_parser_and_fixed_paths() {
        let source = fixture();
        let bytes = toml::to_string_pretty(&source.dimensions).unwrap();
        let wrong = format!("unknown = 1\n{bytes}");
        assert!(parse::<ObjDimensions>(wrong.as_bytes(), "test").is_err());
        let wrong = bytes.replacen("width = 8", "width = 65536", 1);
        assert!(parse::<ObjDimensions>(wrong.as_bytes(), "test").is_err());
        assert!(parse::<ObjDimensions>(&[255], "test").is_err());
        let fewer = ObjDimensions {
            entries: source.dimensions.entries,
        };
        let text = toml::to_string_pretty(&fewer).unwrap();
        let cut = text.rfind("[[entries]]").unwrap();
        assert!(parse::<ObjDimensions>(text[..cut].as_bytes(), "test").is_err());
        assert!(cmd(&[]).is_err());
        assert!(cmd(&["unknown".into(), "rom".into(), "out".into()]).is_err());
    }

    #[test]
    fn background_json_is_direct_strict_and_deterministic() {
        let source = fixture();
        let background = source.background.as_ref().unwrap();
        let bytes = encode_background(background).unwrap();
        assert_eq!(bytes, encode_background(background).unwrap());
        assert_eq!(&parse_background(&bytes).unwrap(), background);
        assert!(bytes.starts_with(b"{\n  \"entries\": ["));
        assert!(bytes.len() as u64 <= MAX_BG_DESCRIPTORS_FILE_BYTES);
        assert!(parse_background(&[255]).is_err());
        assert!(parse_background(&bytes[..bytes.len() - 1]).is_err());
        let mut trailing = bytes.clone();
        trailing.extend_from_slice(b" true");
        assert!(parse_background(&trailing).is_err());
        assert!(parse_background(br#"{"entries":[],"entries":[]}"#).is_err());
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for invalid in 0..6 {
            let mut changed = value.clone();
            match invalid {
                0 => changed["entries"][0]["unknown"] = serde_json::json!(0),
                1 => changed["entries"][0]["graphics_source"] = serde_json::json!(4294967296u64),
                2 => changed["entries"][0]["selectors"][0] = serde_json::json!(256),
                3 => {
                    changed["entries"][0]
                        .as_object_mut()
                        .unwrap()
                        .remove("selectors");
                }
                4 => {
                    changed["entries"][0]["extra_transfers"]
                        .as_array_mut()
                        .unwrap()
                        .pop();
                }
                5 => changed["entries"] = serde_json::json!([]),
                _ => unreachable!(),
            }
            assert!(parse_background(&serde_json::to_vec(&changed).unwrap()).is_err());
        }
    }

    #[test]
    fn background_is_required_and_rehashed_edits_are_not_base_data() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let mut source = fixture();
        extract(&path, &mut source).unwrap();
        let original = fs::read(path.join(BG_DESCRIPTORS_FILE)).unwrap();
        assert!(original.len() > MAX_FILE_BYTES as usize);
        let mut edited = source.background.clone().unwrap();
        edited.entries[0].selectors[0] ^= 1;
        let bytes = encode_background(&edited).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[1].sha256 = gba_core::ffta_rom::digest(&bytes);
        fs::write(path.join(BG_DESCRIPTORS_FILE), &bytes).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(&path, &source).unwrap_err().contains("reproduce"));
        assert!(extract(&path, &mut source).is_err());
        assert_eq!(
            fs::read(path.join(BG_DESCRIPTORS_FILE)).unwrap(),
            bytes.as_slice()
        );
        fs::write(path.join(BG_DESCRIPTORS_FILE), original).unwrap();
        for change in 0..4 {
            let mut bad = source.manifest.clone();
            match change {
                0 => {
                    bad.resources.remove(1);
                }
                1 => bad.resources.push(bad.resources[1].clone()),
                2 => bad.resources[1].path = "../outside".into(),
                _ => bad.resources[1].sources[0].offset += 1,
            }
            fs::write(
                path.join(MANIFEST_FILE),
                toml::to_string_pretty(&bad).unwrap(),
            )
            .unwrap();
            assert!(verify(&path, &source).is_err());
        }
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&source.manifest).unwrap(),
        )
        .unwrap();
        fs::remove_file(path.join(BG_DESCRIPTORS_FILE)).unwrap();
        assert!(verify(&path, &source).is_err());
    }

    #[test]
    fn palette_json_preserves_words_and_rejects_malformed_or_forged_data() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let mut source = fixture();
        extract(&path, &mut source).unwrap();
        let bytes = fs::read(path.join(BG_BASE_PALETTES_FILE)).unwrap();
        let palettes = parse_palettes(&bytes).unwrap();
        assert_eq!(encode_palettes(&palettes).unwrap(), bytes);
        assert_eq!(palettes.entries[0].banks[0][0], 0x8123);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for invalid in 0..6 {
            let mut changed = value.clone();
            match invalid {
                0 => changed["entries"][0]["unknown"] = serde_json::json!(0),
                1 => changed["entries"][0]["banks"][0][0] = serde_json::json!(65536),
                2 => {
                    changed["entries"][0]["banks"][0]
                        .as_array_mut()
                        .unwrap()
                        .pop();
                }
                3 => changed["entries"][0]["banks"] = serde_json::json!([]),
                4 => changed["entries"] = serde_json::json!([]),
                5 => changed["entries"][0]["banks"][0][0] = serde_json::json!(1),
                _ => unreachable!(),
            }
            assert!(parse_palettes(&serde_json::to_vec(&changed).unwrap()).is_err());
        }
        assert!(parse_palettes(&bytes[..bytes.len() - 1]).is_err());
        // Equal decoded words with different source bytes would change guest
        // branches/reads, so even a coherent re-encoding is not valid base data.
        let mut recoded = palettes.clone();
        let words = recoded.entries[0].source_bytes().unwrap();
        let mut encoded = (((words.len() as u32) << 8) | 0x10).to_le_bytes().to_vec();
        for group in words.chunks(8) {
            encoded.push(0);
            encoded.extend_from_slice(group);
        }
        recoded.entries[0].encoded_source = encoded;
        let recoded = encode_palettes(&recoded).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[2].sha256 = gba_core::ffta_rom::digest(&recoded);
        fs::write(path.join(BG_BASE_PALETTES_FILE), recoded).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(&path, &source).unwrap_err().contains("reproduce"));
        let mut edited = palettes;
        edited.entries[0].banks[0][0] ^= 0x8000;
        let word = edited.entries[0].banks[0][0].to_le_bytes();
        edited.entries[0].encoded_source[4..6].copy_from_slice(&word);
        let edited = encode_palettes(&edited).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[2].sha256 = gba_core::ffta_rom::digest(&edited);
        fs::write(path.join(BG_BASE_PALETTES_FILE), edited).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(&path, &source).unwrap_err().contains("reproduce"));
        fs::write(path.join(BG_BASE_PALETTES_FILE), bytes).unwrap();
        for invalid in 0..4 {
            let mut manifest = source.manifest.clone();
            match invalid {
                0 => {
                    manifest.resources.remove(2);
                }
                1 => manifest.resources[2].path = "../outside".into(),
                2 => manifest.resources[2].dependencies.clear(),
                3 => manifest.resources[2].sources[0].offset += 4,
                _ => unreachable!(),
            }
            fs::write(
                path.join(MANIFEST_FILE),
                toml::to_string_pretty(&manifest).unwrap(),
            )
            .unwrap();
            assert!(verify(&path, &source).is_err());
        }
        fs::remove_file(path.join(BG_BASE_PALETTES_FILE)).unwrap();
        assert!(verify(&path, &source).is_err());
    }

    #[test]
    fn per_family_input_bounds_and_complete_set_limit() {
        fn padded(mut bytes: Vec<u8>, length: usize) -> Vec<u8> {
            bytes.resize(length, b' ');
            bytes
        }
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let mut source = fixture();
        extract(&path, &mut source).unwrap();
        let names = [
            MANIFEST_FILE,
            OBJ_DIMENSIONS_FILE,
            BG_DESCRIPTORS_FILE,
            BG_BASE_PALETTES_FILE,
        ];
        let original: Vec<_> = names
            .iter()
            .map(|name| fs::read(path.join(name)).unwrap())
            .collect();
        let bg = padded(original[2].clone(), MAX_BG_DESCRIPTORS_FILE_BYTES as usize);
        let palettes = padded(
            original[3].clone(),
            MAX_BG_BASE_PALETTES_FILE_BYTES as usize,
        );
        let mut manifest = source.manifest.clone();
        manifest.resources[1].sha256 = gba_core::ffta_rom::digest(&bg);
        fs::write(path.join(BG_DESCRIPTORS_FILE), &bg).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        verify(&path, &source).unwrap();
        manifest.resources[2].sha256 = gba_core::ffta_rom::digest(&palettes);
        fs::write(path.join(BG_BASE_PALETTES_FILE), &palettes).unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verify(&path, &source).unwrap_err().contains("total size"));
        for (name, limit) in [
            (MANIFEST_FILE, MAX_MANIFEST_FILE_BYTES),
            (OBJ_DIMENSIONS_FILE, MAX_FILE_BYTES),
            (BG_DESCRIPTORS_FILE, MAX_BG_DESCRIPTORS_FILE_BYTES),
            (BG_BASE_PALETTES_FILE, MAX_BG_BASE_PALETTES_FILE_BYTES),
        ] {
            for (name, bytes) in names.iter().zip(&original) {
                fs::write(path.join(name), bytes).unwrap();
            }
            fs::File::options()
                .write(true)
                .open(path.join(name))
                .unwrap()
                .set_len(limit + 1)
                .unwrap();
            assert!(verify(&path, &source)
                .unwrap_err()
                .contains(&format!("at most {limit}")));
        }
    }

    #[test]
    fn competing_publishers_leave_one_verified_dataset() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("data");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    extract(&path, &mut fixture()).unwrap()
                })
            })
            .collect();
        let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(outcomes.iter().filter(|&&v| v == "created").count(), 1);
        verify(&path, &fixture()).unwrap();
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[test]
    #[ignore = "requires the user's exact USA and EUR ROMs; no game data is bundled"]
    fn regional_cli_extraction_verification_and_identity() {
        let temp = tempfile::tempdir().unwrap();
        let mut decoded = Vec::new();
        let mut manifests = Vec::new();
        for (env, profile) in [
            ("FFTA_TEST_ROM_USA", "usa"),
            ("FFTA_TEST_ROM_EUR", "europe"),
        ] {
            let rom_path = std::env::var(env).unwrap();
            let output = temp.path().join(profile);
            let output_arg = output.to_str().unwrap().to_owned();
            cmd(&["extract".into(), rom_path.clone(), output_arg.clone()]).unwrap();
            let manifest = fs::read(output.join(MANIFEST_FILE)).unwrap();
            cmd(&["extract".into(), rom_path.clone(), output_arg.clone()]).unwrap();
            assert_eq!(manifest, fs::read(output.join(MANIFEST_FILE)).unwrap());
            cmd(&["verify".into(), rom_path.clone(), output_arg]).unwrap();
            let source = Extraction::from_rom(&fs::read(&rom_path).unwrap()).unwrap();
            assert_eq!(verify(&output, &source).unwrap(), source.dimensions);
            decoded.push(fs::read(output.join(OBJ_DIMENSIONS_FILE)).unwrap());
            manifests.push(manifest);
        }
        assert_eq!(decoded[0], decoded[1]);
        assert_ne!(manifests[0], manifests[1]);
        let usa = fs::read(std::env::var("FFTA_TEST_ROM_USA").unwrap()).unwrap();
        assert!(verify(
            &temp.path().join("europe"),
            &Extraction::from_rom(&usa).unwrap()
        )
        .is_err());
    }

    #[test]
    #[ignore = "requires the user's exact USA and EUR ROMs; no game data is bundled"]
    fn runtime_files_bind_transactionally_and_survive_slots_undo_and_backup_reboot() {
        use gba_core::Bus;
        use std::sync::Arc;
        for env in ["FFTA_TEST_ROM_USA", "FFTA_TEST_ROM_EUR"] {
            let temp = tempfile::tempdir().unwrap();
            let rom = fs::read(std::env::var(env).unwrap()).unwrap();
            let mut source = Extraction::from_rom(&rom).unwrap();
            let path = temp.path().join("decoded");
            extract(&path, &mut source).unwrap();
            let mut machine = super::super::make_machine(rom, None);
            let boot = machine.save_state().unwrap();
            attach_from_directory(&mut machine, &path).unwrap();
            let resources = machine.bus.rom_resources().unwrap().clone();
            assert_eq!(
                resources.file_sha256(),
                gba_core::ffta_rom::digest(&fs::read(path.join(OBJ_DIMENSIONS_FILE)).unwrap())
            );
            assert_eq!(boot, machine.save_state().unwrap());
            let offset = source.manifest.resources[0].sources[0].offset;
            let expected = machine.bus.read32(0x08000000 + offset);
            let save = temp.path().join("saves/test.sav");
            fs::create_dir_all(save.parent().unwrap()).unwrap();
            let mut slots = super::super::save_state::Slots::new(&save, &machine.bus.rom);
            let preview = vec![[0; 4]; 240 * 160];
            slots.save(&machine, 1, &preview, false).unwrap();
            machine.cpu.regs[4] = 37;
            let before = machine.save_state().unwrap();
            slots.load(&mut machine, 1).unwrap();
            assert_eq!(machine.cpu.regs[4], 0);
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            slots.undo(&mut machine).unwrap();
            assert_eq!(machine.save_state().unwrap(), before);
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            // A new activation must reject absent files; a running session
            // retains its already verified immutable in-memory resources.
            fs::remove_file(path.join(OBJ_DIMENSIONS_FILE)).unwrap();
            assert!(attach_from_directory(&mut machine, &path).is_err());
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            let mut fresh = super::super::make_machine(machine.bus.rom.clone(), None);
            assert!(attach_from_directory(&mut fresh, &path).is_err());
            assert!(fresh.bus.rom_resources().is_none());
            assert_eq!(machine.bus.read32(0x08000000 + offset), expected);
            let count = resources.stats();
            machine.load_state(&boot).unwrap();
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            assert_eq!(resources.stats(), count);
            let length = machine.bus.save_data().unwrap().len();
            let backups = super::super::save_backups::Backups::new(&save);
            fs::write(&save, vec![7; length]).unwrap();
            backups.capture_startup().unwrap();
            let generation = backups.choices(length)[0].generation.unwrap();
            machine.cpu.regs[4] = 99;
            backups.restore(generation, &mut machine, &boot).unwrap();
            assert_eq!(machine.cpu.regs[4], 0);
            assert!(Arc::ptr_eq(
                machine.bus.rom_resources().unwrap(),
                &resources
            ));
            assert_eq!(machine.bus.save_data().unwrap(), vec![7; length]);
            assert_eq!(machine.bus.read32(0x08000000 + offset), expected);
            assert!(resources.stats().bus_reads > 0);
        }
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_resources_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real");
        let alias = temp.path().join("alias");
        let mut source = fixture();
        extract(&real, &mut source).unwrap();
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        assert!(verify(&alias, &source).is_err());
        fs::rename(real.join(OBJ_DIMENSIONS_FILE), temp.path().join("outside")).unwrap();
        std::os::unix::fs::symlink(temp.path().join("outside"), real.join(OBJ_DIMENSIONS_FILE))
            .unwrap();
        assert!(verify(&real, &source).is_err());
        fs::remove_file(real.join(OBJ_DIMENSIONS_FILE)).unwrap();
        fs::rename(temp.path().join("outside"), real.join(OBJ_DIMENSIONS_FILE)).unwrap();
        fs::rename(
            real.join(BG_DESCRIPTORS_FILE),
            temp.path().join("outside-bg"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            temp.path().join("outside-bg"),
            real.join(BG_DESCRIPTORS_FILE),
        )
        .unwrap();
        assert!(verify(&real, &source).is_err());
    }
}
