//! Scream Tracker 3 modules: the game's music (spec M1b §3.2, §4.1). Only what the data uses
//! is read; anything else is an error rather than wrong sound.

use std::fmt;

/// Channel slots in an S3M file.
pub const CHANNELS: usize = 32;
/// Rows in every pattern.
pub const ROWS: usize = 64;
/// Order list entry that playback skips.
pub const ORDER_SKIP: u8 = 254;
/// Order list entry that ends the song in Scream Tracker. The game's modules hold several
/// sections separated by it; FMOD skips it like [`ORDER_SKIP`].
pub const ORDER_END: u8 = 255;
/// Note byte meaning "no note".
pub const NO_NOTE: u8 = 255;
/// Note byte meaning "cut the note".
pub const NOTE_CUT: u8 = 254;

const HEADER_BYTES: usize = 96;
const SAMPLE_HEADER_BYTES: usize = 80;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub title: String,
    /// Pattern numbers in play order as stored, with the markers [`ORDER_SKIP`] and
    /// [`ORDER_END`]; the game starts sections by their index in this list.
    pub orders: Vec<u8>,
    pub initial_speed: u8,
    pub initial_tempo: u8,
    /// 0..=64.
    pub global_volume: u8,
    /// 0..=127; the game never changes it.
    pub master_volume: u8,
    /// False for modules flagged mono: every channel plays in the centre.
    pub stereo: bool,
    pub channels: [Channel; CHANNELS],
    /// Instrument numbers in patterns are 1-based indices into this list.
    pub samples: Vec<Sample>,
    pub patterns: Vec<Pattern>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Channel {
    pub enabled: bool,
    /// 0 = left ... 15 = right: the file's default panning, or Scream Tracker's 3 for the left
    /// half (L1-L8) and 12 for the right half (R1-R8).
    pub pan: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Sample {
    pub name: String,
    /// Playback rate of middle C (C-4), in Hz.
    pub c2spd: u32,
    /// 0..=64.
    pub volume: u8,
    /// Loop start and end in samples; `None` plays once.
    pub looped: Option<(u32, u32)>,
    /// Signed 16-bit (the file's 8-bit samples shifted left by 8). Empty for an empty slot.
    pub data: Vec<i16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    /// [`ROWS`] rows of [`CHANNELS`] cells.
    pub rows: Vec<[Cell; CHANNELS]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    /// High nibble octave, low nibble semitone; or [`NO_NOTE`] or [`NOTE_CUT`].
    pub note: u8,
    /// 0 = none.
    pub instrument: u8,
    /// 0..=64, or `None` when the cell has no volume.
    pub volume: Option<u8>,
    /// 0 = none, 1 = A ... 26 = Z.
    pub command: u8,
    pub info: u8,
}

impl Default for Cell {
    fn default() -> Cell {
        Cell {
            note: NO_NOTE,
            instrument: 0,
            volume: None,
            command: 0,
            info: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum S3mError {
    /// Not an S3M file, or its structure does not hold together.
    Malformed(String),
    /// A valid feature the game's music never uses.
    Unsupported(String),
}

impl fmt::Display for S3mError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            S3mError::Malformed(problem) => write!(f, "not a valid S3M module: {problem}"),
            S3mError::Unsupported(what) => write!(f, "unsupported S3M feature: {what}"),
        }
    }
}

impl std::error::Error for S3mError {}

fn malformed(problem: impl Into<String>) -> S3mError {
    S3mError::Malformed(problem.into())
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, S3mError> {
    bytes
        .get(offset..offset + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| malformed(format!("ends before byte {offset}")))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, S3mError> {
    bytes
        .get(offset..offset + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| malformed(format!("ends before byte {offset}")))
}

fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim_end().to_owned()
}

impl Module {
    /// # Errors
    ///
    /// [`S3mError`] when the bytes are not an S3M module or use a feature the game's music
    /// does not.
    pub fn parse(bytes: &[u8]) -> Result<Module, S3mError> {
        if bytes.len() < HEADER_BYTES || &bytes[44..48] != b"SCRM" || bytes[29] != 16 {
            return Err(malformed("no S3M header"));
        }
        let order_count = usize::from(u16_at(bytes, 32)?);
        let sample_count = usize::from(u16_at(bytes, 34)?);
        let pattern_count = usize::from(u16_at(bytes, 36)?);
        let sample_format = u16_at(bytes, 42)?;
        if sample_format != 2 {
            return Err(S3mError::Unsupported(format!(
                "sample format {sample_format} (only unsigned samples, 2)"
            )));
        }
        let mut channels = [Channel::default(); CHANNELS];
        for (slot, channel) in channels.iter_mut().enumerate() {
            let setting = bytes[64 + slot];
            if setting < 16 {
                *channel = Channel {
                    enabled: true,
                    pan: if setting >= 8 { 12 } else { 3 },
                };
            } else if setting != 255 && setting & 0x80 == 0 {
                return Err(S3mError::Unsupported(format!(
                    "channel {slot} is an AdLib channel ({setting})"
                )));
            }
        }
        let orders_start = HEADER_BYTES;
        let order_bytes = bytes
            .get(orders_start..orders_start + order_count)
            .ok_or_else(|| malformed("the order list runs past the end"))?;
        let orders = order_bytes.to_vec();
        if let Some(&bad) = orders
            .iter()
            .find(|&&order| order < ORDER_SKIP && usize::from(order) >= pattern_count)
        {
            return Err(malformed(format!(
                "order names pattern {bad} of {pattern_count}"
            )));
        }
        let pointers_start = orders_start + order_count;
        let pointer = |index: usize| -> Result<usize, S3mError> {
            Ok(usize::from(u16_at(bytes, pointers_start + 2 * index)?) * 16)
        };
        // 252 here means a table of default pannings follows the pointers.
        if bytes[53] == 252 {
            let pans_start = pointers_start + 2 * (sample_count + pattern_count);
            let pans = bytes
                .get(pans_start..pans_start + CHANNELS)
                .ok_or_else(|| malformed("the panning table runs past the end"))?;
            for (channel, &pan) in channels.iter_mut().zip(pans) {
                if pan & 0x20 != 0 {
                    channel.pan = pan & 0x0F;
                }
            }
        }
        let samples = (0..sample_count)
            .map(|index| parse_sample(bytes, pointer(index)?, index))
            .collect::<Result<Vec<_>, _>>()?;
        let patterns = (0..pattern_count)
            .map(|index| parse_pattern(bytes, pointer(sample_count + index)?, index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Module {
            title: text(&bytes[..28]),
            orders,
            initial_speed: bytes[49],
            initial_tempo: bytes[50],
            global_volume: bytes[48],
            master_volume: bytes[51] & 0x7F,
            stereo: bytes[51] & 0x80 != 0,
            channels,
            samples,
            patterns,
        })
    }
}

fn parse_sample(bytes: &[u8], at: usize, index: usize) -> Result<Sample, S3mError> {
    let header = bytes
        .get(at..at + SAMPLE_HEADER_BYTES)
        .ok_or_else(|| malformed(format!("sample {index} header runs past the end")))?;
    let name = text(&header[48..76]);
    match header[0] {
        0 => {
            return Ok(Sample {
                name,
                ..Sample::default()
            });
        }
        1 => {}
        kind => {
            return Err(S3mError::Unsupported(format!(
                "sample {index} is of type {kind} (AdLib)"
            )));
        }
    }
    let flags = header[31];
    if header[30] != 0 || flags & 0b110 != 0 {
        return Err(S3mError::Unsupported(format!(
            "sample {index} is packed, stereo or 16-bit"
        )));
    }
    let segment = (usize::from(header[13]) << 16) | usize::from(u16_at(header, 14)?);
    let length = u32_at(header, 16)?;
    let data_start = segment * 16;
    let raw = bytes
        .get(data_start..data_start + length as usize)
        .ok_or_else(|| malformed(format!("sample {index} data runs past the end")))?;
    let (loop_start, loop_end) = (u32_at(header, 20)?, u32_at(header, 24)?);
    let looped = (flags & 1 != 0 && loop_end > loop_start)
        .then(|| (loop_start.min(length), loop_end.min(length)));
    Ok(Sample {
        name,
        c2spd: u32_at(header, 32)?,
        volume: header[28].min(64),
        looped,
        data: raw
            .iter()
            .map(|&byte| i16::from((byte ^ 0x80) as i8) << 8)
            .collect(),
    })
}

fn parse_pattern(bytes: &[u8], at: usize, index: usize) -> Result<Pattern, S3mError> {
    let mut rows = vec![[Cell::default(); CHANNELS]; ROWS];
    if at == 0 {
        return Ok(Pattern { rows });
    }
    let packed_length = usize::from(u16_at(bytes, at)?);
    let packed = bytes
        .get(at + 2..at + packed_length)
        .ok_or_else(|| malformed(format!("pattern {index} runs past the end")))?;
    let mut position = 0;
    let mut next = || -> Result<u8, S3mError> {
        let byte = packed
            .get(position)
            .copied()
            .ok_or_else(|| malformed(format!("pattern {index} ends inside a row")));
        position += 1;
        byte
    };
    for row in &mut rows {
        loop {
            let what = next()?;
            if what == 0 {
                break;
            }
            let cell = &mut row[usize::from(what & 31)];
            if what & 32 != 0 {
                cell.note = next()?;
                cell.instrument = next()?;
            }
            if what & 64 != 0 {
                cell.volume = Some(next()?.min(64));
            }
            if what & 128 != 0 {
                let command = next()?;
                // Scream Tracker ignores commands beyond Z; TR0-MUS has one.
                cell.command = if command <= 26 { command } else { 0 };
                cell.info = next()?;
            }
        }
    }
    Ok(Pattern { rows })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a minimal S3M: the order list as given, one sample and the given packed
    /// patterns, 4 enabled channels.
    pub(crate) fn build(
        orders: &[u8],
        sample: &[u8],
        looped: Option<(u32, u32)>,
        patterns: &[Vec<u8>],
    ) -> Vec<u8> {
        let order_count = orders.len();
        let mut bytes = vec![0u8; HEADER_BYTES];
        bytes[..4].copy_from_slice(b"Test");
        bytes[28] = 0x1A;
        bytes[29] = 16;
        bytes[32..34].copy_from_slice(&u16::try_from(order_count).unwrap().to_le_bytes());
        bytes[34..36].copy_from_slice(&1u16.to_le_bytes());
        bytes[36..38].copy_from_slice(&u16::try_from(patterns.len()).unwrap().to_le_bytes());
        bytes[40..42].copy_from_slice(&0x1320u16.to_le_bytes());
        bytes[42..44].copy_from_slice(&2u16.to_le_bytes());
        bytes[44..48].copy_from_slice(b"SCRM");
        bytes[48] = 64;
        bytes[49] = 6;
        bytes[50] = 125;
        bytes[51] = 48;
        bytes[64..96].fill(255);
        bytes[64..68].copy_from_slice(&[0, 8, 1, 9]);
        bytes.extend_from_slice(orders);
        let pointers_at = bytes.len();
        bytes.resize(pointers_at + 2 * (1 + patterns.len()), 0);
        let pad = |bytes: &mut Vec<u8>| bytes.resize(bytes.len().div_ceil(16) * 16, 0);
        pad(&mut bytes);
        let sample_at = bytes.len();
        bytes.resize(sample_at + SAMPLE_HEADER_BYTES, 0);
        pad(&mut bytes);
        let data_at = bytes.len();
        bytes.extend_from_slice(sample);
        pad(&mut bytes);
        {
            let header = &mut bytes[sample_at..sample_at + SAMPLE_HEADER_BYTES];
            header[0] = 1;
            let segment = data_at / 16;
            header[13] = (segment >> 16) as u8;
            header[14..16].copy_from_slice(&((segment & 0xFFFF) as u16).to_le_bytes());
            header[16..20].copy_from_slice(&u32::try_from(sample.len()).unwrap().to_le_bytes());
            if let Some((start, end)) = looped {
                header[20..24].copy_from_slice(&start.to_le_bytes());
                header[24..28].copy_from_slice(&end.to_le_bytes());
                header[31] = 1;
            }
            header[28] = 64;
            header[32..36].copy_from_slice(&8363u32.to_le_bytes());
            header[48..52].copy_from_slice(b"Beep");
            header[76..80].copy_from_slice(b"SCRS");
        }
        let mut pattern_pointers = Vec::new();
        for packed in patterns {
            let at = bytes.len();
            pattern_pointers.push(at);
            bytes.extend_from_slice(&u16::try_from(packed.len() + 2).unwrap().to_le_bytes());
            bytes.extend_from_slice(packed);
            pad(&mut bytes);
        }
        let mut write_pointer = |slot: usize, at: usize| {
            bytes[pointers_at + 2 * slot..pointers_at + 2 * slot + 2]
                .copy_from_slice(&u16::try_from(at / 16).unwrap().to_le_bytes());
        };
        write_pointer(0, sample_at);
        for (index, &at) in pattern_pointers.iter().enumerate() {
            write_pointer(1 + index, at);
        }
        bytes
    }

    /// A cell to pack: (row, channel, note, instrument, volume, command, info).
    pub(crate) type TestCell = (usize, u8, u8, u8, Option<u8>, u8, u8);

    /// Packs 64 rows of cells.
    pub(crate) fn pack(cells: &[TestCell]) -> Vec<u8> {
        let mut packed = Vec::new();
        for row in 0..ROWS {
            for &(_, channel, note, instrument, volume, command, info) in
                cells.iter().filter(|cell| cell.0 == row)
            {
                let mut what = channel;
                if note != NO_NOTE || instrument != 0 {
                    what |= 32;
                }
                if volume.is_some() {
                    what |= 64;
                }
                if command != 0 {
                    what |= 128;
                }
                packed.push(what);
                if what & 32 != 0 {
                    packed.extend([note, instrument]);
                }
                if let Some(volume) = volume {
                    packed.push(volume);
                }
                if command != 0 {
                    packed.extend([command, info]);
                }
            }
            packed.push(0);
        }
        packed
    }

    #[test]
    fn header_channels_samples_and_patterns_are_read() {
        let pattern = pack(&[
            (0, 1, 0x40, 1, Some(32), 1, 4),
            (63, 3, NOTE_CUT, 0, None, 0, 0),
        ]);
        let module = Module::parse(&build(
            &[0, ORDER_SKIP, 0],
            &[0x80, 0xFF, 0x00],
            Some((1, 3)),
            &[pattern],
        ))
        .unwrap();
        assert_eq!(module.title, "Test");
        assert_eq!(module.orders, [0, ORDER_SKIP, 0]);
        assert_eq!(
            (
                module.initial_speed,
                module.initial_tempo,
                module.global_volume
            ),
            (6, 125, 64)
        );
        assert!(!module.stereo);
        assert_eq!(
            module.channels[0],
            Channel {
                enabled: true,
                pan: 3
            }
        );
        assert_eq!(
            module.channels[1],
            Channel {
                enabled: true,
                pan: 12
            }
        );
        assert!(!module.channels[4].enabled);
        let sample = &module.samples[0];
        assert_eq!(
            (sample.c2spd, sample.volume, sample.looped),
            (8363, 64, Some((1, 3)))
        );
        let cell = module.patterns[0].rows[0][1];
        assert_eq!(
            (
                cell.note,
                cell.instrument,
                cell.volume,
                cell.command,
                cell.info
            ),
            (0x40, 1, Some(32), 1, 4)
        );
        assert_eq!(module.patterns[0].rows[63][3].note, NOTE_CUT);
        assert_eq!(module.patterns[0].rows[1][1], Cell::default());
    }

    #[test]
    fn unsigned_samples_become_signed_16_bit() {
        // 0x80 is silence in an unsigned sample; reading it as signed would play a loud offset.
        let module = Module::parse(&build(&[0], &[0x80, 0xFF, 0x00], None, &[pack(&[])])).unwrap();
        assert_eq!(module.samples[0].data, [0, 127 << 8, -128 << 8]);
        assert_eq!(module.samples[0].looped, None);
    }

    #[test]
    fn the_whole_order_list_is_kept_with_its_section_markers() {
        // The game starts sections after the first 255 (musicSetOrder); dropping them would
        // leave nothing to start.
        let module = Module::parse(&build(
            &[0, ORDER_END, ORDER_SKIP, 0],
            &[0x80],
            None,
            &[pack(&[])],
        ))
        .unwrap();
        assert_eq!(module.orders, [0, ORDER_END, ORDER_SKIP, 0]);
    }

    #[test]
    fn orders_naming_missing_patterns_are_malformed() {
        assert!(matches!(
            Module::parse(&build(&[0, 3], &[0x80], None, &[pack(&[])])),
            Err(S3mError::Malformed(_))
        ));
    }

    #[test]
    fn features_the_music_does_not_use_are_refused() {
        // A 16-bit sample read as 8-bit would play as noise; refusing says what is missing.
        let mut bytes = build(&[0, ORDER_END], &[0x80], None, &[pack(&[])]);
        let sample_at = usize::from(u16::from_le_bytes([
            bytes[HEADER_BYTES + 2],
            bytes[HEADER_BYTES + 3],
        ])) * 16;
        bytes[sample_at + 31] |= 4;
        assert!(matches!(
            Module::parse(&bytes),
            Err(S3mError::Unsupported(_))
        ));
        let mut signed = build(&[0], &[0x80], None, &[pack(&[])]);
        signed[42] = 1;
        assert!(matches!(
            Module::parse(&signed),
            Err(S3mError::Unsupported(_))
        ));
    }

    #[test]
    fn truncated_patterns_are_malformed() {
        let mut pattern = pack(&[]);
        pattern.truncate(10);
        assert!(matches!(
            Module::parse(&build(&[0], &[0x80], None, &[pattern])),
            Err(S3mError::Malformed(_))
        ));
    }
}
