//! The LZW decoder shared by BPK images and HAF animation frames. The two formats differ only
//! in which codes mean "clear" and "end" (spec M1a §3.2, §3.5).

use std::fmt;

const MIN_WIDTH: u32 = 9;
const MAX_WIDTH: u32 = 12;
const DICTIONARY_SIZE: usize = 1 << MAX_WIDTH;
const FIRST_FREE_CODE: u16 = 258;

/// The two special codes of a variant.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Codes {
    pub clear: u16,
    pub end: u16,
}

/// BPK images: 256 ends a stream, 257 clears the dictionary (the reverse of GIF).
pub(crate) const BPK: Codes = Codes {
    clear: 257,
    end: 256,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LzwError {
    /// A code that is neither a literal, a dictionary entry nor the next free code.
    InvalidCode { code: u16, next: u16 },
    /// The data ran out before the end code.
    UnexpectedEnd,
}

impl fmt::Display for LzwError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LzwError::InvalidCode { code, next } => {
                write!(f, "invalid LZW code {code} (next free code {next})")
            }
            LzwError::UnexpectedEnd => write!(f, "the data ends before the LZW end code"),
        }
    }
}

impl std::error::Error for LzwError {}

/// Where decoding stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decoded {
    /// Bits consumed, including the end code when decoding ended on it.
    pub bits: usize,
    /// True when the end code was read; false when `limit` bytes were produced first.
    pub ended: bool,
}

/// Decodes one stream from `input` (LSB-first bit packing), appending bytes to `out`. Stops at
/// the end code, or as soon as `out` holds `limit` bytes.
pub(crate) fn decode(
    input: &[u8],
    codes: Codes,
    limit: usize,
    out: &mut Vec<u8>,
) -> Result<Decoded, LzwError> {
    let mut prefix = [0u16; DICTIONARY_SIZE];
    let mut suffix = [0u8; DICTIONARY_SIZE];
    for byte in 0..=255u8 {
        suffix[usize::from(byte)] = byte;
    }
    let mut stack = Vec::with_capacity(DICTIONARY_SIZE);
    let total_bits = input.len() * 8;
    let mut position = 0usize;
    let mut width = MIN_WIDTH;
    let mut next = FIRST_FREE_CODE;
    let mut previous: Option<u16> = None;

    loop {
        if out.len() >= limit {
            out.truncate(limit);
            return Ok(Decoded {
                bits: position,
                ended: false,
            });
        }
        if position + width as usize > total_bits {
            return Err(LzwError::UnexpectedEnd);
        }
        let code = read_bits(input, position, width);
        position += width as usize;

        if code == codes.clear {
            width = MIN_WIDTH;
            next = FIRST_FREE_CODE;
            previous = None;
            continue;
        }
        if code == codes.end {
            return Ok(Decoded {
                bits: position,
                ended: true,
            });
        }
        let Some(prev) = previous else {
            if code > 255 {
                return Err(LzwError::InvalidCode { code, next });
            }
            out.push(u8::try_from(code).expect("literal"));
            previous = Some(code);
            continue;
        };

        let string_code = if code < next {
            code
        } else if code == next {
            prev
        } else {
            return Err(LzwError::InvalidCode { code, next });
        };
        stack.clear();
        let mut walk = string_code;
        while walk > 255 {
            stack.push(suffix[usize::from(walk)]);
            walk = prefix[usize::from(walk)];
        }
        stack.push(u8::try_from(walk).expect("root is a literal"));
        let head = *stack.last().expect("non-empty");
        out.extend(stack.iter().rev());
        if code == next {
            out.push(head);
        }

        if usize::from(next) < DICTIONARY_SIZE {
            let entry = usize::from(next);
            prefix[entry] = prev;
            suffix[entry] = head;
            next += 1;
            if u32::from(next) == 1 << width && width < MAX_WIDTH {
                width += 1;
            }
        }
        previous = Some(code);
    }
}

/// Reads `width` bits starting at bit `position`, least significant bit first.
fn read_bits(input: &[u8], position: usize, width: u32) -> u16 {
    let byte = position / 8;
    let mut window = 0u32;
    for (index, &value) in input[byte..input.len().min(byte + 3)].iter().enumerate() {
        window |= u32::from(value) << (8 * index);
    }
    let code = (window >> (position % 8)) & ((1 << width) - 1);
    u16::try_from(code).expect("at most 12 bits")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs codes of the given widths LSB-first, as the original's encoder does.
    pub(crate) fn pack(codes: &[(u16, u32)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut accumulator = 0u64;
        let mut bits = 0u32;
        for &(code, width) in codes {
            accumulator |= u64::from(code) << bits;
            bits += width;
            while bits >= 8 {
                bytes.push((accumulator & 0xFF) as u8);
                accumulator >>= 8;
                bits -= 8;
            }
        }
        if bits > 0 {
            bytes.push((accumulator & 0xFF) as u8);
        }
        bytes
    }

    fn run(codes: &[(u16, u32)], variant: Codes) -> Result<(Vec<u8>, Decoded), LzwError> {
        let mut out = Vec::new();
        let decoded = decode(&pack(codes), variant, usize::MAX, &mut out)?;
        Ok((out, decoded))
    }

    #[test]
    fn literals_and_dictionary_entries_decode() {
        // "ABAB": A, B, then code 258 (= "AB", added after the second code), then end.
        let (out, decoded) = run(&[(257, 9), (65, 9), (66, 9), (258, 9), (256, 9)], BPK).unwrap();
        assert_eq!(out, b"ABAB");
        assert!(decoded.ended);
        assert_eq!(decoded.bits, 45);
    }

    #[test]
    fn the_kwkwk_case_uses_the_previous_string_plus_its_first_byte() {
        // "AAA": A, then 258 before it exists (= "A" + "A").
        let (out, _) = run(&[(257, 9), (65, 9), (258, 9), (256, 9)], BPK).unwrap();
        assert_eq!(out, b"AAA");
    }

    #[test]
    fn a_clear_resets_the_dictionary_and_the_code_width() {
        // After a clear, 258 is the KwKwK of the new dictionary, not the old "AB".
        let (out, _) = run(
            &[
                (257, 9),
                (65, 9),
                (66, 9),
                (257, 9),
                (67, 9),
                (258, 9),
                (256, 9),
            ],
            BPK,
        )
        .unwrap();
        assert_eq!(out, b"ABCCC");
    }

    #[test]
    fn codes_widen_to_ten_bits_after_entry_511() {
        // A long run of literals fills the dictionary to 512 entries; the next code is 10 bits.
        let mut codes = vec![(257, 9), (0, 9)];
        for _ in 0..254 {
            codes.push((1, 9));
        }
        codes.push((2, 10));
        codes.push((256, 10));
        let (out, _) = run(&codes, BPK).unwrap();
        assert_eq!(out.len(), 256);
        assert_eq!(*out.last().unwrap(), 2);
    }

    #[test]
    fn codes_widen_up_to_twelve_bits_and_no_further() {
        // Large images fill the whole 4096-entry dictionary; one bit too many or too few after
        // a width change garbles everything that follows.
        let mut codes = vec![(257, 9), (7, 9)];
        let (mut next, mut width) = (258u32, 9u32);
        for _ in 0..4000 {
            codes.push((7, width));
            if next < 4096 {
                next += 1;
                if next == 1 << width && width < 12 {
                    width += 1;
                }
            }
        }
        assert_eq!(width, 12);
        codes.push((256, 12));
        let (out, decoded) = run(&codes, BPK).unwrap();
        assert!(decoded.ended);
        assert_eq!(out.len(), 4001);
        assert!(out.iter().all(|&byte| byte == 7));
    }

    #[test]
    fn a_code_beyond_the_dictionary_is_an_error() {
        // A corrupt file must fail loudly, not draw garbage.
        assert_eq!(
            run(&[(257, 9), (65, 9), (300, 9)], BPK).unwrap_err(),
            LzwError::InvalidCode {
                code: 300,
                next: 258
            }
        );
    }

    #[test]
    fn a_non_literal_right_after_a_clear_is_an_error() {
        assert!(matches!(
            run(&[(257, 9), (258, 9)], BPK),
            Err(LzwError::InvalidCode { .. })
        ));
    }

    #[test]
    fn truncated_data_is_an_error() {
        // A cut-off download or a damaged archive must not hang or panic.
        assert_eq!(
            run(&[(257, 9), (65, 9)], BPK).unwrap_err(),
            LzwError::UnexpectedEnd
        );
    }

    #[test]
    fn decoding_stops_at_the_limit_even_without_an_end_code() {
        // ENDANI.haf frame 200 has its end code at the wrong width; the 38400-pixel limit saves it.
        // Partial BPK decodes stop the same way.
        let mut out = Vec::new();
        let decoded = decode(
            &pack(&[(257, 9), (65, 9), (66, 9), (67, 9), (999, 10)]),
            BPK,
            2,
            &mut out,
        )
        .unwrap();
        assert_eq!(out, b"AB");
        assert!(!decoded.ended);
    }
}
