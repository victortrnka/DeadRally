//! FastTracker 2 modules used as banks of sound effects (spec M1b §3.2, §4.1). The game
//! triggers their instruments one by one and never plays their pattern, so only the
//! instruments are read.

use std::fmt;

const HEADER_START: usize = 60;
const SIGNATURE: &[u8] = b"Extended Module: ";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bank {
    /// Linear frequency table (FastTracker's default); false means Amiga periods.
    pub linear_frequencies: bool,
    /// Indexed by effect number - 1; `None` for an instrument without sound.
    pub instruments: Vec<Option<Instrument>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instrument {
    pub name: String,
    /// Signed 16-bit (8-bit samples shifted left by 8).
    pub data: Vec<i16>,
    pub looping: Looping,
    /// 0..=64.
    pub volume: u8,
    /// In 1/128 semitone.
    pub finetune: i8,
    /// In semitones.
    pub relative_note: i8,
    /// 0 = left, 128 = centre, 255 = right.
    pub panning: u8,
    pub fadeout: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Looping {
    None,
    /// Start and length in samples.
    Forward {
        start: u32,
        length: u32,
    },
    PingPong {
        start: u32,
        length: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XmError {
    Malformed(String),
    Unsupported(String),
}

impl fmt::Display for XmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XmError::Malformed(problem) => write!(f, "not a valid XM module: {problem}"),
            XmError::Unsupported(what) => write!(f, "unsupported XM feature: {what}"),
        }
    }
}

impl std::error::Error for XmError {}

fn malformed(problem: impl Into<String>) -> XmError {
    XmError::Malformed(problem.into())
}

fn slice(bytes: &[u8], start: usize, length: usize) -> Result<&[u8], XmError> {
    bytes
        .get(start..start + length)
        .ok_or_else(|| malformed(format!("ends before byte {}", start + length)))
}

fn byte_at(bytes: &[u8], offset: usize) -> Result<u8, XmError> {
    slice(bytes, offset, 1).map(|b| b[0])
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, XmError> {
    slice(bytes, offset, 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, XmError> {
    slice(bytes, offset, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim_end().to_owned()
}

impl Bank {
    /// # Errors
    ///
    /// [`XmError`] when the bytes are not an XM module, do not end exactly after the last
    /// sample, or use a feature the game's effect banks do not.
    pub fn parse(bytes: &[u8]) -> Result<Bank, XmError> {
        if !bytes.starts_with(SIGNATURE) {
            return Err(malformed("no XM signature"));
        }
        let header_size = u32_at(bytes, HEADER_START)? as usize;
        let pattern_count = usize::from(u16_at(bytes, HEADER_START + 10)?);
        let instrument_count = usize::from(u16_at(bytes, HEADER_START + 12)?);
        let flags = u16_at(bytes, HEADER_START + 14)?;
        let mut position = HEADER_START + header_size;
        for _ in 0..pattern_count {
            let header_length = u32_at(bytes, position)? as usize;
            let packed = usize::from(u16_at(bytes, position + 7)?);
            position += header_length + packed;
        }
        let mut instruments = Vec::with_capacity(instrument_count);
        for index in 0..instrument_count {
            let (instrument, next) = parse_instrument(bytes, position, index)?;
            instruments.push(instrument);
            position = next;
        }
        if position != bytes.len() {
            return Err(malformed(format!(
                "{} bytes after the last instrument",
                bytes.len().saturating_sub(position)
            )));
        }
        Ok(Bank {
            linear_frequencies: flags & 1 != 0,
            instruments,
        })
    }
}

fn parse_instrument(
    bytes: &[u8],
    at: usize,
    index: usize,
) -> Result<(Option<Instrument>, usize), XmError> {
    let size = u32_at(bytes, at)? as usize;
    let name = text(slice(bytes, at + 4, 22)?);
    let sample_count = u16_at(bytes, at + 27)?;
    if sample_count == 0 {
        return Ok((None, at + size));
    }
    if sample_count > 1 {
        return Err(XmError::Unsupported(format!(
            "instrument {} has {sample_count} samples",
            index + 1
        )));
    }
    let header = slice(bytes, at, size)?;
    let sample_header_size = u32_at(header, 29)? as usize;
    let (volume_type, panning_type, vibrato_depth) = (
        byte_at(header, 233)?,
        byte_at(header, 234)?,
        byte_at(header, 237)?,
    );
    if volume_type & 1 != 0 || panning_type & 1 != 0 || vibrato_depth != 0 {
        return Err(XmError::Unsupported(format!(
            "instrument {} has an envelope or auto-vibrato",
            index + 1
        )));
    }
    let fadeout = u16_at(header, 239)?;
    let sample = at + size;
    let length = u32_at(bytes, sample)? as usize;
    let (loop_start, loop_length) = (u32_at(bytes, sample + 4)?, u32_at(bytes, sample + 8)?);
    let sample_header = slice(bytes, sample, sample_header_size)?;
    let (volume, finetune, kind, panning, relative_note) = (
        byte_at(sample_header, 12)?.min(64),
        byte_at(sample_header, 13)? as i8,
        byte_at(sample_header, 14)?,
        byte_at(sample_header, 15)?,
        byte_at(sample_header, 16)? as i8,
    );
    let data_start = sample + sample_header_size;
    let raw = slice(bytes, data_start, length)?;
    if length == 0 {
        // A silent instrument, like one without a sample.
        return Ok((None, data_start));
    }
    let sixteen_bit = kind & 0x10 != 0;
    let (data, scale): (Vec<i16>, u32) = if sixteen_bit {
        if !length.is_multiple_of(2) {
            return Err(malformed(format!(
                "instrument {} has an odd 16-bit length",
                index + 1
            )));
        }
        let mut value = 0i16;
        let data = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                value = value.wrapping_add(i16::from_le_bytes(*pair));
                value
            })
            .collect();
        (data, 2)
    } else {
        let mut value = 0i8;
        let data = raw
            .iter()
            .map(|&delta| {
                value = value.wrapping_add(delta as i8);
                i16::from(value) << 8
            })
            .collect();
        (data, 1)
    };
    // Kept inside the sample, as FastTracker 2 does.
    let samples = u32::try_from(data.len()).unwrap_or(u32::MAX);
    let start = (loop_start / scale).min(samples);
    let loop_samples = (loop_length / scale).min(samples - start);
    let looping = match (kind & 3, loop_samples) {
        (_, 0) | (0, _) => Looping::None,
        (1, _) => Looping::Forward {
            start,
            length: loop_samples,
        },
        (2, _) => Looping::PingPong {
            start,
            length: loop_samples,
        },
        (other, _) => {
            return Err(malformed(format!(
                "instrument {} has loop type {other}",
                index + 1
            )));
        }
    };
    let instrument = Instrument {
        name,
        data,
        looping,
        volume,
        finetune,
        relative_note,
        panning,
        fadeout,
    };
    Ok((Some(instrument), data_start + length))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) struct TestSample {
        pub deltas: Vec<u8>,
        pub sixteen_bit: bool,
        pub loop_type: u8,
        pub loop_start: u32,
        pub loop_length: u32,
        pub relative_note: i8,
        pub finetune: i8,
        pub panning: u8,
    }

    pub(crate) fn sample(deltas: &[u8]) -> TestSample {
        TestSample {
            deltas: deltas.to_vec(),
            sixteen_bit: false,
            loop_type: 0,
            loop_start: 0,
            loop_length: 0,
            relative_note: 0,
            finetune: 0,
            panning: 128,
        }
    }

    /// An XM with one empty pattern and the given instruments (`None` = empty instrument).
    pub(crate) fn build(instruments: &[Option<TestSample>]) -> Vec<u8> {
        let mut bytes = SIGNATURE.to_vec();
        bytes.resize(HEADER_START, 0);
        bytes[37] = 0x1A;
        bytes[58..60].copy_from_slice(&0x104u16.to_le_bytes());
        let mut header = vec![0u8; 276];
        header[..4].copy_from_slice(&276u32.to_le_bytes());
        header[4..6].copy_from_slice(&1u16.to_le_bytes());
        header[8..10].copy_from_slice(&8u16.to_le_bytes());
        header[10..12].copy_from_slice(&1u16.to_le_bytes());
        header[12..14].copy_from_slice(&u16::try_from(instruments.len()).unwrap().to_le_bytes());
        header[14..16].copy_from_slice(&1u16.to_le_bytes());
        header[16..18].copy_from_slice(&6u16.to_le_bytes());
        header[18..20].copy_from_slice(&125u16.to_le_bytes());
        bytes.extend(header);
        bytes.extend(9u32.to_le_bytes());
        bytes.push(0);
        bytes.extend(64u16.to_le_bytes());
        bytes.extend(0u16.to_le_bytes());
        for instrument in instruments {
            let Some(sample) = instrument else {
                let mut empty = vec![0u8; 29];
                empty[..4].copy_from_slice(&29u32.to_le_bytes());
                bytes.extend(empty);
                continue;
            };
            let mut header = vec![0u8; 263];
            header[..4].copy_from_slice(&263u32.to_le_bytes());
            header[4..8].copy_from_slice(b"Boom");
            header[27..29].copy_from_slice(&1u16.to_le_bytes());
            header[29..33].copy_from_slice(&40u32.to_le_bytes());
            header[239..241].copy_from_slice(&256u16.to_le_bytes());
            bytes.extend(header);
            let mut sample_header = vec![0u8; 40];
            sample_header[..4]
                .copy_from_slice(&u32::try_from(sample.deltas.len()).unwrap().to_le_bytes());
            sample_header[4..8].copy_from_slice(&sample.loop_start.to_le_bytes());
            sample_header[8..12].copy_from_slice(&sample.loop_length.to_le_bytes());
            sample_header[12] = 48;
            sample_header[13] = sample.finetune as u8;
            sample_header[14] = sample.loop_type | if sample.sixteen_bit { 0x10 } else { 0 };
            sample_header[15] = sample.panning;
            sample_header[16] = sample.relative_note as u8;
            bytes.extend(sample_header);
            bytes.extend(&sample.deltas);
        }
        bytes
    }

    #[test]
    fn headers_too_short_for_their_fields_are_malformed_not_a_crash() {
        // Data of an unknown version is loaded with a warning; a crash would end the game
        // before it could say what is wrong.
        let instrument_at = HEADER_START + 276 + 9;
        let mut short_instrument = build(&[Some(sample(&[1, 2, 3]))]);
        short_instrument[instrument_at..instrument_at + 4].copy_from_slice(&40u32.to_le_bytes());
        assert!(matches!(
            Bank::parse(&short_instrument),
            Err(XmError::Malformed(_))
        ));
        let mut short_sample = build(&[Some(sample(&[1, 2, 3]))]);
        short_sample[instrument_at + 29..instrument_at + 33].copy_from_slice(&8u32.to_le_bytes());
        assert!(matches!(
            Bank::parse(&short_sample),
            Err(XmError::Malformed(_))
        ));
    }

    #[test]
    fn loops_are_kept_inside_their_sample() {
        // A loop past the sample's end would read past the data; FastTracker clamps it.
        let mut long_loop = sample(&[1; 100]);
        (
            long_loop.loop_type,
            long_loop.loop_start,
            long_loop.loop_length,
        ) = (1, 50, 1000);
        let mut far_loop = sample(&[1; 100]);
        (
            far_loop.loop_type,
            far_loop.loop_start,
            far_loop.loop_length,
        ) = (1, u32::MAX - 10, 100);
        let bank = Bank::parse(&build(&[Some(long_loop), Some(far_loop)])).unwrap();
        let looping = |index: usize| bank.instruments[index].as_ref().unwrap().looping;
        assert_eq!(
            looping(0),
            Looping::Forward {
                start: 50,
                length: 50
            }
        );
        assert_eq!(looping(1), Looping::None, "a loop that starts past the end");
    }

    #[test]
    fn instruments_and_their_samples_are_read() {
        let mut looped = sample(&[10, 5, 0xFB, 0]);
        looped.loop_type = 2;
        looped.loop_start = 1;
        looped.loop_length = 2;
        looped.relative_note = -12;
        looped.finetune = 16;
        looped.panning = 200;
        let bank = Bank::parse(&build(&[None, Some(looped)])).unwrap();
        assert!(bank.linear_frequencies);
        assert_eq!(bank.instruments[0], None);
        let instrument = bank.instruments[1].as_ref().unwrap();
        assert_eq!(instrument.name, "Boom");
        // Deltas 10, +5, -5, 0 give 10, 15, 10, 10.
        assert_eq!(instrument.data, [10 << 8, 15 << 8, 10 << 8, 10 << 8]);
        assert_eq!(
            instrument.looping,
            Looping::PingPong {
                start: 1,
                length: 2
            }
        );
        assert_eq!(
            (
                instrument.volume,
                instrument.finetune,
                instrument.relative_note,
                instrument.panning,
                instrument.fadeout
            ),
            (48, 16, -12, 200, 256)
        );
    }

    #[test]
    fn sixteen_bit_samples_count_their_loop_in_samples() {
        // The file gives 16-bit lengths in bytes; a loop read as samples would run past the end.
        let mut wide = sample(&[0x00, 0x01, 0x00, 0x01, 0x00, 0xFE]);
        wide.sixteen_bit = true;
        wide.loop_type = 1;
        wide.loop_start = 2;
        wide.loop_length = 4;
        let bank = Bank::parse(&build(&[Some(wide)])).unwrap();
        let instrument = bank.instruments[0].as_ref().unwrap();
        assert_eq!(instrument.data, [256, 512, 0]);
        assert_eq!(
            instrument.looping,
            Looping::Forward {
                start: 1,
                length: 2
            }
        );
    }

    #[test]
    fn trailing_bytes_and_envelopes_are_refused() {
        let mut bytes = build(&[Some(sample(&[1]))]);
        bytes.push(0);
        assert!(matches!(Bank::parse(&bytes), Err(XmError::Malformed(_))));
        let mut enveloped = build(&[Some(sample(&[1]))]);
        let instrument_at = HEADER_START + 276 + 9;
        enveloped[instrument_at + 233] = 1;
        assert!(matches!(
            Bank::parse(&enveloped),
            Err(XmError::Unsupported(_))
        ));
    }
}
