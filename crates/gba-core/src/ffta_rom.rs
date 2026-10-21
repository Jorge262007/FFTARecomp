//! Patch eligibility data, shared with the installer. No game code is embedded.
use sha2::{Digest, Sha256};

pub const SHA256: &str = "43fc8204c6dceee58828aebc7af0c72eb807e99f35ad641c8bb0a4fa8b6edc19";
pub const EUROPE_SHA256: &str = "8fab1e390cb9d5e695bfdb86e81246892e1aa8c0cb4014db8235ff76538ea632";
const MAP_PRODUCERS_START: usize = 0x1af08;
const MAP_PRODUCERS_END: usize = 0x1b5d0;
const USA_MAP_PRODUCERS_SHA256: &str =
    "ce26cfe5654123ddf633d32314054c816d6a70315473dcc86928d5cef7237f2c";
const EUROPE_MAP_PRODUCERS_SHA256: &str =
    "1a4aff2ffa798cf2b2e329f1945c8817cffe8a5458189febf3a5ef634570e829";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    Usa,
    Europe,
}

pub fn version(rom: &[u8]) -> Option<Version> {
    version_for_sha256(&digest(rom))
}
fn version_for_sha256(hash: &str) -> Option<Version> {
    match hash {
        SHA256 => Some(Version::Usa),
        EUROPE_SHA256 => Some(Version::Europe),
        _ => None,
    }
}
/// Additional immutable ROM entries verified with full-state/audio replay.
/// These are addresses only, not guest bytes or RAM snapshot translations.
pub fn native_rom_entries(hash: &str) -> &'static [u32] {
    match hash {
        EUROPE_SHA256 => &[
            0x0801_0ec5,
            0x0801_0f41,
            0x0802_2eb9,
            0x0802_2eed,
            0x0802_2f01,
            0x0802_2f19,
            0x0802_2f47,
            0x0802_3125,
            0x0802_314f,
            0x0802_315d,
            0x0802_3169,
            0x0802_3175,
            0x0809_21cf,
            0x0809_21d5,
            0x0809_2485,
            0x0809_248d,
            0x0809_2493,
            0x0814_5c19,
        ],
        _ => &[],
    }
}

/// The map expansion hooks inspect runtime DMA descriptors and verify every
/// reconstructed tile against the transfer. This ROM-region digest additionally
/// pins the producer code independently for each exact regional image.
pub fn map_producers_verified(rom: &[u8]) -> bool {
    let Some(version) = version(rom) else {
        return false;
    };
    let Some(code) = rom.get(MAP_PRODUCERS_START..MAP_PRODUCERS_END) else {
        return false;
    };
    let expected = match version {
        Version::Usa => USA_MAP_PRODUCERS_SHA256,
        Version::Europe => EUROPE_MAP_PRODUCERS_SHA256,
    };
    digest(code) == expected
}
pub const HOOK_TAG: u64 = 0x4646544153505231;
pub const RAM_SPRITE_START: u32 = 0x030065a8;
pub const RAM_SPRITE_END: u32 = 0x03006888;
pub const RAM_SOURCE_START: usize = 0xa3915c;
pub const RAM_SOURCE_END: usize = 0xa39458;
pub const VIS_START: u32 = 0x030062ec;
pub const VIS_RETURN: u32 = 0x0300640e;
pub const VIS_END: u32 = 0x03006414;
pub const VIS_SOURCE_START: usize = 0xa38ea0;
pub const VIS_SOURCE_END: usize = 0xa38fc8;
const USA_VIS_SHA256: &str = "1e988998a21067596046e2b303da1ad9b6924c18146f396ce0bf5f6e038433c7";
const USA_DRAW_A: &str = "6be04b5192e1a3e6776819aa834b155911206bc53444ba3bde476d7b2b14063f";
const USA_DRAW_B: &str = "d883a53ce5bc4c3b4dd08bf37fddef575da1c3cae2c6d30886fbbbc9976eaac6";
const USA_RAM_CODE: &str = "ce5416f21676ea64415b11aa832634fddd97e304b2d6ae29bb0a5e42828a1afc";
const EUR_DRAW_A: &str = "991e33b72f908114b349f73626f506c67f3e055981c5e85131333c1ff5f1914e";
const EUR_DRAW_B: &str = "b56470154f1dcb540bb03e60427d09569722de720c30ba50c5cc52af2e0e7702";
const EUR_RAM_CODE: &str = "ce5416f21676ea64415b11aa832634fddd97e304b2d6ae29bb0a5e42828a1afc";
const EUR_VIS_CODE: &str = "1e988998a21067596046e2b303da1ad9b6924c18146f396ce0bf5f6e038433c7";
const EUR_SIZE_TABLE: &str = "6ff886b23cde2726a4ceb9cba8c5569bbb4dc6a1185769547f78dfbcb7e2d991";

/// Exact-build addresses for host-side sprite expansion. Both ROM identity and
/// each copied/native function body are verified before these hooks can run.
#[derive(Clone, Copy, Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SpriteProfile {
    pub draw_a: u32,
    pub draw_b: u32,
    pub draw_a_end: u32,
    pub draw_b_end: u32,
    pub ram_start: u32,
    pub ram_end: u32,
    pub ram_source: usize,
    pub ram_source_end: usize,
    pub vis_start: u32,
    pub vis_return: u32,
    pub vis_end: u32,
    pub vis_source: usize,
    pub vis_source_end: usize,
    pub size_table: usize,
    pub stats_list: u32,
}
const USA_PROFILE: SpriteProfile = SpriteProfile {
    draw_a: 0x080014c8,
    draw_b: 0x080016d4,
    draw_a_end: 0x080016d4,
    draw_b_end: 0x080018a0,
    ram_start: 0x030065a8,
    ram_end: 0x03006888,
    ram_source: 0xa3915c,
    ram_source_end: 0xa39458,
    vis_start: 0x030062ec,
    vis_return: 0x0300640e,
    vis_end: 0x03006414,
    vis_source: 0xa38ea0,
    vis_source_end: 0xa38fc8,
    size_table: 0xa398dc,
    stats_list: 0x083916fa,
};
const EUROPE_PROFILE: SpriteProfile = SpriteProfile {
    draw_a: 0x08001500,
    draw_b: 0x0800170c,
    draw_a_end: 0x0800170c,
    draw_b_end: 0x080018d8,
    ram_start: 0x030065a8,
    ram_end: 0x03006888,
    ram_source: 0xd96f68,
    ram_source_end: 0xd97264,
    vis_start: 0x030062ec,
    vis_return: 0x0300640e,
    vis_end: 0x03006414,
    vis_source: 0xd96cac,
    vis_source_end: 0xd96dd4,
    size_table: 0x14ced8,
    stats_list: 0x08395102,
};
pub fn sprite_profile(rom: &[u8]) -> Option<SpriteProfile> {
    let (p, a, b, r, v, table) = match version(rom)? {
        Version::Usa => (
            &USA_PROFILE,
            USA_DRAW_A,
            USA_DRAW_B,
            USA_RAM_CODE,
            USA_VIS_SHA256,
            "",
        ),
        Version::Europe => (
            &EUROPE_PROFILE,
            EUR_DRAW_A,
            EUR_DRAW_B,
            EUR_RAM_CODE,
            EUR_VIS_CODE,
            EUR_SIZE_TABLE,
        ),
    };
    let check = |start: usize, end: usize, expected: &str| {
        rom.get(start..end)
            .is_some_and(|bytes| digest(bytes) == expected)
    };
    let table_ok = if table.is_empty() {
        true
    } else {
        rom.get(p.size_table..p.size_table + 48)
            .is_some_and(|bytes| digest(bytes) == table)
    };
    (check(
        p.draw_a as usize - 0x08000000,
        p.draw_a_end as usize - 0x08000000,
        a,
    ) && check(
        p.draw_b as usize - 0x08000000,
        p.draw_b_end as usize - 0x08000000,
        b,
    ) && check(p.ram_source, p.ram_source_end, r)
        && check(p.vis_source, p.vis_source_end, v)
        && table_ok)
        .then_some(*p)
}
pub fn verify_visibility(rom: &[u8]) -> bool {
    sprite_profile(rom).is_some()
}
pub const CHECKS: &[(usize, &[u8])] = &[
    (0x14c8, &[0xf0, 0xb5]),
    (0x159c, &[0x83, 0xe0]),
    (0x15aa, &[0x7c, 0xdc]),
    (0x15fe, &[0x52, 0xdb]),
    (0x160c, &[0x4b, 0xdc]),
    (0x169a, &[0x08, 0x35]),
    (0x16d4, &[0xf0, 0xb5]),
    (0x1778, &[0x70, 0xdb]),
    (0x1788, &[0x68, 0xdc]),
    (0x17c2, &[0x4b, 0xdb]),
    (0x17d2, &[0x43, 0xdc]),
    (0x1856, &[0x08, 0x33]),
    (RAM_SOURCE_START, &[0xf0, 0xb5]),
    (0xa3930c, &[0x18, 0x80]),
];
const FUNCTIONS: &[(usize, usize, &str)] = &[
    (
        0x14c8,
        0x16d4,
        "6be04b5192e1a3e6776819aa834b155911206bc53444ba3bde476d7b2b14063f",
    ),
    (
        0x16d4,
        0x18a0,
        "d883a53ce5bc4c3b4dd08bf37fddef575da1c3cae2c6d30886fbbbc9976eaac6",
    ),
    (
        RAM_SOURCE_START,
        RAM_SOURCE_END,
        "ce5416f21676ea64415b11aa832634fddd97e304b2d6ae29bb0a5e42828a1afc",
    ),
];
pub fn digest(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn verify_hooks(rom: &[u8]) -> bool {
    CHECKS
        .iter()
        .all(|(off, expected)| rom.get(*off..off + expected.len()) == Some(*expected))
        && FUNCTIONS.iter().all(|(start, end, hash)| {
            rom.get(*start..*end)
                .is_some_and(|bytes| digest(bytes) == *hash)
        })
}
pub fn eligible(rom: &[u8]) -> bool {
    version(rom).is_some() && sprite_profile(rom).is_some() && map_producers_verified(rom)
}
pub fn sprite_pc(pc: u32) -> bool {
    (0x080014c8..0x080016c8).contains(&pc)
        || (0x080016d4..0x08001882).contains(&pc)
        || (0x08001500..0x080016f4).contains(&pc)
        || (0x0800170c..0x080018ba).contains(&pc)
        || (RAM_SPRITE_START..RAM_SPRITE_END).contains(&pc)
        || (VIS_START..VIS_END).contains(&pc)
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    #[test]
    fn native_entries_are_exact_image_rom_hints() {
        let entries = native_rom_entries(EUROPE_SHA256);
        assert!(!entries.is_empty());
        assert!(entries.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(entries.iter().all(|key| key & 1 == 1
            && (0x0800_0000..0x0900_0000).contains(&(key & !1))));
        assert!(native_rom_entries(SHA256).is_empty());
        assert!(native_rom_entries(&"0".repeat(64)).is_empty());
    }

    #[test]
    fn exact_rom_hashes_select_distinct_profiles() {
        assert_eq!(version_for_sha256(SHA256), Some(Version::Usa));
        assert_eq!(version_for_sha256(EUROPE_SHA256), Some(Version::Europe));
        assert_eq!(
            version_for_sha256("0000000000000000000000000000000000000000000000000000000000000000"),
            None
        );
    }
    #[test]
    #[ignore = "requires the user's USA and Europe ROMs; no ROM is bundled"]
    fn regional_sprite_profiles_match_exact_user_roms() {
        let usa = std::fs::read(std::env::var("FFTA_TEST_ROM_USA").unwrap()).unwrap();
        let eur = std::fs::read(std::env::var("FFTA_TEST_ROM_EUR").unwrap()).unwrap();
        assert_eq!(sprite_profile(&usa).unwrap().draw_a, 0x080014c8);
        assert_eq!(sprite_profile(&eur).unwrap().draw_a, 0x08001500);
        assert!(map_producers_verified(&usa));
        assert!(map_producers_verified(&eur));
        assert!(eligible(&usa));
        assert!(eligible(&eur));
        let mut changed = eur.clone();
        changed[0x1500] ^= 1;
        assert!(sprite_profile(&changed).is_none());
    }
}
