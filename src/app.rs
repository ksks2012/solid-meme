use cpal::traits::StreamTrait;
use crate::audio::{play_samples, WaveformData};
use hound::{WavReader, WavWriter};
use rfd::FileDialog;
use std::sync::Arc;

pub struct SoundApp {
    pub raw_waveform: WaveformData,
    pub processed_waveform: WaveformData,
    pub spec: Option<hound::WavSpec>,
    pub file_loaded: bool,
    pub playing_stream: Option<Arc<cpal::Stream>>,
    pub zoom: f32,
    pub offset: f32,
    pub playing_original: bool,
}

impl SoundApp {
    pub fn new() -> Self {
        Self {
            raw_waveform: WaveformData::new(),
            processed_waveform: WaveformData::new(),
            spec: None,
            file_loaded: false,
            playing_stream: None,
            zoom: 1.0,
            offset: 0.0,
            playing_original: false,
        }
    }

    pub fn load_file(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
            if let Ok(mut reader) = WavReader::open(&path) {
                let spec = reader.spec();
                let raw_samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
                println!("Loaded raw samples count: {}", raw_samples.len());
                let samples_f32: Vec<f32> = raw_samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                self.raw_waveform = WaveformData::from_samples(raw_samples.clone(), samples_f32.clone());
                self.processed_waveform = WaveformData::from_samples(raw_samples, samples_f32);
                self.spec = Some(spec);
                self.file_loaded = true;
                self.zoom = 1.0;
                self.offset = 0.0;
            }
        }
    }

    pub fn remove_silence(&mut self, silence_threshold: f32, min_silence_len: usize) {
        if !self.file_loaded || self.spec.is_none() {
            return;
        }
        let spec = self.spec.unwrap();
        let channels = spec.channels as usize;
        let sample_rate = spec.sample_rate as usize;
        let total_samples = self.processed_waveform.samples_raw.len();

        let mut result_samples = Vec::new();
        let mut silence_count = 0;

        for i in (0..total_samples).step_by(channels) {
            let mut frame_amplitude = 0.0;
            for ch in 0..channels {
                let sample = self.processed_waveform.samples_raw[i + ch] as f32;
                frame_amplitude += sample.abs() / i16::MAX as f32;
            }
            frame_amplitude /= channels as f32;

            if frame_amplitude < silence_threshold {
                silence_count += 1;
            } else {
                if silence_count < min_silence_len / (sample_rate / 1000) {
                    for _ in 0..silence_count {
                        for _ in 0..channels {
                            result_samples.push(0);
                        }
                    }
                }
                silence_count = 0;
                for ch in 0..channels {
                    result_samples.push(self.processed_waveform.samples_raw[i + ch]);
                }
            }
        }

        self.processed_waveform.samples_raw = result_samples;
        self.processed_waveform.samples = self.processed_waveform.samples_raw.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
    }

    pub fn save_file(&self) {
        if let Some(spec) = self.spec {
            if let Some(path) = FileDialog::new()
                .add_filter("WAV", &["wav"])
                .set_file_name("output.wav")
                .save_file()
            {
                if let Ok(mut writer) = WavWriter::create(&path, spec) {
                    for &sample in &self.processed_waveform.samples_raw {
                        writer.write_sample(sample).unwrap();
                    }
                    writer.finalize().unwrap();
                    println!("Saved to {:?}", path);
                }
            }
        }
    }

    pub fn play_original(&mut self) {
        if self.file_loaded && self.spec.is_some() {
            let samples = self.raw_waveform.samples_raw.clone();
            let spec = self.spec.unwrap();
            println!("Playing original samples count: {}", samples.len());
            self.playing_original = true;
            play_samples(&mut self.playing_stream, samples, spec, &self.raw_waveform.current_idx);
        } else {
            println!("No file loaded, cannot play original audio");
        }
    }

    pub fn play_processed(&mut self) {
        if self.file_loaded && self.spec.is_some() {
            let samples = self.processed_waveform.samples_raw.clone();
            let spec = self.spec.unwrap();
            println!("Playing processed samples count: {}", samples.len());
            self.playing_original = false;
            play_samples(&mut self.playing_stream, samples, spec, &self.processed_waveform.current_idx);
        } else {
            println!("No file loaded, cannot play processed audio");
        }
    }

    pub fn stop_playback(&mut self) {
        self.playing_stream = None;
        *self.raw_waveform.current_idx.lock().unwrap() = 0;
        *self.processed_waveform.current_idx.lock().unwrap() = 0;
    }

    pub fn pause_playback(&mut self) {
        if let Some(stream) = &self.playing_stream {
            stream.pause().expect("Failed to pause stream");
        }
    }

    pub fn resume_playback(&mut self) {
        if let Some(stream) = &self.playing_stream {
            stream.play().expect("Failed to resume stream");
        }
    }

    pub fn jump_to_position(&mut self, sample_idx: usize) {
        let current_idx = if self.playing_original {
            &self.raw_waveform.current_idx
        } else {
            &self.processed_waveform.current_idx
        };
        *current_idx.lock().unwrap() = sample_idx.min(if self.playing_original {
            self.raw_waveform.samples_raw.len()
        } else {
            self.processed_waveform.samples_raw.len()
        });
    }
}