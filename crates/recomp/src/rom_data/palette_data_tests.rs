//! Whole palette-file preparation and comparison with retained original outputs.
use super::*;
use std::{collections::BTreeSet, sync::Arc};

#[test]
#[ignore = "requires exact regional ROMs, existing e4 datasets/palette captures and new FFTA_STATE_TEST_PALETTE_DATA_ROOT"]
fn regional_palette_files_match_captures_and_retain_previous_datasets() {
    let root = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_PALETTE_DATA_ROOT").unwrap());
    assert!(!root.exists(), "preserve earlier palette-data evidence");
    let old_root = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_OLD_DATASETS_ROOT").unwrap());
    let captures = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_PALETTE_CAPTURES").unwrap());
    let mut data_hashes = Vec::new();
    let mut reports = Vec::new();
    for (env, profile) in [
        ("FFTA_TEST_ROM_USA", "usa"),
        ("FFTA_TEST_ROM_EUR", "europe"),
    ] {
        let rom = fs::read(std::env::var_os(env).unwrap()).unwrap();
        let mut source = Extraction::from_rom(&rom).unwrap();
        let catalog = source.palettes.as_ref().unwrap();
        assert_eq!(catalog.decoded.entries.len(), 162);
        let mut unique = BTreeSet::new();
        let mut raw = BTreeSet::new();
        for (id, (palette, provenance)) in catalog
            .decoded
            .entries
            .iter()
            .zip(&catalog.sources)
            .enumerate()
        {
            let offset = provenance.range.offset;
            let expected = fs::read(
                captures
                    .join(profile)
                    .join(format!("palette-{offset:08x}.bin")),
            )
            .unwrap();
            assert_eq!(palette.source_bytes().unwrap(), expected);
            let bytes = &rom[offset as usize..(offset + provenance.range.length) as usize];
            assert_eq!(gba_core::ffta_rom::digest(bytes), provenance.range.sha256);
            assert_eq!(provenance.range.role, format!("base-palette/{id}"));
            unique.insert(offset);
            if provenance.encoding == gba_core::rom_data::bg_palettes::Encoding::Raw {
                raw.insert(offset);
            }
        }
        assert_eq!((unique.len(), raw.len()), (127, 2));
        let base = root.join(profile);
        let offline = base.join("offline");
        extract(&offline, &mut source).unwrap();
        assert_eq!(fs::read_dir(&offline).unwrap().count(), 4);
        let original = fs::read(offline.join(BG_BASE_PALETTES_FILE)).unwrap();
        let parsed = parse_palettes(&original).unwrap();
        assert_eq!(parsed, source.palettes.as_ref().unwrap().decoded);
        assert_eq!(original, encode_palettes(&parsed).unwrap());
        data_hashes.push(gba_core::ffta_rom::digest(&original));

        let mut machine = crate::make_machine(rom, None);
        let snapshot = machine.save_state().unwrap();
        attach_from_directory(&mut machine, &offline).unwrap();
        let original_owner = machine.bus.rom_resources().unwrap().clone();
        assert_eq!(machine.save_state().unwrap(), snapshot);
        let mut edited = parsed;
        edited.entries[0].banks[0][0] ^= 0x8000;
        let words: Vec<u8> = edited.entries[0]
            .banks
            .iter()
            .flatten()
            .flat_map(|word| word.to_le_bytes())
            .collect();
        let mut encoded = ((words.len() as u32) << 8).to_le_bytes().to_vec();
        encoded.extend_from_slice(&words);
        edited.entries[0].encoded_source = encoded;
        let edited = encode_palettes(&edited).unwrap();
        let mut manifest = source.manifest.clone();
        manifest.resources[2].sha256 = gba_core::ffta_rom::digest(&edited);
        fs::write(offline.join(BG_BASE_PALETTES_FILE), edited).unwrap();
        fs::write(
            offline.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(attach_from_directory(&mut machine, &offline).is_err());
        assert!(Arc::ptr_eq(
            machine.bus.rom_resources().unwrap(),
            &original_owner
        ));
        assert_eq!(machine.save_state().unwrap(), snapshot);
        let mut annotated = original.clone();
        annotated.extend_from_slice(b"\n ");
        manifest.resources[2].sha256 = gba_core::ffta_rom::digest(&annotated);
        fs::write(offline.join(BG_BASE_PALETTES_FILE), annotated).unwrap();
        fs::write(
            offline.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        attach_from_directory(&mut machine, &offline).unwrap();
        let current = machine.bus.rom_resources().unwrap().clone();
        assert_ne!(current.dataset_sha256(), original_owner.dataset_sha256());
        assert_eq!(current.file_sha256(), original_owner.file_sha256());
        assert_eq!(
            current.background_file_sha256(),
            original_owner.background_file_sha256()
        );
        assert_eq!(machine.save_state().unwrap(), snapshot);
        fs::remove_file(offline.join(BG_BASE_PALETTES_FILE)).unwrap();
        assert!(attach_from_directory(&mut machine, &offline).is_err());
        assert!(Arc::ptr_eq(machine.bus.rom_resources().unwrap(), &current));
        fs::write(offline.join(BG_BASE_PALETTES_FILE), &original).unwrap();
        fs::write(
            offline.join(MANIFEST_FILE),
            toml::to_string_pretty(&source.manifest).unwrap(),
        )
        .unwrap();

        let old = old_root.join(profile).join("offline dataset");
        let old_names = [MANIFEST_FILE, OBJ_DIMENSIONS_FILE, BG_DESCRIPTORS_FILE];
        let old_bytes: Vec<_> = old_names
            .iter()
            .map(|name| fs::read(old.join(name)).unwrap())
            .collect();
        let old_manifest: Manifest = parse(&old_bytes[0], "previous manifest").unwrap();
        assert_eq!(
            (
                old_manifest.schema,
                old_manifest.extractor_revision,
                old_manifest.decoder_revision
            ),
            (1, 4, 3)
        );
        assert_eq!(
            old_manifest.source_rom_sha256,
            source.manifest.source_rom_sha256
        );
        let managed = base.join("resources");
        let old_generation = managed
            .join(&old_manifest.source_rom_sha256)
            .join("v1/e4-d3/generation-retained");
        fs::create_dir_all(&old_generation).unwrap();
        for (name, bytes) in old_names.iter().zip(&old_bytes) {
            fs::write(old_generation.join(name), bytes).unwrap();
        }
        fs::write(
            old_generation.parent().unwrap().join("current"),
            b"generation-retained\n",
        )
        .unwrap();
        assert!(verify(&old_generation, &source).is_err());
        let first = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(first.action, "created");
        assert!(first.directory.parent().unwrap().ends_with("v1/e6-d5"));
        let reused = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(reused.action, "reused");
        assert_eq!(first.directory, reused.directory);
        fs::write(first.directory.join(BG_BASE_PALETTES_FILE), b"broken").unwrap();
        let repaired = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(repaired.action, "repaired");
        assert_ne!(repaired.directory, first.directory);
        assert_eq!(
            fs::read(first.directory.join(BG_BASE_PALETTES_FILE)).unwrap(),
            b"broken"
        );
        assert_eq!(
            fs::read(repaired.directory.join(BG_BASE_PALETTES_FILE)).unwrap(),
            original
        );
        fs::remove_file(repaired.directory.join(BG_BASE_PALETTES_FILE)).unwrap();
        let ready = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(ready.action, "repaired");
        let relative = ready
            .directory
            .strip_prefix(&managed)
            .unwrap()
            .to_path_buf();
        let moved = base.join("moved resources");
        fs::rename(&managed, &moved).unwrap();
        let relocated = preparation::prepare(&moved, &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(relocated.action, "reused");
        assert_eq!(relocated.directory, moved.join(relative));
        let retained = moved.join(old_generation.strip_prefix(&managed).unwrap());
        for (name, bytes) in old_names.iter().zip(&old_bytes) {
            assert_eq!(fs::read(old.join(name)).unwrap(), *bytes);
            assert_eq!(fs::read(retained.join(name)).unwrap(), *bytes);
        }
        assert_eq!(
            fs::read(retained.parent().unwrap().join("current")).unwrap(),
            b"generation-retained\n"
        );
        reports.push(serde_json::json!({"profile":profile,"palette_file_sha256":data_hashes.last().unwrap(),"palette_file_bytes":original.len(),"old_revision_retained":true,"source_equal":true,"whole_set_identity":true,"create_reuse_repair_move":true}));
        eprintln!("PALETTE_DATA profile={profile} records=162 unique=127 codecs=2 source_equal=true old_e4_retained=true gameplay_consumer=false");
    }
    assert_eq!(data_hashes[0], data_hashes[1]);
    fs::write(
        root.join("results.json"),
        serde_json::to_vec_pretty(&reports).unwrap(),
    )
    .unwrap();
}
