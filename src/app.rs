use cpal::traits::StreamTrait;
use crate::audio::{play_samples, WaveformData};
use hound::{WavReader, WavWriter};
use rfd::FileDialog;
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub struct SoundApp {
    pub raw_waveform: WaveformData,
    pub processed_waveform: WaveformData,
    pub spec: Option<hound::WavSpec>,
    pub file_loaded: bool,
    pub zoom: f32,
    pub offset: f32,
    pub processed_ready: bool,
    pub silence_threshold: f32,
    pub min_silence_len: usize,
    pub is_processing: bool,
    pub processing_progress: f32,
    pub progress_rx: Option<Receiver<f32>>, // Persistent receiver for progress
    pub result_rx: Option<Receiver<(Vec<(usize, usize)>, Option<Vec<i16>>)>>, // Persistent receiver for results
}

impl SoundApp {
    pub fn new() -> Self {
        Self {
            raw_waveform: WaveformData::new(),
            processed_waveform: WaveformData::new(),
            spec: None,
            file_loaded: false,
            zoom: 1.0,
            offset: 0.0,
            processed_ready: false,
            silence_threshold: 0.01,
            min_silence_len: 1000,
            is_processing: false,
            processing_progress: 0.0,
            progress_rx: None,
            result_rx: None,
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
                self.processed_ready = false;
                self.is_processing = false;
                self.processing_progress = 0.0;
            }
        }
    }

    pub fn detect_silence_background(&mut self) {
        if self.is_processing || !self.file_loaded || self.spec.is_none() {
            return;
        }
        self.is_processing = true;
        self.processing_progress = 0.0;

        let (progress_tx, progress_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        self.progress_rx = Some(progress_rx);
        self.result_rx = Some(result_rx);

        let spec = self.spec.unwrap();
        let channels = spec.channels as usize;
        let sample_rate = spec.sample_rate as usize;
        let samples = self.raw_waveform.samples_raw.clone();
        let total_samples = samples.len();
        let threshold = self.silence_threshold;
        let min_len = self.min_silence_len;

        thread::spawn(move || {
            let mut silence_segments = Vec::new();
            let mut silence_count = 0;
            let mut silence_start = 0;

            for i in (0..total_samples).step_by(channels) {
                let mut frame_amplitude = 0.0;
                for ch in 0..channels {
                    if i + ch < total_samples {
                        let sample = samples[i + ch] as f32;
                        frame_amplitude += sample.abs() / i16::MAX as f32;
                    }
                }
                frame_amplitude /= channels as f32;

                if frame_amplitude < threshold {
                    if silence_count == 0 {
                        silence_start = i;
                    }
                    silence_count += 1;
                } else if silence_count > 0 {
                    let min_samples = min_len * sample_rate / 1000;
                    if silence_count >= min_samples {
                        silence_segments.push((silence_start, i));
                    }
                    silence_count = 0;
                }

                let progress = i as f32 / total_samples as f32;
                if (progress * 100.0) as usize % 1 == 0 { // Update every 1%
                    let _ = progress_tx.send(progress); // Ignore send failure
                }
            }

            if silence_count >= min_len * sample_rate / 1000 {
                silence_segments.push((silence_start, total_samples));
            }

            let _ = result_tx.send((silence_segments, None)); // Ignore send failure
        });
    }

    pub fn remove_all_silence_background(&mut self) {
        if self.is_processing || !self.file_loaded || self.spec.is_none() {
            return;
        }
        self.is_processing = true;
        self.processing_progress = 0.0;

        let (progress_tx, progress_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        self.progress_rx = Some(progress_rx);
        self.result_rx = Some(result_rx);

        let spec = self.spec.unwrap();
        let channels = spec.channels as usize;
        let sample_rate = spec.sample_rate as usize;
        let samples = self.raw_waveform.samples_raw.clone();
        let total_samples = samples.len();
        let threshold = self.silence_threshold;
        let min_len = self.min_silence_len;

        thread::spawn(move || {
            let mut silence_segments = Vec::new();
            let mut silence_count = 0;
            let mut silence_start = 0;
            let mut result_samples = Vec::new();
            let mut last_end = 0;

            for i in (0..total_samples).step_by(channels) {
                let mut frame_amplitude = 0.0;
                for ch in 0..channels {
                    if i + ch < total_samples {
                        let sample = samples[i + ch] as f32;
                        frame_amplitude += sample.abs() / i16::MAX as f32;
                    }
                }
                frame_amplitude /= channels as f32;

                if frame_amplitude < threshold {
                    if silence_count == 0 {
                        silence_start = i;
                    }
                    silence_count += 1;
                } else if silence_count > 0 {
                    let min_samples = min_len * sample_rate / 1000;
                    if silence_count >= min_samples {
                        silence_segments.push((silence_start, i));
                        for j in (last_end..silence_start).step_by(channels) {
                            for ch in 0..channels {
                                if j + ch < total_samples {
                                    result_samples.push(samples[j + ch]);
                                }
                            }
                        }
                        last_end = i;
                    }
                    silence_count = 0;
                }

                let progress = i as f32 / total_samples as f32;
                if (progress * 100.0) as usize % 1 == 0 {
                    let _ = progress_tx.send(progress);
                }
            }

            if silence_count >= min_len * sample_rate / 1000 {
                silence_segments.push((silence_start, total_samples));
            }

            for i in (last_end..total_samples).step_by(channels) {
                for ch in 0..channels {
                    if i + ch < total_samples {
                        result_samples.push(samples[i + ch]);
                    }
                }
            }

            let _ = result_tx.send((silence_segments, Some(result_samples)));
        });
    }

    pub fn update_processing(&mut self) {
        if let Some(ref rx) = self.progress_rx {
            while let Ok(progress) = rx.try_recv() {
                self.processing_progress = progress;
            }
        }
        if let Some(ref rx) = self.result_rx {
            if let Ok((silence_segments, result_samples)) = rx.try_recv() {
                self.raw_waveform.silence_segments = silence_segments;
                if let Some(samples) = result_samples {
                    self.processed_waveform.samples_raw = samples;
                    self.processed_waveform.samples = self.processed_waveform.samples_raw.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    self.processed_ready = true;
                }
                self.is_processing = false;
                self.progress_rx = None;
                self.result_rx = None;
            }
        }
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
            if let Some(stream) = &self.processed_waveform.playing_stream {
                stream.pause().expect("Failed to pause processed stream");
            }
            let samples = self.raw_waveform.samples_raw.clone();
            let spec = self.spec.unwrap();
            println!("Playing original samples count: {}", samples.len());
            play_samples(&mut self.raw_waveform.playing_stream, samples, spec, &self.raw_waveform.current_idx);
        }
    }

    pub fn play_processed(&mut self) {
        if self.file_loaded && self.spec.is_some() && self.processed_ready {
            if let Some(stream) = &self.raw_waveform.playing_stream {
                stream.pause().expect("Failed to pause original stream");
            }
            let samples = self.processed_waveform.samples_raw.clone();
            let spec = self.spec.unwrap();
            println!("Playing processed samples count: {}", samples.len());
            play_samples(&mut self.processed_waveform.playing_stream, samples, spec, &self.processed_waveform.current_idx);
        }
    }

    pub fn pause_original(&mut self) {
        if let Some(stream) = &self.raw_waveform.playing_stream {
            stream.pause().expect("Failed to pause original stream");
        }
    }

    pub fn pause_processed(&mut self) {
        if let Some(stream) = &self.processed_waveform.playing_stream {
            stream.pause().expect("Failed to pause processed stream");
        }
    }

    pub fn resume_original(&mut self) {
        if let Some(stream) = &self.processed_waveform.playing_stream {
            stream.pause().expect("Failed to pause original stream");
        }
        if let Some(stream) = &self.raw_waveform.playing_stream {
            stream.play().expect("Failed to resume original stream");
        }
    }

    pub fn resume_processed(&mut self) {
        if let Some(stream) = &self.raw_waveform.playing_stream {
            stream.pause().expect("Failed to pause original stream");
        }
        if let Some(stream) = &self.processed_waveform.playing_stream {
            stream.play().expect("Failed to resume processed stream");
        }
    }

    pub fn stop_original(&mut self) {
        self.raw_waveform.playing_stream = None;
        *self.raw_waveform.current_idx.lock().unwrap() = 0;
    }

    pub fn stop_processed(&mut self) {
        self.processed_waveform.playing_stream = None;
        *self.processed_waveform.current_idx.lock().unwrap() = 0;
    }

    pub fn jump_to_position(&mut self, sample_idx: usize, is_original: bool) {
        let waveform = if is_original { &mut self.raw_waveform } else { &mut self.processed_waveform };
        *waveform.current_idx.lock().unwrap() = sample_idx.min(waveform.samples_raw.len());
    }
}