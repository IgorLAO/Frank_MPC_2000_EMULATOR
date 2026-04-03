mod audio;

use audio::AudioEngine;
use eframe::egui;
use egui::{Color32, CornerRadius, Stroke, StrokeKind, Vec2};
use rodio::Sink;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MPC Emulator")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MPC Emulator",
        options,
        Box::new(|_cc| Ok(Box::new(MpcApp::new()))),
    )
}

struct MpcApp {
    audio: Option<AudioEngine>,
    /// 16 pad samples: pad index 0-15
    pad_samples: [Option<Arc<Vec<u8>>>; 16],
    /// Per-pad sink for restart semantics
    pad_sinks: Vec<Option<Sink>>,
}

impl MpcApp {
    fn new() -> Self {
        let audio = AudioEngine::new();
        let pad_sinks = (0..16).map(|_| None).collect();
        MpcApp {
            audio,
            pad_samples: Default::default(),
            pad_sinks,
        }
    }

    fn trigger_pad(&mut self, pad_idx: usize) {
        let Some(data) = self.pad_samples[pad_idx].clone() else {
            return;
        };
        let Some(engine) = &self.audio else { return };

        // If there's an existing sink, stop it and replace
        if let Some(existing) = &self.pad_sinks[pad_idx] {
            existing.stop();
        }

        if let Some(sink) = engine.create_sink() {
            engine.play_into_sink(&sink, data);
            self.pad_sinks[pad_idx] = Some(sink);
        }
    }

    fn is_pad_playing(&self, pad_idx: usize) -> bool {
        if let Some(Some(sink)) = self.pad_sinks.get(pad_idx) {
            !sink.empty()
        } else {
            false
        }
    }

    fn stop_all(&mut self) {
        for sink_opt in &self.pad_sinks {
            if let Some(sink) = sink_opt {
                sink.stop();
            }
        }
    }
}

impl eframe::App for MpcApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Collect pads to trigger (to avoid borrow issues)
        let mut triggered: Vec<usize> = Vec::new();
        let mut stop_all = false;

        // Space key triggers Stop All
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            stop_all = true;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MPC Emulator");
            ui.add_space(8.0);

            if ui
                .add(
                    egui::Button::new("■  Stop All")
                        .fill(Color32::from_rgb(160, 40, 40))
                        .min_size(Vec2::new(120.0, 32.0)),
                )
                .clicked()
            {
                stop_all = true;
            }

            ui.add_space(8.0);

            let pad_size = Vec2::new(100.0, 100.0);
            let spacing = 8.0;

            egui::Grid::new("pad_grid")
                .num_columns(4)
                .spacing([spacing, spacing])
                .show(ui, |ui| {
                    for row in 0..4 {
                        for col in 0..4 {
                            let pad_idx = row * 4 + col;
                            let pad_num = pad_idx + 1;
                            let has_sample = self.pad_samples[pad_idx].is_some();
                            let is_playing = self.is_pad_playing(pad_idx);

                            let (rect, response) =
                                ui.allocate_exact_size(pad_size, egui::Sense::click());

                            if response.clicked() {
                                triggered.push(pad_idx);
                            }

                            if ui.is_rect_visible(rect) {
                                let bg_color = if is_playing {
                                    Color32::from_rgb(200, 160, 40) // bright highlight while playing
                                } else if has_sample {
                                    Color32::from_rgb(60, 80, 60)
                                } else {
                                    Color32::from_rgb(60, 60, 80)
                                };
                                let border_color = if is_playing {
                                    Color32::from_rgb(255, 220, 80)
                                } else {
                                    Color32::from_rgb(120, 120, 160)
                                };

                                let painter = ui.painter();
                                painter.rect(
                                    rect,
                                    CornerRadius::same(6),
                                    bg_color,
                                    Stroke::new(2.0, border_color),
                                    StrokeKind::Outside,
                                );

                                painter.text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{}", pad_num),
                                    egui::FontId::proportional(20.0),
                                    Color32::WHITE,
                                );
                            }
                        }
                        ui.end_row();
                    }
                });
        });

        if stop_all {
            self.stop_all();
        } else {
            for pad_idx in triggered {
                self.trigger_pad(pad_idx);
            }
        }

        // Request repaint for active state updates
        ctx.request_repaint();
    }
}
