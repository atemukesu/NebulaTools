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
                                .clamp_range(0.0001..=36000.0)
                                .speed(0.1)
                                .max_decimals(4)
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

        // --- Bottom Panel: Dopesheet / Timeline ---
        egui::TopBottomPanel::bottom("creator_dopesheet")
            .resizable(true)
            .min_height(120.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
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

                    if ui
                        .button(self.i18n.tr("kf_add"))
                        .on_hover_text("Add Keyframe at Current Frame")
                        .clicked()
                    {
                        let cf = self.creator.preview_frame_idx as u32;
                        self.creator
                            .keyframes
                            .insert(cf, self.creator.config.clone());
                    }
                    if ui
                        .button(self.i18n.tr("kf_update"))
                        .on_hover_text("Update Keyframe")
                        .clicked()
                    {
                        let cf = self.creator.preview_frame_idx as u32;
                        if self.creator.keyframes.contains_key(&cf) {
                            self.creator
                                .keyframes
                                .insert(cf, self.creator.config.clone());
                        }
                    }
                    if ui.button(self.i18n.tr("kf_remove")).clicked() {
                        let cf = self.creator.preview_frame_idx as u32;
                        self.creator.keyframes.remove(&cf);
                    }
                    ui.add_space(16.0);

                    let mut max_frame = (self.creator.config.duration_secs
                        * self.creator.config.target_fps as f32)
                        .ceil() as i32;
                    max_frame = max_frame.max(100);
                    ui.label(format!(
                        "{}: {} / {}",
                        self.i18n.tr("frame_count_label"),
                        self.creator.preview_frame_idx,
                        max_frame
                    ));
                });

                ui.add_space(4.0);

                // Custom Dopesheet Timeline Canvas
                let dopesheet_height = ui.available_height().max(100.0);
                let channels_width = 180.0;
                let ruler_h = 24.0;
                let track_h = 24.0;

                ui.horizontal(|ui| {
                    // --- LEFT PANEL: Channels ---
                    let (channels_rect, _) = ui.allocate_exact_size(
                        egui::vec2(channels_width, dopesheet_height),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter_at(channels_rect);
                    // Panel Background (Blender Dopesheet Channels BG is dark grey)
                    painter.rect_filled(channels_rect, 0.0, egui::Color32::from_rgb(34, 34, 34));

                    // Dope Sheet Summary Header Row
                    let summary_rect = egui::Rect::from_min_size(
                        channels_rect.min,
                        egui::vec2(channels_rect.width(), ruler_h),
                    );
                    painter.rect_filled(summary_rect, 0.0, egui::Color32::from_rgb(45, 45, 45));
                    painter.text(
                        summary_rect.min + egui::vec2(8.0, 5.0),
                        egui::Align2::LEFT_TOP,
                        self.i18n.tr("dopesheet_summary"),
                        egui::FontId::proportional(12.0),
                        egui::Color32::from_rgb(220, 220, 220),
                    );

                    // Emitter Track Row
                    let track_y_min = channels_rect.top() + ruler_h;
                    let track_rect = egui::Rect::from_min_size(
                        egui::pos2(channels_rect.left(), track_y_min),
                        egui::vec2(channels_rect.width(), track_h),
                    );
                    // Track label background (slightly lighter than base)
                    painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgb(40, 40, 40));
                    painter.text(
                        track_rect.min + egui::vec2(16.0, 5.0),
                        egui::Align2::LEFT_TOP,
                        format!("⏷ {}", self.i18n.tr("emitter_settings")),
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );

                    // --- RIGHT PANEL: Timeline Canvas in ScrollArea ---
                    egui::ScrollArea::horizontal()
                        .id_source("dopesheet_scroll")
                        .show(ui, |ui| {
                            let mut max_frame = (self.creator.config.duration_secs
                                * self.creator.config.target_fps as f32)
                                .ceil() as i32;
                            max_frame = max_frame.max(100);

                            // Pixels per frame (Blender usually defines a step width)
                            let frame_w = 12.0;
                            let canvas_width =
                                (max_frame as f32 * frame_w).max(ui.available_width());

                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(canvas_width, dopesheet_height),
                                egui::Sense::click_and_drag(),
                            );

                            if ui.is_rect_visible(rect) {
                                let painter = ui.painter_at(rect);

                                // Main Background (Blender Dopesheet Tracks BG)
                                painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(57, 57, 57));

                                // Track background (Alternate Row Color)
                                let track_bg = egui::Rect::from_min_size(
                                    egui::pos2(rect.left(), track_y_min),
                                    egui::vec2(rect.width(), track_h),
                                );
                                painter.rect_filled(
                                    track_bg,
                                    0.0,
                                    egui::Color32::from_rgb(50, 50, 50),
                                );

                                // Ruler Background
                                let ruler_rect = egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(rect.width(), ruler_h),
                                );
                                painter.rect_filled(
                                    ruler_rect,
                                    0.0,
                                    egui::Color32::from_rgb(60, 60, 60),
                                );

                                // Grid lines & text in Ruler/Timeline
                                for f in 0..=max_frame {
                                    let x = rect.left() + (f as f32 * frame_w);
                                    if f % 10 == 0 {
                                        // Thick vertical line through track
                                        painter.line_segment(
                                            [
                                                egui::pos2(x, rect.top() + ruler_h / 2.0),
                                                egui::pos2(x, rect.bottom()),
                                            ],
                                            egui::Stroke::new(
                                                1.0,
                                                egui::Color32::from_rgb(45, 45, 45),
                                            ),
                                        );
                                        // Ruler text
                                        painter.text(
                                            egui::pos2(x + 2.0, rect.top() + 2.0),
                                            egui::Align2::LEFT_TOP,
                                            format!("{}", f),
                                            egui::FontId::proportional(10.0),
                                            egui::Color32::from_rgb(200, 200, 200),
                                        );
                                    } else if f % 5 == 0 {
                                        // Minor vertical line through track
                                        painter.line_segment(
                                            [
                                                egui::pos2(x, rect.top() + ruler_h * 0.75),
                                                egui::pos2(x, rect.bottom()),
                                            ],
                                            egui::Stroke::new(
                                                1.0,
                                                egui::Color32::from_rgb(48, 48, 48),
                                            ),
                                        );
                                    }
                                }

                                // Draw Keyframes Data Row by Row
                                // Blender shows "Summary" keyframes on top, and individual below
                                let keys: Vec<u32> =
                                    self.creator.keyframes.keys().copied().collect();

                                // Reusable closure to draw Blender-style diamond keyframe
                                let draw_diamond =
                                    |pos: egui::Pos2, active: bool, p: &egui::Painter| {
                                        let r = 4.0; // Diamond radius
                                        let pts = vec![
                                            egui::pos2(pos.x, pos.y - r),
                                            egui::pos2(pos.x + r, pos.y),
                                            egui::pos2(pos.x, pos.y + r),
                                            egui::pos2(pos.x - r, pos.y),
                                        ];
                                        let (fill, stroke) = if active {
                                            (
                                                egui::Color32::from_rgb(255, 204, 0),
                                                egui::Color32::BLACK,
                                            ) // Yellow
                                        } else {
                                            (
                                                egui::Color32::from_rgb(200, 200, 200),
                                                egui::Color32::BLACK,
                                            ) // Grey/White
                                        };
                                        p.add(egui::Shape::convex_polygon(
                                            pts,
                                            fill,
                                            egui::Stroke::new(0.5, stroke),
                                        ));
                                    };

                                let summary_center_y = summary_rect.center().y;
                                let track_center_y = track_bg.center().y;

                                for k in keys {
                                    let kx = rect.left() + (k as f32 * frame_w);
                                    let is_current = (self.creator.preview_frame_idx as u32) == k;

                                    // Draw on the summary track (always active looking if there's any key below)
                                    draw_diamond(
                                        egui::pos2(kx, summary_center_y),
                                        is_current,
                                        &painter,
                                    );
                                    // Draw on the emitter track
                                    draw_diamond(
                                        egui::pos2(kx, track_center_y),
                                        is_current,
                                        &painter,
                                    );
                                }

                                // Playhead (Blender Blue)
                                let curr_f = self.creator.preview_frame_idx as f32;
                                let h_x = rect.left() + (curr_f * frame_w);
                                let base_y = rect.top() + ruler_h;

                                // Playhead vertical line spans whole canvas
                                painter.line_segment(
                                    [egui::pos2(h_x, rect.top()), egui::pos2(h_x, rect.bottom())],
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(81, 140, 255)),
                                );

                                // Playhead Ruler Block (Upper marker)
                                painter.rect_filled(
                                    egui::Rect::from_center_size(
                                        egui::pos2(h_x, base_y - ruler_h / 2.0),
                                        egui::vec2(frame_w.max(8.0), ruler_h),
                                    ),
                                    1.0,
                                    egui::Color32::from_rgb(81, 140, 255).linear_multiply(0.5),
                                );

                                // Current frame text in the marker
                                painter.text(
                                    egui::pos2(h_x + 3.0, base_y - ruler_h / 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    format!("{}", self.creator.preview_frame_idx),
                                    egui::FontId::proportional(11.0),
                                    egui::Color32::WHITE,
                                );

                                // Mouse Interactions
                                if response.dragged() || response.clicked() {
                                    if let Some(pos) = response.interact_pointer_pos() {
                                        let rel_x = pos.x - rect.left();
                                        let f = (rel_x / frame_w).round() as i32;
                                        self.creator.preview_frame_idx = f.clamp(0, max_frame);
                                        // Update configuration context on hover/scrub
                                        if let Some(config) = self
                                            .creator
                                            .keyframes
                                            .get(&(self.creator.preview_frame_idx as u32))
                                        {
                                            self.creator.config = config.clone();
                                        }
                                    }
                                }
                            }
                        });
                });
            });

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
        let kf_vec: Vec<_> = self
            .creator
            .keyframes
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        let frames = editor::simulate(&self.creator.config, &kf_vec);
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
