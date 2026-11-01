//! Opt-in developer tools. No ROM patch or save-file access.
use crate::{ffta_rom, MemMap};

#[derive(Default)]
pub struct State { pending: bool }

/// Let the original result screen finish before ordinary scene cleanup.
pub fn tick(mem: &mut MemMap) {
    if !mem.ffta_debug.pending { return; }
    let callback=u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().unwrap());
    if callback!=0x0802_5115 { mem.ffta_debug.pending=false; return; }
    let flags=u16::from_le_bytes(mem.ewram[0xf424..0xf426].try_into().unwrap());
    if flags & 8 == 0 {
        mem.ewram[0xf390]=8;
        mem.ffta_debug.pending=false;
        eprintln!("FFTA_DEBUG_WIN frame={} result_screen_finished=true cleanup_requested=true",mem.frames);
    }
}

/// Request the original EUR battle-result pipeline, rather than killing units
/// or jumping the CPU out of a running function. Only invoked on a host key edge.
pub fn request_victory(mem: &mut MemMap) -> Result<(), &'static str> {
    if std::env::var("FFTA_DEBUG").as_deref() != Ok("1") {
        return Err("debug disabled (FFTA_DEBUG=1 required)");
    }
    if ffta_rom::version(&mem.rom) != Some(ffta_rom::Version::Europe)
        || mem.rom.get(0x125d04..0x125d0c)
            != Some(&[0x20, 0x1c, 0x96, 0x30, 0x01, 0x21, 0x01, 0x80][..])
        || mem.rom.get(0x960d2..0x960da)
            != Some(&[0x40,0x46,0xdc,0x30,0x01,0x88,0x07,0x1c][..])
        || mem.rom.get(0x96298..0x9629c) != Some(&0x0809_87f6u32.to_le_bytes()[..])
    {
        return Err("unsupported ROM or battle-result code");
    }
    request_verified(mem)
}

fn request_verified(mem: &mut MemMap) -> Result<(), &'static str> {
    // The scheduler installs this callback only while a battle scene exists.
    let callback = u32::from_le_bytes(mem.iwram[0x2804..0x2808].try_into().unwrap());
    let flags = u16::from_le_bytes(mem.ewram[0xf424..0xf426].try_into().unwrap());
    let result = u16::from_le_bytes(mem.ewram[0xf426..0xf428].try_into().unwrap());
    if callback != 0x0802_5115 || flags & 4 == 0 || mem.ewram[0xf390] == 8 {
        return Err("no active battle");
    }
    if result != 0 || flags & 8 != 0 {
        return Err("battle result already pending");
    }
    let root=u32::from_le_bytes(mem.ewram[0xf4a8..0xf4ac].try_into().unwrap());
    let Some(offset)=root.checked_sub(0x0200_0000).map(|p|p as usize)
        .filter(|p|p & 3 == 0 && *p+0x1c0<=mem.ewram.len()) else {
        return Err("invalid battle controller");
    };
    // Success stores in EUR 08125CDC..08125D0A: clear busy/cancel, request
    // result presentation (bit 3), publish success=1. Bit 4 follows the same
    // mission flag used by the original script. Never alter unit HP or saves.
    let mission_flag = u16::from_le_bytes(mem.ewram[0x3c28..0x3c2a].try_into().unwrap());
    let next = (flags & 0xff7e) | 8 | if mission_flag == 0 { 16 } else { 0 };
    mem.ewram[0xf424..0xf426].copy_from_slice(&next.to_le_bytes());
    mem.ewram[0xf426..0xf428].copy_from_slice(&1u16.to_le_bytes());
    // Original result handler, with one valid return-stack slot. This bypasses
    // input waits (for example end-turn facing) without jumping the guest PC.
    mem.ewram[offset+0xdc..offset+0xe2].copy_from_slice(&[0x6a,0,1,0,7,0]);
    mem.ffta_debug.pending=true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unrecognized_rom_is_never_modified() {
        let mut m=MemMap::new(vec![0;192]);
        let before=m.ewram.clone();
        assert!(request_victory(&mut m).is_err());
        assert!(before==m.ewram);
    }
    #[test]
    fn cleanup_waits_for_result_handler() {
        let mut m=MemMap::new(vec![0;192]);
        m.iwram[0x2804..0x2808].copy_from_slice(&0x0802_5115u32.to_le_bytes());
        m.ewram[0xf424]=8;
        m.ffta_debug.pending=true;
        tick(&mut m);
        assert_eq!(m.ewram[0xf390],0);
        assert!(m.ffta_debug.pending);
        m.ewram[0xf424]=0;
        tick(&mut m);
        assert_eq!(m.ewram[0xf390],8);
        assert!(!m.ffta_debug.pending);
    }
    #[test]
    fn no_battle_is_untouched() {
        let mut m = MemMap::new(vec![0; 192]);
        let before = m.ewram.clone();
        assert_eq!(request_verified(&mut m), Err("no active battle"));
        assert_eq!(m.ewram, before);
    }
    #[test]
    fn success_is_one_shot_and_keeps_other_memory() {
        for mission in [0u16, 1] {
            let mut m = MemMap::new(vec![0; 192]);
            m.iwram[0x2804..0x2808].copy_from_slice(&0x0802_5115u32.to_le_bytes());
            m.ewram[0xf4a8..0xf4ac].copy_from_slice(&0x0200_f4e8u32.to_le_bytes());
            m.ewram[0xf424..0xf426].copy_from_slice(&0x8085u16.to_le_bytes());
            m.ewram[0x3c28..0x3c2a].copy_from_slice(&mission.to_le_bytes());
            let mut expected=m.ewram.clone();
            expected[0xf424..0xf428].copy_from_slice(&[if mission==0 {0x1c}else{0x0c},0x80,1,0]);
            expected[0xf5c4..0xf5ca].copy_from_slice(&[0x6a,0,1,0,7,0]);
            assert_eq!(request_verified(&mut m), Ok(()));
            assert!(m.ewram==expected, "unexpected RAM change");
            assert!(request_verified(&mut m).is_err());
            assert!(m.ewram==expected, "repeated request changed RAM");
        }
    }
}
