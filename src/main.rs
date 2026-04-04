mod audio;
mod loop_recorder;
mod recording;

use audio::AudioEngine;
use loop_recorder::LoopRecorder;
use recording::{list_input_devices, RecordingEngine};
use eframe::egui;
use egui::{Color32, CornerRadius, Stroke, StrokeKind, Vec2};
use rodio::Sink;
use std::sync::Arc;
use std::time::Instant;

struct LoopPlayback {
    loop_idx: usize,
    start: Instant,
    next_event_idx: usize,
}

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
    recorder: RecordingEngine,
    /// Buffer from the last completed recording, waiting to be assigned to a pad
    pending_record_buffer: Option<Arc<Vec<u8>>>,
    /// Time (in seconds) when each pad's long-press began; None if not held
    pad_press_start: [Option<f64>; 16],
    loop_recorder: LoopRecorder,
    /// Index of the currently selected loop in loop_recorder.loops
    selected_loop: Option<usize>,
    /// Active loop playback state (None = not playing)
    loop_playback: Option<LoopPlayback>,
    /// Available mic input device names (populated at startup)
    available_mic_devices: Vec<String>,
    /// Selected mic device name (None = use system default)
    selected_mic_device: Option<String>,
}

impl MpcApp {
    fn new() -> Self {
        let audio = AudioEngine::new();
        let pad_sinks = (0..16).map(|_| None).collect();
        MpcApp {
            audio,
            pad_samples: Default::default(),
            pad_sinks,
            recorder: RecordingEngine::new(),
            pending_record_buffer: None,
            pad_press_start: [None; 16],
            loop_recorder: LoopRecorder::new(),
            selected_loop: None,
            loop_playback: None,
            available_mic_devices: list_input_devices(),
            selected_mic_device: None,
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

const PAD_KEY_LABELS: [&str; 16] = [
    "Q", "W", "E", "-", "A", "S", "D", "F", "Z", "X", "C", "V", "1", "2", "3", "4",
];

impl eframe::App for MpcApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Collect pads to trigger (to avoid borrow issues)
        let mut triggered: Vec<usize> = Vec::new();
        let mut stop_all = false;
        let mut toggle_record = false;
        let mut toggle_loop_record = false;
        let mut play_loop = false;
        let mut stop_loop = false;

        // Space key triggers Stop All
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            stop_all = true;
        }

        // R key toggles recording (takes priority over pad trigger)
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            toggle_record = true;
        }

        // L key toggles loop recording
        if ctx.input(|i| i.key_pressed(egui::Key::L)) {
            toggle_loop_record = true;
        }

        // P key plays selected loop
        if ctx.input(|i| i.key_pressed(egui::Key::P)) {
            play_loop = true;
        }

        // O key stops loop playback
        if ctx.input(|i| i.key_pressed(egui::Key::O)) {
            stop_loop = true;
        }

        // Keyboard pad triggers (R removed — used for recording)
        ctx.input(|i| {
            let key_pad_map: [(egui::Key, usize); 15] = [
                (egui::Key::Q, 0),
                (egui::Key::W, 1),
                (egui::Key::E, 2),
                (egui::Key::A, 4),
                (egui::Key::S, 5),
                (egui::Key::D, 6),
                (egui::Key::F, 7),
                (egui::Key::Z, 8),
                (egui::Key::X, 9),
                (egui::Key::C, 10),
                (egui::Key::V, 11),
                (egui::Key::Num1, 12),
                (egui::Key::Num2, 13),
                (egui::Key::Num3, 14),
                (egui::Key::Num4, 15),
            ];
            for (key, pad_idx) in &key_pad_map {
                if i.key_pressed(*key) {
                    triggered.push(*pad_idx);
                }
            }
        });

        egui::SidePanel::right("loop_sidebar")
            .min_width(160.0)
            .max_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Loops");
                ui.add_space(4.0);
                if self.loop_recorder.loops.is_empty() {
                    ui.colored_label(Color32::from_rgb(140, 140, 140), "No loops recorded");
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(ui.available_height() - 70.0)
                        .show(ui, |ui| {
                            for (idx, lp) in self.loop_recorder.loops.iter().enumerate() {
                                let is_selected = self.selected_loop == Some(idx);
                                let is_playing = self.loop_playback.as_ref().map_or(false, |p| p.loop_idx == idx);
                                let bg = if is_playing {
                                    Color32::from_rgb(40, 140, 80)
                                } else if is_selected {
                                    Color32::from_rgb(60, 120, 200)
                                } else {
                                    Color32::from_rgb(50, 50, 70)
                                };
                                let label = if is_playing {
                                    format!("▶ {}", lp.name)
                                } else {
                                    lp.name.clone()
                                };
                                let response = ui.add(
                                    egui::Button::new(label)
                                        .fill(bg)
                                        .min_size(Vec2::new(ui.available_width(), 28.0)),
                                );
                                if response.clicked() {
                                    self.selected_loop = Some(idx);
                                }
                            }
                        });
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                let can_play = self.selected_loop
                    .map_or(false, |idx| idx < self.loop_recorder.loops.len());
                let is_playing = self.loop_playback.is_some();

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            can_play,
                            egui::Button::new("▶ Play (P)")
                                .fill(Color32::from_rgb(40, 120, 60))
                                .min_size(Vec2::new(80.0, 28.0)),
                        )
                        .clicked()
                    {
                        play_loop = true;
                    }
                    if ui
                        .add_enabled(
                            is_playing,
                            egui::Button::new("■ Stop (O)")
                                .fill(Color32::from_rgb(120, 50, 50))
                                .min_size(Vec2::new(72.0, 28.0)),
                        )
                        .clicked()
                    {
                        stop_loop = true;
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MPC Emulator");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
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

                ui.add_space(12.0);

                let is_recording = self.recorder.is_recording();
                let rec_label = if is_recording { "⏹  Stop Rec" } else { "⏺  Record (R)" };
                let rec_color = if is_recording {
                    Color32::from_rgb(180, 50, 50)
                } else {
                    Color32::from_rgb(100, 100, 100)
                };
                if ui
                    .add(
                        egui::Button::new(rec_label)
                            .fill(rec_color)
                            .min_size(Vec2::new(140.0, 32.0)),
                    )
                    .clicked()
                {
                    toggle_record = true;
                }

                // Blinking red indicator while recording
                if is_recording {
                    let t = ctx.input(|i| i.time);
                    let blink_on = (t * 2.0) as u64 % 2 == 0;
                    if blink_on {
                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover());
                        ui.painter().circle_filled(
                            dot_rect.center(),
                            8.0,
                            Color32::from_rgb(255, 50, 50),
                        );
                    } else {
                        ui.add_space(16.0);
                    }
                }

                ui.add_space(12.0);

                let is_loop_rec = self.loop_recorder.is_recording();
                let loop_rec_label = if is_loop_rec {
                    "⏹  Stop Loop (L)"
                } else {
                    "⏺  Loop Rec (L)"
                };
                let loop_rec_color = if is_loop_rec {
                    Color32::from_rgb(60, 120, 180)
                } else {
                    Color32::from_rgb(100, 100, 100)
                };
                if ui
                    .add(
                        egui::Button::new(loop_rec_label)
                            .fill(loop_rec_color)
                            .min_size(Vec2::new(150.0, 32.0)),
                    )
                    .clicked()
                {
                    toggle_loop_record = true;
                }

                // Blinking blue indicator while loop recording
                if is_loop_rec {
                    let t = ctx.input(|i| i.time);
                    let blink_on = (t * 2.0) as u64 % 2 == 0;
                    if blink_on {
                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover());
                        ui.painter().circle_filled(
                            dot_rect.center(),
                            8.0,
                            Color32::from_rgb(80, 160, 255),
                        );
                    } else {
                        ui.add_space(16.0);
                    }
                }

                // Show "assign to pad" hint when a buffer is waiting
                if self.pending_record_buffer.is_some() {
                    ui.add_space(8.0);
                    ui.colored_label(
                        Color32::from_rgb(255, 220, 80),
                        "Click a pad to assign recording",
                    );
                }
            });

            ui.add_space(4.0);

            // Mic device selector row
            ui.horizontal(|ui| {
                ui.label("Mic device:");
                let selected_label = self
                    .selected_mic_device
                    .as_deref()
                    .unwrap_or("System Default");
                let is_recording = self.recorder.is_recording();
                egui::ComboBox::from_id_salt("mic_device_select")
                    .selected_text(selected_label)
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.selected_mic_device,
                            None,
                            "System Default",
                        );
                        for name in self.available_mic_devices.clone() {
                            let label = name.clone();
                            ui.selectable_value(
                                &mut self.selected_mic_device,
                                Some(name),
                                label,
                            );
                        }
                    });
                if ui
                    .add_enabled(!is_recording, egui::Button::new("↺"))
                    .on_hover_text("Refresh device list")
                    .clicked()
                {
                    self.available_mic_devices = list_input_devices();
                }
                // Show which device is actually capturing when recording is active
                if is_recording {
                    let active = self
                        .recorder
                        .active_device()
                        .unwrap_or("System Default");
                    ui.colored_label(
                        Color32::from_rgb(255, 180, 80),
                        format!("● {active}"),
                    );
                }
            });

            ui.add_space(4.0);

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
                            let is_playing = self.is_pad_playing(pad_idx);
                            let pending_assign = self.pending_record_buffer.is_some();

                            let (rect, response) =
                                ui.allocate_exact_size(pad_size, egui::Sense::click_and_drag());

                            let now = ctx.input(|i| i.time);

                            // Track long-press: start timer when pointer goes down
                            if response.is_pointer_button_down_on() {
                                if self.pad_press_start[pad_idx].is_none() {
                                    self.pad_press_start[pad_idx] = Some(now);
                                }
                            } else {
                                self.pad_press_start[pad_idx] = None;
                            }

                            let hold_secs = self.pad_press_start[pad_idx]
                                .map(|start| (now - start) as f32)
                                .unwrap_or(0.0);
                            let hold_progress = (hold_secs / 3.0).min(1.0);
                            let long_press_complete = hold_progress >= 1.0;

                            // Clear sample when long press completes
                            if long_press_complete {
                                if self.pad_samples[pad_idx].is_some() {
                                    self.pad_samples[pad_idx] = None;
                                    if let Some(sink) = &self.pad_sinks[pad_idx] {
                                        sink.stop();
                                    }
                                    self.pad_sinks[pad_idx] = None;
                                }
                                self.pad_press_start[pad_idx] = None;
                            }

                            let has_sample = self.pad_samples[pad_idx].is_some();

                            if response.clicked() && !long_press_complete {
                                if pending_assign {
                                    // Assign recorded buffer to this pad
                                    if let Some(buf) = self.pending_record_buffer.take() {
                                        self.pad_samples[pad_idx] = Some(buf);
                                    }
                                } else {
                                    triggered.push(pad_idx);
                                }
                            }

                            if ui.is_rect_visible(rect) {
                                let bg_color = if is_playing {
                                    Color32::from_rgb(200, 160, 40) // bright highlight while playing
                                } else if hold_progress > 0.0 {
                                    Color32::from_rgb(120, 40, 40) // darkening during long-press
                                } else if pending_assign && !has_sample {
                                    Color32::from_rgb(40, 60, 100) // assignable empty pad hint
                                } else if has_sample {
                                    Color32::from_rgb(60, 80, 60)
                                } else {
                                    Color32::from_rgb(60, 60, 80)
                                };
                                let border_color = if is_playing {
                                    Color32::from_rgb(255, 220, 80)
                                } else if hold_progress > 0.0 {
                                    Color32::from_rgb(255, 80, 80)
                                } else if pending_assign {
                                    Color32::from_rgb(100, 160, 255)
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

                                // Long-press progress fill at the bottom of the pad
                                if hold_progress > 0.0 {
                                    let bar_height = 6.0;
                                    let bar_rect = egui::Rect::from_min_max(
                                        egui::pos2(rect.min.x, rect.max.y - bar_height),
                                        egui::pos2(
                                            rect.min.x + rect.width() * hold_progress,
                                            rect.max.y,
                                        ),
                                    );
                                    painter.rect_filled(
                                        bar_rect,
                                        CornerRadius::same(3),
                                        Color32::from_rgb(255, 80, 80),
                                    );
                                }

                                painter.text(
                                    rect.center() - Vec2::new(0.0, 8.0),
                                    egui::Align2::CENTER_CENTER,
                                    PAD_KEY_LABELS[pad_idx],
                                    egui::FontId::proportional(22.0),
                                    Color32::WHITE,
                                );
                                let pad_label = if has_sample {
                                    format!("{} ●", pad_num)
                                } else {
                                    format!("{}", pad_num)
                                };
                                painter.text(
                                    rect.center() + Vec2::new(0.0, 14.0),
                                    egui::Align2::CENTER_CENTER,
                                    pad_label,
                                    egui::FontId::proportional(13.0),
                                    Color32::from_rgb(180, 180, 180),
                                );
                            }
                        }
                        ui.end_row();
                    }
                });
        });

        if toggle_record {
            if self.recorder.is_recording() {
                let wav = self.recorder.stop();
                if !wav.is_empty() {
                    self.pending_record_buffer = Some(Arc::new(wav));
                }
            } else {
                let started = self.recorder.start(self.selected_mic_device.as_deref());
                if !started {
                    // Selected device unavailable — try system default
                    self.recorder.start(None);
                }
            }
        }

        if toggle_loop_record {
            if self.loop_recorder.is_recording() {
                self.loop_recorder.stop();
            } else {
                self.loop_recorder.start();
            }
        }

        if stop_loop {
            self.loop_playback = None;
        }

        if play_loop {
            if let Some(idx) = self.selected_loop {
                if idx < self.loop_recorder.loops.len() {
                    self.loop_playback = Some(LoopPlayback {
                        loop_idx: idx,
                        start: Instant::now(),
                        next_event_idx: 0,
                    });
                }
            }
        }

        // Tick loop playback: fire any events whose timestamp has been reached,
        // then loop back continuously using the recorded duration for precise timing.
        let mut loop_triggered: Vec<usize> = Vec::new();
        let mut playback_done = false;
        if let Some(pb) = &mut self.loop_playback {
            if pb.loop_idx < self.loop_recorder.loops.len() {
                let lp = &self.loop_recorder.loops[pb.loop_idx];
                if lp.events.is_empty() || lp.duration_ms == 0 {
                    playback_done = true;
                } else {
                    let elapsed_ms = pb.start.elapsed().as_millis() as u64;
                    while pb.next_event_idx < lp.events.len()
                        && lp.events[pb.next_event_idx].elapsed_ms <= elapsed_ms
                    {
                        loop_triggered.push(lp.events[pb.next_event_idx].pad_index);
                        pb.next_event_idx += 1;
                    }
                    // All events consumed — restart for next iteration
                    if pb.next_event_idx >= lp.events.len() {
                        pb.start = pb.start
                            + std::time::Duration::from_millis(lp.duration_ms);
                        pb.next_event_idx = 0;
                    }
                }
            } else {
                playback_done = true;
            }
        }
        if playback_done {
            self.loop_playback = None;
        }
        for pad_idx in loop_triggered {
            self.trigger_pad(pad_idx);
        }

        if stop_all {
            self.stop_all();
            self.loop_playback = None;
        } else {
            for pad_idx in triggered {
                self.loop_recorder.record_event(pad_idx);
                self.trigger_pad(pad_idx);
            }
        }

        // Request repaint for active state updates
        ctx.request_repaint();
    }
}
