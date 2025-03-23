use eframe::egui;
use hound::{WavReader, WavWriter};
use rfd::FileDialog; // For file selection dialog

struct SoundApp {
    samples: Vec<f32>, // Normalized samples for waveform display
    processed_samples: Vec<i16>, // Processed raw samples
    spec: Option<hound::WavSpec>,
    file_loaded: bool,
}

impl SoundApp {
    fn new() -> Self {
        Self {
            samples: Vec::new(),
            processed_samples: Vec::new(),
            spec: None,
            file_loaded: false,
        }
    }

    fn load_file(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
            if let Ok(mut reader) = WavReader::open(&path) {
                let spec = reader.spec();
                let raw_samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
                // Normalize to -1.0 to 1.0 for waveform display
                self.samples = raw_samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
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
            // Pop up "Save As" dialog
            if let Some(path) = FileDialog::new()
                .add_filter("WAV", &["wav"])
                .set_file_name("output.wav") // Default file name
                .save_file()
            {
                if let Ok(mut writer) = WavWriter::create(&path, spec) {
                    for &sample in &self.processed_samples {
                        writer.write_sample(sample).unwrap();
                    }
                    writer.finalize().unwrap();
                    println!("Saved to {:?}", path); // Optional: display save path
                }
            }
        }
    }
}

impl eframe::App for SoundApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Podcast Editing Tool");

            // Button area
            ui.horizontal(|ui| {
                if ui.button("Load Audio").clicked() {
                    self.load_file();
                }
                if ui.button("Remove Silence").clicked() {
                    self.remove_silence(0.01, 1000); // Threshold 0.01, minimum silence 1 second
                }
                if ui.button("Export").clicked() {
                    self.save_file();
                }
            });

            // Waveform display
            if self.file_loaded {
                ui.label("Audio Waveform:");
                let painter = ui.painter();
                let rect = ui.available_rect_before_wrap();
                let height = rect.height().min(200.0);
                let width = rect.width();
                let pos = rect.min;

                // Draw background
                painter.rect_filled(rect, 0.0, egui::Color32::GRAY);

                // Draw waveform
                let step = self.samples.len() as f32 / width;
                let mut points = Vec::new();
                for x in 0..width as usize {
                    let sample_idx = (x as f32 * step) as usize;
                    if sample_idx < self.samples.len() {
                        let y = self.samples[sample_idx];
                        let y_pos = pos.y + height * (0.5 - y * 0.5); // Center display
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