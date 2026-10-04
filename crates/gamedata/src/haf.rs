//! HAF animations: the intro (`SANIM.haf`) and the two end animations (spec M1a §3.5).

use std::fmt;
use std::path::{Path, PathBuf};

use crate::image::{PALETTE_BYTES, Palette, PaletteError};
use crate::lzw::{self, LzwError};

pub const FRAME_WIDTH: u32 = 320;
pub const FRAME_HEIGHT: u32 = 120;
/// Every frame is a full 320x120 picture.
pub const FRAME_PIXELS: usize = (FRAME_WIDTH * FRAME_HEIGHT) as usize;

const GIF_TRAILER: u8 = 0x3B;
const LZW_MINIMUM_CODE_SIZE: u8 = 8;

/// One decoded frame: its palette (the game uses entries 16..=255) and 320x120 pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HafFrame {
    pub palette: Palette,
    pub pixels: Vec<u8>,
}

/// An animation. Frames from a file are decoded on request: the intro is 21 MB.
#[derive(Debug)]
pub struct Animation {
    path: PathBuf,
    /// Effect to trigger with each frame (0 = none); played from M1b on.
    pub effects: Vec<u8>,
    /// Ticks between the previous frame and this one.
    pub delays: Vec<u8>,
    frames: Frames,
}

#[derive(Debug)]
enum Frames {
    /// The file's bytes and each frame record's byte range.
    Encoded {
        data: Vec<u8>,
        ranges: Vec<(usize, usize)>,
    },
    /// Already decoded, for tests and tools.
    Decoded(Vec<HafFrame>),
}

#[derive(Debug)]
pub enum HafError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Malformed {
        path: PathBuf,
        problem: String,
    },
    Frame {
        path: PathBuf,
        index: usize,
        problem: String,
    },
}

impl fmt::Display for HafError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HafError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            HafError::Malformed { path, problem } => {
                write!(
                    f,
                    "{} is not a valid HAF animation: {problem}",
                    path.display()
                )
            }
            HafError::Frame {
                path,
                index,
                problem,
            } => {
                write!(f, "{} frame {index}: {problem}", path.display())
            }
        }
    }
}

impl std::error::Error for HafError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HafError::Read { source, .. } => Some(source),
            HafError::Malformed { .. } | HafError::Frame { .. } => None,
        }
    }
}

impl Animation {
    /// Reads the file and its frame table; no frame is decoded yet.
    ///
    /// # Errors
    ///
    /// [`HafError`] when the file cannot be read or its records do not fill it exactly.
    pub fn open(path: &Path) -> Result<Animation, HafError> {
        let data = std::fs::read(path).map_err(|source| HafError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Animation::from_bytes(path.to_path_buf(), data)
    }

    /// Like [`Animation::open`] for bytes already in memory; `path` is only used in messages.
    ///
    /// # Errors
    ///
    /// [`HafError::Malformed`] when the records do not fill the data exactly.
    pub fn from_bytes(path: PathBuf, data: Vec<u8>) -> Result<Animation, HafError> {
        let malformed = |problem: String| HafError::Malformed {
            path: path.clone(),
            problem,
        };
        if data.len() < 2 {
            return Err(malformed("no frame count".into()));
        }
        let count = usize::from(u16::from_le_bytes([data[0], data[1]]));
        let table_end = 2 + 2 * count;
        if data.len() < table_end {
            return Err(malformed(format!(
                "{count} frames announced, the tables do not fit"
            )));
        }
        let effects = data[2..2 + count].to_vec();
        let delays = data[2 + count..table_end].to_vec();
        let mut ranges = Vec::with_capacity(count);
        let mut position = table_end;
        for index in 0..count {
            if position + 2 > data.len() {
                return Err(malformed(format!("frame {index} of {count} is missing")));
            }
            let length = usize::from(u16::from_le_bytes([data[position], data[position + 1]]));
            let start = position + 2;
            if start + length > data.len() {
                return Err(malformed(format!("frame {index} runs past the end")));
            }
            ranges.push((start, start + length));
            position = start + length;
        }
        if position != data.len() {
            return Err(malformed(format!(
                "{} bytes after the last frame",
                data.len() - position
            )));
        }
        Ok(Animation {
            path,
            effects,
            delays,
            frames: Frames::Encoded { data, ranges },
        })
    }

    /// An animation from frames already decoded; every frame gets no effect.
    ///
    /// # Panics
    ///
    /// If `delays` and `frames` differ in length.
    #[must_use]
    pub fn from_frames(delays: Vec<u8>, frames: Vec<HafFrame>) -> Animation {
        assert_eq!(delays.len(), frames.len(), "one delay per frame");
        Animation {
            path: PathBuf::from("(decoded frames)"),
            effects: vec![0; frames.len()],
            delays,
            frames: Frames::Decoded(frames),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.delays.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.delays.is_empty()
    }

    /// Decodes frame `index`.
    ///
    /// # Errors
    ///
    /// [`HafError::Frame`] when the record is corrupt.
    ///
    /// # Panics
    ///
    /// If `index >= len()`.
    pub fn frame(&self, index: usize) -> Result<HafFrame, HafError> {
        match &self.frames {
            Frames::Encoded { data, ranges } => {
                let (start, end) = ranges[index];
                decode_frame(&data[start..end]).map_err(|problem| HafError::Frame {
                    path: self.path.clone(),
                    index,
                    problem,
                })
            }
            Frames::Decoded(frames) => Ok(frames[index].clone()),
        }
    }
}

/// A frame record: palette, LZW minimum code size, GIF sub-blocks, trailer.
fn decode_frame(record: &[u8]) -> Result<HafFrame, String> {
    if record.len() < PALETTE_BYTES + 2 {
        return Err(format!("{} bytes is too short for a frame", record.len()));
    }
    let palette = Palette::from_bytes(&record[..PALETTE_BYTES])
        .map_err(|error: PaletteError| error.to_string())?;
    if record[PALETTE_BYTES] != LZW_MINIMUM_CODE_SIZE {
        return Err(format!(
            "LZW minimum code size {}, expected 8",
            record[PALETTE_BYTES]
        ));
    }
    let mut stream = Vec::with_capacity(record.len());
    let mut position = PALETTE_BYTES + 1;
    loop {
        let Some(&length) = record.get(position) else {
            return Err("the sub-blocks run past the end of the frame".into());
        };
        position += 1;
        if length == 0 {
            break;
        }
        let end = position + usize::from(length);
        let block = record
            .get(position..end)
            .ok_or("a sub-block runs past the end of the frame")?;
        stream.extend_from_slice(block);
        position = end;
    }
    if record.get(position) != Some(&GIF_TRAILER) {
        return Err("missing the 0x3B trailer".into());
    }
    let mut pixels = Vec::with_capacity(FRAME_PIXELS);
    match lzw::decode(&stream, lzw::GIF, FRAME_PIXELS, &mut pixels) {
        // ENDANI.haf frame 200 writes its end code at the wrong width; the frame is complete
        // anyway, and the original stops there too.
        Ok(_) | Err(LzwError::UnexpectedEnd) if pixels.len() == FRAME_PIXELS => {
            Ok(HafFrame { palette, pixels })
        }
        Ok(_) => Err(format!(
            "{} pixels, a frame has {FRAME_PIXELS}",
            pixels.len()
        )),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame whose pixels are all literals, re-cleared every 250 codes so codes stay 9 bits.
    fn frame_record(pixel: u8, pixel_count: usize) -> Vec<u8> {
        let mut codes = vec![256u16];
        for index in 0..pixel_count {
            if index > 0 && index % 250 == 0 {
                codes.push(256);
            }
            codes.push(u16::from(pixel));
        }
        codes.push(257);
        let mut stream = Vec::new();
        let (mut accumulator, mut bits) = (0u32, 0u32);
        for code in codes {
            accumulator |= u32::from(code) << bits;
            bits += 9;
            while bits >= 8 {
                stream.push((accumulator & 0xFF) as u8);
                accumulator >>= 8;
                bits -= 8;
            }
        }
        if bits > 0 {
            stream.push((accumulator & 0xFF) as u8);
        }
        let mut record = vec![1u8; PALETTE_BYTES];
        record.push(LZW_MINIMUM_CODE_SIZE);
        for block in stream.chunks(255) {
            record.push(u8::try_from(block.len()).unwrap());
            record.extend_from_slice(block);
        }
        record.push(0);
        record.push(GIF_TRAILER);
        record
    }

    fn animation(records: &[Vec<u8>], delays: &[u8]) -> Vec<u8> {
        let mut data = u16::try_from(records.len()).unwrap().to_le_bytes().to_vec();
        data.extend(std::iter::repeat_n(0, records.len()));
        data.extend_from_slice(delays);
        for record in records {
            data.extend_from_slice(&u16::try_from(record.len()).unwrap().to_le_bytes());
            data.extend_from_slice(record);
        }
        data
    }

    fn parse(data: Vec<u8>) -> Result<Animation, HafError> {
        Animation::from_bytes(PathBuf::from("TEST.HAF"), data)
    }

    #[test]
    fn frames_decode_to_a_full_320x120_picture() {
        let anim = parse(animation(&[frame_record(17, FRAME_PIXELS)], &[4])).unwrap();
        assert_eq!(anim.len(), 1);
        assert_eq!(anim.delays, [4]);
        let frame = anim.frame(0).unwrap();
        assert_eq!(frame.pixels.len(), FRAME_PIXELS);
        assert!(frame.pixels.iter().all(|&pixel| pixel == 17));
        assert_eq!(frame.palette.0[0], [1, 1, 1]);
    }

    #[test]
    fn a_short_frame_is_an_error() {
        // A frame that leaves part of the screen undrawn would show garbage from the last one.
        let anim = parse(animation(&[frame_record(17, 100)], &[4])).unwrap();
        assert!(
            anim.frame(0)
                .unwrap_err()
                .to_string()
                .contains("100 pixels")
        );
    }

    #[test]
    fn records_must_fill_the_file_exactly() {
        let mut data = animation(&[frame_record(17, FRAME_PIXELS)], &[4]);
        data.push(0);
        assert!(matches!(parse(data), Err(HafError::Malformed { .. })));
        let mut data = animation(&[frame_record(17, FRAME_PIXELS)], &[4]);
        data.truncate(data.len() - 1);
        assert!(matches!(parse(data), Err(HafError::Malformed { .. })));
    }

    #[test]
    fn a_missing_trailer_is_an_error() {
        let mut record = frame_record(17, FRAME_PIXELS);
        *record.last_mut().unwrap() = 0;
        let anim = parse(animation(&[record], &[4])).unwrap();
        assert!(anim.frame(0).unwrap_err().to_string().contains("trailer"));
    }
}
