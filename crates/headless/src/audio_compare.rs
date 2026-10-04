//! `compare-audio`: does our render sound like a recording of the original (spec M1b §5)?
//! The two are aligned by their loudness envelopes, then compared second by second and octave
//! by octave. Waveforms are never compared: Wine resamples, and FMOD's mixer is not ours.

use crate::wav::Wav;

/// Envelope resolution: 10 ms windows.
const WINDOWS_PER_SECOND: usize = 100;
/// The furthest the two may be apart at the start.
const MAX_LAG_SECONDS: usize = 10;
/// Seconds quieter than this in both files are left out of the loudness comparison.
const SILENCE_DB: f64 = -50.0;
/// Octave band centres compared, in Hz.
pub const BANDS: [f64; 8] = [63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];
const FFT_SIZE: usize = 8192;
/// A tempo needs at least this many 10 s pieces that line up; with fewer it is not measured.
pub const MIN_TEMPO_PIECES: usize = 3;

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// How much later the original's content starts, in ms (negative: ours is later).
    pub lag_ms: f64,
    /// Correlation of the two loudness envelopes at that lag, -1..1.
    pub correlation: f64,
    pub overlap_seconds: f64,
    /// Per-second loudness difference (ours - original), in dB.
    pub loudness_median_db: f64,
    pub loudness_max_db: f64,
    /// Per band: centre in Hz, difference in dB.
    pub bands: Vec<(f64, f64)>,
    /// How much faster ours plays, in percent: the slope of the lag measured in each 10 s
    /// piece of the overlap (pieces that do not line up clearly are left out).
    pub tempo_percent: f64,
    /// The 10 s pieces the tempo was measured on.
    pub tempo_pieces: usize,
    /// How much higher ours sounds, in cents (100 = a semitone), from the spectra's best
    /// match on a logarithmic frequency axis.
    pub pitch_cents: f64,
    /// Stereo image: the left-to-right level difference of ours minus the original's, in dB,
    /// in the 10 s piece where it is largest. Mono measures cannot see panning.
    pub balance_db: f64,
    /// The 10 s pieces the balance was measured on.
    pub balance_pieces: usize,
}

/// Left level minus right level in dB over frames `from..to`, or `None` when the piece is
/// silent or the file is mono.
fn balance(wav: &Wav, from: usize, to: usize) -> Option<f64> {
    if wav.channels != 2 {
        return None;
    }
    let frames = &wav.samples.as_chunks::<2>().0
        [from.min(wav.samples.len() / 2)..to.min(wav.samples.len() / 2)];
    let power = |side: usize| {
        frames
            .iter()
            .map(|frame| (f64::from(frame[side]) / 32768.0).powi(2))
            .sum::<f64>()
            / frames.len().max(1) as f64
    };
    let (left, right) = (db(power(0)), db(power(1)));
    (left > SILENCE_DB && right > SILENCE_DB).then_some(left - right)
}

/// Mean square of each 10 ms window.
fn envelope(signal: &[f64], rate: u32) -> Vec<f64> {
    let window = (rate as usize / WINDOWS_PER_SECOND).max(1);
    signal
        .chunks_exact(window)
        .map(|chunk| chunk.iter().map(|s| s * s).sum::<f64>() / window as f64)
        .collect()
}

fn db(power: f64) -> f64 {
    10.0 * (power + 1e-12).log10()
}

/// The lag (in windows) at which `ours` best matches `original`, searched in `range`, and the
/// correlation there. `ours[i]` lines up with `original[i + lag]`. Lags that overlap less than
/// half of the shorter envelope are not considered: music repeats, and a short overlap can
/// match by chance.
fn best_lag(original: &[f64], ours: &[f64], range: std::ops::RangeInclusive<i64>) -> (i64, f64) {
    let needed = (original.len().min(ours.len()) / 2).max(WINDOWS_PER_SECOND);
    let a: Vec<f64> = original.iter().map(|&p| db(p)).collect();
    let b: Vec<f64> = ours.iter().map(|&p| db(p)).collect();
    let mut best = (0, f64::MIN);
    for lag in range {
        let pairs: Vec<(f64, f64)> = (0..b.len())
            .filter_map(|i| {
                let j = i as i64 + lag;
                (j >= 0 && (j as usize) < a.len()).then(|| (a[j as usize], b[i]))
            })
            .collect();
        if pairs.len() < needed {
            continue;
        }
        let n = pairs.len() as f64;
        let (mean_a, mean_b) = (
            pairs.iter().map(|p| p.0).sum::<f64>() / n,
            pairs.iter().map(|p| p.1).sum::<f64>() / n,
        );
        let (mut cov, mut var_a, mut var_b) = (0.0, 0.0, 0.0);
        for (x, y) in &pairs {
            cov += (x - mean_a) * (y - mean_b);
            var_a += (x - mean_a).powi(2);
            var_b += (y - mean_b).powi(2);
        }
        let correlation = cov / (var_a * var_b).sqrt().max(1e-12);
        if correlation > best.1 {
            best = (lag, correlation);
        }
    }
    best
}

/// Averaged power spectrum of `signal`: Hz per bin, and power per bin.
fn spectrum(signal: &[f64], rate: u32) -> (f64, Vec<f64>) {
    let hann: Vec<f64> = (0..FFT_SIZE)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / FFT_SIZE as f64).cos())
        .collect();
    let mut power = vec![0.0; FFT_SIZE / 2];
    let mut frames = 0;
    for chunk in signal.as_chunks::<FFT_SIZE>().0.iter().step_by(2) {
        let mut re: Vec<f64> = chunk.iter().zip(&hann).map(|(s, w)| s * w).collect();
        let mut im = vec![0.0; FFT_SIZE];
        fft(&mut re, &mut im);
        for (bin, slot) in power.iter_mut().enumerate() {
            *slot += re[bin] * re[bin] + im[bin] * im[bin];
        }
        frames += 1;
    }
    for slot in &mut power {
        *slot /= f64::from(frames.max(1)) * (FFT_SIZE * FFT_SIZE) as f64;
    }
    (f64::from(rate) / FFT_SIZE as f64, power)
}

/// Power per octave band, in dB (full scale = 0).
fn band_levels(bin_hz: f64, power: &[f64]) -> Vec<f64> {
    BANDS
        .iter()
        .map(|&centre| {
            let (low, high) = (centre / 2f64.sqrt(), centre * 2f64.sqrt());
            let energy: f64 = power
                .iter()
                .enumerate()
                .filter(|(bin, _)| {
                    let hz = *bin as f64 * bin_hz;
                    hz >= low && hz < high
                })
                .map(|(_, p)| p)
                .sum();
            db(energy)
        })
        .collect()
}

/// The logarithmic axis for the pitch comparison: 100 Hz to 4 kHz in 1-cent steps.
const PITCH_LOW_HZ: f64 = 100.0;
const PITCH_STEPS: usize = 6386;
/// The furthest pitch difference searched, in cents.
const PITCH_REACH: i64 = 200;

/// The spectrum in dB, resampled to 1-cent steps from [`PITCH_LOW_HZ`].
fn log_spectrum(bin_hz: f64, power: &[f64]) -> Vec<f64> {
    (0..PITCH_STEPS)
        .map(|step| {
            let hz = PITCH_LOW_HZ * 2f64.powf(step as f64 / 1200.0);
            let position = hz / bin_hz;
            let (low, fraction) = (position.floor() as usize, position.fract());
            let value = power.get(low).copied().unwrap_or(0.0) * (1.0 - fraction)
                + power.get(low + 1).copied().unwrap_or(0.0) * fraction;
            db(value)
        })
        .collect()
}

/// How many cents higher `ours` is than `theirs`: the shift that best lines up their
/// logarithmic spectra.
fn pitch_cents(theirs: &[f64], ours: &[f64]) -> f64 {
    let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len() as f64;
    let (mean_t, mean_o) = (mean(theirs), mean(ours));
    let best = (-PITCH_REACH..=PITCH_REACH)
        .map(|shift| {
            let score: f64 = (0..PITCH_STEPS as i64)
                .filter_map(|i| {
                    let j = i - shift;
                    (j >= 0 && j < PITCH_STEPS as i64)
                        .then(|| (ours[i as usize] - mean_o) * (theirs[j as usize] - mean_t))
                })
                .sum();
            (shift, score)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("a non-empty range");
    best.0 as f64
}

/// In-place radix-2 FFT.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut length = 2;
    while length <= n {
        let angle = -2.0 * std::f64::consts::PI / length as f64;
        for start in (0..n).step_by(length) {
            for k in 0..length / 2 {
                let (w_re, w_im) = ((angle * k as f64).cos(), (angle * k as f64).sin());
                let (a, b) = (start + k, start + k + length / 2);
                let (t_re, t_im) = (re[b] * w_re - im[b] * w_im, re[b] * w_im + im[b] * w_re);
                re[b] = re[a] - t_re;
                im[b] = im[a] - t_im;
                re[a] += t_re;
                im[a] += t_im;
            }
        }
        length <<= 1;
    }
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

pub fn compare(original: &Wav, ours: &Wav) -> Report {
    let (a, b) = (original.mono(), ours.mono());
    let (ea, eb) = (envelope(&a, original.rate), envelope(&b, ours.rate));
    let reach = (MAX_LAG_SECONDS * WINDOWS_PER_SECOND) as i64;
    let (lag, correlation) = best_lag(&ea, &eb, -reach..=reach);
    let start = (-lag).max(0) as usize;
    let end = eb.len().min((ea.len() as i64 - lag).max(0) as usize);
    let overlap = end.saturating_sub(start);
    // Per second of the overlap.
    let mut differences: Vec<f64> = (start..end)
        .step_by(WINDOWS_PER_SECOND)
        .filter(|&i| i + WINDOWS_PER_SECOND <= end)
        .filter_map(|i| {
            let ours: f64 =
                eb[i..i + WINDOWS_PER_SECOND].iter().sum::<f64>() / WINDOWS_PER_SECOND as f64;
            let j = (i as i64 + lag) as usize;
            let theirs: f64 =
                ea[j..j + WINDOWS_PER_SECOND].iter().sum::<f64>() / WINDOWS_PER_SECOND as f64;
            (db(ours) > SILENCE_DB || db(theirs) > SILENCE_DB).then(|| db(ours) - db(theirs))
        })
        .collect();
    let loudness_max_db = differences.iter().fold(0.0f64, |max, d| max.max(d.abs()));
    let mut absolute: Vec<f64> = differences.iter().map(|d| d.abs()).collect();
    let loudness_median_db = median(&mut absolute);
    differences.clear();
    // Bands over the overlap, each file at its own rate.
    let slice = |signal: &[f64], rate: u32, from: usize, to: usize| -> Vec<f64> {
        let per_window = rate as usize / WINDOWS_PER_SECOND;
        signal[(from * per_window).min(signal.len())..(to * per_window).min(signal.len())].to_vec()
    };
    let theirs_from = (start as i64 + lag) as usize;
    let (ours_bin, ours_power) = spectrum(&slice(&b, ours.rate, start, end), ours.rate);
    let (their_bin, their_power) = spectrum(
        &slice(&a, original.rate, theirs_from, theirs_from + overlap),
        original.rate,
    );
    let ours_bands = band_levels(ours_bin, &ours_power);
    let their_bands = band_levels(their_bin, &their_power);
    let pitch_cents = pitch_cents(
        &log_spectrum(their_bin, &their_power),
        &log_spectrum(ours_bin, &ours_power),
    );
    let bands = BANDS
        .iter()
        .zip(ours_bands.iter().zip(&their_bands))
        .map(|(&centre, (o, t))| (centre, o - t))
        .collect();
    // Tempo: the lag of every 10 s piece, then the least-squares slope of lag against time.
    let piece = 10 * WINDOWS_PER_SECOND;
    let points: Vec<(f64, f64)> = (start..end.saturating_sub(piece))
        .step_by(piece)
        .filter_map(|from| {
            let offset = from as i64;
            let (found, correlation) = best_lag(
                &ea,
                &eb[from..from + piece],
                offset + lag - 30..=offset + lag + 30,
            );
            (correlation >= 0.5).then(|| {
                (
                    (from + piece / 2) as f64 / WINDOWS_PER_SECOND as f64,
                    (found - offset) as f64 * 10.0,
                )
            })
        })
        .collect();
    let tempo_percent = if points.len() >= MIN_TEMPO_PIECES {
        let n = points.len() as f64;
        let (mean_x, mean_y) = (
            points.iter().map(|p| p.0).sum::<f64>() / n,
            points.iter().map(|p| p.1).sum::<f64>() / n,
        );
        let covariance: f64 = points
            .iter()
            .map(|(x, y)| (x - mean_x) * (y - mean_y))
            .sum();
        let variance: f64 = points.iter().map(|(x, _)| (x - mean_x).powi(2)).sum();
        // Milliseconds of lag per second: per mille; a later and later original means ours is faster.
        covariance / variance / 10.0
    } else {
        0.0
    };
    // Balance: each 10 s piece of the overlap, at each file's own rate.
    let frame = |window: usize, rate: u32| window * rate as usize / WINDOWS_PER_SECOND;
    let balances: Vec<f64> = (start..end.saturating_sub(piece - 1))
        .step_by(piece)
        .filter_map(|from| {
            let theirs_from = (from as i64 + lag) as usize;
            let theirs = balance(
                original,
                frame(theirs_from, original.rate),
                frame(theirs_from + piece, original.rate),
            )?;
            let ours = balance(ours, frame(from, ours.rate), frame(from + piece, ours.rate))?;
            Some(ours - theirs)
        })
        .collect();
    let balance_db =
        balances.iter().copied().fold(
            0.0f64,
            |largest, d| if d.abs() > largest.abs() { d } else { largest },
        );
    Report {
        lag_ms: lag as f64 * 10.0,
        correlation,
        overlap_seconds: overlap as f64 / WINDOWS_PER_SECOND as f64,
        loudness_median_db,
        loudness_max_db,
        bands,
        tempo_percent,
        tempo_pieces: points.len(),
        pitch_cents,
        balance_db,
        balance_pieces: balances.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tone whose loudness changes every 200 ms in a pattern that never repeats, so the
    /// envelopes align in one place only.
    fn tone(rate: u32, seconds: f64, delay: f64, gain: f64) -> Wav {
        let frames = (f64::from(rate) * seconds) as usize;
        let level_of = |segment: u64| {
            let mixed = segment
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407)
                >> 33;
            0.1 + 0.8 * (mixed % 1000) as f64 / 1000.0
        };
        let samples = (0..frames)
            .flat_map(|i| {
                let t = i as f64 / f64::from(rate) - delay;
                let level = if t < 0.0 {
                    0.0
                } else {
                    level_of((t * 5.0) as u64)
                };
                let value =
                    (level * gain * (2.0 * std::f64::consts::PI * 440.0 * t).sin() * 32767.0)
                        as i16;
                [value, value]
            })
            .collect();
        Wav {
            rate,
            channels: 2,
            samples,
        }
    }

    #[test]
    fn the_same_sound_at_another_rate_and_delay_compares_equal() {
        // The original records at 44.1 kHz and starts a little late; neither is a difference.
        let original = tone(44_100, 12.0, 0.5, 1.0);
        let ours = tone(48_000, 12.0, 0.0, 1.0);
        let report = compare(&original, &ours);
        assert!((report.lag_ms - 500.0).abs() <= 10.0, "{report:?}");
        assert!(report.loudness_median_db < 0.5, "{report:?}");
        assert!(report.tempo_percent.abs() <= 0.05, "{report:?}");
        let (_, at_440) = report.bands[3];
        assert!(at_440.abs() < 1.0, "{report:?}");
        assert!(report.pitch_cents.abs() <= 1.0, "{report:?}");
        assert!(report.balance_db.abs() <= 0.1, "{report:?}");
    }

    #[test]
    fn a_faster_render_shows_its_tempo_difference() {
        // The same pattern played 1 % faster: its pieces arrive earlier and earlier.
        let original = tone(44_100, 60.0, 0.0, 1.0);
        let faster = tone(48_000, 60.0, 0.0, 1.0);
        let squeezed: Vec<i16> = (0..(faster.samples.len() as f64 / 1.01) as usize / 2)
            .flat_map(|i| {
                let source = ((i as f64 * 1.01) as usize).min(faster.samples.len() / 2 - 1);
                [faster.samples[2 * source], faster.samples[2 * source + 1]]
            })
            .collect();
        let ours = Wav {
            rate: 48_000,
            channels: 2,
            samples: squeezed,
        };
        let report = compare(&original, &ours);
        assert!((report.tempo_percent - 1.0).abs() <= 0.1, "{report:?}");
        assert!(report.tempo_pieces >= 4, "{report:?}");
    }

    #[test]
    fn a_sharper_render_shows_its_pitch_difference() {
        // A semitone is 100 cents: 440 Hz against 466.16 Hz, same loudness pattern.
        let original = tone(44_100, 6.0, 0.0, 1.0);
        let semitone = 2f64.powf(1.0 / 12.0);
        let mut ours = tone(48_000, 6.0, 0.0, 1.0);
        for (i, frame) in ours.samples.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let t = i as f64 / 48_000.0;
            let value =
                (0.5 * (2.0 * std::f64::consts::PI * 440.0 * semitone * t).sin() * 32767.0) as i16;
            frame.fill(value);
        }
        let report = compare(&original, &ours);
        assert!((report.pitch_cents - 100.0).abs() <= 2.0, "{report:?}");
    }

    #[test]
    fn a_narrower_stereo_image_shows_as_a_balance_difference() {
        // Mono measures cannot see panning: a channel moved towards the centre sounds just as
        // loud in mono.
        let mut original = tone(44_100, 25.0, 0.0, 1.0);
        for frame in original.samples.as_chunks_mut::<2>().0 {
            frame[1] /= 2;
        }
        let ours = tone(48_000, 25.0, 0.0, 1.0);
        let report = compare(&original, &ours);
        assert!((report.balance_db + 6.0).abs() < 0.5, "{report:?}");
        assert_eq!(report.balance_pieces, 2, "{report:?}");
    }

    #[test]
    fn a_louder_render_shows_as_a_loudness_difference() {
        let original = tone(44_100, 6.0, 0.0, 0.5);
        let ours = tone(48_000, 6.0, 0.0, 1.0);
        let report = compare(&original, &ours);
        assert!((report.loudness_median_db - 6.0).abs() < 0.5, "{report:?}");
    }
}
