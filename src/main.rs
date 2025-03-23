use eframe::egui;
use hound::{WavReader, WavWriter};
use rfd::FileDialog;
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use std::thread;
use std::sync::{Arc, Mutex};

struct SoundApp {
    raw_samples: Vec<f32>,        // Original waveform (for display)
    processed_samples: Vec<f32>,  // Processed waveform (for display)
    raw_samples_raw: Vec<i16>,    // Original audio data (for playback)
    processed_samples_raw: Vec<i16>, // Processed audio data (for playback)
    spec: Option<hound::WavSpec>,
    file_loaded: bool,
    playing_stream: Option<Arc<cpal::Stream>>,
    zoom: f32,
    offset: f32,
    current_raw_idx: Arc<Mutex<usize>>,  // Playback progress of original waveform
    current_proc_idx: Arc<Mutex<usize>>, // Playback progress of processed waveform
    playing_original: bool,              // Indicates if the currently playing waveform is the original
}

impl SoundApp {
    fn new() -> Self {
        Self {
            raw_samples: Vec::new(),
            processed_samples: Vec::new(),
            raw_samples_raw: Vec::new(),
            processed_samples_raw: Vec::new(),
            spec: None,
            file_loaded: false,
            playing_stream: None,
            zoom: 1.0,
            offset: 0.0,
            current_raw_idx: Arc::new(Mutex::new(0)),
            current_proc_idx: Arc::new(Mutex::new(0)),
            playing_original: false,
        }
    }

    fn load_file(&mut self) {
        if let Some(path) = FileDialog::new().add_filter("WAV", &["wav"]).pick_file() {
            if let Ok(mut reader) = WavReader::open(&path) {
                let spec = reader.spec();
                let raw_samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
                println!("Loaded raw samples count: {}", raw_samples.len());
                self.raw_samples = raw_samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                self.processed_samples = self.raw_samples.clone();
                self.raw_samples_raw = raw_samples.clone();
                self.processed_samples_raw = raw_samples;
                self.spec = Some(spec);
                self.file_loaded = true;
                self.zoom = 1.0;
                self.offset = 0.0;
                *self.current_raw_idx.lock().unwrap() = 0;
                *self.current_proc_idx.lock().unwrap() = 0;
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
        let total_samples = self.processed_samples_raw.len();

        let mut result_samples = Vec::new();
        let mut silence_count = 0;

        for i in (0..total_samples).step_by(channels) {
            let mut frame_amplitude = 0.0;
            for ch in 0..channels {
                let sample = self.processed_samples_raw[i + ch] as f32;
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
                    result_samples.push(self.processed_samples_raw[i + ch]);
                }
            }
        }

        self.processed_samples_raw = result_samples;
        self.processed_samples = self.processed_samples_raw.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
    }

    fn save_file(&self) {
        if let Some(spec) = self.spec {
            if let Some(path) = FileDialog::new()
                .add_filter("WAV", &["wav"])
                .set_file_name("output.wav")
                .save_file()
            {
                if let Ok(mut writer) = WavWriter::create(&path, spec) {
                    for &sample in &self.processed_samples_raw {
                        writer.write_sample(sample).unwrap();
                    }
                    writer.finalize().unwrap();
                    println!("Saved to {:?}", path);
                }
            }
        }
    }

    fn play_samples(&mut self, samples: Vec<i16>, spec: hound::WavSpec, is_original: bool) {
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
        let current_idx = if is_original {
            Arc::clone(&self.current_raw_idx)
        } else {
            Arc::clone(&self.current_proc_idx)
        };
        *current_idx.lock().unwrap() = 0;

        let stream = device.build_output_stream(
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
        ).expect("Failed to build output stream");

        stream.play().expect("Failed to play stream");

        let stream = Arc::new(stream);
        self.playing_stream = Some(stream.clone());
        self.playing_original = is_original;

        thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(
                (sample_len as u64 * 1000) / (spec.sample_rate as u64 * spec.channels as u64)
            ));
        });
    }

    fn stop_playback(&mut self) {
        self.playing_stream = None;
        *self.current_raw_idx.lock().unwrap() = 0;
        *self.current_proc_idx.lock().unwrap() = 0;
    }

    fn play_original(&mut self) {
        if self.file_loaded && self.spec.is_some() {
            let samples = self.raw_samples_raw.clone();
            let spec = self.spec.unwrap();
            println!("Playing original samples count: {}", samples.len());
            self.play_samples(samples, spec, true);
        } else {
            println!("No file loaded, cannot play original audio");
        }
    }

    fn play_processed(&mut self) {
        if self.file_loaded && self.spec.is_some() {
            let samples = self.processed_samples_raw.clone();
            let spec = self.spec.unwrap();
            println!("Playing processed samples count: {}", samples.len());
            self.play_samples(samples, spec, false);
        } else {
            println!("No file loaded, cannot play processed audio");
        }
    }

    fn draw_waveform(&self, painter: &egui::Painter, rect: egui::Rect, samples: &[f32], current_idx: f32, current_time: f32, is_original: bool) {
        let pos = rect.min;
        let height = rect.height();
        let width = rect.width();

        let total_samples = samples.len() as f32;
        let sample_rate = self.spec.unwrap().sample_rate as f32;
        let total_seconds = total_samples / sample_rate;

        let samples_per_pixel = total_samples / width / self.zoom;
        let start_sample = (self.offset * samples_per_pixel).max(0.0).min(total_samples - 1.0) as usize;

        // Draw waveform
        let mut points = Vec::new();
        for x in 0..width as usize {
            let sample_idx = (start_sample as f32 + x as f32 * samples_per_pixel) as usize;
            if sample_idx < samples.len() {
                let y = samples[sample_idx];
                let y_pos = pos.y + height * (0.5 - y * 0.5);
                points.push(egui::Pos2::new(pos.x + x as f32, y_pos));
            }
        }
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, egui::Color32::WHITE)));

        // Draw playback progress line (only when playing the corresponding waveform)
        if self.playing_stream.is_some() && self.playing_original == is_original && current_idx < total_samples {
            let progress_x = pos.x + (current_idx / total_samples * width * self.zoom) - self.offset;
            if progress_x >= pos.x && progress_x <= pos.x + width {
                painter.line_segment(
                    [egui::Pos2::new(progress_x, pos.y), egui::Pos2::new(progress_x, pos.y + height)],
                    egui::Stroke::new(1.0, egui::Color32::RED),
                );
            }
            painter.text(
                egui::Pos2::new(pos.x + width - 50.0, pos.y + 10.0),
                egui::Align2::RIGHT_TOP,
                format!("{:.1}s", current_time),
                egui::FontId::default(),
                egui::Color32::RED,
            );
        }

        // Draw timeline
        let time_step = (total_seconds / width * 100.0).max(1.0);
        for sec in (0..total_seconds as usize).step_by(time_step as usize) {
            let x = pos.x + (sec as f32 / total_seconds * width * self.zoom) - self.offset;
            if x >= pos.x && x <= pos.x + width {
                painter.text(
                    egui::Pos2::new(x, pos.y + height + 20.0),
                    egui::Align2::CENTER_TOP,
                    format!("{}s", sec),
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                );
            }
        }
    }
}

impl eframe::App for SoundApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Sound Editing Tool");

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Load Audio").clicked() {
                        self.load_file();
                    }
                    if ui.button("Remove Silence").clicked() {
                        self.remove_silence(0.01, 1000);
                    }
                    if ui.button("Export").clicked() {
                        self.save_file();
                    }
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("Play Original").clicked() {
                        self.play_original();
                    }
                    if ui.button("Play Processed").clicked() {
                        self.play_processed();
                    }
                    if ui.button("Stop").clicked() {
                        self.stop_playback();
                    }
                });

                ui.add_space(30.0); // Add larger spacing

                if self.file_loaded {
                    let spec = self.spec.unwrap();
                    let sample_rate = spec.sample_rate as f32;
                    let current_raw_idx = *self.current_raw_idx.lock().unwrap() as f32;
                    let current_proc_idx = *self.current_proc_idx.lock().unwrap() as f32;
                    let current_raw_time = current_raw_idx / sample_rate;
                    let current_proc_time = current_proc_idx / sample_rate;

                    ui.label("Original Waveform:");
                    let raw_response = ui.allocate_rect(
                        egui::Rect::from_min_size(ui.cursor().min, egui::Vec2::new(ui.available_width(), 200.0)),
                        egui::Sense::click_and_drag(),
                    );

                    ui.add_space(40.0); // Add spacing between waveforms

                    ui.label("Processed Waveform:");
                    let proc_response = ui.allocate_rect(
                        egui::Rect::from_min_size(ui.cursor().min, egui::Vec2::new(ui.available_width(), 200.0)),
                        egui::Sense::click_and_drag(),
                    );

                    let painter = ui.painter();
                    let width = ui.available_width();
                    let raw_rect = raw_response.rect;
                    painter.rect_filled(raw_rect, 0.0, egui::Color32::GRAY);
                    self.draw_waveform(&painter, raw_rect, &self.raw_samples, current_raw_idx, current_raw_time, true);

                    let proc_rect = proc_response.rect;
                    painter.rect_filled(proc_rect, 0.0, egui::Color32::GRAY);
                    self.draw_waveform(&painter, proc_rect, &self.processed_samples, current_proc_idx, current_proc_time, false);

                    ui.input(|i| {
                        for response in &[raw_response, proc_response] {
                            let rect = response.rect;
                            if i.scroll_delta.y != 0.0 && rect.contains(i.pointer.hover_pos().unwrap_or_default()) {
                                let zoom_factor = if i.scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
                                self.zoom *= zoom_factor;
                                self.zoom = self.zoom.max(0.1).min(100.0);
                            }
                            if i.pointer.primary_down() && rect.contains(i.pointer.hover_pos().unwrap_or_default()) {
                            let delta = i.pointer.delta();
                            let total_samples: f32 = self.raw_samples.len() as f32;
                                    let samples_per_pixel = total_samples / width / self.zoom;
                                    self.offset -= delta.x;
                                    self.offset = self.offset.max(0.0).min(total_samples / samples_per_pixel - width);
                            }
                        }
                    });

                    ctx.request_repaint();
                } else {
                    ui.label("Please load a WAV file first");
                }
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::Vec2::new(800.0, 600.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Sound Editing Tool",
        options,
        Box::new(|_cc| Box::new(SoundApp::new())),
    )
}