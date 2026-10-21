//! Offline interpretation of FFTA background patch packets.
//! These ordered writes are not rendered pictures or live guest state.

#[derive(Clone, Copy)]
pub struct Limits {
    pub destination_bytes: usize,
    pub written_bytes: usize,
    pub packets: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Patch<T> {
    pub byte_offset: u16,
    pub values: Vec<T>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Patches<T> {
    /// Preserve packet order, including repeated writes to the same cell.
    pub packets: Vec<Patch<T>>,
    /// Packet-stream extent through its terminator, excluding padding and
    /// separately addressed dictionary entries.
    pub consumed: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PlanePatches {
    pub header: u32,
    pub dictionary_offset: Option<usize>,
    pub payload: Patches<u32>,
}

impl PlanePatches {
    /// Only indexed packets have verified calculated-plane destinations.
    /// The isolated original literal branch writes to region0 instead.
    pub fn calculated_plane_writes(&self) -> Result<&Patches<u32>, String> {
        if self.dictionary_offset.is_none() {
            return Err("Unindexed plane packet destination semantics are not verified".into());
        }
        Ok(&self.payload)
    }
}

/// Offset:u16, byte-count:u16, literal halfwords; with a dictionary, the
/// count and indices are u8 instead. A zero next offset terminates the stream;
/// the first offset may be zero and still carries a packet.
pub fn decode_halfwords(
    input: &[u8],
    dictionary: Option<&[u16]>,
    limits: Limits,
) -> Result<Patches<u16>, String> {
    let mut cursor = Cursor { input, position: 0 };
    let mut offset = cursor.halfword()?;
    let mut packets = Vec::new();
    let mut written = 0;
    loop {
        let count = if dictionary.is_some() {
            cursor.byte()? as usize
        } else {
            let length = cursor.halfword()? as usize;
            if length % 2 != 0 {
                return Err("Odd halfword patch byte count".into());
            }
            length / 2
        };
        check_patch(offset, count, 2, packets.len(), &mut written, limits)?;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(if let Some(dictionary) = dictionary {
                *dictionary
                    .get(cursor.byte()? as usize)
                    .ok_or("Halfword patch dictionary index out of bounds")?
            } else {
                cursor.halfword()?
            });
        }
        packets.push(Patch {
            byte_offset: offset,
            values,
        });
        offset = cursor.halfword()?;
        if offset == 0 {
            break;
        }
    }
    Ok(Patches {
        packets,
        consumed: cursor.position,
    })
}

/// Plane header bit31 selects a compact halfword dictionary at the low28-bit
/// relative offset; bit30 selects u16 rather than u8 dictionary indices.
/// Indexed packets have a u8 value count. Unindexed packets use a u16 byte
/// count and literal u32 pairs. The original writer's signed count excludes
/// lengths >=0x8000; strict decoding also requires complete aligned pairs.
/// Literal decoding describes payload syntax only. Use the guarded accessor
/// before treating these values as calculated-plane writes.
pub fn decode_planes(input: &[u8], limits: Limits) -> Result<PlanePatches, String> {
    let mut cursor = Cursor { input, position: 0 };
    let header = cursor.word()?;
    let dictionary_offset = (header & 0x8000_0000 != 0).then_some((header & 0x0fff_ffff) as usize);
    if dictionary_offset
        .is_some_and(|offset| offset < 4 || offset % 2 != 0 || offset >= input.len())
    {
        return Err("Invalid plane patch dictionary offset".into());
    }
    let mut offset = cursor.halfword()?;
    let mut packets = Vec::new();
    let mut written = 0;
    loop {
        let count = if dictionary_offset.is_some() {
            cursor.byte()? as usize
        } else {
            let length = cursor.halfword()? as usize;
            if length % 4 != 0 || length >= 0x8000 {
                return Err("Invalid literal plane patch byte count".into());
            }
            length / 4
        };
        check_patch(offset, count, 4, packets.len(), &mut written, limits)?;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(if let Some(dictionary) = dictionary_offset {
                let index = if header & 0x4000_0000 != 0 {
                    cursor.halfword()? as usize
                } else {
                    cursor.byte()? as usize
                };
                let mut entry = Cursor {
                    input,
                    position: dictionary + index * 2,
                };
                let low = entry.halfword()? as u32;
                if low & 0x8000 != 0 {
                    (low | (entry.halfword()? as u32) << 16) & 0x7fff_7fff
                } else {
                    low | ((low + 1) << 16)
                }
            } else {
                cursor.word()?
            });
        }
        packets.push(Patch {
            byte_offset: offset,
            values,
        });
        offset = cursor.halfword()?;
        if offset == 0 {
            break;
        }
    }
    if dictionary_offset.is_some_and(|offset| cursor.position > offset) {
        return Err("Plane patch packets overlap their dictionary".into());
    }
    Ok(PlanePatches {
        header,
        dictionary_offset,
        payload: Patches {
            packets,
            consumed: cursor.position,
        },
    })
}

fn check_patch(
    offset: u16,
    count: usize,
    width: usize,
    packet: usize,
    written: &mut usize,
    limits: Limits,
) -> Result<(), String> {
    let length = count * width;
    if offset as usize % width != 0 || offset as usize + length > limits.destination_bytes {
        return Err("Patch destination is misaligned or out of bounds".into());
    }
    if packet >= limits.packets || length > limits.written_bytes.saturating_sub(*written) {
        return Err("Patch packet/write budget exceeded".into());
    }
    *written += length;
    Ok(())
}

struct Cursor<'a> {
    input: &'a [u8],
    position: usize,
}
impl Cursor<'_> {
    fn byte(&mut self) -> Result<u8, String> {
        let value = *self
            .input
            .get(self.position)
            .ok_or("Truncated background patch data")?;
        self.position += 1;
        Ok(value)
    }
    fn halfword(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes([self.byte()?, self.byte()?]))
    }
    fn word(&mut self) -> Result<u32, String> {
        Ok(self.halfword()? as u32 | (self.halfword()? as u32) << 16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const LIMITS: Limits = Limits {
        destination_bytes: 16,
        written_bytes: 32,
        packets: 4,
    };

    #[test]
    fn halfwords_preserve_zero_first_offset_order_and_trailing_padding() {
        let input = [
            0, 0, 4, 0, 0x34, 0x12, 0xcd, 0xab, 2, 0, 2, 0, 0xff, 0x7f, 0, 0, 0xcc,
        ];
        let value = decode_halfwords(&input, None, LIMITS).unwrap();
        assert_eq!(value.consumed, 16);
        assert_eq!(
            value.packets,
            [
                Patch {
                    byte_offset: 0,
                    values: vec![0x1234, 0xabcd]
                },
                Patch {
                    byte_offset: 2,
                    values: vec![0x7fff]
                }
            ]
        );
        for end in 0..16 {
            assert!(decode_halfwords(&input[..end], None, LIMITS).is_err());
        }
        let indexed = [0, 0, 3, 1, 0, 1, 0, 0];
        assert_eq!(
            decode_halfwords(&indexed, Some(&[12, 34]), LIMITS)
                .unwrap()
                .packets[0]
                .values,
            [34, 12, 34]
        );
        assert!(decode_halfwords(&indexed, Some(&[12]), LIMITS).is_err());
    }

    #[test]
    fn plane_literal_pairs_and_both_dictionary_index_widths() {
        let raw = [0, 0, 0, 0, 4, 0, 4, 0, 0x34, 0x12, 0xcd, 0xab, 0, 0, 0xcc];
        let value = decode_planes(&raw, LIMITS).unwrap();
        assert_eq!(
            value.payload.packets[0],
            Patch {
                byte_offset: 4,
                values: vec![0xabcd1234]
            }
        );
        assert_eq!(value.payload.consumed, 14);
        assert!(value.calculated_plane_writes().is_err());
        for wide in [false, true] {
            let mut input = (0x8000_0010u32 | if wide { 0x4000_0000 } else { 0 })
                .to_le_bytes()
                .to_vec();
            input.extend_from_slice(&[0, 0, 2, 0]);
            if wide {
                input.push(0);
            }
            input.push(1);
            if wide {
                input.push(0);
            }
            input.extend_from_slice(&[0, 0]);
            input.resize(16, 0xcc);
            // Entry0 implies (0x12,0x13); entry1 explicitly stores two values.
            input.extend_from_slice(&[0x12, 0, 0x34, 0x92, 0xcd, 0xab]);
            let value = decode_planes(&input, LIMITS).unwrap();
            assert_eq!(
                value.calculated_plane_writes().unwrap().packets[0].values,
                [0x00130012, 0x2bcd1234]
            );
            assert_eq!(value.payload.consumed, if wide { 13 } else { 11 });
            for end in 0..input.len() {
                assert!(decode_planes(&input[..end], LIMITS).is_err());
            }
        }
    }

    #[test]
    fn malformed_lengths_offsets_terminators_and_budgets_are_rejected() {
        for input in [
            &[0, 0, 3, 0, 1, 2, 3, 0, 0][..],
            &[1, 0, 2, 0, 1, 2, 0, 0][..],
            &[16, 0, 2, 0, 1, 2, 0, 0][..],
            &[0, 0, 0, 0][..],
        ] {
            assert!(decode_halfwords(input, None, LIMITS).is_err());
        }
        let raw = [0, 0, 0, 0, 0, 0, 4, 0, 1, 2, 3, 4, 0, 0];
        for end in 0..raw.len() {
            assert!(decode_planes(&raw[..end], LIMITS).is_err());
        }
        for (at, value) in [(4, 2), (6, 2), (7, 0x80)] {
            let mut bad = raw;
            bad[at] = value;
            assert!(decode_planes(&bad, LIMITS).is_err());
        }
        for limits in [
            Limits {
                packets: 0,
                ..LIMITS
            },
            Limits {
                written_bytes: 3,
                ..LIMITS
            },
            Limits {
                destination_bytes: 3,
                ..LIMITS
            },
        ] {
            assert!(decode_planes(&raw, limits).is_err());
        }
        for pointer in [0, 3, 4, 16, u32::MAX & 0x0fff_ffff] {
            let mut bad = raw;
            bad[..4].copy_from_slice(&(0x8000_0000 | pointer).to_le_bytes());
            assert!(decode_planes(&bad, LIMITS).is_err());
        }
    }
}
