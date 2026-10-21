//! Bounded host PCM queue for delayed 1x audio while the guest advances faster.
//! Allocate once on the producer; no file handles, I/O or consumer allocation.
use std::collections::VecDeque;
use std::io;

type Pair = (i16, i16);
const RATE: usize = gba_core::mem::AUDIO_RATE_HZ as usize;
pub const MAX_PAIRS: usize = RATE * 2;
const RECENT_PAIRS: usize = RATE / 4;
const FADE_PAIRS: usize = RATE / 100;

#[derive(Clone, Copy, Default)]
pub struct Stats {
    pub queued_pairs: usize,
    pub resyncs: u64,
    pub skipped_pairs: u64,
}

pub struct NormalAudio {
    samples: VecDeque<Pair>,
    // A copy of the old read head lets the consumer crossfade into recent PCM
    // without retaining a second unbounded timeline or making a hard cut.
    tail: [Pair; FADE_PAIRS],
    tail_len: usize,
    tail_at: usize,
    resyncs: u64,
    skipped_pairs: u64,
}

impl NormalAudio {
    pub fn new() -> io::Result<Self> {
        let mut samples = VecDeque::new();
        samples.try_reserve_exact(MAX_PAIRS).map_err(io::Error::other)?;
        Ok(Self {
            samples, tail: [(0, 0); FADE_PAIRS], tail_len: 0, tail_at: 0,
            resyncs: 0, skipped_pairs: 0,
        })
    }

    pub fn push(&mut self, samples: &[i16]) -> io::Result<()> {
        if samples.len() % 2 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput,
                "incomplete stereo audio pair"));
        }
        let incoming = samples.len() / 2;
        let mut skip_input = 0;
        if incoming > MAX_PAIRS.saturating_sub(self.samples.len()) {
            // Preserve what the listener would hear next, including an earlier
            // unfinished fade if another large producer burst overtakes it.
            let count = self.samples.len().min(FADE_PAIRS);
            for i in 0..count {
                self.tail[i] = self.next_pair().unwrap();
            }
            self.tail_len = count;
            self.tail_at = 0;
            let skip = (self.samples.len() + incoming).saturating_sub(RECENT_PAIRS);
            let old = skip.min(self.samples.len());
            self.samples.drain(..old);
            skip_input = skip - old;
            self.resyncs = self.resyncs.saturating_add(1);
            self.skipped_pairs = self.skipped_pairs.saturating_add(skip as u64);
        }
        self.samples.extend(samples[skip_input * 2..].chunks_exact(2)
            .map(|pair| (pair[0], pair[1])));
        debug_assert!(self.samples.len() <= MAX_PAIRS);
        Ok(())
    }

    pub fn next_pair(&mut self) -> Option<Pair> {
        let pair = self.samples.pop_front()?;
        if self.tail_at >= self.tail_len { return Some(pair); }
        let old = self.tail[self.tail_at];
        let weight = self.tail_at as i32;
        let denominator = self.tail_len.saturating_sub(1).max(1) as i32;
        let mix = |a: i16, b: i16| {
            ((i32::from(a) * (denominator - weight) + i32::from(b) * weight)
                / denominator) as i16
        };
        self.tail_at += 1;
        Some((mix(old.0, pair.0), mix(old.1, pair.1)))
    }

    pub fn stats(&self) -> Stats {
        Stats { queued_pairs: self.samples.len(), resyncs: self.resyncs,
            skipped_pairs: self.skipped_pairs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_stereo_and_reuse_after_empty_without_allocation() {
        let mut queue = NormalAudio::new().unwrap();
        let capacity = queue.samples.capacity();
        queue.push(&[1, -1, 32767, -32768]).unwrap();
        assert_eq!(queue.next_pair(), Some((1, -1)));
        assert_eq!(queue.next_pair(), Some((32767, -32768)));
        assert_eq!(queue.next_pair(), None);
        queue.push(&[7, 9]).unwrap();
        assert_eq!(queue.next_pair(), Some((7, 9)));
        assert!(queue.push(&[3]).is_err());
        assert_eq!(queue.next_pair(), None);
        assert_eq!(queue.samples.capacity(), capacity);
    }

    #[test]
    fn overflow_replaces_old_backlog_with_recent_pcm_and_crossfades_stereo() {
        let mut queue = NormalAudio::new().unwrap();
        let capacity = queue.samples.capacity();
        let old: Vec<_> = (0..MAX_PAIRS).flat_map(|_| [12_000, -12_000]).collect();
        queue.push(&old).unwrap();
        assert_eq!(queue.stats().queued_pairs, MAX_PAIRS);
        let new: Vec<_> = (0..RECENT_PAIRS).flat_map(|_| [-12_000, 12_000]).collect();
        queue.push(&new).unwrap();
        assert_eq!(queue.stats().queued_pairs, RECENT_PAIRS);
        assert_eq!(queue.stats().resyncs, 1);
        assert!(queue.stats().skipped_pairs > 0);
        assert_eq!(queue.next_pair(), Some((12_000, -12_000)));
        let mut last = 12_000;
        for _ in 1..FADE_PAIRS {
            let (l, r) = queue.next_pair().unwrap();
            assert_eq!(l, -r);
            assert!((l - last).abs() <= 38, "hard cut in the crossfade");
            last = l;
        }
        assert_eq!(last, -12_000);
        assert_eq!(queue.next_pair(), Some((-12_000, 12_000)));
        assert_eq!(queue.samples.capacity(), capacity);
    }

    #[test]
    fn oversized_batches_and_repeated_overflows_keep_newest_pairs_and_fixed_storage() {
        let mut queue = NormalAudio::new().unwrap();
        let capacity = queue.samples.capacity();
        let huge: Vec<_> = (0..MAX_PAIRS * 3).flat_map(|i| [i as i16, !(i as i16)]).collect();
        queue.push(&huge).unwrap();
        assert_eq!(queue.stats().queued_pairs, RECENT_PAIRS);
        let first = (MAX_PAIRS * 3 - RECENT_PAIRS) as i16;
        assert_eq!(queue.next_pair(), Some((first, !first)));
        for _ in 0..100 {
            queue.push(&huge).unwrap();
            assert_eq!(queue.stats().queued_pairs, RECENT_PAIRS);
            for _ in 0..FADE_PAIRS / 2 {
                let (l, r) = queue.next_pair().unwrap();
                assert!((i32::from(l) + i32::from(r)).abs() <= 1);
            }
            assert_eq!(queue.samples.capacity(), capacity);
        }
        assert_eq!(queue.stats().resyncs, 101);
    }

    #[test]
    fn prolonged_two_four_and_eight_x_producers_cannot_grow_latency_or_storage() {
        for speed in [2, 4, 8] {
            let mut queue = NormalAudio::new().unwrap();
            let capacity = queue.samples.capacity();
            let chunk = vec![1234; 1024 * speed * 2];
            for _ in 0..6000 {
                queue.push(&chunk).unwrap();
                assert!(queue.stats().queued_pairs <= MAX_PAIRS);
                for _ in 0..1024 { assert!(queue.next_pair().is_some()); }
                assert_eq!(queue.samples.capacity(), capacity);
            }
            assert!(queue.stats().resyncs > 0);
        }
    }
}
