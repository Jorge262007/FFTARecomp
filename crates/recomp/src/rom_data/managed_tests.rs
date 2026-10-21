//! Regional three-file preparation/activation checks. Fixtures stay private.
use super::*;
use std::sync::Arc;

#[test]
#[ignore = "requires exact ROM, prior e3 TOML dataset and absent FFTA_STATE_TEST_JSON_ROOT"]
fn json_revision_retains_previous_background_generation() {
    let root = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_JSON_ROOT").unwrap());
    assert!(!root.exists(), "preserve prior revision evidence");
    let old = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_OLD_BG_DATASET").unwrap());
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let mut source = Extraction::from_rom(&rom).unwrap();
    let old_names = [MANIFEST_FILE, OBJ_DIMENSIONS_FILE, "bg-descriptors.toml"];
    let old_bytes: Vec<_> = old_names
        .iter()
        .map(|name| fs::read(old.join(name)).unwrap())
        .collect();
    let old_manifest: Manifest = parse(&old_bytes[0], "old manifest").unwrap();
    assert_eq!(
        (
            old_manifest.schema,
            old_manifest.extractor_revision,
            old_manifest.decoder_revision
        ),
        (1, 3, 2)
    );
    assert_eq!(
        old_manifest.source_rom_sha256,
        source.manifest.source_rom_sha256
    );
    assert_eq!(old_manifest.resources[1].path, "bg-descriptors.toml");
    let old_directory = root
        .join(&old_manifest.source_rom_sha256)
        .join("v1/e3-d2/generation-retained");
    fs::create_dir_all(&old_directory).unwrap();
    for (name, bytes) in old_names.iter().zip(&old_bytes) {
        fs::write(old_directory.join(name), bytes).unwrap();
    }
    fs::write(
        old_directory.parent().unwrap().join("current"),
        b"generation-retained\n",
    )
    .unwrap();
    assert!(verify(&old_directory, &source).is_err());
    let prepared = preparation::prepare(&root, &mut source, &mut |_| Ok(())).unwrap();
    assert_eq!(prepared.action, "created");
    assert!(prepared.directory.parent().unwrap().ends_with("v1/e6-d5"));
    let json = fs::read(prepared.directory.join(BG_DESCRIPTORS_FILE)).unwrap();
    let decoded = parse_background(&json).unwrap();
    let prior: Descriptors = parse(&old_bytes[2], "prior BG").unwrap();
    assert_eq!(decoded, prior);
    assert_eq!(
        decoded.source_bytes().unwrap(),
        source.background.as_ref().unwrap().source_bytes().unwrap()
    );
    let logical: u64 = [
        MANIFEST_FILE,
        OBJ_DIMENSIONS_FILE,
        BG_DESCRIPTORS_FILE,
        BG_BASE_PALETTES_FILE,
    ]
    .iter()
    .map(|name| fs::metadata(prepared.directory.join(name)).unwrap().len())
    .sum();
    assert!(logical as usize <= MAX_DATASET_BYTES);
    let reused = preparation::prepare(&root, &mut source, &mut |_| Ok(())).unwrap();
    assert_eq!(reused.action, "reused");
    assert_eq!(reused.directory, prepared.directory);
    for (name, bytes) in old_names.iter().zip(&old_bytes) {
        assert_eq!(fs::read(old.join(name)).unwrap(), *bytes);
        assert_eq!(fs::read(old_directory.join(name)).unwrap(), *bytes);
    }
    assert_eq!(
        fs::read(old_directory.parent().unwrap().join("current")).unwrap(),
        b"generation-retained\n"
    );
    fs::write(root.join("json-result.txt"), format!("previous_revision_retained=true\nnew_revision=e6-d5\nbg_json_bytes={}\ngeneration_logical_bytes={logical}\nsource_equal=true\nreuse=true\n", json.len())).unwrap();
}

#[test]
#[ignore = "requires exact private regional ROMs, prior e2-d1 EUR dataset and new FFTA_STATE_TEST_MANAGED_ROOT"]
fn regional_background_dataset_lifecycle_and_revision_isolation() {
    let root = PathBuf::from(
        std::env::var_os("FFTA_STATE_TEST_MANAGED_ROOT").expect("new managed output root"),
    );
    assert!(!root.exists(), "preserve earlier managed evidence");
    let legacy = PathBuf::from(
        std::env::var_os("FFTA_STATE_TEST_LEGACY_DATASET").expect("existing EUR e2-d1 fixture"),
    );
    let legacy_manifest = fs::read(legacy.join(MANIFEST_FILE)).unwrap();
    let legacy_obj = fs::read(legacy.join(OBJ_DIMENSIONS_FILE)).unwrap();
    let old: Manifest = parse(&legacy_manifest, "legacy manifest").unwrap();
    assert_eq!(
        (old.schema, old.extractor_revision, old.decoder_revision),
        (1, 2, 1)
    );
    assert_eq!(old.source_rom_sha256, gba_core::ffta_rom::EUROPE_SHA256);
    assert_eq!(old.resources.len(), 1);
    assert_eq!(
        old.resources[0].sha256,
        gba_core::ffta_rom::digest(&legacy_obj)
    );
    let mut reports = Vec::new();
    for (env, profile) in [
        ("FFTA_TEST_ROM_USA", "usa"),
        ("FFTA_TEST_ROM_EUR", "europe"),
    ] {
        let rom_path = std::env::var(env).unwrap();
        let rom = fs::read(&rom_path).unwrap();
        let mut source = Extraction::from_rom(&rom).unwrap();
        let base = root.join(profile);
        let offline = base.join("offline");
        cmd(&[
            "extract".into(),
            rom_path.clone(),
            offline.to_str().unwrap().into(),
        ])
        .unwrap();
        let originals: Vec<_> = [
            MANIFEST_FILE,
            OBJ_DIMENSIONS_FILE,
            BG_DESCRIPTORS_FILE,
            BG_BASE_PALETTES_FILE,
        ]
        .iter()
        .map(|name| fs::read(offline.join(name)).unwrap())
        .collect();
        cmd(&[
            "extract".into(),
            rom_path.clone(),
            offline.to_str().unwrap().into(),
        ])
        .unwrap();
        cmd(&["verify".into(), rom_path, offline.to_str().unwrap().into()]).unwrap();
        assert_eq!(fs::read_dir(&offline).unwrap().count(), 4);
        let mut machine = crate::make_machine(rom, None);
        let before = machine.save_state().unwrap();
        attach_from_directory(&mut machine, &offline).unwrap();
        let original_arc = machine.bus.rom_resources().unwrap().clone();
        let original_identity = original_arc.dataset_sha256().to_owned();
        assert_eq!(before, machine.save_state().unwrap());
        let mut background = parse_background(&originals[2]).unwrap();
        background.entries[0].selectors[0] ^= 1;
        let edited = encode_background(&background).unwrap();
        let mut manifest: Manifest = parse(&originals[0], "manifest").unwrap();
        manifest.resources[1].sha256 = gba_core::ffta_rom::digest(&edited);
        fs::write(offline.join(BG_DESCRIPTORS_FILE), &edited).unwrap();
        fs::write(
            offline.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(attach_from_directory(&mut machine, &offline).is_err());
        assert!(Arc::ptr_eq(
            machine.bus.rom_resources().unwrap(),
            &original_arc
        ));
        assert_eq!(before, machine.save_state().unwrap());
        // Equal semantic data with a different captured file hash has a
        // different complete dataset identity, even though OBJ bytes match.
        let mut annotated = originals[2].clone();
        annotated.extend_from_slice(b"\n \n");
        manifest.resources[1].sha256 = gba_core::ffta_rom::digest(&annotated);
        fs::write(offline.join(BG_DESCRIPTORS_FILE), annotated).unwrap();
        fs::write(
            offline.join(MANIFEST_FILE),
            toml::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        attach_from_directory(&mut machine, &offline).unwrap();
        assert_ne!(
            machine.bus.rom_resources().unwrap().dataset_sha256(),
            original_identity
        );
        assert_eq!(
            machine.bus.rom_resources().unwrap().file_sha256(),
            original_arc.file_sha256()
        );
        assert_eq!(before, machine.save_state().unwrap());
        fs::remove_file(offline.join(BG_DESCRIPTORS_FILE)).unwrap();
        let current = machine.bus.rom_resources().unwrap().clone();
        assert!(attach_from_directory(&mut machine, &offline).is_err());
        assert!(Arc::ptr_eq(machine.bus.rom_resources().unwrap(), &current));
        for (name, bytes) in [
            MANIFEST_FILE,
            OBJ_DIMENSIONS_FILE,
            BG_DESCRIPTORS_FILE,
            BG_BASE_PALETTES_FILE,
        ]
        .iter()
        .zip(&originals)
        {
            fs::write(offline.join(name), bytes).unwrap();
        }
        let managed = base.join("resources");
        let old_revision = managed
            .join(&source.manifest.source_rom_sha256)
            .join("v1/e2-d1");
        if profile == "europe" {
            let old_directory = old_revision.join("generation-legacy");
            fs::create_dir_all(&old_directory).unwrap();
            fs::write(old_directory.join(MANIFEST_FILE), &legacy_manifest).unwrap();
            fs::write(old_directory.join(OBJ_DIMENSIONS_FILE), &legacy_obj).unwrap();
            fs::write(old_revision.join("current"), b"generation-legacy\n").unwrap();
            assert!(verify(&old_directory, &source).is_err());
        }
        let first = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(first.action, "created");
        let reused = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(reused.action, "reused");
        assert_eq!(first.directory, reused.directory);
        for (name, bytes) in [
            MANIFEST_FILE,
            OBJ_DIMENSIONS_FILE,
            BG_DESCRIPTORS_FILE,
            BG_BASE_PALETTES_FILE,
        ]
        .iter()
        .zip(&originals)
        {
            assert_eq!(fs::read(first.directory.join(name)).unwrap(), *bytes);
        }
        fs::remove_file(first.directory.join(BG_DESCRIPTORS_FILE)).unwrap();
        let repaired = preparation::prepare(&managed, &mut source, &mut |_| Ok(())).unwrap();
        assert_eq!(repaired.action, "repaired");
        assert_ne!(first.directory, repaired.directory);
        assert!(!first.directory.join(BG_DESCRIPTORS_FILE).exists());
        assert_eq!(
            fs::read(repaired.directory.join(BG_DESCRIPTORS_FILE)).unwrap(),
            originals[2]
        );
        if profile == "europe" {
            assert_eq!(
                fs::read(old_revision.join("current")).unwrap(),
                b"generation-legacy\n"
            );
            assert_eq!(
                fs::read(old_revision.join("generation-legacy/manifest.toml")).unwrap(),
                legacy_manifest
            );
            assert_eq!(
                fs::read(old_revision.join("generation-legacy/obj-dimensions.toml")).unwrap(),
                legacy_obj
            );
        }
        reports.push((
            profile.to_owned(),
            source.manifest.source_rom_sha256,
            gba_core::ffta_rom::digest(&originals[0]),
            gba_core::ffta_rom::digest(&originals[1]),
            gba_core::ffta_rom::digest(&originals[2]),
            original_identity,
        ));
        eprintln!("BG_MANAGED profile={profile} files=4 create_reuse_repair=true original_generation_retained=true activation_transactional=true whole_set_identity=true palette_consumer_checked=false");
    }
    assert_eq!(reports[0].3, reports[1].3);
    assert_eq!(reports[0].4, reports[1].4);
    assert_ne!(reports[0].2, reports[1].2);
    assert_ne!(reports[0].5, reports[1].5);
    assert_eq!(
        fs::read(legacy.join(MANIFEST_FILE)).unwrap(),
        legacy_manifest
    );
    assert_eq!(
        fs::read(legacy.join(OBJ_DIMENSIONS_FILE)).unwrap(),
        legacy_obj
    );
    #[derive(serde::Serialize)]
    struct Report {
        checks: Vec<(String, String, String, String, String, String)>,
    }
    fs::write(
        root.join("results.toml"),
        toml::to_string_pretty(&Report { checks: reports }).unwrap(),
    )
    .unwrap();
}
