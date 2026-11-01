//! Focused EUR managed-file acceptance; Wine selected, Linux/USA pending.
use super::*;
use std::sync::Arc;

#[test]
#[ignore = "requires EUR ROM, retained e7 dataset/typed02 JSON and absent FFTA_STATE_TEST_PACKET_MANAGED_ROOT"]
fn european_packet_dataset_publishes_authenticates_and_retains_e7() {
    let root = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_PACKET_MANAGED_ROOT").unwrap());
    assert!(!root.exists(), "preserve managed evidence");
    let old = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_OLD_E7_DATASET").unwrap());
    let typed = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_PACKET_TYPED_JSON").unwrap());
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    assert_eq!(
        gba_core::ffta_rom::digest(&rom),
        gba_core::ffta_rom::EUROPE_SHA256
    );
    let names = [
        MANIFEST_FILE,
        OBJ_DIMENSIONS_FILE,
        BG_DESCRIPTORS_FILE,
        BG_BASE_PALETTES_FILE,
        BG_ALTERNATE_PALETTES_FILE,
        BG_PACKET_PROGRAMS_FILE,
    ];
    let old_files: Vec<_> = names[..5]
        .iter()
        .map(|name| fs::read(old.join(name)).unwrap())
        .collect();
    let prior: Manifest = parse(&old_files[0], "retained e7 manifest").unwrap();
    assert_eq!(
        (
            prior.schema,
            prior.extractor_revision,
            prior.decoder_revision
        ),
        (1, 7, 6)
    );
    let mut source = Extraction::from_rom(&rom).unwrap();
    assert_eq!(
        (
            source.manifest.schema,
            source.manifest.extractor_revision,
            source.manifest.decoder_revision
        ),
        (1, 8, 7)
    );
    assert_eq!(source.manifest.resources.len(), 5);
    assert_eq!(source.manifest.resources[4].id, BG_PACKET_PROGRAMS_ID);
    assert_eq!(
        source.manifest.resources[4].dependencies,
        [BG_DESCRIPTORS_ID]
    );
    assert_eq!(source.manifest.resources[4].sources.len(), 352);
    let offline = root.join("offline dataset");
    extract(&offline, &mut source).unwrap();
    assert_eq!(fs::read_dir(&offline).unwrap().count(), 6);
    let files: Vec<_> = names
        .iter()
        .map(|name| fs::read(offline.join(name)).unwrap())
        .collect();
    assert_eq!(files[5], fs::read(&typed).unwrap());
    for index in 1..5 {
        assert_eq!(
            files[index], old_files[index],
            "prior decoded file unchanged"
        );
    }
    let programs = parse_packet_programs(&files[5]).unwrap();
    assert_eq!(encode_packet_programs(&programs).unwrap(), files[5]);
    let managed = root.join("resources");
    let retained = managed
        .join(&prior.source_rom_sha256)
        .join("v1/e7-d6/generation-retained");
    fs::create_dir_all(&retained).unwrap();
    for (name, bytes) in names[..5].iter().zip(&old_files) {
        fs::write(retained.join(name), bytes).unwrap();
    }
    fs::write(
        retained.parent().unwrap().join("current"),
        b"generation-retained\n",
    )
    .unwrap();
    assert!(verify(&retained, &source).is_err());
    let first = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
    assert_eq!(first.action, "created");
    assert!(first.directory.parent().unwrap().ends_with("v1/e8-d7"));
    let reused = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
    assert_eq!(reused.action, "reused");
    assert_eq!(reused.directory, first.directory);
    fs::remove_file(first.directory.join(BG_PACKET_PROGRAMS_FILE)).unwrap();
    let repaired_missing = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
    assert_eq!(repaired_missing.action, "repaired");
    assert!(!first.directory.join(BG_PACKET_PROGRAMS_FILE).exists());
    fs::write(
        repaired_missing.directory.join(BG_PACKET_PROGRAMS_FILE),
        b"partial-packet",
    )
    .unwrap();
    let repaired = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
    assert_eq!(repaired.action, "repaired");
    assert_eq!(
        fs::read(repaired_missing.directory.join(BG_PACKET_PROGRAMS_FILE)).unwrap(),
        b"partial-packet"
    );
    let mut machine = crate::make_machine(rom, None);
    let snapshot = machine.save_state().unwrap();
    attach_from_directory(&mut machine, &offline).unwrap();
    let owner = machine.bus.rom_resources().unwrap().clone();
    assert_eq!(machine.save_state().unwrap(), snapshot);
    let mut forged = programs;
    let index = forged
        .halfwords
        .programs
        .iter()
        .position(|p| p.encoded_source[0] == 1)
        .unwrap();
    let program = &forged.halfwords.programs[index];
    let mut encoded = program.encoded_source.clone();
    encoded[8] ^= 0x80;
    forged.halfwords.programs[index] = gba_core::rom_data::bg_packet_data::Program::decode(
        &encoded,
        gba_core::rom_data::bg_packet_data::Role::Halfwords,
        program.relative_source,
    )
    .unwrap()
    .0;
    let forged_bytes = encode_packet_programs(&forged).unwrap();
    let mut manifest = source.manifest.clone();
    manifest.resources[4].sha256 = gba_core::ffta_rom::digest(&forged_bytes);
    fs::write(offline.join(BG_PACKET_PROGRAMS_FILE), forged_bytes).unwrap();
    fs::write(
        offline.join(MANIFEST_FILE),
        toml::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    assert!(verify(&offline, &source).is_err());
    assert!(attach_from_directory(&mut machine, &offline).is_err());
    assert!(Arc::ptr_eq(machine.bus.rom_resources().unwrap(), &owner));
    assert_eq!(machine.save_state().unwrap(), snapshot);
    let mut annotated = files[5].clone();
    annotated.extend_from_slice(b"\n \n");
    manifest.resources[4].sha256 = gba_core::ffta_rom::digest(&annotated);
    fs::write(offline.join(BG_PACKET_PROGRAMS_FILE), annotated).unwrap();
    fs::write(
        offline.join(MANIFEST_FILE),
        toml::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    attach_from_directory(&mut machine, &offline).unwrap();
    let current = machine.bus.rom_resources().unwrap().clone();
    assert_ne!(current.dataset_sha256(), owner.dataset_sha256());
    assert_eq!(current.file_sha256(), owner.file_sha256());
    assert_eq!(
        current.background_file_sha256(),
        owner.background_file_sha256()
    );
    assert_eq!(current.palette_file_sha256(), owner.palette_file_sha256());
    assert_eq!(
        current.alternate_palette_file_sha256(),
        owner.alternate_palette_file_sha256()
    );
    assert_eq!(machine.save_state().unwrap(), snapshot);
    fs::remove_file(offline.join(BG_PACKET_PROGRAMS_FILE)).unwrap();
    assert!(attach_from_directory(&mut machine, &offline).is_err());
    assert!(Arc::ptr_eq(machine.bus.rom_resources().unwrap(), &current));
    for (name, bytes) in names.iter().zip(&files) {
        fs::write(offline.join(name), bytes).unwrap();
    }
    // Reject manifest paths even when the fixed source file remains present.
    manifest = source.manifest.clone();
    manifest.resources[4].path = "../outside.json".into();
    fs::write(
        offline.join(MANIFEST_FILE),
        toml::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    assert!(attach_from_directory(&mut machine, &offline).is_err());
    assert!(Arc::ptr_eq(machine.bus.rom_resources().unwrap(), &current));
    fs::write(offline.join(MANIFEST_FILE), &files[0]).unwrap();
    fs::File::options()
        .write(true)
        .open(offline.join(BG_PACKET_PROGRAMS_FILE))
        .unwrap()
        .set_len(MAX_BG_PACKET_PROGRAMS_FILE_BYTES + 1)
        .unwrap();
    assert!(verify(&offline, &source).unwrap_err().contains("at most"));
    fs::write(offline.join(BG_PACKET_PROGRAMS_FILE), &files[5]).unwrap();
    assert!(check_dataset_budget(
        MAX_MANIFEST_FILE_BYTES as usize,
        MAX_FILE_BYTES as usize,
        MAX_BG_DESCRIPTORS_FILE_BYTES as usize,
        MAX_BG_BASE_PALETTES_FILE_BYTES as usize,
        MAX_BG_ALTERNATE_PALETTES_FILE_BYTES as usize,
        MAX_BG_PACKET_PROGRAMS_FILE_BYTES as usize,
        MAX_BG_MAIN_GRAPHICS_FILE_BYTES as usize,
        MAX_BG_TRANSFER_PROGRAMS_FILE_BYTES as usize,
        MAX_SHORT_TEXTS_FILE_BYTES as usize,
        MAX_TEXT_CATALOGS_FILE_BYTES as usize,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
            0,
            0,
            0,
            0,

            0,

            0,
            0,

            0,
            0,

            0,
                0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,        0, 0, 0, 0,
        0,
        0,
        0,
    )
    .is_err());
    let active_relative = repaired
        .directory
        .strip_prefix(&managed)
        .unwrap()
        .to_path_buf();
    let retained_relative = retained.strip_prefix(&managed).unwrap().to_path_buf();
    let moved = root.join("moved resources");
    fs::rename(&managed, &moved).unwrap();
    let relocated = preparation::prepare(&moved, &mut source, &mut |_| Ok(())).unwrap();
    assert_eq!(relocated.action, "reused");
    assert_eq!(relocated.directory, moved.join(active_relative));
    for (name, bytes) in names.iter().zip(&files) {
        assert_eq!(fs::read(relocated.directory.join(name)).unwrap(), *bytes);
    }
    for (name, bytes) in names[..5].iter().zip(&old_files) {
        assert_eq!(
            fs::read(moved.join(&retained_relative).join(name)).unwrap(),
            *bytes
        );
        assert_eq!(fs::read(old.join(name)).unwrap(), *bytes);
    }
    assert_eq!(
        fs::read(
            moved
                .join(retained_relative)
                .parent()
                .unwrap()
                .join("current")
        )
        .unwrap(),
        b"generation-retained\n"
    );
    fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "region":"europe","files":6,"resources":5,"packet_sources":352,
            "packet_json_sha256":gba_core::ffta_rom::digest(&files[5]),
            "retained_e7_unchanged":true,"prior_resource_files_unchanged":true,
            "create_reuse_missing_corrupt_repair_move":true,"whole_set_identity":true,
            "activation_transactional":true,"coherent_edit_rejected":true,"file_guards":true,
            "packet_runtime":"verified-only","linux":"pending","usa":"pending",
        }))
        .unwrap(),
    )
    .unwrap();
    eprintln!("BG_PACKET_MANAGED europe files=6 packet_sources=352 e7_retained=true lifecycle=true transactional_activation=true packet_runtime=verified-only linux=pending usa=pending");
}
