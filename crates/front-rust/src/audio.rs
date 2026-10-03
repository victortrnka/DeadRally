use std::collections::VecDeque;
use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, FromSample, SampleFormat, SizedSample, StreamConfig};
use deadrally_core::{AUDIO_CHANNELS, AUDIO_SAMPLE_RATE};

/// Interleaved stereo samples waiting to be played; the audio thread pops from the front.
pub type SampleQueue = Arc<Mutex<VecDeque<i16>>>;

/// Opens the default output device at 44.1 kHz stereo and starts playing from `queue`.
/// Missing samples are played as silence. cpal does not resample, so a device that cannot run
/// at 44.1 kHz is an error.
pub fn start(queue: SampleQueue) -> Result<cpal::Stream, Box<dyn Error>> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no audio output device")?;
    let channels = u16::try_from(AUDIO_CHANNELS)?;
    let format = device
        .supported_output_configs()?
        .filter(|config| {
            config.channels() == channels
                && config.min_sample_rate() <= AUDIO_SAMPLE_RATE
                && config.max_sample_rate() >= AUDIO_SAMPLE_RATE
        })
        .map(|config| config.sample_format())
        .min_by_key(|format| match format {
            SampleFormat::I16 => 0,
            SampleFormat::F32 => 1,
            _ => 2,
        })
        .ok_or("the audio device cannot play 44.1 kHz stereo")?;
    let config = StreamConfig {
        channels,
        sample_rate: AUDIO_SAMPLE_RATE,
        buffer_size: BufferSize::Default,
    };
    let stream = match format {
        SampleFormat::I16 => build::<i16>(&device, config, queue)?,
        SampleFormat::F32 => build::<f32>(&device, config, queue)?,
        other => return Err(format!("unsupported sample format {other}").into()),
    };
    stream.play()?;
    Ok(stream)
}

fn build<T: SizedSample + FromSample<i16>>(
    device: &cpal::Device,
    config: StreamConfig,
    queue: SampleQueue,
) -> Result<cpal::Stream, cpal::Error> {
    device.build_output_stream(
        config,
        move |out: &mut [T], _| {
            let mut queue = queue.lock().unwrap_or_else(PoisonError::into_inner);
            for slot in out.iter_mut() {
                *slot = T::from_sample(queue.pop_front().unwrap_or(0));
            }
        },
        |error| eprintln!("audio error: {error}"),
        None,
    )
}
