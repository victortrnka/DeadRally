//! `.CMF` files in `MUSICS.BPA`: S3M and XM modules with every byte scrambled
//! (`getMusicStream`; spec M1b §3.1).

/// The plain module: byte `b` at position `p` becomes `rotl8(b, p mod 7) - 109 - 17 p`.
#[must_use]
pub fn decode(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .enumerate()
        .map(|(position, &byte)| {
            let shift = u32::try_from(position % 7).expect("below 7");
            let key = 109u8.wrapping_add(17u8.wrapping_mul(position as u8));
            byte.rotate_left(shift).wrapping_sub(key)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The original's tool, inverted: what `decode` must undo.
    fn encode(plain: &[u8]) -> Vec<u8> {
        plain
            .iter()
            .enumerate()
            .map(|(position, &byte)| {
                let shift = u32::try_from(position % 7).unwrap();
                let key = 109u8.wrapping_add(17u8.wrapping_mul(position as u8));
                byte.wrapping_add(key).rotate_right(shift)
            })
            .collect()
    }

    #[test]
    fn decoding_undoes_the_scrambling_at_every_position() {
        // The key depends on the position modulo 7 and modulo 256; 3000 bytes cover both.
        let plain: Vec<u8> = (0..3000u32).map(|n| (n * 31 % 251) as u8).collect();
        assert_eq!(decode(&encode(&plain)), plain);
    }

    #[test]
    fn the_key_follows_the_original_formula() {
        // Pinned without the test's encoder: a wrong rotation or key garbles every module.
        // Position 0: no rotation, key 109. Position 1: rotate by 1, key 126. Position 7: no
        // rotation again, key 109 + 119 = 228.
        let mut bytes = vec![0u8; 8];
        bytes[0] = 109;
        bytes[1] = 0x81;
        bytes[7] = 228;
        let plain = decode(&bytes);
        assert_eq!(plain[0], 0);
        assert_eq!(plain[1], 0x03u8.wrapping_sub(126));
        assert_eq!(plain[7], 0);
    }
}
