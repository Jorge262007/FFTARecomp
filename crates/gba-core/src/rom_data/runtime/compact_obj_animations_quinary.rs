//! Typed compact OBJ projection; gaps remain owned by prior resources or ROM.
use super::*;
use crate::rom_data::compact_obj_animations_quinary as model;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuinaryCompactObjStats { pub bus_reads: u64, pub source_bytes: u64 }

pub(super) struct QuinaryCompactObjResources {
    data: model::CompactAnimationsQuinary,
    counters: [AtomicU64; 2],
}
impl QuinaryCompactObjResources {
    pub(super) fn source_page(index: usize) -> bool { index >> 12 == 0x6c5 }
    pub(super) fn new(data: model::CompactAnimationsQuinary, sources: &[SourceRange]) -> Result<Self, String> {
        let spans = data.source_spans()?;
        if spans.len() != sources.len() { return Err("Quinary compact OBJ source binding count mismatch".into()); }
        for ((offset, bytes, role), source) in spans.iter().zip(sources) {
            if source.offset != *offset || source.length as usize != bytes.len()
                || source.role != *role || source.sha256 != ffta_rom::digest(bytes) {
                return Err("Quinary compact OBJ source binding mismatch".into());
            }
        }
        Ok(Self { data, counters: std::array::from_fn(|_| AtomicU64::new(0)) })
    }
    pub(super) fn guest_read<const N: usize>(&self, index: usize, rom: &[u8], prior: Option<[u8; N]>) -> Option<[u8; N]> {
        if !model::SPANS.iter().any(|&(start, end)| index < end as usize && index.saturating_add(N) > start as usize) { return prior; }
        let mut source_bytes = 0;
        let bytes = std::array::from_fn(|i| {
            let at = index + i;
            if let Some(byte) = self.data.byte_at(at as u32) { source_bytes += 1; byte }
            else { prior.as_ref().map_or_else(|| rom[at], |b| b[i]) }
        });
        self.counters[0].fetch_add(1, Ordering::Relaxed);
        self.counters[1].fetch_add(source_bytes, Ordering::Relaxed);
        Some(bytes)
    }
    pub(super) fn stats(&self) -> QuinaryCompactObjStats {
        QuinaryCompactObjStats { bus_reads: self.counters[0].load(Ordering::Relaxed), source_bytes: self.counters[1].load(Ordering::Relaxed) }
    }
    pub(super) fn resident_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.data.lists.capacity() * std::mem::size_of::<model::IndexedList>()
            + self.data.lists.iter().map(|l| l.list.objects.capacity() * std::mem::size_of::<crate::rom_data::obj_lists::Object>()).sum::<usize>()
            + self.data.sequences.capacity() * std::mem::size_of::<model::IndexedSequence>()
            + self.data.sequences.iter().map(|s| s.sequence.frames.capacity() * std::mem::size_of::<crate::rom_data::compact_obj_animations::Frame>()).sum::<usize>()
    }
}
