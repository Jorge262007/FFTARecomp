//! Focused Wine indexed graphics integration; controlled probes, no ordinary fixture forcing.
use super::*;
use gba_core::{Bus, Machine, capi::RT_API};
use recomp_core::analyze::{Analysis, Block, View};
use std::{cell::Cell, collections::BTreeSet, sync::Arc};
fn root() -> PathBuf {
    PathBuf::from(std::env::var_os("FFTA_TABLE_GRAPHICS_RUNTIME_ROOT").unwrap())
}
fn dataset() -> PathBuf {
    root().join("offline dataset")
}
thread_local! {static FALLBACK:Cell<[u64;2]>=const{Cell::new([0;2])};static COMPILED:Cell<[u64;2]>=const{Cell::new([0;2])};}
fn counts(m: *mut std::ffi::c_void) -> [u64; 2] {
    unsafe {
        let r = (&*(m as *const Machine)).bus.rom_resources().unwrap();
        [
            r.table_graphics_stats().unwrap().source_bytes,
            r.indexed_graphics_stats().unwrap().source_bytes,
        ]
    }
}
fn account(before: [u64; 2], after: [u64; 2]) {
    COMPILED.with(|c| {
        let old = c.get();
        c.set(std::array::from_fn(|i| old[i] + after[i] - before[i]));
    });
}
extern "C" fn read8(m: *mut std::ffi::c_void, at: u32) -> u32 {
    let before = counts(m);
    let v = (RT_API.read8)(m, at);
    account(before, counts(m));
    v
}
extern "C" fn read16(m: *mut std::ffi::c_void, at: u32) -> u32 {
    let before = counts(m);
    let v = (RT_API.read16)(m, at);
    account(before, counts(m));
    v
}
extern "C" fn read32(m: *mut std::ffi::c_void, at: u32) -> u32 {
    let before = counts(m);
    let v = (RT_API.read32)(m, at);
    account(before, counts(m));
    v
}
// A generated fallback shell executes memory reads through the real interpreter.
// Account that callback too; direct read8/16/32 wrappers alone miss those bytes.
extern "C" fn interp_one(m: *mut std::ffi::c_void) -> u32 {
    let before = counts(m);
    let value = (RT_API.interp_one)(m);
    let after = counts(m);
    FALLBACK.with(|cell| {
        let old = cell.get();
        cell.set(std::array::from_fn(|j| old[j] + after[j] - before[j]));
    });
    account(before, after);
    value
}

fn spans() -> &'static Vec<(u32, u32)> {
    static V: std::sync::OnceLock<Vec<(u32, u32)>> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        let v: Vec<gba_core::rom_data::SourceRange> = serde_json::from_slice(
            &fs::read(root().parent().unwrap().join("source-spans.json")).unwrap(),
        )
        .unwrap();
        v.into_iter()
            .map(|s| (s.offset, s.offset + s.length))
            .collect()
    })
}
fn owned(at: u32) -> bool {
    spans().iter().any(|&(a, b)| (a..b).contains(&at))
}
fn prior_owned(at: u32) -> bool {
    static V: std::sync::OnceLock<Vec<gba_core::rom_data::SourceRange>> =
        std::sync::OnceLock::new();
    V.get_or_init(|| {
        serde_json::from_slice(
            &fs::read(root().parent().unwrap().join("prior-indexed-spans.json")).unwrap(),
        )
        .unwrap()
    })
    .iter()
    .any(|s| (s.offset..s.offset + s.length).contains(&at))
}
fn poison(m: &mut Machine) {
    for &(a, b) in spans() {
        m.bus.rom[a as usize..b as usize].fill(0xcd);
    }
}
fn poison_prior(m: &mut Machine) {
    m.bus.rom[0x71e57c..0x71e590].fill(0xab);
}
#[derive(Clone, Copy)]
struct Call {
    entry: u32,
    address: u32,
    mirror: u32,
    kind: u8,
    end: u32,
    index: u32,
}
fn family(c: Call) -> usize {
    usize::from(c.kind)
}
fn stop(c: Call) -> u32 {
    if c.kind == 1 {
        0x03007f00
    } else {
        0x08000000 + c.end
    }
}
fn setup(m: &mut Machine, c: Call) {
    m.bus.ffta_sprites.enabled = false;
    m.cpu.regs = std::array::from_fn(|i| 0x11000000 + i as u32);
    if c.kind == 0 {
        let op = u16::from_le_bytes(
            m.bus.rom[c.entry as usize..c.entry as usize + 2]
                .try_into()
                .unwrap(),
        );
        let width = match op & 0xf800 {
            0x8800 => 2,
            0x6800 => 4,
            _ => {
                assert_eq!(op & 0xf800, 0x7800);
                1
            }
        };
        m.cpu.regs[usize::from((op >> 3) & 7)] =
            c.mirror + c.address - u32::from((op >> 6) & 31) * width;
    } else if c.kind == 1 {
        m.cpu.regs[..4].copy_from_slice(&[0x02000100, 0x08000000 + c.address, 0, 1]);
    } else {
        m.bus.ewram[0x1f6e] = c.index as u8;
        m.cpu.regs[0] = 0x02001f6e;
        m.cpu.regs[1] = 0x08000000 + c.address;
    }
    m.cpu.regs[13] = 0x03007c00;
    m.cpu.regs[14] = 0x03007f01;
    m.cpu.regs[15] = 0x08000000 + c.entry;
    m.cpu.cpsr = 0xbf;
}
fn interpreted(m: &mut Machine, c: Call, visited: &mut BTreeSet<(u32, bool)>) -> [u64; 2] {
    let mut n = 0;
    let mut observed = [0; 2];
    while m.cpu.regs[15] != stop(c) {
        assert!(n < 100000);
        let pc = m.cpu.regs[15];
        assert!(m.cpu.thumb());
        let at = (pc - 0x08000000) as usize;
        let op = u16::from_le_bytes([m.bus.rom[at], m.bus.rom[at + 1]]);
        let width = match op & 0xf800 {
            0x7800 => 1,
            0x8800 => 2,
            0x6800 => 4,
            _ => 0,
        };
        let mut loads = Vec::new();
        if width != 0 {
            loads.push((
                m.cpu.regs[usize::from((op >> 3) & 7)] + u32::from((op >> 6) & 31) * width,
                width,
            ));
        } else if op & 0xf200 == 0x5000 && ((op >> 9) & 7) >= 3 {
            let kind = (op >> 9) & 7;
            let width = match kind {
                3 | 6 => 1,
                4 => 4,
                _ => 2,
            };
            loads.push((
                m.cpu.regs[usize::from((op >> 3) & 7)] + m.cpu.regs[usize::from((op >> 6) & 7)],
                width,
            ));
        } else if op & 0xf800 == 0xc800 {
            let base = m.cpu.regs[usize::from((op >> 8) & 7)];
            for i in 0..(op & 255).count_ones() {
                loads.push((base + i * 4, 4));
            }
        }
        for (address, width) in loads {
            if (8..=13).contains(&(address >> 24)) {
                let physical = (address & 0x1ffffff) & !(width - 1);
                for offset in physical..physical + width {
                    observed[0] += u64::from(owned(offset));
                    observed[1] += u64::from(prior_owned(offset));
                }
            }
        }
        visited.insert((pc, true));
        m.bus.frame_ready = false;
        m.step();
        n += 1;
    }
    observed
}
fn check_state(m: &Machine, expected: &[u8], label: &str) {
    let actual = m.save_state().unwrap();
    if actual != expected {
        fs::write(root().join(format!("{label}-actual.ffstate")), &actual).unwrap();
        fs::write(root().join(format!("{label}-expected.ffstate")), expected).unwrap();
        let differences: Vec<_> = actual
            .iter()
            .zip(expected)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i)
            .take(20)
            .collect();
        panic!(
            "fullstate-clock mismatch {label}, firstoffsets{differences:?}, lengths{}/{}",
            actual.len(),
            expected.len()
        );
    }
}

fn native_prefixes(rom: &[u8]) -> serde_json::Value {
    let mut calls = Vec::new();
    let blank = Call {
        entry: 0xd34aa,
        address: 0,
        mirror: 0x08000000,
        kind: 0,
        end: 0xd34ac,
        index: 0,
    };
    calls.push(Call {
        address: 0x436f74,
        mirror: 0x08000000,
        ..blank
    });
    calls.push(Call {
        address: 0x4425a4,
        mirror: 0x0a000000,
        ..blank
    });
    calls.push(Call {
        address: 0x45fda8,
        mirror: 0x0c000000,
        ..blank
    });
    calls.push(Call {
        address: 0x46a4d4,
        mirror: 0x08000000,
        ..blank
    });
    calls.push(Call {
        address: 0x47656c,
        mirror: 0x0a000000,
        ..blank
    });
    calls.push(Call {
        address: 0xb678d0,
        mirror: 0x0c000000,
        ..blank
    });
    calls.push(Call {
        address: 0x451840,
        mirror: 0x08000000,
        ..blank
    });
    calls.push(Call {
        address: 0x4635bc,
        mirror: 0x0a000000,
        ..blank
    });
    calls.push(Call {
        entry: 0xd08a2,
        address: 0x71e58f,
        end: 0xd08a4,
        ..blank
    });
    calls.push(Call {
        entry: 0x4a13c,
        address: 0x436f73,
        end: 0x4a13e,
        ..blank
    });
    calls.push(Call {
        entry: 0xd34aa,
        address: 0x71e5e0,
        end: 0xd34ac,
        ..blank
    });
    calls.push(Call {
        entry: 0xd34aa,
        address: 0x71e58f,
        end: 0xd34ac,
        ..blank
    });
    calls.push(Call {
        kind: 1,
        entry: 0x54a0,
        end: 0,
        address: 0x43cc54,
        ..blank
    });
    calls.push(Call {
        kind: 2,
        entry: 0x30660,
        end: 0x30668,
        address: 0x71e590,
        index: 4,
        ..blank
    });
    calls.push(Call {
        kind: 2,
        entry: 0x30660,
        end: 0x30668,
        address: 0x71e5a4,
        index: 4,
        ..blank
    });
    calls.push(Call {
        kind: 2,
        entry: 0x30660,
        end: 0x30668,
        address: 0x71e5b8,
        index: 4,
        ..blank
    });
    calls.push(Call {
        kind: 2,
        entry: 0x30660,
        end: 0x30668,
        address: 0x71e5cc,
        index: 4,
        ..blank
    });
    calls.push(Call {
        kind: 2,
        entry: 0x30660,
        end: 0x30668,
        address: 0x71e644,
        index: 4,
        ..blank
    });
    calls.push(Call {
        kind: 2,
        entry: 0x30660,
        end: 0x30668,
        address: 0x71e680,
        index: 4,
        ..blank
    });
    calls.push(Call {
        kind: 2,
        entry: 0x30660,
        end: 0x30668,
        address: 0x71e6bc,
        index: 4,
        ..blank
    });
    calls.push(Call {
        kind: 2,
        entry: 0x30660,
        end: 0x30668,
        address: 0x71e6d0,
        index: 4,
        ..blank
    });
    calls.push(Call {
        kind: 2,
        entry: 0x30660,
        end: 0x30668,
        address: 0x71e590,
        index: 255,
        ..blank
    });
    assert_eq!(calls.len(), 22);
    let mut raw = crate::make_machine(rom.to_vec(), None);
    let initial = raw.save_state().unwrap();
    let mut visited: [_; 3] = std::array::from_fn(|_| BTreeSet::new());
    let mut expected = Vec::new();
    let mut expected_counts = Vec::new();
    for &c in &calls {
        raw.load_state(&initial).unwrap();
        setup(&mut raw, c);
        expected_counts.push(interpreted(&mut raw, c, &mut visited[family(c)]));
        expected.push(raw.save_state().unwrap());
    }
    let view = View {
        rom,
        ewram: None,
        iwram: None,
        bios: None,
    };
    let compiled_instructions: usize = visited.iter().map(BTreeSet::len).sum();
    let mut libraries = Vec::new();
    let mut tables = Vec::new();
    for (group, visited) in visited.iter().enumerate() {
        assert!(!visited.is_empty());
        assert!(
            calls
                .iter()
                .filter(|c| family(**c) == group)
                .all(|&c| !visited.iter().any(|&(pc, _)| pc == stop(c)))
        );
        let blocks = visited
            .iter()
            .map(|&(addr, thumb)| {
                let mut instrs = vec![if thumb {
                    armv4t::decode_thumb(view.read16(addr), addr)
                } else {
                    armv4t::decode_arm(view.read32(addr), addr)
                }];
                if thumb {
                    let next = armv4t::decode_thumb(view.read16(addr + 2), addr + 2);
                    if armv4t::fuse_thumb_bl(&instrs[0], &next).is_some() {
                        instrs.push(next);
                    }
                }
                Block {
                    addr,
                    thumb,
                    instrs,
                }
            })
            .collect();
        let object = root().join(format!("table-graphics-family{group}.o"));
        {
            recomp_core::tcc_memory::build(
                &Analysis { blocks },
                &view,
                &recomp_core::tcc_memory::library_path().unwrap(),
                &object,
                &mut |_| {},
            )
            .unwrap();
        }
        let (library, table) = crate::load_native(&object).unwrap();
        libraries.push(library);
        tables.push(table);
    }
    let mut typed = crate::make_machine(rom.to_vec(), None);
    let mut native = crate::make_machine(rom.to_vec(), None);
    for m in [&mut typed, &mut native] {
        attach_from_directory(m, &dataset()).unwrap();
    }
    let api = gba_core::capi::RtApi {
        interp_one,
        read8,
        read16,
        read32,
        ..RT_API
    };
    COMPILED.with(|c| c.set([0; 2]));
    FALLBACK.with(|c| c.set([0; 2]));
    let mut dispatch = 0;
    let mut observations = Vec::new();
    for (i, &c) in calls.iter().enumerate() {
        for m in [&mut typed, &mut native] {
            m.load_state(&initial).unwrap();
            setup(m, c);
            poison(m);
            poison_prior(m);
        }
        let typed_before = counts(&mut typed as *mut Machine as *mut std::ffi::c_void);
        assert_eq!(
            interpreted(&mut typed, c, &mut BTreeSet::new()),
            expected_counts[i]
        );
        let typed_after = counts(&mut typed as *mut Machine as *mut std::ffi::c_void);
        let typed_delta = std::array::from_fn::<_, 2, _>(|j| typed_after[j] - typed_before[j]);
        let native_before = counts(&mut native as *mut Machine as *mut std::ffi::c_void);
        let before = COMPILED.with(Cell::get);
        while native.cpu.regs[15] != stop(c) {
            assert!(dispatch < 200000);
            native.bus.frame_ready = false;
            let actual_before = counts(&mut native as *mut Machine as *mut std::ffi::c_void);
            let api_before = COMPILED.with(Cell::get);
            tables[family(c)]
                .get(native.cpu.regs[15] | u32::from(native.cpu.thumb()))
                .unwrap_or_else(|| panic!("uncompiled case{i} PC{:08x}", native.cpu.regs[15]))(
                &api,
                &mut native as *mut Machine as *mut std::ffi::c_void,
            );
            let actual_after = counts(&mut native as *mut Machine as *mut std::ffi::c_void);
            let api_after = COMPILED.with(Cell::get);
            assert_eq!(
                std::array::from_fn::<_, 2, _>(|j| actual_after[j] - actual_before[j]),
                std::array::from_fn::<_, 2, _>(|j| api_after[j] - api_before[j]),
                "all callbacks case{i}"
            );
            dispatch += 1;
        }
        let after = COMPILED.with(Cell::get);
        let native_after = counts(&mut native as *mut Machine as *mut std::ffi::c_void);
        let native_delta = std::array::from_fn::<_, 2, _>(|j| native_after[j] - native_before[j]);
        let api_delta = std::array::from_fn::<_, 2, _>(|j| after[j] - before[j]);
        observations.push(serde_json::json!({"case":i,"entry":c.entry,"kind":c.kind,"expected":expected_counts[i],"typed":typed_delta,"native":native_delta,"callbacks":api_delta}));
        fs::write(
            root().join("native-observations.json"),
            serde_json::to_vec_pretty(&observations).unwrap(),
        )
        .unwrap();
        assert_eq!(typed_delta, expected_counts[i], "typed case{i}");
        assert_eq!(native_delta, expected_counts[i], "native case{i}");
        assert_eq!(api_delta, expected_counts[i], "callbacks case{i}");
        for (kind, m) in [("typed", &mut typed), ("native", &mut native)] {
            m.bus.rom.copy_from_slice(rom);
            check_state(m, &expected[i], &format!("{kind}-case{i}"));
        }
    }
    let observed = COMPILED.with(Cell::get);
    assert_eq!(
        observed,
        expected_counts
            .iter()
            .fold([0; 2], |a, b| [a[0] + b[0], a[1] + b[1]])
    );
    assert!(observed[0] > 0 && observed[1] > 0);
    assert_eq!(libraries.len(), 3);
    serde_json::json!({"full_state_clock_cases":calls.len(),"single_load_cases":12,"original_newbank_decoder_calls":1,"raw_selector_prefix_cases":9,"compiled_new_source_bytes":observed[0],"compiled_prior_indexed_bytes":observed[1],"fallback_source_bytes":FALLBACK.with(Cell::get),"compiled_instructions":compiled_instructions,"native_case_families":3,"each_family_excludes_stop_boundaries":true,"dispatch_coverage_only":dispatch,"all53667_raw_newsource_and_prior20_poisoned_after_Load_setup":true,"observations":observations})
}
fn bus_lifecycle(rom: &[u8]) -> serde_json::Value {
    let mut m = crate::make_machine(rom.to_vec(), None);
    attach_from_directory(&mut m, &dataset()).unwrap();
    let owner = m.bus.rom_resources().unwrap().clone();
    let initial = m.save_state().unwrap();
    poison(&mut m);
    for &(a, b) in spans() {
        for at in a..b {
            assert_eq!(m.bus.read8(0x0c000000 + at), rom[at as usize]);
        }
    }
    let s = owner.table_graphics_stats().unwrap();
    assert_eq!(
        [
            s.bus_reads,
            s.source_bytes,
            s.decoded_literal_bytes,
            s.selector_bytes,
            s.banks_used,
            s.groups_used
        ],
        [53667, 53667, 24328, 160, 40, 8]
    );
    let sources = owner.ui_graphics_used_sources();
    let banks: serde_json::Value =
        serde_json::from_slice(&fs::read(root().parent().unwrap().join("new-banks.json")).unwrap())
            .unwrap();
    for bank in banks.as_array().unwrap() {
        let at = bank["source_offset"].as_u64().unwrap() as u32;
        let n = bank["source_bytes"].as_u64().unwrap();
        assert!(sources.contains(&(at, n)));
    }
    assert_eq!(
        owner.ui_graphics_stats().unwrap().localized_source_bytes,
        0,
        "old non-English subset semantics retained"
    );
    let pointer = &mut m as *mut Machine as *mut std::ffi::c_void;
    for &(a, b) in spans() {
        for edge in [a, b] {
            for delta in [-3, -1, 0, 1] {
                let at = (edge as i32 + delta) as usize;
                assert_eq!(
                    (RT_API.read16)(pointer, 0x0a000000 + at as u32),
                    u32::from(u16::from_le_bytes(rom[at..at + 2].try_into().unwrap()))
                );
                assert_eq!(
                    (RT_API.read32)(pointer, 0x08000000 + at as u32),
                    u32::from_le_bytes(rom[at..at + 4].try_into().unwrap())
                );
            }
        }
    }
    poison_prior(&mut m);
    assert_eq!(
        (RT_API.read32)(pointer, 0x0871e58f),
        u32::from_le_bytes(rom[0x71e58f..0x71e593].try_into().unwrap())
    );
    assert!(owner.indexed_graphics_stats().unwrap().source_bytes > 0);
    m.bus.rom[0x71e5e0..0x71e5e4].fill(0xef);
    let before = owner.table_graphics_stats().unwrap();
    assert_eq!(
        (RT_API.read32)(pointer, 0x0871e5df),
        u32::from_le_bytes(rom[0x71e5df..0x71e5e3].try_into().unwrap())
    );
    assert_eq!(m.bus.read8(0x0871e5e0), rom[0x71e5e0]);
    assert_eq!(
        owner.table_graphics_stats().unwrap().source_bytes,
        before.source_bytes + 1
    );
    let before = owner.table_graphics_stats().unwrap();
    m.bus.rom[0x43bc28] = 0xab;
    assert_eq!(m.bus.read8(0x0843bc28), rom[0x43bc28]);
    assert_eq!(owner.table_graphics_stats().unwrap(), before);
    m.bus.rom.copy_from_slice(rom);
    let unavailable = root().join("unavailable dataset");
    fs::rename(dataset(), &unavailable).unwrap();
    assert!(attach_from_directory(&mut m, &dataset()).is_err());
    assert!(Arc::ptr_eq(m.bus.rom_resources().unwrap(), &owner));
    m.load_state(&initial).unwrap();
    let mut slots = crate::save_state::Slots::new(&root().join("private.sav"), rom);
    slots.save(&m, 1, &vec![[0; 4]; 240 * 160], false).unwrap();
    m.cpu.regs[4] = 123;
    let changed = m.save_state().unwrap();
    slots.load(&mut m, 1).unwrap();
    assert_eq!(m.save_state().unwrap(), initial);
    slots.undo(&mut m).unwrap();
    assert_eq!(m.save_state().unwrap(), changed);
    assert!(Arc::ptr_eq(m.bus.rom_resources().unwrap(), &owner));
    poison(&mut m);
    assert_eq!(m.bus.read8(0x08436f74), rom[0x436f74]);
    m.bus.rom.copy_from_slice(rom);
    fs::rename(&unavailable, dataset()).unwrap();
    for kind in 0..3 {
        let mut a = crate::make_machine(rom.to_vec(), None);
        let mut b = crate::make_machine(rom.to_vec(), None);
        attach_from_directory(&mut b, &dataset()).unwrap();
        for m in [&mut a, &mut b] {
            if kind == 0 {
                m.bus.write32(0x040000d4, 0x0871e580);
                m.bus.write32(0x040000d8, 0x02010000);
                m.bus.write16(0x040000dc, 24);
            } else {
                m.cpu.regs[..3].copy_from_slice(&[
                    0x0a71e580,
                    0x02010000,
                    if kind == 2 { 16 } else { 24 },
                ]);
                m.cpu.regs[15] = 0x02000000;
                m.cpu.cpsr = 0xbf;
                m.bus
                    .write16(0x02000000, if kind == 2 { 0xdf0c } else { 0xdf0b });
            }
        }
        poison(&mut b);
        poison_prior(&mut b);
        for m in [&mut a, &mut b] {
            if kind == 0 {
                m.bus.write16(0x040000de, 0x8000);
            } else {
                m.step();
            }
        }
        b.bus.rom.copy_from_slice(rom);
        assert_eq!(
            b.save_state().unwrap(),
            a.save_state().unwrap(),
            "mixedtabletransfer{kind} fullstate-clock"
        );
        let owner = b.bus.rom_resources().unwrap();
        assert!(owner.table_graphics_stats().unwrap().source_bytes >= 32);
        assert!(owner.indexed_graphics_stats().unwrap().source_bytes >= 16);
    }
    serde_json::json!({"all53667_newsource_bytes":true,"all40banks8groups160selector24328literalbytes":true,"all48canonical_spanedges_widths_mirrors_priorindexed_rawandoldUI":true,"unavailable_files_Load_slot_Undo_owner_retained":true,"DMA_CpuSet_CpuFastSet_fullstate_clock":3,"DMA_CpuSet_halfwords":24,"CpuFastSet_words":16})
}
#[test]
#[ignore = "private EUR table_graphics managed/native integration; Wine only, Linux/USA pending"]
fn european_table_graphics_managed_native_and_lifecycle() {
    let root = root();
    assert!(!root.exists());
    fs::create_dir_all(&root).unwrap();
    let rom = fs::read(std::env::var_os("FFTA_STATE_TEST_ROM").unwrap()).unwrap();
    let old = PathBuf::from(std::env::var_os("FFTA_STATE_TEST_DATASET").unwrap());
    let prior: Manifest =
        toml::from_str(&fs::read_to_string(old.join(MANIFEST_FILE)).unwrap()).unwrap();
    assert_eq!(
        (
            prior.extractor_revision,
            prior.decoder_revision,
            prior.resources.len()
        ),
        (91, 90, 83)
    );
    let mut source = Extraction::from_rom(&rom).unwrap();
    assert_eq!(
        (
            source.manifest.extractor_revision,
            source.manifest.decoder_revision,
            source.manifest.resources.len()
        ),
        (106, 105, 96)
    );
    extract(&dataset(), &mut source).unwrap();
    assert_eq!(fs::read_dir(dataset()).unwrap().count(), 94);
    assert_eq!(&source.manifest.coverage[..prior.coverage.len()], prior.coverage);
    let slot = source
        .manifest
        .resources
        .iter()
        .position(|r| r.id == UI_CUSTOM_GRAPHICS_ID)
        .unwrap();
    assert_eq!(slot, 15);
    for (new, prior_entry) in source.manifest.resources.iter().zip(&prior.resources) {
        assert_eq!(new.id, prior_entry.id);
        if new.id != UI_CUSTOM_GRAPHICS_ID {
            assert_eq!(new, prior_entry);
            assert_eq!(
                fs::read(dataset().join(&new.path)).unwrap(),
                fs::read(old.join(&prior_entry.path)).unwrap()
            );
        }
    }
    let entry = &source.manifest.resources[slot];
    assert_eq!(entry.sources.len(), 111);
    for source in &prior.resources[slot].sources {
        assert!(entry.sources.contains(source));
    }
    let bytes = fs::read(dataset().join(UI_CUSTOM_GRAPHICS_FILE)).unwrap();
    let manifest = fs::read(dataset().join(MANIFEST_FILE)).unwrap();
    let current = parse_ui_graphics(&bytes).unwrap();
    let previous =
        parse_ui_graphics(&fs::read(old.join(UI_CUSTOM_GRAPHICS_FILE)).unwrap()).unwrap();
    let expected: UiGraphics = serde_json::from_slice(
        &fs::read(root.parent().unwrap().join("independent-ui-graphics.json")).unwrap(),
    )
    .unwrap();
    let mut without_alias = current.clone();
    let alias = without_alias.selectors.pop().unwrap();
    assert_eq!(alias.id, "clan-table-alias");
    assert_eq!(without_alias, expected);
    assert_eq!(current.banks.len(), 56);
    assert_eq!(current.selectors.len(), 11);
    assert_eq!(current.selectors[..2], previous.selectors);
    assert_eq!(current.shared_obj_banks, previous.shared_obj_banks);
    for bank in &previous.banks {
        assert!(current.banks.contains(bank));
    }
    assert_eq!(current, source.ui_graphics.as_ref().unwrap().decoded);
    let expected_sources: Vec<gba_core::rom_data::SourceRange> = serde_json::from_slice(
        &fs::read(root.parent().unwrap().join("expected-ui-sources.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(&entry.sources[..expected_sources.len()], expected_sources);
    let interrupted = root.join("interrupted");
    fs::create_dir(&interrupted).unwrap();
    let mut reached = false;
    assert!(
        write_dataset(&interrupted, &mut source, &mut |p, b| {
            if p.file_name().unwrap() == UI_CUSTOM_GRAPHICS_FILE {
                reached = true;
                Err("injected table_graphics file failure".into())
            } else {
                write_synced(p, b)
            }
        })
        .is_err()
    );
    assert!(reached);
    assert!(!interrupted.join(MANIFEST_FILE).exists());
    assert!(verify(&interrupted, &source).is_err());
    eprintln!(
        "ROM_DATA_TABLE_GRAPHICS_STAGE publication84/other82exact/old16UIexact/interruption PASS"
    );
    let mut m = crate::make_machine(rom.clone(), None);
    attach_from_directory(&mut m, &dataset()).unwrap();
    let owner = m.bus.rom_resources().unwrap().clone();
    let state = m.save_state().unwrap();
    for case in 0..14 {
        let mut bad = source.manifest.clone();
        let mut data = bytes.clone();
        match case {
            0..=4 => {
                let mut v = parse_ui_graphics(&data).unwrap();
                match case {
                    0 => v.banks[0].words[0] ^= 1,
                    1 => v.selectors[2].fallback_variant = Some(0),
                    2 => {
                        v.banks.pop();
                    }
                    3 => {
                        v.selectors.pop();
                    }
                    _ => v.selectors[2].pointers[0].source_offset += 1,
                };
                data = encode_ui_graphics(&v).unwrap();
            }
            5 => bad.resources[slot].sources[0].offset += 1,
            6 => bad.resources[slot].sources[0].length -= 1,
            7 => bad.resources[slot].sources[0].sha256 = "00".repeat(32),
            8 => bad.resources[slot].sources[0].role = "other".into(),
            9 => {
                let mut v: serde_json::Value = serde_json::from_slice(&data).unwrap();
                v["unexpected"] = true.into();
                data = serde_json::to_vec(&v).unwrap();
            }
            10 => bad.resources[slot]
                .dependencies
                .push(ENCOUNTER_HEADERS_ID.into()),
            11 => bad.resources[slot]
                .sources
                .push(gba_core::rom_data::SourceRange {
                    offset: 0x71e57a,
                    length: 2,
                    sha256: gba_core::ffta_rom::digest(&rom[0x71e57a..0x71e57c]),
                    role: "counterfeit-shared".into(),
                }),
            12 => bad.resources[slot].format = "counterfeit".into(),
            _ => {
                bad.coverage.pop();
            }
        }
        bad.resources[slot].sha256 = gba_core::ffta_rom::digest(&data);
        fs::write(dataset().join(UI_CUSTOM_GRAPHICS_FILE), data).unwrap();
        fs::write(
            dataset().join(MANIFEST_FILE),
            toml::to_string_pretty(&bad).unwrap(),
        )
        .unwrap();
        assert!(
            attach_from_directory(&mut m, &dataset()).is_err(),
            "table_graphics attach guard{case}"
        );
        assert!(Arc::ptr_eq(m.bus.rom_resources().unwrap(), &owner));
        assert_eq!(m.save_state().unwrap(), state);
    }
    fs::write(dataset().join(UI_CUSTOM_GRAPHICS_FILE), &bytes).unwrap();
    fs::write(dataset().join(MANIFEST_FILE), &manifest).unwrap();
    let files = read_dataset(&dataset()).unwrap();
    let saved = source.ui_graphics.take();
    assert!(source.validate_files(files.files()).is_err());
    source.ui_graphics = saved;
    let mut missing = files.files();
    missing.ui_graphics = None;
    assert!(source.validate_files(missing).is_err());
    eprintln!("ROM_DATA_TABLE_GRAPHICS_STAGE attach14/fullset2/failedownerfullstate PASS");
    let native = native_prefixes(&rom);
    eprintln!("ROM_DATA_TABLE_GRAPHICS_STAGE native22/fullstate-clock/source53667poisoned PASS");
    let bus = bus_lifecycle(&rom);
    let result = serde_json::json!({"status":"PASS","revision":[1,92,91],"files":84,"resources":83,"other82_records_files_exact":true,"old16banks_twoinline_sharedOBJ_exact":true,"additional_source_bytes":53667,"additional_runtime_spans":80,"attach_guards":14,"fullset_guards":2,"newfile_interruption":true,"native":native,"bus_lifecycle":bus,"dataset_sha256":owner.dataset_sha256(),"manifest_sha256":gba_core::ffta_rom::digest(&manifest),"ui_graphics_JSON_sha256":gba_core::ffta_rom::digest(&bytes),"actualCLI":"pending","ordinary_positive":"pending","Linux":"pending","USA":"pending","nativeWindows":"pending","new32_9":"pending","performance_measurements":false});
    fs::write(
        root.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    eprintln!(
        "ROM_DATA_TABLE_GRAPHICS_RUNTIME e92-d91/files84/resources83/attach14/fullset2/native22/DMA3 PASS"
    );
}
