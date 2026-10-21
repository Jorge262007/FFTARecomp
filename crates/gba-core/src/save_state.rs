//! Versioned, ROM/BIOS-bound machine snapshots. No host pointers or ROM bytes.
use crate::{Cpu, Machine, MemMap};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAGIC: &[u8; 8] = b"FFTA-ST\0";
// Increment whenever serialized layout or execution semantics become incompatible.
const VERSION: u32 = 1;
const HEADER: usize = 8 + 4 + 32 * 3;
pub const MAX_STATE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Snapshot {
    cpu: Cpu,
    bus: MemMap,
}

#[derive(Serialize)]
struct SnapshotRef<'a> {
    cpu: &'a Cpu,
    bus: &'a MemMap,
}

impl Machine {
    pub fn save_state(&self) -> Result<Vec<u8>, String> {
        let payload = postcard::to_allocvec(&SnapshotRef {
            cpu: &self.cpu,
            bus: &self.bus,
        })
        .map_err(|e| format!("Cannot encode save state: {e}"))?;
        if payload.len() > MAX_STATE_BYTES - HEADER {
            return Err("Save state exceeds the size limit".into());
        }
        let mut out = Vec::with_capacity(HEADER + payload.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&Sha256::digest(&self.bus.rom));
        out.extend_from_slice(&Sha256::digest(&self.bus.bios));
        out.extend_from_slice(&Sha256::digest(&payload));
        out.extend_from_slice(&payload);
        Ok(out)
    }

    /// Decode and validate entirely before replacing the live machine.
    pub fn load_state(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() < HEADER || data.len() > MAX_STATE_BYTES || &data[..8] != MAGIC {
            return Err("Not a valid FFTARecomp save state".into());
        }
        if u32::from_le_bytes(data[8..12].try_into().unwrap()) != VERSION {
            return Err("Incompatible save state version".into());
        }
        if data[12..44] != Sha256::digest(&self.bus.rom)[..] {
            return Err("This save state belongs to a different ROM".into());
        }
        if data[44..76] != Sha256::digest(&self.bus.bios)[..] {
            return Err("This save state uses a different BIOS".into());
        }
        let payload = &data[HEADER..];
        if data[76..108] != Sha256::digest(payload)[..] {
            return Err("Save state is damaged (checksum mismatch)".into());
        }
        let (mut state, rest): (Snapshot, _) = postcard::take_from_bytes(payload)
            .map_err(|e| format!("Cannot decode save state: {e}"))?;
        if !rest.is_empty() {
            return Err("Unexpected data after save state".into());
        }
        state.bus.validate_state(&self.bus)?;
        // Keep host resources and cancelled input requests out of the file.
        std::mem::swap(&mut state.bus.rom, &mut self.bus.rom);
        std::mem::swap(&mut state.bus.rom_resources, &mut self.bus.rom_resources);
        std::mem::swap(&mut state.bus.bios, &mut self.bus.bios);
        std::mem::swap(
            &mut state.bus.ffta_diagnostics,
            &mut self.bus.ffta_diagnostics,
        );
        std::mem::swap(
            &mut state.bus.ffta_battle_mouse,
            &mut self.bus.ffta_battle_mouse,
        );
        state.bus.ffta_battle_mouse.cancel();
        state.bus.zoom_scratch = std::mem::take(&mut self.bus.zoom_scratch);
        self.cpu = state.cpu;
        self.bus = state.bus;
        // Cached RAM decodes must never reference instructions from the old timeline.
        self.decode_cache = crate::exec::DecodeCache::default();
        Ok(())
    }
}

/// Serde's built-in arrays stop at 32 elements. Encode larger arrays as a
/// fixed-size tuple, with exact-length decoding and no architecture-sized bytes.
pub(crate) mod array {
    use serde::{
        de::{Error, SeqAccess, Visitor},
        ser::SerializeTuple,
        Deserializer, Serialize, Serializer,
    };
    pub fn serialize<T: Serialize, S: Serializer, const N: usize>(
        a: &[T; N],
        s: S,
    ) -> Result<S::Ok, S::Error> {
        let mut tuple = s.serialize_tuple(N)?;
        for value in a {
            tuple.serialize_element(value)?;
        }
        tuple.end()
    }
    pub fn deserialize<'de, T: serde::Deserialize<'de>, D: Deserializer<'de>, const N: usize>(
        d: D,
    ) -> Result<[T; N], D::Error> {
        struct ArrayVisitor<T, const N: usize>(std::marker::PhantomData<T>);
        impl<'de, T: serde::Deserialize<'de>, const N: usize> Visitor<'de> for ArrayVisitor<T, N> {
            type Value = [T; N];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{N} array elements")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::with_capacity(N);
                for _ in 0..N {
                    values.push(
                        seq.next_element()?
                            .ok_or_else(|| A::Error::custom("Incomplete array"))?,
                    );
                }
                values
                    .try_into()
                    .map_err(|_| A::Error::custom("Invalid array length"))
            }
        }
        d.deserialize_tuple(N, ArrayVisitor::<T, N>(std::marker::PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Bus;

    fn machine() -> Machine {
        let mut rom = vec![0; 512];
        rom[..4].copy_from_slice(&0xeaff_fffeu32.to_le_bytes()); // ARM spin loop
        rom[256..265].copy_from_slice(b"FLASH_V12");
        let mut m = Machine::new(rom);
        m.bus.write16(0x04000100, 0xff00);
        m.bus.write16(0x04000102, 0x0080); // running timer
        m.bus.write16(0x04000084, 0x0080); // sound master
        m.bus.write16(0x04000062, 0xf080);
        m.bus.write16(0x04000064, 0x8700); // active square voice
        m
    }

    #[test]
    fn round_trip_replays_cpu_video_audio_and_backup() {
        let mut m = machine();
        m.cpu.regs[3] = 0x12345678;
        m.cpu.set_cpsr(crate::Mode::Fiq as u32);
        m.cpu.regs[8] = 0xfeed1234;
        m.cpu.regs[13] = 0x03007e00;
        m.cpu.set_cpsr(crate::Mode::Sys as u32);
        m.bus.ewram[125] = 99;
        // Save mid-command, not just backup bytes.
        let flash = m.bus.flash.as_mut().unwrap();
        flash.write(0x5555, 0xaa);
        flash.write(0x2aaa, 0x55);
        flash.write(0x5555, 0xa0);
        m.bus.tick(12345);
        let original = m.save_state().unwrap();
        let continue_machine = |m: &mut Machine| {
            m.bus.flash.as_mut().unwrap().write(0x0008, 0x42);
            m.run_frame(1_000_000);
            m.run_frame(1_000_000);
        };
        continue_machine(&mut m);
        let expected = m.save_state().unwrap();
        assert_eq!(m.bus.flash.as_ref().unwrap().data[8], 0x42);
        m.load_state(&original).unwrap();
        m.cpu.set_cpsr(crate::Mode::Fiq as u32);
        assert_eq!(m.cpu.regs[8], 0xfeed1234);
        assert_eq!(m.cpu.regs[13], 0x03007e00);
        m.cpu.set_cpsr(crate::Mode::Sys as u32);
        assert_eq!(m.save_state().unwrap(), original);
        continue_machine(&mut m);
        assert_eq!(m.save_state().unwrap(), expected);
    }

    #[test]
    fn enhanced_audio_and_host_layers_round_trip() {
        let mut m = machine();
        m.bus.mp2k = Some(Box::new(crate::mp2k::Mp2kHle::new(&[])));
        m.bus.gax = Some(Box::new(crate::gax::GaxHle::new_v3()));
        m.bus.rdrv = Some(Box::new(crate::rdrv::RdrvHle::new(crate::rdrv::RdrvSig {
            blob_rom_off: 0,
        })));
        m.bus
            .ffta_dialogue
            .pixels
            .push(crate::ffta_dialogue::UiPixel {
                color: 0x1234,
                ..Default::default()
            });
        let bytes = m.save_state().unwrap();
        m.bus.mp2k = None;
        m.bus.gax = None;
        m.bus.rdrv = None;
        m.bus.ffta_dialogue.pixels.clear();
        m.load_state(&bytes).unwrap();
        assert_eq!(m.save_state().unwrap(), bytes);
    }

    #[test]
    fn rejects_damage_wrong_game_bios_and_version_without_mutation() {
        let mut m = machine();
        let original = m.save_state().unwrap();
        for offset in [0, 8, 12, 44, 76, HEADER + 3] {
            let mut bad = original.clone();
            bad[offset] ^= 1;
            assert!(m.load_state(&bad).is_err(), "offset {offset}");
            assert_eq!(m.save_state().unwrap(), original);
        }
        for size in [0, 7, HEADER - 1, original.len() - 1] {
            assert!(m.load_state(&original[..size]).is_err());
            assert_eq!(m.save_state().unwrap(), original);
        }
    }

    #[test]
    fn invalid_memory_layout_is_rejected_transactionally() {
        let mut m = machine();
        let original = m.save_state().unwrap();
        m.bus.ewram.clear();
        let bad = m.save_state().unwrap();
        m = machine();
        assert!(m.load_state(&bad).is_err());
        assert_eq!(m.save_state().unwrap(), original);
    }
}
