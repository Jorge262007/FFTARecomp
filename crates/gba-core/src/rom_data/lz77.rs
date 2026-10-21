//! Bounded decoding for verified GBA LZ77 asset ranges.
//!
//! This is an offline decoder, not a BIOS replacement. It does not change
//! guest cycles, copies or the active dataset. Callers must establish asset
//! roots and semantics; a successful decode alone does not identify an asset.

#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    /// Exact decoded bytes, including any resource-specific headers.
    pub bytes: Vec<u8>,
    /// Header and stream bytes actually consumed; excludes alignment padding.
    pub consumed: usize,
}

/// Decode one 0x10 stream, bounded by the supplied input and output budget.
/// Trailing bytes belong to the caller. Overlapping copies and a final match
/// clipped at the declared output size follow the BIOS/HLE stream contract.
/// Invalid history references are rejected rather than synthesizing zeroes.
pub fn decode(input: &[u8], output_budget: usize) -> Result<Block, String> {
    let header = input.get(..4).ok_or("Truncated LZ77 header")?;
    if header[0] != 0x10 {
        return Err("Expected GBA LZ77 type 0x10".into());
    }
    let size = (u32::from_le_bytes(header.try_into().unwrap()) >> 8) as usize;
    if size == 0 || size > output_budget {
        return Err(format!("LZ77 output size {size} exceeds budget or is zero"));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| "Cannot allocate bounded LZ77 output")?;
    let mut position = 4;
    while bytes.len() < size {
        let flags = take(input, &mut position)?;
        for bit in (0..8).rev() {
            if bytes.len() == size {
                break;
            }
            if flags & (1 << bit) == 0 {
                bytes.push(take(input, &mut position)?);
            } else {
                let first = take(input, &mut position)? as usize;
                let second = take(input, &mut position)? as usize;
                let length = (first >> 4) + 3;
                let distance = ((first & 15) << 8 | second) + 1;
                if distance > bytes.len() {
                    return Err(format!(
                        "LZ77 back-reference before output start at {}",
                        position - 2
                    ));
                }
                for _ in 0..length.min(size - bytes.len()) {
                    bytes.push(bytes[bytes.len() - distance]);
                }
            }
        }
    }
    Ok(Block {
        bytes,
        consumed: position,
    })
}

fn take(input: &[u8], position: &mut usize) -> Result<u8, String> {
    let byte = *input.get(*position).ok_or("Truncated LZ77 stream")?;
    *position += 1;
    Ok(byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_overlap_and_exact_consumption_match_both_bios_variants() {
        // One literal, an overlapping18-byte match and a final literal.
        let source = [0x10, 20, 0, 0, 0x40, b'A', 0xf0, 0, b'B', 0xcc, 0xdd];
        let decoded = decode(&source, 20).unwrap();
        let mut expected = vec![b'A'; 19];
        expected.push(b'B');
        assert_eq!(
            decoded,
            Block {
                bytes: expected.clone(),
                consumed: 9
            }
        );
        for swi in [0x11, 0x12] {
            let mut machine = crate::Machine::new(source.to_vec());
            machine.cpu.regs[0] = 0x0800_0000;
            machine.cpu.regs[1] = 0x0600_0020;
            machine.bus.vram.fill(0xee);
            assert!(crate::hle::bios_call(
                &mut machine.cpu,
                &mut machine.bus,
                swi
            ));
            assert_eq!(&machine.bus.vram[0x20..0x34], expected);
            assert_eq!(machine.bus.vram[0x1f], 0xee);
            assert_eq!(machine.bus.vram[0x34], 0xee);
        }
    }

    #[test]
    fn partial_final_match_and_flag_group_stop_at_declared_size() {
        let source = [0x10, 2, 0, 0, 0x40, b'X', 0xf0, 0];
        assert_eq!(
            decode(&source, 2).unwrap(),
            Block {
                bytes: b"XX".to_vec(),
                consumed: 8
            }
        );
        let source = [0x10, 1, 0, 0, 0x7f, b'Y'];
        assert_eq!(
            decode(&source, 1).unwrap(),
            Block {
                bytes: b"Y".to_vec(),
                consumed: 6
            }
        );
    }

    #[test]
    fn rejects_all_truncated_prefixes_bad_history_types_and_sizes() {
        let source = [0x10, 20, 0, 0, 0x40, b'A', 0xf0, 0, b'B'];
        for end in 0..source.len() {
            assert!(decode(&source[..end], 20).is_err());
        }
        for source in [
            &[0x10, 3, 0, 0, 0x80, 0, 0][..],
            &[0x10, 4, 0, 0, 0x40, b'A', 0, 1][..],
            &[0x11, 1, 0, 0, 0, b'A'][..],
            &[0x10, 0, 0, 0][..],
            &[0x10, 0xff, 0xff, 0xff][..],
        ] {
            assert!(decode(source, 8192).is_err());
        }
        assert!(decode(&source, 19).is_err());
        assert!(decode(&source, 0).is_err());
    }

    #[test]
    fn maximum_history_distance_preserves_byte_order() {
        let mut source = vec![0x10, 0x12, 0x10, 0]; // 4096 +18 decoded bytes.
        let expected: Vec<_> = (0..4096).map(|n| (n % 251) as u8).collect();
        for chunk in expected.chunks(8) {
            source.push(0);
            source.extend_from_slice(chunk);
        }
        source.extend_from_slice(&[0x80, 0xff, 0xff]); // length18, distance4096.
        let result = decode(&source, 4114).unwrap();
        assert_eq!(&result.bytes[..4096], expected);
        assert_eq!(&result.bytes[4096..], &expected[..18]);
        assert_eq!(result.consumed, source.len());
    }
}
