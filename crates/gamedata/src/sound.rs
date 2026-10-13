//! The sound files in `MUSICS.BPA` (spec M1b §3.1): which entry is music and which is a bank
//! of effects, and loading them.

use std::fmt;

use crate::bpa::{Archive, BpaError};
use crate::cmf;
use crate::s3m::{Module, S3mError};
use crate::xm::{Bank, Instrument, Looping, XmError};

/// The archive that holds every sound.
pub const ARCHIVE: &str = "MUSICS.BPA";

/// S3M music: the menus', every track's.
pub const MUSIC: [&str; 11] = [
    "MEN-MUS.CMF",
    "TR0-MUS.CMF",
    "TR1-MUS.CMF",
    "TR2-MUS.CMF",
    "TR3-MUS.CMF",
    "TR4-MUS.CMF",
    "TR5-MUS.CMF",
    "TR6-MUS.CMF",
    "TR7-MUS.CMF",
    "TR8-MUS.CMF",
    "TR9-MUS.CMF",
];

/// XM banks of effects: intro, end animations, races, menus.
pub const EFFECTS: [&str; 5] = [
    "SANIM-E.CMF",
    "ENDANI-E.CMF",
    "ENDANI0E.CMF",
    "GEN-EFE.CMF",
    "MEN-SAM.CMF",
];

#[derive(Debug)]
pub enum SoundError {
    Archive(BpaError),
    Music { name: String, error: S3mError },
    Effects { name: String, error: XmError },
}

impl fmt::Display for SoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SoundError::Archive(error) => write!(f, "{error}"),
            SoundError::Music { name, error } => write!(f, "{ARCHIVE}/{name}: {error}"),
            SoundError::Effects { name, error } => write!(f, "{ARCHIVE}/{name}: {error}"),
        }
    }
}

impl std::error::Error for SoundError {}

/// # Errors
///
/// [`SoundError`] when the entry is missing or is not a valid S3M module.
pub fn load_music(archive: &Archive, name: &str) -> Result<Module, SoundError> {
    let bytes = archive.read(name).map_err(SoundError::Archive)?;
    Module::parse(&cmf::decode(bytes)).map_err(|error| SoundError::Music {
        name: name.to_owned(),
        error,
    })
}

/// # Errors
///
/// [`SoundError`] when the entry is missing or is not a valid XM bank.
pub fn load_effects(archive: &Archive, name: &str) -> Result<Bank, SoundError> {
    let bytes = archive.read(name).map_err(SoundError::Archive)?;
    Bank::parse(&cmf::decode(bytes)).map_err(|error| SoundError::Effects {
        name: name.to_owned(),
        error,
    })
}

/// The race's calls (`GEN-EFE.CMF`) DeadRally's race record call is made of: "It's a new lap
/// record" (effect 4) and "Race over" (effect 5).
const LAP_RECORD_CALL: usize = 4;
const RACE_OVER_CALL: usize = 5;
/// The race record call's effect number, after the bank's 44.
pub const RACE_RECORD_CALL: u8 = 45;
/// The cuts, at zero crossings of the known release's samples: "It's a new" up to sample 9289
/// of the lap record's call, "race" from 779 to 8367 of the race over's, and "record" from
/// 13587 of the lap record's to its end.
const ITS_A_NEW: usize = 9289;
const RACE: (usize, usize) = (779, 8367);
const RECORD_FROM: usize = 13_587;
/// The race over's call plays two semitones above the lap record's (relative notes -17 and
/// -19), so its part is brought to the lap record's rate: 2^(2/12) of its samples a sample, in
/// 16.16 fixed point.
const RACE_STEP: u64 = 73_562;
/// At the lap record's 14065 Hz: the fade out of "It's a new" (6 ms), the fades of "race" (8 ms)
/// and the pause before "record" (40 ms).
const HEAD_FADE: usize = 84;
const RACE_FADE: usize = 112;
const PAUSE: usize = 563;

/// DeadRally's race record call, "It's a new race record" (the owner's choice), made at load
/// of the race's own calls in the announcer's voice: "It's a new" and "record" from the lap
/// record's call, "race" from the race over's. `None` when the bank lacks them or they are not the known release's.
pub fn race_record_call(bank: &Bank) -> Option<Instrument> {
    let lap = bank.instruments.get(LAP_RECORD_CALL - 1)?.as_ref()?;
    let over = bank.instruments.get(RACE_OVER_CALL - 1)?.as_ref()?;
    if lap.data.len() <= RECORD_FROM
        || over.data.len() < RACE.1
        || i16::from(over.relative_note) != i16::from(lap.relative_note) + 2
    {
        return None;
    }
    let mut data = lap.data[..ITS_A_NEW].to_vec();
    fade(&mut data, HEAD_FADE, false);
    let mut race = resample(&over.data[RACE.0..RACE.1], RACE_STEP);
    fade(&mut race, RACE_FADE, true);
    fade(&mut race, RACE_FADE, false);
    data.extend(race);
    data.extend(std::iter::repeat_n(0, PAUSE));
    data.extend_from_slice(&lap.data[RECORD_FROM..]);
    Some(Instrument {
        name: "Race Record".to_owned(),
        data,
        looping: Looping::None,
        volume: lap.volume,
        finetune: lap.finetune,
        relative_note: lap.relative_note,
        panning: lap.panning,
        fadeout: lap.fadeout,
    })
}

/// `samples` read `step` (16.16) apart, between two samples in a straight line.
fn resample(samples: &[i16], step: u64) -> Vec<i16> {
    let count = ((samples.len() as u64) << 16) / step;
    (0..count)
        .map(|i| {
            let at = i * step;
            let k = (at >> 16) as usize;
            let fraction = i64::try_from(at & 0xFFFF).unwrap_or(0);
            let a = i64::from(samples[k]);
            let b = i64::from(samples[(k + 1).min(samples.len() - 1)]);
            (a + (((b - a) * fraction) >> 16)) as i16
        })
        .collect()
}

/// The first (`rising`) or last `length` samples faded in a straight line.
fn fade(samples: &mut [i16], length: usize, rising: bool) {
    let length = length.min(samples.len());
    let count = samples.len();
    for i in 0..length {
        let at = if rising { i } else { count - 1 - i };
        let gain = i64::try_from(i).unwrap_or(0);
        let total = i64::try_from(length).unwrap_or(1);
        samples[at] = (i64::from(samples[at]) * gain / total) as i16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instrument(data: Vec<i16>, relative_note: i8) -> Option<Instrument> {
        Some(Instrument {
            name: String::new(),
            data,
            looping: Looping::None,
            volume: 64,
            finetune: 0,
            relative_note,
            panning: 128,
            fadeout: 0,
        })
    }

    fn bank(lap: Vec<i16>, over: Vec<i16>, over_note: i8) -> Bank {
        let mut instruments = vec![None; 44];
        instruments[LAP_RECORD_CALL - 1] = instrument(lap, -19);
        instruments[RACE_OVER_CALL - 1] = instrument(over, over_note);
        Bank {
            linear_frequencies: true,
            instruments,
        }
    }

    #[test]
    fn the_race_record_call_says_its_a_new_race_record_in_the_lap_record_s_voice() {
        // "It's a new" is the lap record call's start, "record" its end, and "race" the race over
        // call's start brought to the lap record's rate (two semitones lower), so that the
        // announcer says it in one voice at one pitch, at the lap record call's note.
        let lap: Vec<i16> = (0..21_833).map(|i| (i % 1000) as i16 + 1).collect();
        let over = vec![1000; 17_179];
        let call = race_record_call(&bank(lap.clone(), over, -17)).expect("a call");
        assert_eq!(call.relative_note, -19);
        assert_eq!(
            call.data[..ITS_A_NEW - HEAD_FADE],
            lap[..ITS_A_NEW - HEAD_FADE]
        );
        assert!(call.data.ends_with(&lap[RECORD_FROM..]));
        let race = call.data.len() - ITS_A_NEW - PAUSE - (lap.len() - RECORD_FROM);
        assert_eq!(race as u64, (((RACE.1 - RACE.0) as u64) << 16) / RACE_STEP);
        assert_eq!(
            call.data[ITS_A_NEW + race / 2],
            1000,
            "the race part, whole between its fades"
        );
        assert!(
            call.data[ITS_A_NEW + race..ITS_A_NEW + race + PAUSE]
                .iter()
                .all(|&v| v == 0)
        );
    }

    #[test]
    fn there_is_no_race_record_call_without_the_known_calls() {
        assert!(race_record_call(&bank(vec![1; 21_833], vec![1; 17_179], -19)).is_none());
        assert!(race_record_call(&bank(vec![1; 100], vec![1; 17_179], -17)).is_none());
        let empty = Bank {
            linear_frequencies: true,
            instruments: Vec::new(),
        };
        assert!(race_record_call(&empty).is_none());
    }
}
