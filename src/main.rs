mod app;
mod i18n;
mod player;
mod renderer;

use app::NebulaToolsApp;

fn main() -> eframe::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_title(format!("NebulaTools v{}", version)),
        ..Default::default()
    };
    eframe::run_native(
        "nebula_tools",
        options,
        Box::new(|cc| Box::new(NebulaToolsApp::new(cc))),
    )
}
