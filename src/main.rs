mod app;
mod i18n;
mod player;
mod renderer;

use app::NebulaToolsApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("NebulaTools (Rust)"),
        ..Default::default()
    };
    eframe::run_native(
        "nebula_tools",
        options,
        Box::new(|cc| Box::new(NebulaToolsApp::new(cc))),
    )
}
