use eframe::egui;
use hound::{WavReader, WavWriter};
use rfd::FileDialog;
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use std::thread;
use std::sync::Arc;

struct SoundApp {
    samples: Vec<f32>,
    raw_samples: Vec<i16>,
    processed_samples: Vec<i16>,
    spec: Option<hound::WavSpec>,
    file_loaded: bool,
    playing_stream: Option<Arc<cpal::Stream>>, // Store the currently playing stream
}

impl SoundApp {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
            raw_samples: Vec::new(),
            processed_samples: Vec::new(),
            spec: None,
            file_loaded: false,
            playing_stream: None,
        }
    }

    fn load_file(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
            if let Ok(mut reader) = WavReader::open(&path) {
                let spec = reader.spec();
                let raw_samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
                println!("Loaded raw samples count: {}", raw_samples.len());
                self.samples = raw_samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                self.raw_samples = raw_samples.clone();
                self.processed_samples = raw_samples;
                self.spec = Some(spec);
                self.file_loaded = true;
            }
        }
    }

    fn remove_silence(&mut self, silence_threshold: f32, min_silence_len: usize) {
        if !self.file_loaded || self.spec.is_none() {
            return;
        }
        let spec = self.spec.unwrap();
        let channels = spec.channels as usize;
        let sample_rate = spec.sample_rate as usize;
        let total_samples = self.processed_samples.len();

        let mut result_samples = Vec::new();
        let mut silence_count = 0;

        for i in (0..total_samples).step_by(channels) {
            let mut frame_amplitude = 0.0;
            for ch in 0..channels {
                let sample = self.processed_samples[i + ch] as f32;
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
                    result_samples.push(self.processed_samples[i + ch]);
                }
            }
        }

        self.processed_samples = result_samples;
        self.samples = self.processed_samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
    }

    fn save_file(&self) {
        if let Some(spec) = self.spec {
            if let Some(path) = FileDialog::new()
                .add_filter("WAV", &["wav"])
                .set_file_name("output.wav")
                .save_file()
            {
                if let Ok(mut writer) = WavWriter::create(&path, spec) {
                    for &sample in &self.processed_samples {
                        writer.write_sample(sample).unwrap();
                    }
                    writer.finalize().unwrap();
                    println!("Saved to {:?}", path);
                }
            }
        }
    }

    fn play_samples(&mut self, samples: Vec<i16>, spec: hound::WavSpec) {
        let sample_len = samples.len();
        let host = cpal::default_host();
        let device = host.default_output_device().expect("No output device available");
        let config = cpal::StreamConfig {
            channels: spec.channels,
            sample_rate: cpal::SampleRate(spec.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        println!("Playing samples count: {}, Sample rate: {}, Channels: {}", sample_len, spec.sample_rate, spec.channels);
        println!("Device: {:?}", device.name());
        println!("Config: {:?}", config);

        let samples = samples.clone();
        let mut sample_idx = 0;

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(spec.channels as usize) {
                    for sample in frame {
                        if sample_idx < sample_len {
                            *sample = samples[sample_idx] as f32 / i16::MAX as f32;
                            sample_idx += 1;
                        } else {
                            *sample = 0.0;
                        }
                    }
                }
            },
            |err| eprintln!("Audio error: {}", err),
             None,
        ).expect("Failed to build output stream");

        stream.play().expect("Failed to play stream");

        // Store the stream in the structure and wait in the background thread
        let stream = Arc::new(stream);
        self.playing_stream = Some(stream.clone());

        thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(
            (sample_len as u64 * 1000) / (spec.sample_rate as u64 * spec.channels as u64)
            ));
            // The stream will be automatically destroyed here
        });
    }

    fn play_original(&mut self) {
        if self.file_loaded && self.spec.is_some() {
            let samples = self.raw_samples.clone();
            let spec = self.spec.unwrap();
            println!("Playing original samples count: {}", samples.len());
            self.play_samples(samples, spec);
        } else {
            println!("No file loaded, cannot play original audio");
        }
    }

    fn play_processed(&mut self) {
        if self.file_loaded && self.spec.is_some() {
            let samples = self.processed_samples.clone();
            let spec = self.spec.unwrap();
            println!("Playing processed samples count: {}", samples.len());
            self.play_samples(samples, spec);
        } else {
            println!("No file loaded, cannot play processed audio");
        }
    }
}

impl eframe::App for SoundApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Podcast Editing Tool");

            ui.horizontal(|ui| {
                if ui.button("Load Audio").clicked() {
                    self.load_file();
                }
                if ui.button("Remove Silence").clicked() {
                    self.remove_silence(0.01, 1000);
                }
                if ui.button("Play Original").clicked() {
                    self.play_original();
                }
                if ui.button("Play Processed").clicked() {
                    self.play_processed();
                }
                if ui.button("Export").clicked() {
                    self.save_file();
                }
            });

            if self.file_loaded {
                ui.label("Audio Waveform:");
                let painter = ui.painter();
                let rect = ui.available_rect_before_wrap();
                let height = rect.height().min(200.0);
                let width = rect.width();
                let pos = rect.min;

                painter.rect_filled(rect, 0.0, egui::Color32::GRAY);

                let step = self.samples.len() as f32 / width;
                let mut points = Vec::new();
                for x in 0..width as usize {
                    let sample_idx = (x as f32 * step) as usize;
                    if sample_idx < self.samples.len() {
                        let y = self.samples[sample_idx];
                        let y_pos = pos.y + height * (0.5 - y * 0.5);
                        points.push(egui::Pos2::new(pos.x + x as f32, y_pos));
                    }
                }
                painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, egui::Color32::WHITE)));
            } else {
                ui.label("Please load a WAV file first");
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(800.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Podcast Editing Tool",
        options,
        Box::new(|_cc| Box::new(SoundApp::new())),
    )
}