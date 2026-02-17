use super::app::NebulaToolsApp;
use eframe::egui;

impl NebulaToolsApp {
    pub(crate) fn show_preview_workflow(&mut self, ctx: &egui::Context) {
        // --- Side Panel: Left ---
        egui::SidePanel::left("metadata_side")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading(self.i18n.tr("metadata"));
                ui.separator();

                if let Some(header) = &self.player.header {
                    ui.vertical_centered_justified(|ui: &mut egui::Ui| {
                        egui::Grid::new("meta_grid")
                            .num_columns(2)
                            .spacing([12.0, 6.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(self.i18n.tr("version"));
                                ui.label(
                                    egui::RichText::new(format!("v{}", header.version)).strong(),
                                );
                                ui.end_row();

                                ui.label(self.i18n.tr("fps"));
                                ui.label(
                                    egui::RichText::new(format!("{}", header.target_fps)).strong(),
                                );
                                ui.end_row();

                                ui.label(self.i18n.tr("total_frames"));
                                ui.label(
                                    egui::RichText::new(format!("{}", header.total_frames))
                                        .strong(),
                                );
                                ui.end_row();

                                if header.target_fps > 0 {
                                    ui.label(self.i18n.tr("duration"));
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.2} s",
                                            header.total_frames as f32 / header.target_fps as f32
                                        ))
                                        .strong(),
                                    );
                                    ui.end_row();
                                }

                                ui.label(self.i18n.tr("keyframe_count"));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}",
                                        self.player.keyframe_indices.len()
                                    ))
                                    .strong(),
                                );
                                ui.end_row();

                                ui.label(self.i18n.tr("textures"));
                                ui.label(
                                    egui::RichText::new(format!("{}", header.texture_count))
                                        .strong(),
                                );
                                ui.end_row();
                            });
                    });

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(self.i18n.tr("attributes")).strong());
                    ui.horizontal(|ui| {
                        let has_alpha = (header.attributes & 0x01) != 0;
                        let has_size = (header.attributes & 0x02) != 0;
                        ui.set_enabled(false);
                        ui.checkbox(&mut has_alpha.clone(), self.i18n.tr("has_alpha"));
                        ui.checkbox(&mut has_size.clone(), self.i18n.tr("has_size"));
                    });

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(self.i18n.tr("bbox")).strong());
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Min:");
                                ui.label(format!(
                                    "{:.2}, {:.2}, {:.2}",
                                    header.bbox_min[0], header.bbox_min[1], header.bbox_min[2]
                                ));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Max:");
                                ui.label(format!(
                                    "{:.2}, {:.2}, {:.2}",
                                    header.bbox_max[0], header.bbox_max[1], header.bbox_max[2]
                                ));
                            });
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();

                    egui::CollapsingHeader::new(self.i18n.tr("texture_list"))
                        .default_open(false)
                        .show(ui, |ui| {
                            for (i, tex) in self.player.textures.iter().enumerate() {
                                ui.label(format!("{}: {}", i, tex.path));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  Rows: {}, Cols: {}",
                                        tex.rows, tex.cols
                                    ))
                                    .weak(),
                                );
                            }
                        });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.label(format!(
                        "{}: {}",
                        self.i18n.tr("particle_count"),
                        self.player.particles.len()
                    ));
                }

                if let Some(err) = &self.error_msg {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, err);
                }
            });

        // --- Bottom Panel ---
        egui::TopBottomPanel::bottom("playback_strip")
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let play_label = if self.player.is_playing {
                        self.i18n.tr("pause")
                    } else {
                        self.i18n.tr("play")
                    };
                    if ui.button(play_label).clicked() {
                        self.player.is_playing = !self.player.is_playing;
                    }
                    if ui.button(self.i18n.tr("stop")).clicked() {
                        self.player.is_playing = false;
                        let _ = self.player.seek_to(0);
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if let Some(header) = &self.player.header {
                        let max_frame = header.total_frames.saturating_sub(1);
                        let mut visual_frame = self
                            .scrub_frame
                            .unwrap_or(self.player.current_frame_idx.max(0) as u32);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut self.show_grid, "Grid");
                            ui.add_space(8.0);
                            ui.label(format!("/ {}", max_frame));

                            let drag_res = ui.add(
                                egui::DragValue::new(&mut visual_frame)
                                    .clamp_range(0..=max_frame)
                                    .speed(1.0),
                            );

                            ui.add_space(8.0);
                            let slider_width = ui.available_width() - 8.0;
                            let slider_res = ui.add_sized(
                                [slider_width, ui.spacing().interact_size.y],
                                egui::Slider::new(&mut visual_frame, 0..=max_frame)
                                    .show_value(false)
                                    .trailing_fill(true),
                            );

                            let is_scrubbing = slider_res.dragged() || drag_res.dragged();
                            let stop_scrubbing = slider_res.drag_stopped()
                                || drag_res.drag_stopped()
                                || drag_res.lost_focus();

                            if is_scrubbing {
                                self.player.is_playing = false;
                                self.scrub_frame = Some(visual_frame);
                                if visual_frame == (self.player.current_frame_idx + 1) as u32 {
                                    let _ = self.player.seek_to(visual_frame);
                                }
                            }

                            if stop_scrubbing {
                                let _ = self.player.seek_to(visual_frame);
                                self.scrub_frame = None;
                            }
                        });
                    }
                });
                ui.add_space(6.0);
            });

        // --- Central Panel ---
        let particles_data = self.prepare_render_data();
        egui::CentralPanel::default().show(ctx, |ui| {
            self.paint_3d_viewport(ui, ctx, &particles_data);

            // Extra particle count overlay
            // (FPS is handled inside paint_3d_viewport)
        });
    }
}
