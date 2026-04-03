use eframe::egui;
use egui::{Color32, CornerRadius, Stroke, StrokeKind, Vec2};

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
        Box::new(|_cc| Ok(Box::new(MpcApp::default()))),
    )
}

#[derive(Default)]
struct MpcApp {}

impl eframe::App for MpcApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MPC Emulator");
            ui.add_space(16.0);
            show_pad_grid(ui);
        });
    }
}

fn show_pad_grid(ui: &mut egui::Ui) {
    let pad_size = Vec2::new(100.0, 100.0);
    let spacing = 8.0;

    egui::Grid::new("pad_grid")
        .num_columns(4)
        .spacing([spacing, spacing])
        .show(ui, |ui| {
            for row in 0..4 {
                for col in 0..4 {
                    let pad_num = row * 4 + col + 1;
                    draw_pad(ui, pad_num, pad_size);
                }
                ui.end_row();
            }
        });
}

fn draw_pad(ui: &mut egui::Ui, pad_num: usize, size: Vec2) {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let bg_color = Color32::from_rgb(60, 60, 80);
        let border_color = Color32::from_rgb(120, 120, 160);

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
