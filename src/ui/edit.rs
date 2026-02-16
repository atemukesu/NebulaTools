use super::app::NebulaToolsApp;
use crate::player;
use eframe::egui;

impl NebulaToolsApp {
    pub(crate) fn show_edit_workflow(&mut self, ctx: &egui::Context) {
        // Ensure decoded frame data is ready when entering edit mode
        if self.edit.decoded_frames.is_none() && self.player.header.is_some() {
            match self.player.decode_all_frames() {
                Ok(frames) => {
                    let header = self.player.header.clone().unwrap();
                    self.edit.trim_end = header.total_frames.saturating_sub(1);
                    self.edit.new_fps = header.target_fps;
                    self.edit.edited_header = Some(header);
                    self.edit.decoded_frames = Some(frames);
                    self.edit.status_msg = None;
                }
                Err(e) => {
                    self.edit.status_msg = Some(format!("Decode failed: {}", e));
                }
            }
        }

        egui::SidePanel::left("edit_side")
            .resizable(true)
            .default_width(360.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading(self.i18n.tr("edit_tools"));
                    ui.separator();

                    if self.player.header.is_none() {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(self.i18n.tr("no_file_loaded")).italics());
                        return;
                    }

                    // ===== 1. Animation Speed =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_anim_speed"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(self.i18n.tr("edit_anim_speed_desc")).weak());
                        ui.add_space(6.0);
                        ui.radio_value(
                            &mut self.edit.speed_mode,
                            0,
                            self.i18n.tr("speed_mode_fps_only"),
                        );
                        if self.edit.speed_mode == 0 {
                            ui.indent("fps_only_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("speed_mode_fps_only_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("new_fps"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.new_fps)
                                            .clamp_range(1..=240)
                                            .speed(1.0),
                                    );
                                });
                            });
                        }

                        ui.add_space(4.0);
                        ui.radio_value(
                            &mut self.edit.speed_mode,
                            1,
                            self.i18n.tr("speed_mode_interp"),
                        );
                        if self.edit.speed_mode == 1 {
                            ui.indent("interp_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("speed_mode_interp_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("speed_factor"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.speed_factor)
                                            .clamp_range(0.1..=10.0)
                                            .speed(0.05)
                                            .fixed_decimals(2),
                                    );
                                });
                                if let Some(ref frames) = self.edit.decoded_frames {
                                    let new_count = ((frames.len() as f32) / self.edit.speed_factor)
                                        .round()
                                        as usize;
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "  {} → {} {}",
                                            frames.len(),
                                            new_count,
                                            self.i18n.tr("frame")
                                        ))
                                        .weak(),
                                    );
                                }
                            });
                        }

                        ui.add_space(4.0);
                        ui.radio_value(
                            &mut self.edit.speed_mode,
                            2,
                            self.i18n.tr("speed_mode_both"),
                        );
                        if self.edit.speed_mode == 2 {
                            ui.indent("both_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("speed_mode_both_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("new_fps"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.new_fps)
                                            .clamp_range(1..=240)
                                            .speed(1.0),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("speed_factor"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.speed_factor)
                                            .clamp_range(0.1..=10.0)
                                            .speed(0.05)
                                            .fixed_decimals(2),
                                    );
                                });
                            });
                        }

                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_speed_edit();
                        }
                    });

                    ui.add_space(6.0);

                    // ===== 2. Particle Size =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_particle_size"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(self.i18n.tr("edit_particle_size_desc")).weak(),
                        );
                        ui.add_space(6.0);
                        ui.radio_value(
                            &mut self.edit.size_mode,
                            0,
                            self.i18n.tr("size_mode_scale"),
                        );
                        if self.edit.size_mode == 0 {
                            ui.indent("size_scale_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("size_mode_scale_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("scale_factor"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.size_scale)
                                            .clamp_range(0.01..=100.0)
                                            .speed(0.05)
                                            .fixed_decimals(2),
                                    );
                                });
                            });
                        }
                        ui.add_space(4.0);
                        ui.radio_value(
                            &mut self.edit.size_mode,
                            1,
                            self.i18n.tr("size_mode_uniform"),
                        );
                        if self.edit.size_mode == 1 {
                            ui.indent("size_uniform_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("size_mode_uniform_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("uniform_size"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.size_uniform)
                                            .clamp_range(0.0..=655.0)
                                            .speed(0.01)
                                            .fixed_decimals(2),
                                    );
                                });
                            });
                        }
                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_size_edit();
                        }
                    });

                    ui.add_space(6.0);

                    // ===== 3. Color Adjustment =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_color"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(self.i18n.tr("edit_color_desc")).weak());
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("brightness_factor"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.brightness)
                                    .clamp_range(0.0..=5.0)
                                    .speed(0.01)
                                    .fixed_decimals(2),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("opacity_factor"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.opacity)
                                    .clamp_range(0.0..=5.0)
                                    .speed(0.01)
                                    .fixed_decimals(2),
                            );
                        });
                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_color_edit();
                        }
                    });

                    ui.add_space(6.0);

                    // ===== 4. Position Transform =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_transform"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(self.i18n.tr("edit_transform_desc")).weak());
                        ui.add_space(6.0);
                        ui.label(self.i18n.tr("translate_offset"));
                        ui.horizontal(|ui| {
                            ui.label("X:");
                            ui.add(
                                egui::DragValue::new(&mut self.edit.translate[0])
                                    .speed(0.1)
                                    .fixed_decimals(2),
                            );
                            ui.label("Y:");
                            ui.add(
                                egui::DragValue::new(&mut self.edit.translate[1])
                                    .speed(0.1)
                                    .fixed_decimals(2),
                            );
                            ui.label("Z:");
                            ui.add(
                                egui::DragValue::new(&mut self.edit.translate[2])
                                    .speed(0.1)
                                    .fixed_decimals(2),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("position_scale"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.pos_scale)
                                    .clamp_range(0.01..=100.0)
                                    .speed(0.01)
                                    .fixed_decimals(2),
                            );
                        });
                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_transform_edit();
                        }
                    });

                    ui.add_space(6.0);

                    // ===== 5. Trim =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_trim"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(self.i18n.tr("edit_trim_desc")).weak());
                        ui.add_space(6.0);
                        let max_frame = self
                            .edit
                            .decoded_frames
                            .as_ref()
                            .map(|f| f.len().saturating_sub(1) as u32)
                            .unwrap_or(0);
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("trim_start"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.trim_start)
                                    .clamp_range(0..=max_frame)
                                    .speed(1.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("trim_end"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.trim_end)
                                    .clamp_range(0..=max_frame)
                                    .speed(1.0),
                            );
                        });
                        if let Some(ref frames) = self.edit.decoded_frames {
                            let start = self.edit.trim_start as usize;
                            let end =
                                (self.edit.trim_end as usize).min(frames.len().saturating_sub(1));
                            if end >= start {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  → {} {}",
                                        end - start + 1,
                                        self.i18n.tr("frame")
                                    ))
                                    .weak(),
                                );
                            }
                        }
                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_trim_edit();
                        }
                    });

                    ui.add_space(20.0);
                    ui.separator();

                    // Save
                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 36.0],
                            egui::Button::new(
                                egui::RichText::new(self.i18n.tr("save_file"))
                                    .strong()
                                    .size(16.0),
                            ),
                        )
                        .clicked()
                    {
                        self.save_edited_file();
                    }

                    if let Some(ref msg) = self.edit.status_msg {
                        ui.add_space(8.0);
                        let color = if msg.starts_with('✅') {
                            egui::Color32::from_rgb(80, 200, 80)
                        } else {
                            egui::Color32::from_rgb(255, 100, 100)
                        };
                        ui.colored_label(color, msg.as_str());
                    }
                });
            });

        // Central panel: summary
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.25);
                if let Some(ref header) = self.edit.edited_header {
                    ui.heading(
                        egui::RichText::new(self.i18n.tr("edit_mode"))
                            .size(28.0)
                            .strong(),
                    );
                    ui.add_space(20.0);
                    let frame_count = self
                        .edit
                        .decoded_frames
                        .as_ref()
                        .map(|f| f.len())
                        .unwrap_or(0);
                    let duration = if header.target_fps > 0 {
                        frame_count as f32 / header.target_fps as f32
                    } else {
                        0.0
                    };
                    egui::Grid::new("edit_summary_grid")
                        .num_columns(2)
                        .spacing([20.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(self.i18n.tr("fps"));
                            ui.label(
                                egui::RichText::new(format!("{}", header.target_fps)).strong(),
                            );
                            ui.end_row();
                            ui.label(self.i18n.tr("total_frames"));
                            ui.label(egui::RichText::new(format!("{}", frame_count)).strong());
                            ui.end_row();
                            ui.label(self.i18n.tr("duration"));
                            ui.label(egui::RichText::new(format!("{:.2} s", duration)).strong());
                            ui.end_row();
                            ui.label(self.i18n.tr("bbox"));
                            ui.label(
                                egui::RichText::new(format!(
                                    "({:.1},{:.1},{:.1}) → ({:.1},{:.1},{:.1})",
                                    header.bbox_min[0],
                                    header.bbox_min[1],
                                    header.bbox_min[2],
                                    header.bbox_max[0],
                                    header.bbox_max[1],
                                    header.bbox_max[2],
                                ))
                                .strong(),
                            );
                            ui.end_row();
                        });
                } else {
                    ui.heading(self.i18n.tr("no_file_loaded"));
                }
            });
        });
    }

    // ── Apply helpers ──

    fn apply_speed_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            if let Some(ref mut header) = self.edit.edited_header {
                match self.edit.speed_mode {
                    0 => player::edit_change_fps(header, self.edit.new_fps),
                    1 => {
                        let nf = player::edit_interpolate_frames(frames, self.edit.speed_factor);
                        *frames = nf;
                    }
                    2 => {
                        player::edit_change_fps(header, self.edit.new_fps);
                        let nf = player::edit_interpolate_frames(frames, self.edit.speed_factor);
                        *frames = nf;
                    }
                    _ => {}
                }
                header.total_frames = frames.len() as u32;
                self.edit.trim_end = header.total_frames.saturating_sub(1);
                self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
            }
        }
    }

    fn apply_size_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            match self.edit.size_mode {
                0 => player::edit_scale_size(frames, self.edit.size_scale),
                1 => player::edit_uniform_size(frames, self.edit.size_uniform),
                _ => {}
            }
            self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
        }
    }

    fn apply_color_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            player::edit_adjust_color(frames, self.edit.brightness, self.edit.opacity);
            self.edit.brightness = 1.0;
            self.edit.opacity = 1.0;
            self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
        }
    }

    fn apply_transform_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            if self.edit.translate != [0.0; 3] {
                player::edit_translate(frames, self.edit.translate);
                self.edit.translate = [0.0; 3];
            }
            if (self.edit.pos_scale - 1.0).abs() > 0.001 {
                player::edit_scale_position(frames, self.edit.pos_scale);
                self.edit.pos_scale = 1.0;
            }
            if let Some(ref mut header) = self.edit.edited_header {
                let (bmin, bmax) = player::recalculate_bbox(frames);
                header.bbox_min = bmin;
                header.bbox_max = bmax;
            }
            self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
        }
    }

    fn apply_trim_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            let nf = player::edit_trim_frames(
                frames,
                self.edit.trim_start as usize,
                self.edit.trim_end as usize,
            );
            *frames = nf;
            if let Some(ref mut header) = self.edit.edited_header {
                header.total_frames = frames.len() as u32;
                let (bmin, bmax) = player::recalculate_bbox(frames);
                header.bbox_min = bmin;
                header.bbox_max = bmax;
            }
            self.edit.trim_start = 0;
            self.edit.trim_end = frames.len().saturating_sub(1) as u32;
            self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
        }
    }

    fn save_edited_file(&mut self) {
        let frames = match self.edit.decoded_frames {
            Some(ref f) => f.clone(),
            None => {
                self.edit.status_msg = Some(self.i18n.tr("apply_failed").to_string());
                return;
            }
        };
        let header = match self.edit.edited_header {
            Some(ref h) => h.clone(),
            None => {
                self.edit.status_msg = Some(self.i18n.tr("apply_failed").to_string());
                return;
            }
        };
        let textures = self.player.textures.clone();
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .set_file_name("output.nbl")
            .save_file()
        {
            match self.player.save_file(&path, &header, &textures, &frames) {
                Ok(_) => {
                    self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
                }
                Err(e) => {
                    self.edit.status_msg = Some(format!("{}: {}", self.i18n.tr("apply_failed"), e));
                }
            }
        }
    }
}
