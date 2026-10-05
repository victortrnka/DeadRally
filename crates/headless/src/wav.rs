//! 16-bit PCM WAV files: what `render-audio` writes and what the reference runner records.

use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wav {
    pub rate: u32,
    pub channels: u16,
    /// Interleaved samples.
    pub samples: Vec<i16>,
}

impl Wav {
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let data_bytes = u32::try_from(self.samples.len() * 2)
            .map_err(|_| format!("{}: too long for a WAV file", path.display()))?;
        let block = self.channels * 2;
        let mut bytes = Vec::with_capacity(44 + data_bytes as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&self.channels.to_le_bytes());
        bytes.extend_from_slice(&self.rate.to_le_bytes());
        bytes.extend_from_slice(&(self.rate * u32::from(block)).to_le_bytes());
        bytes.extend_from_slice(&block.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_bytes.to_le_bytes());
        for sample in &self.samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        std::fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Reads a 16-bit PCM WAV. A data chunk whose length was never filled in (a recording
    /// stopped by a signal) runs to the end of the file.
    pub fn read(path: &Path) -> Result<Wav, String> {
        let fail = |problem: &str| format!("{}: {problem}", path.display());
        let bytes = std::fs::read(path).map_err(|error| fail(&error.to_string()))?;
        if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(fail("not a WAV file"));
        }
        let (mut position, mut format) = (12, None);
        while position + 8 <= bytes.len() {
            let id = &bytes[position..position + 4];
            let size = u32::from_le_bytes(
                bytes[position + 4..position + 8]
                    .try_into()
                    .expect("4 bytes"),
            ) as usize;
            let body = position + 8;
            if id == b"fmt " {
                let field =
                    |at: usize| u16::from_le_bytes([bytes[body + at], bytes[body + at + 1]]);
                if size < 16 || body + 16 > bytes.len() {
                    return Err(fail("short fmt chunk"));
                }
                let rate =
                    u32::from_le_bytes(bytes[body + 4..body + 8].try_into().expect("4 bytes"));
                if field(0) != 1 || field(14) != 16 {
                    return Err(fail("only 16-bit PCM is supported"));
                }
                format = Some((field(2), rate));
            } else if id == b"data" {
                let (channels, rate) = format.ok_or_else(|| fail("data before fmt"))?;
                let end = body.saturating_add(size).min(bytes.len());
                let samples = bytes[body..end]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| i16::from_le_bytes(*pair))
                    .collect();
                return Ok(Wav {
                    rate,
                    channels,
                    samples,
                });
            }
            position = body + size + (size & 1);
        }
        Err(fail("no data chunk"))
    }

    /// The left and right channels averaged, as f64 in -1..1.
    pub fn mono(&self) -> Vec<f64> {
        let channels = usize::from(self.channels.max(1));
        self.samples
            .chunks_exact(channels)
            .map(|frame| {
                frame.iter().map(|&s| f64::from(s)).sum::<f64>() / (channels as f64 * 32768.0)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_files_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.wav");
        let wav = Wav {
            rate: 48_000,
            channels: 2,
            samples: vec![1, -1, i16::MAX, i16::MIN],
        };
        wav.write(&path).unwrap();
        assert_eq!(Wav::read(&path).unwrap(), wav);
    }

    #[test]
    fn a_recording_without_its_final_length_still_reads() {
        // parec stopped by a signal leaves the data length at its placeholder.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rec.wav");
        Wav {
            rate: 44_100,
            channels: 2,
            samples: vec![5, 6, 7, 8],
        }
        .write(&path)
        .unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(&path, bytes).unwrap();
        assert_eq!(Wav::read(&path).unwrap().samples, [5, 6, 7, 8]);
    }
}
