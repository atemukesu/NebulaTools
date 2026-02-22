use super::app::{
    AnimKeyframe, CreatorObject, CreatorObjectData, ImageTextConfig, NebulaToolsApp,
    ParameterConfig, ShapeConfig,
};
use crate::editor::EmitterConfig;
use eframe::egui;

impl NebulaToolsApp {
    pub(crate) fn show_creator_workflow(&mut self, ctx: &egui::Context) {
        // Playback logic
        if self.creator.preview_playing {
            if let Some(ref frames) = self.creator.preview_frames {
                let dt = ctx.input(|i| i.stable_dt);
                self.creator.preview_timer += dt;
                let frame_dur = 1.0 / self.creator.target_fps as f32;
                if self.creator.preview_timer >= frame_dur {
                    self.creator.preview_timer -= frame_dur;
                    let next = self.creator.preview_frame_idx + 1;
                    if (next as usize) < frames.len() {
                        self.creator.preview_frame_idx = next;
                    } else {
                        self.creator.preview_frame_idx = 0;
                    }
                }
                ctx.request_repaint();
            }
        }

        // --- Left Panel: Object List + Global Settings ---
        egui::SidePanel::left("creator_left_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading(self.i18n.tr("creator"));
                    ui.separator();

                    // Add object dropdown
                    ui.horizontal(|ui| {
                        ui.menu_button(format!("➕ {}", self.i18n.tr("cr_add_object")), |ui| {
                            if ui.button(self.i18n.tr("cr_add_emitter")).clicked() {
                                self.creator.objects.push(CreatorObject {
                                    name: self.i18n.tr("cr_add_emitter").to_string(),
                                    data: CreatorObjectData::Emitter(EmitterConfig::default()),
                                    ..Default::default()
                                });
                                self.creator.selected_object = Some(self.creator.objects.len() - 1);
                                ui.close_menu();
                            }
                            if ui.button(self.i18n.tr("cr_add_shape")).clicked() {
                                self.creator.objects.push(CreatorObject {
                                    name: self.i18n.tr("cr_add_shape").to_string(),
                                    data: CreatorObjectData::Shape(ShapeConfig::default()),
                                    ..Default::default()
                                });
                                self.creator.selected_object = Some(self.creator.objects.len() - 1);
                                ui.close_menu();
                            }
                            if ui.button(self.i18n.tr("cr_add_param")).clicked() {
                                self.creator.objects.push(CreatorObject {
                                    name: self.i18n.tr("cr_add_param").to_string(),
                                    data: CreatorObjectData::Parameter(ParameterConfig {
                                        is_polar: false,
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                });
                                self.creator.selected_object = Some(self.creator.objects.len() - 1);
                                ui.close_menu();
                            }
                            if ui.button(self.i18n.tr("cr_add_polar")).clicked() {
                                self.creator.objects.push(CreatorObject {
                                    name: self.i18n.tr("cr_add_polar").to_string(),
                                    data: CreatorObjectData::Parameter(ParameterConfig {
                                        is_polar: true,
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                });
                                self.creator.selected_object = Some(self.creator.objects.len() - 1);
                                ui.close_menu();
                            }
                            if ui.button(self.i18n.tr("cr_add_image")).clicked() {
                                self.creator.objects.push(CreatorObject {
                                    name: self.i18n.tr("cr_add_image").to_string(),
                                    data: CreatorObjectData::ImageText(ImageTextConfig {
                                        is_text: false,
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                });
                                self.creator.selected_object = Some(self.creator.objects.len() - 1);
                                ui.close_menu();
                            }
                            if ui.button(self.i18n.tr("cr_add_text")).clicked() {
                                self.creator.objects.push(CreatorObject {
                                    name: self.i18n.tr("cr_add_text").to_string(),
                                    data: CreatorObjectData::ImageText(ImageTextConfig {
                                        is_text: true,
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                });
                                self.creator.selected_object = Some(self.creator.objects.len() - 1);
                                ui.close_menu();
                            }
                        });
                    });

                    ui.separator();

                    // Object list
                    let mut to_remove = None;
                    for (idx, obj) in self.creator.objects.iter_mut().enumerate() {
                        let is_selected = self.creator.selected_object == Some(idx);
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut obj.enabled, "");
                            let type_icon = match &obj.data {
                                CreatorObjectData::Emitter(_) => "✨",
                                CreatorObjectData::Shape(_) => "🔷",
                                CreatorObjectData::Parameter(p) => {
                                    if p.is_polar {
                                        "🌀"
                                    } else {
                                        "📐"
                                    }
                                }
                                CreatorObjectData::ImageText(c) => {
                                    if c.is_text {
                                        "🔤"
                                    } else {
                                        "🖼"
                                    }
                                }
                            };
                            let label = format!("{} {}", type_icon, obj.name);
                            if ui.selectable_label(is_selected, label).clicked() {
                                self.creator.selected_object = Some(idx);
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("❌").clicked() {
                                        to_remove = Some(idx);
                                    }
                                },
                            );
                        });
                    }

                    if let Some(idx) = to_remove {
                        self.creator.objects.remove(idx);
                        if self.creator.selected_object == Some(idx) {
                            self.creator.selected_object = None;
                        } else if let Some(sel) = self.creator.selected_object {
                            if sel > idx {
                                self.creator.selected_object = Some(sel - 1);
                            }
                        }
                    }

                    // --- Global Settings (at bottom) ---
                    ui.add_space(16.0);
                    ui.separator();
                    ui.label(
                        egui::RichText::new(self.i18n.tr("cr_global_settings"))
                            .strong()
                            .size(15.0),
                    );
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("target_fps_setting"));
                        ui.add(
                            egui::DragValue::new(&mut self.creator.target_fps)
                                .clamp_range(1..=1000)
                                .speed(1.0),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("cr_duration"));
                        ui.add(
                            egui::DragValue::new(&mut self.creator.duration_secs)
                                .clamp_range(0.01..=36000.0)
                                .speed(0.1)
                                .max_decimals(2)
                                .suffix(" s"),
                        );
                    });

                    let total_frames =
                        (self.creator.duration_secs * self.creator.target_fps as f32).ceil() as u32;
                    ui.label(
                        egui::RichText::new(format!(
                            "  → {} {}",
                            total_frames,
                            self.i18n.tr("frame")
                        ))
                        .weak(),
                    );

                    ui.add_space(12.0);

                    // Compile + Export buttons
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("cr_compile")))
                                    .strong()
                                    .size(15.0),
                            ),
                        )
                        .clicked()
                    {
                        self.compile_creator();
                    }

                    ui.add_space(4.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new(
                                egui::RichText::new(format!("💾 {}", self.i18n.tr("export_nbl")))
                                    .strong()
                                    .size(15.0),
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

        // --- Right Panel: Object Properties ---
        let selected_obj = self.creator.selected_object;
        if let Some(idx) = selected_obj {
            egui::SidePanel::right("creator_right_panel")
                .resizable(true)
                .default_width(340.0)
                .show(ctx, |ui| {
                    if idx < self.creator.objects.len() {
                        self.show_creator_object_editor(ui, idx);
                    } else {
                        self.creator.selected_object = None;
                    }
                });
        }

        // --- Bottom Panel: Dopesheet Timeline ---
        egui::TopBottomPanel::bottom("creator_dopesheet")
            .resizable(true)
            .min_height(100.0)
            .show(ctx, |ui| {
                self.show_creator_dopesheet(ui);
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

    fn show_creator_dopesheet(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);

        // Playback controls row
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

            // Keyframe operations on selected object
            if self.creator.selected_object.is_some() {
                if ui
                    .button(self.i18n.tr("cr_keyframe_add"))
                    .on_hover_text("Insert keyframe at current frame")
                    .clicked()
                {
                    if let Some(idx) = self.creator.selected_object {
                        let cf = self.creator.preview_frame_idx as u32;
                        let obj = &self.creator.objects[idx];
                        let kf = AnimKeyframe {
                            position: obj.position,
                            rotation: obj.rotation,
                            scale: obj.scale,
                            alpha: obj.alpha,
                        };
                        self.creator.objects[idx].keyframes.insert(cf, kf);
                    }
                }
                if ui
                    .button(self.i18n.tr("cr_keyframe_del"))
                    .on_hover_text("Remove keyframe at current frame")
                    .clicked()
                {
                    if let Some(idx) = self.creator.selected_object {
                        let cf = self.creator.preview_frame_idx as u32;
                        self.creator.objects[idx].keyframes.remove(&cf);
                    }
                }
            }

            ui.add_space(16.0);

            let mut max_frame =
                (self.creator.duration_secs * self.creator.target_fps as f32).ceil() as i32;
            max_frame = max_frame.max(100);
            ui.label(format!(
                "{}: {} / {}",
                self.i18n.tr("frame_count_label"),
                self.creator.preview_frame_idx,
                max_frame,
            ));
        });

        ui.add_space(4.0);

        // --- Dopesheet Canvas ---
        let dopesheet_height = ui.available_height().max(80.0);
        let channels_width = 160.0;
        let ruler_h = 22.0;
        let track_h = 20.0;

        ui.horizontal(|ui| {
            // LEFT: Channel labels
            let (channels_rect, _) = ui.allocate_exact_size(
                egui::vec2(channels_width, dopesheet_height),
                egui::Sense::hover(),
            );
            let painter = ui.painter_at(channels_rect);
            painter.rect_filled(channels_rect, 0.0, egui::Color32::from_rgb(34, 34, 34));

            // Ruler row header
            let ruler_rect = egui::Rect::from_min_size(
                channels_rect.min,
                egui::vec2(channels_rect.width(), ruler_h),
            );
            painter.rect_filled(ruler_rect, 0.0, egui::Color32::from_rgb(45, 45, 45));
            painter.text(
                ruler_rect.min + egui::vec2(8.0, 4.0),
                egui::Align2::LEFT_TOP,
                self.i18n.tr("dopesheet_summary"),
                egui::FontId::proportional(11.0),
                egui::Color32::from_rgb(200, 200, 200),
            );

            // One track per object
            for (i, obj) in self.creator.objects.iter().enumerate() {
                let ty = channels_rect.top() + ruler_h + (i as f32 * track_h);
                if ty + track_h > channels_rect.bottom() {
                    break;
                }
                let trect = egui::Rect::from_min_size(
                    egui::pos2(channels_rect.left(), ty),
                    egui::vec2(channels_rect.width(), track_h),
                );
                let bg = if self.creator.selected_object == Some(i) {
                    egui::Color32::from_rgb(50, 55, 65)
                } else if i % 2 == 0 {
                    egui::Color32::from_rgb(40, 40, 40)
                } else {
                    egui::Color32::from_rgb(38, 38, 38)
                };
                painter.rect_filled(trect, 0.0, bg);
                let label = if obj.name.len() > 18 {
                    format!("{}…", &obj.name[..17])
                } else {
                    obj.name.clone()
                };
                painter.text(
                    trect.min + egui::vec2(8.0, 3.0),
                    egui::Align2::LEFT_TOP,
                    label,
                    egui::FontId::proportional(11.0),
                    if obj.enabled {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(100, 100, 100)
                    },
                );
            }

            // RIGHT: Timeline canvas in scroll area
            egui::ScrollArea::horizontal()
                .id_source("creator_dopesheet_scroll")
                .show(ui, |ui| {
                    let mut max_frame =
                        (self.creator.duration_secs * self.creator.target_fps as f32).ceil() as i32;
                    max_frame = max_frame.max(100);

                    let frame_w = 10.0;
                    let canvas_width = (max_frame as f32 * frame_w).max(ui.available_width());

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(canvas_width, dopesheet_height),
                        egui::Sense::click_and_drag(),
                    );

                    if ui.is_rect_visible(rect) {
                        let painter = ui.painter_at(rect);

                        // Background
                        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(57, 57, 57));

                        // Track backgrounds
                        for i in 0..self.creator.objects.len() {
                            let ty = rect.top() + ruler_h + (i as f32 * track_h);
                            if ty + track_h > rect.bottom() {
                                break;
                            }
                            let bg = if i % 2 == 0 {
                                egui::Color32::from_rgb(50, 50, 50)
                            } else {
                                egui::Color32::from_rgb(53, 53, 53)
                            };
                            painter.rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(rect.left(), ty),
                                    egui::vec2(rect.width(), track_h),
                                ),
                                0.0,
                                bg,
                            );
                        }

                        // Ruler background
                        let ruler_rect =
                            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), ruler_h));
                        painter.rect_filled(ruler_rect, 0.0, egui::Color32::from_rgb(60, 60, 60));

                        // Grid lines
                        for f in 0..=max_frame {
                            let x = rect.left() + f as f32 * frame_w;
                            if f % 10 == 0 {
                                painter.line_segment(
                                    [
                                        egui::pos2(x, rect.top() + ruler_h / 2.0),
                                        egui::pos2(x, rect.bottom()),
                                    ],
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 45)),
                                );
                                painter.text(
                                    egui::pos2(x + 2.0, rect.top() + 2.0),
                                    egui::Align2::LEFT_TOP,
                                    format!("{}", f),
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::from_rgb(200, 200, 200),
                                );
                            } else if f % 5 == 0 {
                                painter.line_segment(
                                    [
                                        egui::pos2(x, rect.top() + ruler_h * 0.75),
                                        egui::pos2(x, rect.bottom()),
                                    ],
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(48, 48, 48)),
                                );
                            }
                        }

                        // Draw keyframe diamonds per object track
                        let draw_diamond = |pos: egui::Pos2, active: bool, p: &egui::Painter| {
                            let r = 4.0;
                            let pts = vec![
                                egui::pos2(pos.x, pos.y - r),
                                egui::pos2(pos.x + r, pos.y),
                                egui::pos2(pos.x, pos.y + r),
                                egui::pos2(pos.x - r, pos.y),
                            ];
                            let (fill, stroke_c) = if active {
                                (egui::Color32::from_rgb(255, 204, 0), egui::Color32::BLACK)
                            } else {
                                (egui::Color32::from_rgb(200, 200, 200), egui::Color32::BLACK)
                            };
                            p.add(egui::Shape::convex_polygon(
                                pts,
                                fill,
                                egui::Stroke::new(0.5, stroke_c),
                            ));
                        };

                        for (i, obj) in self.creator.objects.iter().enumerate() {
                            let ty = rect.top() + ruler_h + (i as f32 * track_h);
                            let cy = ty + track_h / 2.0;
                            if ty + track_h > rect.bottom() {
                                break;
                            }
                            for &kf_frame in obj.keyframes.keys() {
                                let kx = rect.left() + kf_frame as f32 * frame_w;
                                let is_current =
                                    (self.creator.preview_frame_idx as u32) == kf_frame;
                                draw_diamond(egui::pos2(kx, cy), is_current, &painter);
                            }
                        }

                        // Playhead
                        let curr_f = self.creator.preview_frame_idx as f32;
                        let h_x = rect.left() + curr_f * frame_w;

                        painter.line_segment(
                            [egui::pos2(h_x, rect.top()), egui::pos2(h_x, rect.bottom())],
                            egui::Stroke::new(1.5, egui::Color32::from_rgb(81, 140, 255)),
                        );

                        // Playhead marker
                        painter.rect_filled(
                            egui::Rect::from_center_size(
                                egui::pos2(h_x, rect.top() + ruler_h / 2.0),
                                egui::vec2(frame_w.max(8.0), ruler_h),
                            ),
                            1.0,
                            egui::Color32::from_rgb(81, 140, 255).linear_multiply(0.5),
                        );
                        painter.text(
                            egui::pos2(h_x + 3.0, rect.top() + ruler_h / 2.0),
                            egui::Align2::LEFT_CENTER,
                            format!("{}", self.creator.preview_frame_idx),
                            egui::FontId::proportional(10.0),
                            egui::Color32::WHITE,
                        );

                        // Mouse interaction
                        if response.dragged() || response.clicked() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let rel_x = pos.x - rect.left();
                                let f = (rel_x / frame_w).round() as i32;
                                self.creator.preview_frame_idx = f.clamp(0, max_frame);
                            }
                        }
                    }
                });
        });
    }
}

pub(crate) fn color_u8_to_f32(c: [u8; 4]) -> [f32; 4] {
    [
        c[0] as f32 / 255.0,
        c[1] as f32 / 255.0,
        c[2] as f32 / 255.0,
        c[3] as f32 / 255.0,
    ]
}

pub(crate) fn color_f32_to_u8(c: [f32; 4]) -> [u8; 4] {
    [
        (c[0] * 255.0).round().clamp(0.0, 255.0) as u8,
        (c[1] * 255.0).round().clamp(0.0, 255.0) as u8,
        (c[2] * 255.0).round().clamp(0.0, 255.0) as u8,
        (c[3] * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}
