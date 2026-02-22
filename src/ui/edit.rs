use super::app::NebulaToolsApp;
use crate::player;
use eframe::egui;
use std::sync::{Arc, Mutex};

/// Accent color for labels and highlights in the edit panel
const ACCENT: egui::Color32 = egui::Color32::from_rgb(120, 180, 255);
const HINT_COLOR: egui::Color32 = egui::Color32::from_rgb(160, 160, 180);
const PREVIEW_COLOR: egui::Color32 = egui::Color32::from_rgb(100, 220, 160);

impl NebulaToolsApp {
    pub(crate) fn show_edit_workflow(&mut self, ctx: &egui::Context) {
        // Set up header info without decoding all frames (lazy decode)
        if self.edit.edited_header.is_none() && self.player.header.is_some() {
            let header = self.player.header.clone().unwrap();
            self.edit.trim_end = header.total_frames.saturating_sub(1);
            self.edit.new_fps = header.target_fps;
            self.edit.edited_header = Some(header);
        }

        // --- Side Panel: Tool Selection ---
        egui::SidePanel::left("edit_side")
            .resizable(false)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading(self.i18n.tr("edit_tools"));
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                let tools = [
                    (crate::ui::app::EditTool::Speed, "edit_anim_speed"),
                    (crate::ui::app::EditTool::Size, "edit_particle_size"),
                    (crate::ui::app::EditTool::Color, "edit_color"),
                    (crate::ui::app::EditTool::Transform, "edit_transform"),
                    (crate::ui::app::EditTool::Trim, "edit_trim"),
                    (crate::ui::app::EditTool::Compress, "edit_compress"),
                ];

                for (tool, lang_key) in tools {
                    let is_selected = self.edit.selected_tool == tool;
                    let text = self.i18n.tr(lang_key);

                    let response = ui.selectable_label(is_selected, text);
                    if response.clicked() {
                        self.edit.selected_tool = tool;
                    }
                    ui.add_space(4.0);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(8.0);
                    // Save button removed
                    ui.separator();
                });
            });

        // --- Central Panel: Parameters & Summary ---
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.player.header.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label(self.i18n.tr("no_file_loaded"));
                });
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Header of the selected tool
                let title = match self.edit.selected_tool {
                    crate::ui::app::EditTool::Speed => self.i18n.tr("edit_anim_speed"),
                    crate::ui::app::EditTool::Size => self.i18n.tr("edit_particle_size"),
                    crate::ui::app::EditTool::Color => self.i18n.tr("edit_color"),
                    crate::ui::app::EditTool::Transform => self.i18n.tr("edit_transform"),
                    crate::ui::app::EditTool::Trim => self.i18n.tr("edit_trim"),
                    crate::ui::app::EditTool::Compress => self.i18n.tr("edit_compress"),
                };
                ui.label(egui::RichText::new(title).size(26.0).strong().color(ACCENT));
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(12.0);

                // Parameters view
                egui::Frame::none()
                    .fill(ui.visuals().faint_bg_color)
                    .inner_margin(16.0)
                    .rounding(8.0)
                    .show(ui, |ui| match self.edit.selected_tool {
                        crate::ui::app::EditTool::Speed => self.ui_speed_params(ui),
                        crate::ui::app::EditTool::Size => self.ui_size_params(ui),
                        crate::ui::app::EditTool::Color => self.ui_color_params(ui),
                        crate::ui::app::EditTool::Transform => self.ui_transform_params(ui),
                        crate::ui::app::EditTool::Trim => self.ui_trim_params(ui),
                        crate::ui::app::EditTool::Compress => self.ui_compress_params(ui),
                    });

                self.ui_progress_bar(ui);

                ui.add_space(16.0);

                // Status message
                if let Some(ref msg) = self.edit.status_msg {
                    let color = if msg.starts_with('✅') {
                        egui::Color32::from_rgb(80, 200, 80)
                    } else {
                        egui::Color32::from_rgb(255, 100, 100)
                    };
                    ui.colored_label(color, egui::RichText::new(msg).size(14.0));
                    ui.add_space(8.0);
                }

                ui.separator();
                ui.add_space(8.0);

                // Summary stats
                ui.label(
                    egui::RichText::new(self.i18n.tr("metadata"))
                        .size(18.0)
                        .strong()
                        .color(ACCENT),
                );
                ui.add_space(6.0);

                if let Some(ref header) = self.edit.edited_header {
                    let frame_count = self
                        .edit
                        .decoded_frames
                        .as_ref()
                        .map(|f| f.len())
                        .unwrap_or(header.total_frames as usize);
                    let duration = if header.target_fps > 0 {
                        frame_count as f32 / header.target_fps as f32
                    } else {
                        0.0
                    };

                    egui::Grid::new("edit_summary_grid")
                        .num_columns(2)
                        .spacing([20.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(self.i18n.tr("fps")).color(HINT_COLOR));
                            ui.label(
                                egui::RichText::new(format!("{}", header.target_fps)).strong(),
                            );
                            ui.end_row();

                            ui.label(
                                egui::RichText::new(self.i18n.tr("total_frames")).color(HINT_COLOR),
                            );
                            ui.label(egui::RichText::new(format!("{}", frame_count)).strong());
                            ui.end_row();

                            ui.label(
                                egui::RichText::new(self.i18n.tr("duration")).color(HINT_COLOR),
                            );
                            ui.label(egui::RichText::new(format!("{:.2} s", duration)).strong());
                            ui.end_row();

                            ui.label(egui::RichText::new(self.i18n.tr("bbox")).color(HINT_COLOR));
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
                }
            });
        });
    }

    fn ui_speed_params(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(self.i18n.tr("edit_anim_speed_desc"))
                .color(HINT_COLOR)
                .size(14.0),
        );
        ui.add_space(16.0);

        // Get current frame info for preview calculations
        let current_fps = self
            .edit
            .edited_header
            .as_ref()
            .map(|h| h.target_fps)
            .unwrap_or(30);
        let current_frame_count = self
            .edit
            .decoded_frames
            .as_ref()
            .map(|f| f.len())
            .unwrap_or(
                self.edit
                    .edited_header
                    .as_ref()
                    .map(|h| h.total_frames as usize)
                    .unwrap_or(0),
            );

        // ── Mode 0: FPS Only ──
        let mode0_selected = self.edit.speed_mode == 0;
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.edit.speed_mode, 0, "");
            ui.label(
                egui::RichText::new(self.i18n.tr("speed_mode_fps_only"))
                    .strong()
                    .size(15.0),
            );
        });
        ui.indent("speed_mode_0_body", |ui| {
            ui.label(
                egui::RichText::new(self.i18n.tr("speed_mode_fps_only_desc"))
                    .color(HINT_COLOR)
                    .size(12.0),
            );
            ui.add_space(6.0);
            ui.set_enabled(mode0_selected);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("FPS:").strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.new_fps)
                        .clamp_range(1..=240)
                        .speed(1.0),
                );
            });
            // Duration preview
            if current_frame_count > 0 && mode0_selected {
                let old_duration = if current_fps > 0 {
                    current_frame_count as f32 / current_fps as f32
                } else {
                    0.0
                };
                let new_duration = if self.edit.new_fps > 0 {
                    current_frame_count as f32 / self.edit.new_fps as f32
                } else {
                    0.0
                };
                ui.label(
                    egui::RichText::new(format!(
                        "⏱ {:.2}s → {:.2}s  ({} {})",
                        old_duration,
                        new_duration,
                        current_frame_count,
                        self.i18n.tr("frame")
                    ))
                    .color(PREVIEW_COLOR)
                    .size(12.0),
                );
            }
        });

        ui.add_space(12.0);

        // ── Mode 1: Interpolate ──
        let mode1_selected = self.edit.speed_mode == 1;
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.edit.speed_mode, 1, "");
            ui.label(
                egui::RichText::new(self.i18n.tr("speed_mode_interp"))
                    .strong()
                    .size(15.0),
            );
        });
        ui.indent("speed_mode_1_body", |ui| {
            ui.label(
                egui::RichText::new(self.i18n.tr("speed_mode_interp_desc"))
                    .color(HINT_COLOR)
                    .size(12.0),
            );
            ui.add_space(6.0);
            ui.set_enabled(mode1_selected);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("×").strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.speed_factor)
                        .clamp_range(0.1..=10.0)
                        .speed(0.05)
                        .fixed_decimals(2),
                );
            });
            if current_frame_count > 0 && mode1_selected {
                let new_count =
                    ((current_frame_count as f32) / self.edit.speed_factor).round() as usize;
                let new_duration = if current_fps > 0 {
                    new_count as f32 / current_fps as f32
                } else {
                    0.0
                };
                ui.label(
                    egui::RichText::new(format!(
                        "🎞 {} → {} {}  (⏱ {:.2}s)",
                        current_frame_count,
                        new_count,
                        self.i18n.tr("frame"),
                        new_duration
                    ))
                    .color(PREVIEW_COLOR)
                    .size(12.0),
                );
            }
        });

        ui.add_space(12.0);

        // ── Mode 2: Both ──
        let mode2_selected = self.edit.speed_mode == 2;
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.edit.speed_mode, 2, "");
            ui.label(
                egui::RichText::new(self.i18n.tr("speed_mode_both"))
                    .strong()
                    .size(15.0),
            );
        });
        ui.indent("speed_mode_2_body", |ui| {
            ui.label(
                egui::RichText::new(self.i18n.tr("speed_mode_both_desc"))
                    .color(HINT_COLOR)
                    .size(12.0),
            );
            ui.add_space(6.0);
            ui.set_enabled(mode2_selected);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("FPS:").strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.new_fps)
                        .clamp_range(1..=240)
                        .speed(1.0),
                );
                ui.add_space(12.0);
                ui.label(egui::RichText::new("×").strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.speed_factor)
                        .clamp_range(0.1..=10.0)
                        .speed(0.05)
                        .fixed_decimals(2),
                );
            });
            if current_frame_count > 0 && mode2_selected {
                let new_count =
                    ((current_frame_count as f32) / self.edit.speed_factor).round() as usize;
                let new_duration = if self.edit.new_fps > 0 {
                    new_count as f32 / self.edit.new_fps as f32
                } else {
                    0.0
                };
                ui.label(
                    egui::RichText::new(format!(
                        "🎞 {} → {} {}  |  FPS {} → {}  |  ⏱ {:.2}s",
                        current_frame_count,
                        new_count,
                        self.i18n.tr("frame"),
                        current_fps,
                        self.edit.new_fps,
                        new_duration
                    ))
                    .color(PREVIEW_COLOR)
                    .size(12.0),
                );
            }
        });

        ui.add_space(20.0);
        ui.add_space(20.0);
        if ui
            .add_sized(
                [ui.available_width().min(200.0), 32.0],
                egui::Button::new(
                    egui::RichText::new(format!("▶ {}", self.i18n.tr("export_nbl")))
                        .strong()
                        .size(15.0),
                ),
            )
            .clicked()
        {
            let action = match self.edit.speed_mode {
                0 => player::EditAction::ChangeFps(self.edit.new_fps),
                1 => player::EditAction::Interpolate(self.edit.speed_factor),
                2 => {
                    player::EditAction::InterpolateAndFps(self.edit.speed_factor, self.edit.new_fps)
                }
                _ => player::EditAction::ChangeFps(self.edit.new_fps),
            };
            self.start_export(action);
        }
    }

    fn ui_size_params(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(self.i18n.tr("edit_particle_size_desc"))
                .color(HINT_COLOR)
                .size(14.0),
        );
        ui.add_space(16.0);

        // ── Scale ──
        let scale_selected = self.edit.size_mode == 0;
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.edit.size_mode, 0, "");
            ui.label(
                egui::RichText::new(self.i18n.tr("size_mode_scale"))
                    .strong()
                    .size(15.0),
            );
        });
        ui.indent("size_mode_0_body", |ui| {
            ui.label(
                egui::RichText::new(self.i18n.tr("size_mode_scale_desc"))
                    .color(HINT_COLOR)
                    .size(12.0),
            );
            ui.add_space(6.0);
            ui.set_enabled(scale_selected);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("×").strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.size_scale)
                        .clamp_range(0.000001..=100.0)
                        .speed(0.001)
                        .max_decimals(6),
                );
            });
        });

        ui.add_space(12.0);

        // ── Uniform ──
        let uniform_selected = self.edit.size_mode == 1;
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.edit.size_mode, 1, "");
            ui.label(
                egui::RichText::new(self.i18n.tr("size_mode_uniform"))
                    .strong()
                    .size(15.0),
            );
        });
        ui.indent("size_mode_1_body", |ui| {
            ui.label(
                egui::RichText::new(self.i18n.tr("size_mode_uniform_desc"))
                    .color(HINT_COLOR)
                    .size(12.0),
            );
            ui.add_space(6.0);
            ui.set_enabled(uniform_selected);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("=").strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.size_uniform)
                        .clamp_range(0.0..=655.0)
                        .speed(0.01)
                        .fixed_decimals(2),
                );
            });
        });

        ui.add_space(20.0);
        ui.add_space(20.0);
        if ui
            .add_sized(
                [ui.available_width().min(200.0), 32.0],
                egui::Button::new(
                    egui::RichText::new(format!("▶ {}", self.i18n.tr("export_nbl")))
                        .strong()
                        .size(15.0),
                ),
            )
            .clicked()
        {
            let action = match self.edit.size_mode {
                0 => player::EditAction::ScaleSize(self.edit.size_scale),
                1 => player::EditAction::UniformSize(self.edit.size_uniform),
                _ => player::EditAction::ScaleSize(1.0),
            };
            self.start_export(action);
        }
    }

    fn ui_color_params(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(self.i18n.tr("edit_color_desc"))
                .color(HINT_COLOR)
                .size(14.0),
        );
        ui.add_space(16.0);

        egui::Grid::new("color_grid")
            .num_columns(2)
            .spacing([16.0, 12.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new(self.i18n.tr("brightness_factor")).strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.brightness)
                        .clamp_range(0.0..=f32::MAX)
                        .speed(0.01)
                        .max_decimals(4),
                );
                ui.end_row();

                ui.label(egui::RichText::new(self.i18n.tr("opacity_factor")).strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.opacity)
                        .clamp_range(0.0..=f32::MAX)
                        .speed(0.01)
                        .max_decimals(4),
                );
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.add_space(20.0);
        if ui
            .add_sized(
                [ui.available_width().min(200.0), 32.0],
                egui::Button::new(
                    egui::RichText::new(format!("▶ {}", self.i18n.tr("export_nbl")))
                        .strong()
                        .size(15.0),
                ),
            )
            .clicked()
        {
            self.start_export(player::EditAction::AdjustColor(
                self.edit.brightness,
                self.edit.opacity,
            ));
        }
    }

    fn ui_transform_params(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(self.i18n.tr("edit_transform_desc"))
                .color(HINT_COLOR)
                .size(14.0),
        );
        ui.add_space(16.0);

        egui::Grid::new("transform_grid")
            .num_columns(2)
            .spacing([16.0, 12.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new(self.i18n.tr("translate_offset")).strong());
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.edit.translate[0]).speed(0.1));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.edit.translate[1]).speed(0.1));
                    ui.label("Z:");
                    ui.add(egui::DragValue::new(&mut self.edit.translate[2]).speed(0.1));
                });
                ui.end_row();

                ui.label(egui::RichText::new(self.i18n.tr("position_scale")).strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.pos_scale)
                        .clamp_range(0.000001..=100.0)
                        .speed(0.001)
                        .max_decimals(6),
                );
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.add_space(20.0);
        if ui
            .add_sized(
                [ui.available_width().min(200.0), 32.0],
                egui::Button::new(
                    egui::RichText::new(format!("▶ {}", self.i18n.tr("export_nbl")))
                        .strong()
                        .size(15.0),
                ),
            )
            .clicked()
        {
            self.start_export(player::EditAction::Transform(
                self.edit.translate,
                self.edit.pos_scale,
            ));
        }
    }

    fn ui_trim_params(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(self.i18n.tr("edit_trim_desc"))
                .color(HINT_COLOR)
                .size(14.0),
        );
        ui.add_space(16.0);

        let max_frame = self
            .edit
            .decoded_frames
            .as_ref()
            .map(|f| f.len().saturating_sub(1) as u32)
            .unwrap_or(
                self.edit
                    .edited_header
                    .as_ref()
                    .map(|h| h.total_frames.saturating_sub(1))
                    .unwrap_or(0),
            );

        egui::Grid::new("trim_grid")
            .num_columns(2)
            .spacing([16.0, 12.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new(self.i18n.tr("trim_start")).strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.trim_start)
                        .clamp_range(0..=max_frame)
                        .speed(1.0),
                );
                ui.end_row();

                ui.label(egui::RichText::new(self.i18n.tr("trim_end")).strong());
                ui.add(
                    egui::DragValue::new(&mut self.edit.trim_end)
                        .clamp_range(0..=max_frame)
                        .speed(1.0),
                );
                ui.end_row();
            });

        let start = self.edit.trim_start as usize;
        let end = (self.edit.trim_end as usize).min(max_frame as usize);
        if end >= start {
            ui.add_space(8.0);
            let result_count = end - start + 1;
            let fps = self
                .edit
                .edited_header
                .as_ref()
                .map(|h| h.target_fps)
                .unwrap_or(30);
            let duration = if fps > 0 {
                result_count as f32 / fps as f32
            } else {
                0.0
            };
            ui.label(
                egui::RichText::new(format!(
                    "🎞 {} {}  (⏱ {:.2}s)",
                    result_count,
                    self.i18n.tr("frame"),
                    duration
                ))
                .color(PREVIEW_COLOR)
                .size(13.0),
            );
        }

        ui.add_space(20.0);
        ui.add_space(20.0);
        if ui
            .add_sized(
                [ui.available_width().min(200.0), 32.0],
                egui::Button::new(
                    egui::RichText::new(format!("▶ {}", self.i18n.tr("export_nbl")))
                        .strong()
                        .size(15.0),
                ),
            )
            .clicked()
        {
            self.start_export(player::EditAction::Trim(
                self.edit.trim_start,
                self.edit.trim_end,
            ));
        }
    }

    /// Decode all frames on demand. Returns true if frames are available.

    fn ui_compress_params(&mut self, ui: &mut egui::Ui) {
        // Progress logic moved to ui_progress_bar

        // Warning message
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("⚠")
                    .color(egui::Color32::from_rgb(255, 200, 50))
                    .size(18.0),
            );
            ui.label(
                egui::RichText::new(self.i18n.tr("compress_warning"))
                    .color(egui::Color32::from_rgb(255, 200, 50))
                    .strong()
                    .size(14.0),
            );
        });
        ui.add_space(10.0);

        ui.label(
            egui::RichText::new(self.i18n.tr("edit_compress_desc"))
                .color(HINT_COLOR)
                .size(13.0),
        );
        ui.add_space(16.0);

        egui::Grid::new("compress_grid")
            .num_columns(2)
            .spacing([16.0, 14.0])
            .show(ui, |ui| {
                // Keyframe interval
                ui.label(egui::RichText::new(self.i18n.tr("compress_keyframe_interval")).strong());
                ui.vertical(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.edit.compress_keyframe_interval)
                            .clamp_range(0..=120)
                            .speed(1.0),
                    );
                    ui.label(
                        egui::RichText::new(self.i18n.tr("compress_keyframe_interval_desc"))
                            .color(HINT_COLOR)
                            .size(11.0),
                    );
                });
                ui.end_row();

                // Zstd level
                ui.label(egui::RichText::new(self.i18n.tr("compress_zstd_level")).strong());
                ui.vertical(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.edit.compress_zstd_level)
                            .clamp_range(1..=5)
                            .speed(1.0),
                    );
                    ui.label(
                        egui::RichText::new(self.i18n.tr("compress_zstd_level_desc"))
                            .color(HINT_COLOR)
                            .size(11.0),
                    );
                });
                ui.end_row();
            });

        ui.add_space(20.0);
        ui.add_space(20.0);
        if ui
            .add_sized(
                [ui.available_width().min(240.0), 32.0],
                egui::Button::new(
                    egui::RichText::new(format!("▶ {}", self.i18n.tr("compress_export")))
                        .strong()
                        .size(15.0),
                ),
            )
            .clicked()
        {
            self.start_export(player::EditAction::Compress(
                self.edit.compress_keyframe_interval,
            ));
        }
    }

    fn ui_progress_bar(&mut self, ui: &mut egui::Ui) {
        let progress_state = self.edit.compress_progress.as_ref().map(|prog| {
            let p = prog.lock().unwrap();
            (
                p.current_frame,
                p.total_frames,
                p.is_done,
                p.error.clone(),
                p.start_time,
            )
        });

        if let Some((current, total, is_done, error, start_time)) = progress_state {
            if !is_done && error.is_none() {
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                ui.label(
                    egui::RichText::new(self.i18n.tr("compress_progress"))
                        .strong()
                        .size(16.0)
                        .color(ACCENT),
                );
                ui.add_space(8.0);

                let fraction = current as f32 / total.max(1) as f32;

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{} / {}", current, total))
                            .strong()
                            .color(ACCENT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{:.1}%", fraction * 100.0));
                    });
                });

                ui.scope(|ui| {
                    ui.style_mut().visuals.extreme_bg_color = egui::Color32::from_gray(150);
                    ui.add_sized(
                        [ui.available_width(), 16.0],
                        egui::ProgressBar::new(fraction).animate(true).fill(ACCENT),
                    );
                });

                let elapsed = start_time.elapsed().as_secs_f32();
                if current > 0 {
                    let rate = current as f32 / elapsed;
                    let remaining = (total - current) as f32 / rate;

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("ETA: {:.0}s", remaining))
                                .color(HINT_COLOR),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.0} fps", rate)).color(HINT_COLOR),
                            );
                        });
                    });
                }

                ui.ctx().request_repaint();
                return;
            }

            // Done or Error logic
            if is_done {
                self.edit.compress_progress = None;
                self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
            } else if let Some(err) = error {
                self.edit.compress_progress = None;
                self.edit.status_msg = Some(format!("❌ {}", err));
            }
        }
    }

    fn start_export(&mut self, action: player::EditAction) {
        if let Some(ref prog) = self.edit.compress_progress {
            if let Ok(p) = prog.lock() {
                if !p.is_done && p.error.is_none() {
                    return; // Already busy
                }
            }
        }

        let source_path = match self.player.file_path.clone() {
            Some(p) => p,
            None => {
                self.edit.status_msg = Some(self.i18n.tr("no_file_loaded").to_string());
                return;
            }
        };

        if let Some(output_path) = rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .set_file_name("export.nbl")
            .save_file()
        {
            let progress = Arc::new(Mutex::new(player::CompressProgress {
                total_frames: 0,
                current_frame: 0,
                is_done: false,
                error: None,
                start_time: std::time::Instant::now(),
            }));
            self.edit.compress_progress = Some(progress.clone());

            // Use level 3 as broad default, can optimize later
            let zstd_level = self.edit.compress_zstd_level.clamp(1, 22);

            std::thread::spawn(move || {
                if let Err(e) = player::streaming_edit(
                    source_path,
                    output_path,
                    action,
                    zstd_level,
                    progress.clone(),
                ) {
                    if let Ok(mut p) = progress.lock() {
                        p.error = Some(format!("{}", e));
                    }
                }
            });
        }
    }
}
