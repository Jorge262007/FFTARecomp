//! Versioned machine states. Never stores ROM, native code or host pointers.
use crate::{cpu::Cpu, mem::HardwareState, Machine};
use sha2::{Digest, Sha256};
pub(crate) trait Codec: Sized {
    fn put(&self, w: &mut Vec<u8>);
    fn get(r: &mut Reader<'_>) -> Result<Self, String>;
}
pub(crate) struct Reader<'a> {
    pub bytes: &'a [u8],
    pub pos: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).ok_or("state length overflow")?;
        let b = self.bytes.get(self.pos..end).ok_or("truncated state")?;
        self.pos = end;
        Ok(b)
    }
}
macro_rules! number {($($t:ty),*)=>{$(impl Codec for $t {
 fn put(&self,w:&mut Vec<u8>){w.extend_from_slice(&self.to_le_bytes());}
 fn get(r:&mut Reader<'_>)->Result<Self,String>{Ok(Self::from_le_bytes(r.take(std::mem::size_of::<Self>())?.try_into().unwrap()))}
})*}}
number!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
impl Codec for bool {
    fn put(&self, w: &mut Vec<u8>) {
        (*self as u8).put(w)
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        match u8::get(r)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("invalid boolean".into()),
        }
    }
}
impl Codec for usize {
    fn put(&self, w: &mut Vec<u8>) {
        (*self as u64).put(w)
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        usize::try_from(u64::get(r)?).map_err(|_| "state integer overflow".into())
    }
}
impl<T: Codec, const N: usize> Codec for [T; N] {
    fn put(&self, w: &mut Vec<u8>) {
        for v in self {
            v.put(w)
        }
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        let mut v = Vec::with_capacity(N);
        for _ in 0..N {
            v.push(T::get(r)?);
        }
        v.try_into().map_err(|_| "state array".into())
    }
}
impl<T: Codec> Codec for Vec<T> {
    fn put(&self, w: &mut Vec<u8>) {
        self.len().put(w);
        for v in self {
            v.put(w)
        }
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        let n = usize::get(r)?;
        if n > 1_048_576 {
            return Err("state allocation limit".into());
        }
        let mut v = Vec::new();
        for _ in 0..n {
            v.push(T::get(r)?);
        }
        Ok(v)
    }
}
impl<T: Codec> Codec for Option<T> {
    fn put(&self, w: &mut Vec<u8>) {
        self.is_some().put(w);
        if let Some(v) = self {
            v.put(w)
        }
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        if bool::get(r)? {
            Ok(Some(T::get(r)?))
        } else {
            Ok(None)
        }
    }
}
impl<T: Codec> Codec for std::collections::VecDeque<T> {
    fn put(&self, w: &mut Vec<u8>) {
        self.len().put(w);
        for v in self {
            v.put(w)
        }
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        Ok(Vec::<T>::get(r)?.into())
    }
}
macro_rules! fields {($t:ty,$($f:ident),* $(,)?)=>{impl crate::state::Codec for $t {
 fn put(&self,w:&mut Vec<u8>){$(crate::state::Codec::put(&self.$f,w);)*}
 fn get(r:&mut crate::state::Reader<'_>)->Result<Self,String>{Ok(Self{$($f:crate::state::Codec::get(r)?,)*})}
}};}
pub(crate) use fields;
impl Machine {
    pub fn save_state(&self) -> Vec<u8> {
        let mut body = Vec::with_capacity(600_000);
        self.cpu.put(&mut body);
        self.bus.put_hardware(&mut body);
        let mut out = b"FFTAST01".to_vec();
        out.extend_from_slice(&Sha256::digest(&self.bus.rom));
        out.extend_from_slice(&Sha256::digest(&self.bus.bios));
        out.extend_from_slice(&Sha256::digest(&body));
        out.extend(body);
        out
    }
    pub fn load_state(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() > 8 * 1024 * 1024 || data.get(..8) != Some(b"FFTAST01") {
            return Err("unsupported state format".into());
        }
        if data.get(8..40) != Some(Sha256::digest(&self.bus.rom).as_slice()) {
            return Err("state belongs to another ROM".into());
        }
        if data.get(40..72) != Some(Sha256::digest(&self.bus.bios).as_slice()) {
            return Err("state uses another BIOS".into());
        }
        let body = data.get(104..).ok_or("truncated state")?;
        if data.get(72..104) != Some(Sha256::digest(body).as_slice()) {
            return Err("state checksum mismatch".into());
        }
        let mut r = Reader {
            bytes: body,
            pos: 0,
        };
        let cpu = Cpu::get(&mut r)?;
        let hardware = HardwareState::get(&mut r)?;
        if r.pos != body.len() {
            return Err("trailing state data".into());
        }
        hardware.validate()?;
        if !matches!(
            cpu.cpsr & 31,
            0x10 | 0x11 | 0x12 | 0x13 | 0x17 | 0x1b | 0x1f
        ) {
            return Err("invalid CPU mode".into());
        }
        self.cpu = cpu;
        self.bus.apply_hardware(hardware);
        self.decode_cache = Default::default();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{bus::Bus, Machine};
    fn machine() -> Machine {
        let mut m = Machine::new(vec![0xfe, 0xff, 0xff, 0xea]);
        m.cpu.regs[15] = 0x08000000;
        m.cpu.cpsr = 0x1f;
        m
    }
    #[test]
    fn restored_machine_continues_deterministically_with_timers_dma_and_audio() {
        let mut a = machine();
        a.bus.write16(0x04000100, 0xff00);
        a.bus.write16(0x04000102, 0x80);
        for _ in 0..3 {
            a.run_frame(5_000_000);
        }
        let state = a.save_state();
        let mut b = machine();
        b.load_state(&state).unwrap();
        assert_eq!(state, b.save_state());
        for _ in 0..5 {
            a.run_frame(5_000_000);
            b.run_frame(5_000_000);
            assert_eq!(a.save_state(), b.save_state());
        }
    }
    #[test]
    fn damaged_or_other_rom_state_never_changes_live_machine() {
        let mut m = machine();
        m.run_frame(5_000_000);
        let original = m.save_state();
        let mut bad = original.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(m.load_state(&bad).is_err());
        assert_eq!(m.save_state(), original);
        for n in [0, 7, 40, 72, 103, original.len() - 1] {
            assert!(m.load_state(&original[..n]).is_err());
            assert_eq!(m.save_state(), original);
        }
        let mut other = Machine::new(vec![0; 4]);
        assert!(other.load_state(&original).is_err());
    }
}

/// Read a verified state's preview without changing a running machine.
pub fn preview(data: &[u8]) -> Result<(Vec<u8>, Vec<u16>), String> {
    if data.len() > 8 * 1024 * 1024 || data.get(..8) != Some(b"FFTAST01") {
        return Err("unsupported state format".into());
    }
    let body = data.get(104..).ok_or("truncated state")?;
    if data.get(72..104) != Some(Sha256::digest(body).as_slice()) {
        return Err("state checksum mismatch".into());
    }
    let mut r = Reader {
        bytes: body,
        pos: 0,
    };
    let _ = Cpu::get(&mut r)?;
    let hardware = HardwareState::get(&mut r)?;
    if r.pos != body.len() {
        return Err("trailing state data".into());
    }
    hardware.validate()?;
    Ok((hardware.ewram, hardware.framebuffer))
}
