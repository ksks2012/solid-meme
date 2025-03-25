use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use hound::WavSpec;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub struct WaveformData {
    pub samples_raw: Vec<i16>, // Only keep i16 data
    pub current_idx: Arc<Mutex<usize>>,
    pub playing_stream: Option<Arc<cpal::Stream>>,
    pub silence_segments: Vec<(usize, usize)>,
}

impl WaveformData {
    pub fn new() -> Self {
        Self {
            samples_raw: Vec::new(),
            current_idx: Arc::new(Mutex::new(0)),
            playing_stream: None,
            silence_segments: Vec::new(),
        }
    }

    pub fn from_samples(samples_raw: Vec<i16>) -> Self {
        Self {
            samples_raw,
            current_idx: Arc::new(Mutex::new(0)),
            playing_stream: None,
            silence_segments: Vec::new(),
        }
    }
}

pub fn play_samples(
    stream: &mut Option<Arc<cpal::Stream>>,
    samples: Vec<i16>,
    spec: WavSpec,
    current_idx: &Arc<Mutex<usize>>,
) {
    let sample_len = samples.len();
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available");
    let config = cpal::StreamConfig {
        channels: spec.channels,
        sample_rate: cpal::SampleRate(spec.sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let samples = samples.clone();
    let current_idx = Arc::clone(current_idx);
    *current_idx.lock().unwrap() = 0;

    let audio_stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut idx = current_idx.lock().unwrap();
                for frame in data.chunks_mut(spec.channels as usize) {
                    for sample in frame {
                        if *idx < sample_len {
                            *sample = samples[*idx] as f32 / i16::MAX as f32;
                            *idx += 1;
                        } else {
                            *sample = 0.0;
                        }
                    }
                }
            },
            |err| eprintln!("Audio error: {}", err),
            None,
        )
        .expect("Failed to build output stream");

    audio_stream.play().expect("Failed to play stream");

    let audio_stream = Arc::new(audio_stream);
    *stream = Some(audio_stream.clone());

    thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(
            (sample_len as u64 * 1000) / (spec.sample_rate as u64 * spec.channels as u64),
        ));
    });
}