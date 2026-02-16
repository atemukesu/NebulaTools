mod editor;
mod i18n;
mod math;
mod particleex;
mod player;
mod renderer;
mod ui;

use ui::app::NebulaToolsApp;

fn main() -> eframe::Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 750.0])
            .with_title(format!("NebulaTools v{}", version)),
        ..Default::default()
    };
    eframe::run_native(
        "nebula_tools",
        options,
        Box::new(|cc| Box::new(NebulaToolsApp::new(cc))),
    )
}
