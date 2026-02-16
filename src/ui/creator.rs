use super::app::{CreatorPreset, NebulaToolsApp};
use crate::editor::{self, EmitterConfig, EmitterShape};
use crate::player::{self, NblHeader, TextureEntry};
use eframe::egui;

impl NebulaToolsApp {
    pub(crate) fn show_creator_workflow(&mut self, ctx: &egui::Context) {
        // Playback logic for creator preview
        if self.creator.preview_playing {
            if let Some(ref frames) = self.creator.preview_frames {
                let dt = ctx.input(|i| i.stable_dt);
                self.creator.preview_timer += dt;
                let frame_dur = 1.0 / self.creator.config.target_fps as f32;
                if self.creator.preview_timer >= frame_dur {
                    self.creator.preview_timer -= frame_dur;
                    let next = self.creator.preview_frame_idx + 1;
                    if (next as usize) < frames.len() {
                        self.creator.preview_frame_idx = next;
                    } else {
                        self.creator.preview_frame_idx = 0; // loop
                    }
                }
                ctx.request_repaint();
            }
        }

        egui::SidePanel::left("creator_side")
            .resizable(true)
            .default_width(360.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading(self.i18n.tr("creator"));
                    ui.separator();

                    // ===== Preset Selection =====
                    ui.label(
                        egui::RichText::new(self.i18n.tr("preset"))
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(4.0);

                    let presets = [
                        (CreatorPreset::Fireworks, self.i18n.tr("preset_fireworks")),
                        (CreatorPreset::Fountain, self.i18n.tr("preset_fountain")),
                        (CreatorPreset::Spiral, self.i18n.tr("preset_spiral")),
                        (CreatorPreset::Explosion, self.i18n.tr("preset_explosion")),
                        (CreatorPreset::Snow, self.i18n.tr("preset_snow")),
                        (CreatorPreset::Custom, self.i18n.tr("preset_custom")),
                    ];

                    let old_preset = self.creator.preset;
                    ui.horizontal_wrapped(|ui| {
                        for (preset, label) in &presets {
                            ui.selectable_value(&mut self.creator.preset, *preset, *label);
                        }
                    });

                    // Apply preset if changed
                    if self.creator.preset != old_preset {
                        self.creator.config = match self.creator.preset {
                            CreatorPreset::Fireworks => EmitterConfig::preset_fireworks(),
                            CreatorPreset::Fountain => EmitterConfig::preset_fountain(),
                            CreatorPreset::Spiral => EmitterConfig::preset_spiral(),
                            CreatorPreset::Explosion => EmitterConfig::preset_explosion(),
                            CreatorPreset::Snow => EmitterConfig::preset_snow(),
                            CreatorPreset::Custom => EmitterConfig::default(),
                        };
                        self.creator.preview_frames = None;
                    }

                    ui.add_space(10.0);
                    ui.separator();

                    // ===== Emitter Config =====
                    ui.label(
                        egui::RichText::new(self.i18n.tr("emitter_config"))
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(4.0);

                    egui::Grid::new("creator_config_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            // Shape
                            ui.label(format!("{}:", self.i18n.tr("shape")));
                            let shape_label = self.i18n.tr(self.creator.config.shape.i18n_key());
                            egui::ComboBox::from_id_source("emitter_shape")
                                .selected_text(shape_label)
                                .show_ui(ui, |ui| {
                                    for s in EmitterShape::ALL {
                                        ui.selectable_value(
                                            &mut self.creator.config.shape,
                                            s,
                                            self.i18n.tr(s.i18n_key()),
                                        );
                                    }
                                });
                            ui.end_row();

                            match self.creator.config.shape {
                                EmitterShape::Sphere | EmitterShape::Ring => {
                                    ui.label(format!("{}:", self.i18n.tr("radius")));
                                    ui.add(
                                        egui::DragValue::new(&mut self.creator.config.shape_radius)
                                            .clamp_range(0.0..=10000.0)
                                            .speed(0.1)
                                            .fixed_decimals(2),
                                    );
                                    ui.end_row();
                                }
                                EmitterShape::Box => {
                                    ui.label(format!("{}:", self.i18n.tr("box_size")));
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.creator.config.shape_box_size[0],
                                            )
                                            .speed(0.1)
                                            .prefix("X:"),
                                        );
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.creator.config.shape_box_size[1],
                                            )
                                            .speed(0.1)
                                            .prefix("Y:"),
                                        );
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.creator.config.shape_box_size[2],
                                            )
                                            .speed(0.1)
                                            .prefix("Z:"),
                                        );
                                    });
                                    ui.end_row();
                                }
                                _ => {}
                            }

                            // Emission
                            ui.label(self.i18n.tr("emission_rate"));
                            ui.add(
                                egui::DragValue::new(&mut self.creator.config.emission_rate)
                                    .clamp_range(0.0..=100000.0)
                                    .speed(1.0)
                                    .suffix(" /s"),
                            );
                            ui.end_row();

                            ui.label(self.i18n.tr("burst_count"));
                            ui.add(
                                egui::DragValue::new(&mut self.creator.config.burst_count)
                                    .clamp_range(0..=1000000)
                                    .speed(10.0),
                            );
                            ui.end_row();

                            ui.label("");
                            ui.checkbox(
                                &mut self.creator.config.burst_only,
                                self.i18n.tr("burst_only"),
                            );
                            ui.end_row();

                            // Lifetime
                            ui.label(self.i18n.tr("particle_lifetime"));
                            ui.add(
                                egui::DragValue::new(&mut self.creator.config.lifetime_frames)
                                    .clamp_range(1..=100000)
                                    .speed(1.0)
                                    .suffix(" frames"),
                            );
                            ui.end_row();

                            // Speed
                            ui.label(self.i18n.tr("initial_velocity"));
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut self.creator.config.speed_min)
                                        .clamp_range(-10000.0..=10000.0)
                                        .speed(0.5)
                                        .fixed_decimals(1)
                                        .prefix("min:"),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut self.creator.config.speed_max)
                                        .clamp_range(-10000.0..=10000.0)
                                        .speed(0.5)
                                        .fixed_decimals(1)
                                        .prefix("max:"),
                                );
                            });
                            ui.end_row();

                            // Direction
                            ui.label(format!("{}:", self.i18n.tr("direction")));
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut self.creator.config.direction[0])
                                        .speed(0.05)
                                        .prefix("X:"),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut self.creator.config.direction[1])
                                        .speed(0.05)
                                        .prefix("Y:"),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut self.creator.config.direction[2])
                                        .speed(0.05)
                                        .prefix("Z:"),
                                );
                            });
                            ui.end_row();

                            ui.label(format!("{}:", self.i18n.tr("spread")));
                            ui.add(
                                egui::DragValue::new(&mut self.creator.config.spread)
                                    .clamp_range(0.0..=360.0)
                                    .speed(1.0)
                                    .suffix("°"),
                            );
                            ui.end_row();

                            // Physics
                            ui.label(self.i18n.tr("gravity"));
                            ui.add(
                                egui::DragValue::new(&mut self.creator.config.gravity)
                                    .speed(0.1)
                                    .fixed_decimals(1),
                            );
                            ui.end_row();

                            ui.label(format!("{}:", self.i18n.tr("drag")));
                            ui.add(
                                egui::DragValue::new(&mut self.creator.config.drag)
                                    .clamp_range(-10.0..=10.0)
                                    .speed(0.005)
                                    .fixed_decimals(3),
                            );
                            ui.end_row();
                        });

                    ui.add_space(10.0);
                    ui.separator();

                    // ===== Colors =====
                    ui.label(
                        egui::RichText::new(self.i18n.tr("colors_and_size"))
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("color_start"));
                        let mut c = color_u8_to_f32(self.creator.config.color_start);
                        if ui.color_edit_button_rgba_unmultiplied(&mut c).changed() {
                            self.creator.config.color_start = color_f32_to_u8(c);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("color_end"));
                        let mut c = color_u8_to_f32(self.creator.config.color_end);
                        if ui.color_edit_button_rgba_unmultiplied(&mut c).changed() {
                            self.creator.config.color_end = color_f32_to_u8(c);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("size_start"));
                        ui.add(
                            egui::DragValue::new(&mut self.creator.config.size_start)
                                .clamp_range(0.0..=10000.0)
                                .speed(0.1)
                                .fixed_decimals(2),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("size_end"));
                        ui.add(
                            egui::DragValue::new(&mut self.creator.config.size_end)
                                .clamp_range(0.0..=10000.0)
                                .speed(0.1)
                                .fixed_decimals(2),
                        );
                    });

                    ui.add_space(10.0);
                    ui.separator();

                    // ===== Animation settings =====
                    ui.label(
                        egui::RichText::new(self.i18n.tr("animation_settings"))
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("target_fps_setting"));
                        ui.add(
                            egui::DragValue::new(&mut self.creator.config.target_fps)
                                .clamp_range(1..=1000)
                                .speed(1.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("anim_duration"));
                        ui.add(
                            egui::DragValue::new(&mut self.creator.config.duration_secs)
                                .clamp_range(0.01..=36000.0)
                                .speed(1.0)
                                .fixed_decimals(1)
                                .suffix(" s"),
                        );
                    });

                    let total_frames = (self.creator.config.duration_secs
                        * self.creator.config.target_fps as f32)
                        .ceil() as u32;
                    ui.label(
                        egui::RichText::new(format!(
                            "  → {} {}",
                            total_frames,
                            self.i18n.tr("frame")
                        ))
                        .weak(),
                    );

                    ui.add_space(16.0);
                    ui.separator();

                    // ===== Action Buttons =====
                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 36.0],
                            egui::Button::new(
                                egui::RichText::new(self.i18n.tr("generate_preview"))
                                    .strong()
                                    .size(16.0),
                            ),
                        )
                        .clicked()
                    {
                        self.generate_creator_preview();
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
                        self.export_creator_nbl();
                    }

                    if let Some(ref msg) = self.creator.status_msg {
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

        // --- Bottom Panel: Playback for creator preview ---
        if self.creator.preview_frames.is_some() {
            egui::TopBottomPanel::bottom("creator_playback")
                .resizable(false)
                .show(ctx, |ui| {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let play_label = if self.creator.preview_playing {
                            self.i18n.tr("pause")
                        } else {
                            self.i18n.tr("play")
                        };
                        if ui.button(play_label).clicked() {
                            self.creator.preview_playing = !self.creator.preview_playing;
                        }
                        if ui.button(self.i18n.tr("stop")).clicked() {
                            self.creator.preview_playing = false;
                            self.creator.preview_frame_idx = 0;
                        }
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        if let Some(ref frames) = self.creator.preview_frames {
                            let max_frame = frames.len().saturating_sub(1) as i32;
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.checkbox(&mut self.show_grid, self.i18n.tr("grid"));
                                    ui.add_space(8.0);
                                    ui.label(format!("/ {}", max_frame));
                                    let mut f = self.creator.preview_frame_idx;
                                    ui.add_space(8.0);
                                    let slider_width = ui.available_width() - 8.0;
                                    let slider_res = ui.add_sized(
                                        [slider_width, ui.spacing().interact_size.y],
                                        egui::Slider::new(&mut f, 0..=max_frame).show_value(true),
                                    );
                                    if slider_res.changed() {
                                        self.creator.preview_frame_idx = f;
                                    }
                                },
                            );
                        }
                    });
                    ui.add_space(6.0);
                });
        }

        // --- Central Panel: 3D Preview ---
        let particles_data = if let Some(ref frames) = self.creator.preview_frames {
            let idx = (self.creator.preview_frame_idx as usize).min(frames.len().saturating_sub(1));
            self.prepare_render_data_from(&frames[idx])
        } else {
            vec![]
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            self.paint_3d_viewport(ui, ctx, &particles_data);
        });
    }

    fn generate_creator_preview(&mut self) {
        let frames = editor::simulate(&self.creator.config);
        self.creator.preview_frames = Some(frames);
        self.creator.preview_frame_idx = 0;
        self.creator.preview_playing = true;
        self.creator.status_msg = Some(self.i18n.tr("apply_success").to_string());
    }

    pub(crate) fn export_creator_nbl(&mut self) {
        // Generate frames if not already done
        if self.creator.preview_frames.is_none() {
            self.generate_creator_preview();
        }

        let frames = match self.creator.preview_frames {
            Some(ref f) => f.clone(),
            None => {
                self.creator.status_msg = Some(self.i18n.tr("apply_failed").to_string());
                return;
            }
        };

        let (bbox_min, bbox_max) = player::recalculate_bbox(&frames);
        let header = NblHeader {
            version: 1,
            target_fps: self.creator.config.target_fps,
            total_frames: frames.len() as u32,
            texture_count: 0,
            attributes: 0x03,
            bbox_min,
            bbox_max,
        };
        let textures: Vec<TextureEntry> = vec![];

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .set_file_name("new_effect.nbl")
            .save_file()
        {
            match self.player.save_file(&path, &header, &textures, &frames) {
                Ok(_) => {
                    self.creator.status_msg = Some(self.i18n.tr("apply_success").to_string());
                }
                Err(e) => {
                    self.creator.status_msg =
                        Some(format!("{}: {}", self.i18n.tr("apply_failed"), e));
                }
            }
        }
    }
}

fn color_u8_to_f32(c: [u8; 4]) -> [f32; 4] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        c[3] as f32 / 255.0,
    ]
}

fn color_f32_to_u8(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0] * 255.0).round().clamp(0.0, 255.0) as u8,
        (c[1] * 255.0).round().clamp(0.0, 255.0) as u8,
        (c[2] * 255.0).round().clamp(0.0, 255.0) as u8,
        (c[3] * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}
