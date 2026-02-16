use super::app::{AppMode, NebulaToolsApp};
use crate::i18n::Language;
use eframe::egui;

impl NebulaToolsApp {
    pub(crate) fn show_welcome_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                egui::ComboBox::from_id_source("welcome_lang")
                    .selected_text(self.config.lang.display_name())
                    .show_ui(ui, |ui| {
                        let current_lang = self.config.lang;
                        if ui
                            .selectable_label(
                                current_lang == Language::ChineseSimplified,
                                Language::ChineseSimplified.display_name(),
                            )
                            .clicked()
                        {
                            self.update_lang(Language::ChineseSimplified);
                        }
                        if ui
                            .selectable_label(
                                current_lang == Language::English,
                                Language::English.display_name(),
                            )
                            .clicked()
                        {
                            self.update_lang(Language::English);
                        }
                    });
            });

            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.2);
                ui.heading(
                    egui::RichText::new(format!(
                        "{} NebulaTools v{}",
                        self.i18n.tr("welcome"),
                        env!("CARGO_PKG_VERSION")
                    ))
                    .size(40.0)
                    .strong(),
                );
                ui.add_space(40.0);
                ui.horizontal(|ui| {
                    let total_width = 640.0;
                    ui.add_space((ui.available_width() - total_width) / 2.0);
                    let btn_size = egui::vec2(200.0, 60.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(
                                egui::RichText::new(format!(
                                    "📂 {}",
                                    self.i18n.tr("open_existing")
                                ))
                                .size(20.0),
                            )
                            .rounding(8.0),
                        )
                        .clicked()
                    {
                        self.handle_import();
                    }
                    ui.add_space(20.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(
                                egui::RichText::new(format!("✨ {}", self.i18n.tr("create_new")))
                                    .size(20.0),
                            )
                            .rounding(8.0),
                        )
                        .clicked()
                    {
                        self.mode = AppMode::Create;
                    }
                    ui.add_space(20.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(
                                egui::RichText::new(format!(
                                    "🔧 {}",
                                    self.i18n.tr("particleex_btn")
                                ))
                                .size(20.0),
                            )
                            .rounding(8.0),
                        )
                        .clicked()
                    {
                        self.mode = AppMode::Particleex;
                    }
                });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                    ui.colored_label(
                        egui::Color32::GRAY,
                        format!("{}: Atemukesu", self.i18n.tr("author")),
                    );
                });
            });
        });
    }
}
