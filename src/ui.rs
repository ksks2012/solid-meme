use crate::app::SoundApp;
use eframe::egui::{self, Painter, Rect, Sense, Stroke, Color32, Pos2, Align2, FontId, Response};

pub fn draw_ui(app: &mut SoundApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Sound Editing Tool");

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.button("Load Audio").clicked() {
                    app.load_file();
                }
                let detect_button = ui.add_enabled(!app.is_processing, egui::Button::new("Detect Silence"));
                if detect_button.clicked() {
                    app.detect_silence_background();
                }
                let remove_button = ui.add_enabled(!app.is_processing, egui::Button::new("Remove All Silence"));
                if remove_button.clicked() {
                    app.remove_all_silence_background();
                }
                if app.processed_ready && ui.button("Export").clicked() {
                    app.save_file();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Silence Threshold:");
                ui.add(egui::Slider::new(&mut app.silence_threshold, 0.0..=0.1).text("Amplitude"));
                ui.label("Min Silence Length (ms):");
                ui.add(egui::Slider::new(&mut app.min_silence_len, 100..=2000).text("ms"));
            });

            // Show processing progress
            if app.is_processing {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label("Processing...");
                    ui.add(egui::ProgressBar::new(app.processing_progress).show_percentage());
                });
            }

            ui.add_space(10.0);

            if app.file_loaded {
                let spec = app.spec.unwrap();
                let sample_rate = spec.sample_rate as f32;
                let current_raw_idx = *app.raw_waveform.current_idx.lock().unwrap() as f32;
                let current_proc_idx = *app.processed_waveform.current_idx.lock().unwrap() as f32;
                let current_raw_time = current_raw_idx / sample_rate;
                let current_proc_time = current_proc_idx / sample_rate;

                ui.label(format!(
                    "Detected {} silence segments, total {:.1}s",
                    app.raw_waveform.silence_segments.len(),
                    app.raw_waveform.silence_segments.iter().map(|&(s, e)| (e - s) as f32 / sample_rate).sum::<f32>()
                ));

                ui.add_space(30.0);

                ui.horizontal(|ui| {
                    ui.label("Original:");
                    if ui.button("Play").clicked() {
                        app.play_original();
                    }
                    if ui.button("Pause").clicked() {
                        app.pause_original();
                    }
                    if ui.button("Resume").clicked() {
                        app.resume_original();
                    }
                    if ui.button("Stop").clicked() {
                        app.stop_original();
                    }
                });

                ui.add_space(30.0);

                ui.label("Original Waveform:");
                let raw_response = ui.allocate_rect(
                    Rect::from_min_size(ui.cursor().min, egui::Vec2::new(ui.available_width(), 200.0)),
                    Sense::click_and_drag(),
                );

                let mut responses = vec![(raw_response.clone(), true)];

                if app.processed_ready {
                    ui.add_space(100.0);

                    ui.horizontal(|ui| {
                        ui.label("Processed:");
                        if ui.button("Play").clicked() {
                            app.play_processed();
                        }
                        if ui.button("Pause").clicked() {
                            app.pause_processed();
                        }
                        if ui.button("Resume").clicked() {
                            app.resume_processed();
                        }
                        if ui.button("Stop").clicked() {
                            app.stop_processed();
                        }
                    });

                    ui.add_space(30.0);

                    ui.label("Processed Waveform:");
                    let proc_response = ui.allocate_rect(
                        Rect::from_min_size(ui.cursor().min, egui::Vec2::new(ui.available_width(), 200.0)),
                        Sense::click_and_drag(),
                    );
                    responses.push((proc_response, false));
                }

                let painter = ui.painter();
                let width = ui.available_width();

                draw_waveform(
                    &painter,
                    raw_response.rect,
                    &app.raw_waveform.samples,
                    current_raw_idx,
                    current_raw_time,
                    app.raw_waveform.playing_stream.is_some(),
                    sample_rate,
                    app.zoom,
                    app.offset,
                    &app.raw_waveform.silence_segments,
                );
                if app.processed_ready {
                    if let Some(proc_response) = responses.last().map(|(r, _)| r) {
                        draw_waveform(
                            &painter,
                            proc_response.rect,
                            &app.processed_waveform.samples,
                            current_proc_idx,
                            current_proc_time,
                            app.processed_waveform.playing_stream.is_some(),
                            sample_rate,
                            app.zoom,
                            app.offset,
                            &[], // Processed waveform does not display silence markers, as they have been removed
                        );
                    }
                }

                ui.input(|i| {
                    handle_waveform_interaction(app, i, &responses, width);
                });

                ctx.request_repaint();
            } else {
                ui.label("Please load a WAV file first");
            }
        });
    });
}

fn handle_waveform_interaction(app: &mut SoundApp, input: &egui::InputState, responses: &[(Response, bool)], width: f32) {
    for &(ref response, is_original) in responses {
        let rect = response.rect;

        // Zoom
        if input.scroll_delta.y != 0.0 && rect.contains(input.pointer.hover_pos().unwrap_or_default()) {
            let zoom_factor = if input.scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
            app.zoom *= zoom_factor;
            app.zoom = app.zoom.max(0.1).min(100.0);
        }

        // Drag
        if input.pointer.primary_down() && rect.contains(input.pointer.hover_pos().unwrap_or_default()) {
            let delta = input.pointer.delta();
            let total_samples = if is_original {
                app.raw_waveform.samples.len()
            } else {
                app.processed_waveform.samples.len()
            } as f32;
            let samples_per_pixel = total_samples / width / app.zoom;
            app.offset -= delta.x;
            app.offset = app.offset.max(0.0).min(total_samples / samples_per_pixel - width);
        }

        if input.pointer.primary_clicked() && rect.contains(input.pointer.hover_pos().unwrap_or_default()) {
            if let Some(pos) = input.pointer.hover_pos() {
                let total_samples = if is_original {
                    app.raw_waveform.samples.len()
                } else {
                    app.processed_waveform.samples.len()
                } as f32;
                let samples_per_pixel = total_samples / width / app.zoom;
                let sample_idx = ((pos.x - rect.min.x + app.offset) * samples_per_pixel) as usize;
                app.jump_to_position(sample_idx, is_original);
            }
        }
    }
}

fn draw_waveform(
    painter: &Painter,
    rect: Rect,
    samples: &[f32],
    current_idx: f32,
    current_time: f32,
    show_progress: bool,
    sample_rate: f32,
    zoom: f32,
    offset: f32,
    silence_segments: &[(usize, usize)],
) {
    let pos = rect.min;
    let height = rect.height();
    let width = rect.width();

    painter.rect_filled(rect, 0.0, Color32::WHITE);

    let total_samples = samples.len() as f32;
    let total_seconds = total_samples / sample_rate;
    let samples_per_pixel = total_samples / width / zoom;
    let start_sample = (offset * samples_per_pixel).max(0.0).min(total_samples - 1.0) as usize;

    // Draw silence segments
    for &(start, end) in silence_segments {
        let start_x = pos.x + ((start as f32 - offset * samples_per_pixel) / samples_per_pixel).max(0.0);
        let end_x = pos.x + ((end as f32 - offset * samples_per_pixel) / samples_per_pixel).min(width);
        if start_x < end_x && start_x < pos.x + width && end_x > pos.x {
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(start_x, pos.y), Pos2::new(end_x, pos.y + height)),
                0.0,
                Color32::from_gray(200)
            );
        }
    }

    // Draw waveform
    let mut points = Vec::new();
    for x in 0..width as usize {
        let sample_idx = (start_sample as f32 + x as f32 * samples_per_pixel) as usize;
        if sample_idx < samples.len() {
            let y = samples[sample_idx];
            let y_pos = pos.y + height * (0.5 - y * 0.5);
            points.push(Pos2::new(pos.x + x as f32, y_pos));
        }
    }
    painter.add(egui::Shape::line(points, Stroke::new(1.0, Color32::BLACK)));

    if show_progress && current_idx < total_samples {
        let progress_x = pos.x + (current_idx / total_samples * width * zoom) - offset;
        if progress_x >= pos.x && progress_x <= pos.x + width {
            painter.line_segment(
                [Pos2::new(progress_x, pos.y), Pos2::new(progress_x, pos.y + height)],
                Stroke::new(1.0, Color32::RED),
            );
        }
        painter.text(
            Pos2::new(pos.x + width - 50.0, pos.y + 10.0),
            Align2::RIGHT_TOP,
            format!("{:.1}s", current_time),
            FontId::default(),
            Color32::RED,
        );
    }

    let time_step = (total_seconds / width * 100.0).max(1.0);
    for sec in (0..total_seconds as usize).step_by(time_step as usize) {
        let x = pos.x + (sec as f32 / total_seconds * width * zoom) - offset;
        if x >= pos.x && x <= pos.x + width {
            painter.text(
                Pos2::new(x, pos.y + height + 20.0),
                Align2::CENTER_TOP,
                format!("{}s", sec),
                FontId::default(),
                Color32::WHITE,
            );
        }
    }
}

impl eframe::App for SoundApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_processing();
        draw_ui(self, ctx);
    }
}