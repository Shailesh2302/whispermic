//! Mic capture @16kHz mono f32 via cpal.
//! Lock-free: audio callback pushes into a bounded channel, pipeline drains in realtime.
//! Resampling: if device runs at 44.1/48kHz we do a cheap linear decimation to 16k.

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::Sender;

pub const TARGET_RATE: u32 = 16_000;
pub const FRAME: usize = 480; // 30ms @16kHz — matches VAD frame

pub struct MicStream {
    _stream: cpal::Stream,
}

pub fn start_capture(sample_tx: Sender<Vec<f32>>, level_tx: Sender<f32>) -> Result<MicStream> {
    let host = cpal::default_host();
    let device = host.default_input_device().context("no input device found")?;
    let supported = device
        .default_input_config()
        .context("no default input config")?;

    // We request f32 mono; cpal converts if needed.
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: supported.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };
    let in_rate = config.sample_rate.0;
    // stride = round(in_rate / 16000), e.g. 48000 -> 3
    let stride = (in_rate / TARGET_RATE).max(1) as usize;

    let mut carry: Vec<f32> = Vec::with_capacity(FRAME * 2);

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // mono-ize is already 1ch; decimate to 16k
            for (i, &s) in data.iter().enumerate() {
                if i % stride == 0 {
                    carry.push(s.clamp(-1.0, 1.0));
                    if carry.len() == FRAME {
                        let frame = std::mem::replace(&mut carry, Vec::with_capacity(FRAME * 2));
                        let lvl: f32 = frame.iter().map(|x| x * x).sum::<f32>() / FRAME as f32;
                        let _ = level_tx.try_send(lvl.sqrt());
                        let _ = sample_tx.try_send(frame);
                    }
                }
            }
        },
        |err| log::error!("audio stream error: {}", err),
        None,
    )?;
    stream.play()?;
    Ok(MicStream { _stream: stream })
}
