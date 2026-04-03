use eframe::egui;

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
        });
    }
}
