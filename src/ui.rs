use crate::app::SoundApp;
use eframe::egui::{self, Painter, Rect, Sense, Stroke, Color32, Pos2, Align2, FontId};

pub fn draw_ui(app: &mut SoundApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Sound Editing Tool");

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.button("Load Audio").clicked() {
                    app.load_file();
                }
                if ui.button("Remove Silence").clicked() {
                    app.remove_silence(0.01, 1000);
                }
                if ui.button("Export").clicked() {
                    app.save_file();
                }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Play Original").clicked() {
                    app.play_original();
                }
                if ui.button("Play Processed").clicked() {
                    app.play_processed();
                }
                if ui.button("Stop").clicked() {
                    app.stop_playback();
                }
            });

            ui.add_space(30.0);

            if app.file_loaded {
                let spec = app.spec.unwrap();
                let sample_rate = spec.sample_rate as f32;
                let current_raw_idx = *app.raw_waveform.current_idx.lock().unwrap() as f32;
                let current_proc_idx = *app.processed_waveform.current_idx.lock().unwrap() as f32;
                let current_raw_time = current_raw_idx / sample_rate;
                let current_proc_time = current_proc_idx / sample_rate;

                ui.label("Original Waveform:");
                let raw_response = ui.allocate_rect(
                    Rect::from_min_size(ui.cursor().min, egui::Vec2::new(ui.available_width(), 200.0)),
                    Sense::click_and_drag(),
                );

                ui.add_space(50.0);

                ui.label("Processed Waveform:");
                let proc_response = ui.allocate_rect(
                    Rect::from_min_size(ui.cursor().min, egui::Vec2::new(ui.available_width(), 200.0)),
                    Sense::click_and_drag(),
                );

                let painter = ui.painter();
                let width = ui.available_width();
                draw_waveform(
                    &painter,
                    raw_response.rect,
                    &app.raw_waveform.samples,
                    current_raw_idx,
                    current_raw_time,
                    app.playing_stream.is_some() && app.playing_original,
                    spec.sample_rate as f32,
                    app.zoom,
                    app.offset,
                );
                draw_waveform(
                    &painter,
                    proc_response.rect,
                    &app.processed_waveform.samples,
                    current_proc_idx,
                    current_proc_time,
                    app.playing_stream.is_some() && !app.playing_original,
                    spec.sample_rate as f32,
                    app.zoom,
                    app.offset,
                );

                ui.input(|i| {
                    for response in &[raw_response, proc_response] {
                        let rect = response.rect;
                        if i.scroll_delta.y != 0.0 && rect.contains(i.pointer.hover_pos().unwrap_or_default()) {
                            let zoom_factor = if i.scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
                            app.zoom *= zoom_factor;
                            app.zoom = app.zoom.max(0.1).min(100.0);
                        }
                        if i.pointer.primary_down() && rect.contains(i.pointer.hover_pos().unwrap_or_default()) {
                            let delta = i.pointer.delta();
                                let total_samples = app.raw_waveform.samples.len() as f32;
                                let samples_per_pixel = total_samples / width / app.zoom;
                                app.offset -= delta.x;
                                app.offset = app.offset.max(0.0).min(total_samples / samples_per_pixel - width);
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

fn draw_waveform(
    painter: &Painter,
    rect: Rect,
    samples: &[f32],
    current_idx: f32,
    current_time: f32,
    show_progress: bool,
    sample_rate: f32,
    // for zoom feature
    zoom: f32,    
    offset: f32,     
) {
    let pos = rect.min;
    let height = rect.height();
    let width = rect.width();

    painter.rect_filled(rect, 0.0, egui::Color32::GRAY);

    let total_samples = samples.len() as f32;
    let total_seconds = total_samples / sample_rate;
    let samples_per_pixel = total_samples / width / zoom;
    let start_sample = (offset * samples_per_pixel).max(0.0).min(total_samples - 1.0) as usize;

    let mut points = Vec::new();
    for x in 0..width as usize {
        let sample_idx = (start_sample as f32 + x as f32 * samples_per_pixel) as usize;
        if sample_idx < samples.len() {
            let y = samples[sample_idx];
            let y_pos = pos.y + height * (0.5 - y * 0.5);
            points.push(Pos2::new(pos.x + x as f32, y_pos));
        }
    }
    painter.add(egui::Shape::line(points, Stroke::new(1.0, Color32::WHITE)));

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
        draw_ui(self, ctx);
    }
}