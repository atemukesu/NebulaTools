use super::app::NebulaToolsApp;
use crate::particleex;
use crate::player::{self, NblHeader, TextureEntry};
use eframe::egui;

impl NebulaToolsApp {
    pub(crate) fn show_particleex_workflow(&mut self, ctx: &egui::Context) {
        // Playback logic for particleex preview
        if self.pex.preview_playing {
            if let Some(ref frames) = self.pex.preview_frames {
                let dt = ctx.input(|i| i.stable_dt);
                self.pex.preview_timer += dt;
                let frame_dur = 1.0 / self.pex.preview_fps as f32;
                if self.pex.preview_timer >= frame_dur {
                    self.pex.preview_timer -= frame_dur;
                    let next = self.pex.preview_frame_idx + 1;
                    if (next as usize) < frames.len() {
                        self.pex.preview_frame_idx = next;
                    } else {
                        self.pex.preview_frame_idx = 0; // loop
                    }
                }
                ctx.request_repaint();
            }
        }

        egui::SidePanel::left("particleex_side")
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading(self.i18n.tr("particleex_title"));
                    ui.separator();

                    ui.label(
                        egui::RichText::new(self.i18n.tr("particleex_input"))
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(4.0);

                    ui.label(
                        egui::RichText::new(self.i18n.tr("particleex_hint"))
                            .weak()
                            .italics(),
                    );
                    ui.add_space(4.0);

                    // Command text area
                    let text_edit = egui::TextEdit::multiline(&mut self.pex.commands_text)
                        .desired_width(f32::INFINITY)
                        .desired_rows(16)
                        .code_editor()
                        .hint_text("particleex parameter ...");
                    ui.add(text_edit);

                    ui.add_space(16.0);
                    ui.separator();

                    // ===== Action Buttons =====
                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 36.0],
                            egui::Button::new(
                                egui::RichText::new(self.i18n.tr("particleex_compile"))
                                    .strong()
                                    .size(16.0),
                            ),
                        )
                        .clicked()
                    {
                        self.compile_particleex();
                    }

                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 36.0],
                            egui::Button::new(
                                egui::RichText::new(self.i18n.tr("export_nbl"))
                                    .strong()
                                    .size(16.0),
                            ),
                        )
                        .clicked()
                    {
                        self.export_particleex_nbl();
                    }

                    // Status message
                    if let Some(ref msg) = self.pex.status_msg {
                        ui.add_space(8.0);
                        let color = if msg.starts_with('✅') {
                            egui::Color32::from_rgb(80, 200, 80)
                        } else {
                            egui::Color32::from_rgb(255, 100, 100)
                        };
                        ui.colored_label(color, msg.as_str());
                    }

                    // Stats
                    if let Some(ref frames) = self.pex.preview_frames {
                        ui.add_space(12.0);
                        ui.separator();
                        ui.label(
                            egui::RichText::new(self.i18n.tr("particleex_stats"))
                                .strong()
                                .size(15.0),
                        );
                        ui.add_space(4.0);

                        let total_frames = frames.len();
                        let max_particles = frames.iter().map(|f| f.len()).max().unwrap_or(0);
                        let total_particles: usize = frames.iter().map(|f| f.len()).sum();

                        egui::Grid::new("pex_stats_grid")
                            .num_columns(2)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                ui.label(self.i18n.tr("total_frames"));
                                ui.label(format!("{}", total_frames));
                                ui.end_row();

                                ui.label(self.i18n.tr("particleex_max_particles"));
                                ui.label(format!("{}", max_particles));
                                ui.end_row();

                                ui.label(self.i18n.tr("particleex_total_keyframes"));
                                ui.label(format!("{}", total_particles));
                                ui.end_row();

                                ui.label("FPS");
                                ui.label(format!("{}", self.pex.preview_fps));
                                ui.end_row();
                            });
                    }
                });
            });

        // --- Bottom Panel: Playback ---
        if self.pex.preview_frames.is_some() {
            egui::TopBottomPanel::bottom("pex_playback")
                .resizable(false)
                .show(ctx, |ui| {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let play_label = if self.pex.preview_playing {
                            self.i18n.tr("pause")
                        } else {
                            self.i18n.tr("play")
                        };
                        if ui.button(play_label).clicked() {
                            self.pex.preview_playing = !self.pex.preview_playing;
                        }
                        if ui.button(self.i18n.tr("stop")).clicked() {
                            self.pex.preview_playing = false;
                            self.pex.preview_frame_idx = 0;
                        }
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        if let Some(ref frames) = self.pex.preview_frames {
                            let max_frame = frames.len().saturating_sub(1) as i32;
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.checkbox(&mut self.show_grid, self.i18n.tr("grid"));
                                    ui.add_space(8.0);
                                    ui.label(format!("/ {}", max_frame));
                                    let mut f = self.pex.preview_frame_idx;
                                    ui.add_space(8.0);
                                    let slider_width = ui.available_width() - 8.0;
                                    let slider_res = ui.add_sized(
                                        [slider_width, ui.spacing().interact_size.y],
                                        egui::Slider::new(&mut f, 0..=max_frame).show_value(true),
                                    );
                                    if slider_res.changed() {
                                        self.pex.preview_frame_idx = f;
                                    }
                                },
                            );
                        }
                    });
                    ui.add_space(6.0);
                });
        }

        // --- Central Panel: 3D Preview ---
        let particles_data = if let Some(ref frames) = self.pex.preview_frames {
            let idx = (self.pex.preview_frame_idx as usize).min(frames.len().saturating_sub(1));
            self.prepare_render_data_from(&frames[idx])
        } else {
            vec![]
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            self.paint_3d_viewport(ui, ctx, &particles_data);
        });
    }

    fn compile_particleex(&mut self) {
        let text = self.pex.commands_text.clone();
        match particleex::compile(&text) {
            Ok((frames, fps)) => {
                let frame_count = frames.len();
                self.pex.preview_frames = Some(frames);
                self.pex.preview_fps = fps;
                self.pex.preview_frame_idx = 0;
                self.pex.preview_playing = true;
                self.pex.status_msg = Some(format!(
                    "✅ {} {} {}",
                    self.i18n.tr("particleex_compiled"),
                    frame_count,
                    self.i18n.tr("frame")
                ));
            }
            Err(e) => {
                self.pex.status_msg = Some(format!("❌ {}", e));
            }
        }
    }

    pub(crate) fn export_particleex_nbl(&mut self) {
        // Compile if not done
        if self.pex.preview_frames.is_none() {
            self.compile_particleex();
        }

        let frames = match self.pex.preview_frames {
            Some(ref f) => f.clone(),
            None => {
                self.pex.status_msg = Some(self.i18n.tr("apply_failed").to_string());
                return;
            }
        };

        let (bbox_min, bbox_max) = player::recalculate_bbox(&frames);
        let header = NblHeader {
            version: 1,
            target_fps: self.pex.preview_fps,
            total_frames: frames.len() as u32,
            texture_count: 0,
            attributes: 0x03,
            bbox_min,
            bbox_max,
        };
        let textures: Vec<TextureEntry> = vec![];

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .set_file_name("particleex_effect.nbl")
            .save_file()
        {
            match self.player.save_file(&path, &header, &textures, &frames) {
                Ok(_) => {
                    self.pex.status_msg = Some(self.i18n.tr("apply_success").to_string());
                }
                Err(e) => {
                    self.pex.status_msg = Some(format!("{}: {}", self.i18n.tr("apply_failed"), e));
                }
            }
        }
    }
}
