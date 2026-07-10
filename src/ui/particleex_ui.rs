use super::app::{NebulaToolsApp, PexCommandEntry};
use crate::particleex::{
    self, CompileEntry, ParticleexCommand, ParticleexCommandFormat, ParticleexEditorMode,
};
use crate::player::{self, NblHeader};
use eframe::egui;

impl NebulaToolsApp {
    pub(crate) fn show_particleex_workflow(&mut self, ctx: &egui::Context) {
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
                        self.pex.preview_frame_idx = 0;
                    }
                }
                ctx.request_repaint();
            }
        }

        if let Some(fs_idx) = self.pex.fullscreen_entry {
            if fs_idx < self.pex.entries.len() {
                let mut close = false;
                egui::Area::new(egui::Id::new("pex_fullscreen"))
                    .fixed_pos(egui::pos2(0.0, 0.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        let screen = ctx.screen_rect();
                        ui.allocate_exact_size(screen.size(), egui::Sense::hover());

                        let panel_rect = screen.shrink(32.0);
                        ui.painter()
                            .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(160));
                        ui.painter()
                            .rect_filled(panel_rect, 12.0, ui.visuals().window_fill);

                        ui.allocate_ui_at_rect(panel_rect.shrink(16.0), |ui| {
                            self.show_particleex_fullscreen_editor(ui, fs_idx, &mut close);
                        });
                    });
                if close {
                    self.pex.fullscreen_entry = None;
                }
                return;
            } else {
                self.pex.fullscreen_entry = None;
            }
        }

        egui::SidePanel::left("particleex_side")
            .resizable(true)
            .default_width(420.0)
            .min_width(320.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.heading(self.i18n.tr("particleex_title"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button("❓")
                                .on_hover_text(self.i18n.tr("particleex_hint"))
                                .clicked()
                            {
                                self.pex.show_help = !self.pex.show_help;
                            }
                        });
                    });
                    ui.separator();

                    if self.pex.show_help {
                        ui.add_space(4.0);
                        egui::Frame::none()
                            .fill(ui.visuals().faint_bg_color)
                            .inner_margin(8.0)
                            .rounding(6.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("pex_help_title"))
                                        .strong()
                                        .size(16.0),
                                );
                                ui.add_space(8.0);

                                egui::Grid::new("pex_help_grid")
                                    .num_columns(2)
                                    .spacing([12.0, 10.0])
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(self.i18n.tr("pex_help_commands"))
                                                .strong(),
                                        );
                                        ui.label(self.i18n.tr("pex_help_commands_val"));
                                        ui.end_row();

                                        ui.label(
                                            egui::RichText::new(self.i18n.tr("pex_help_vars"))
                                                .strong(),
                                        );
                                        ui.label(self.i18n.tr("pex_help_vars_val"));
                                        ui.end_row();

                                        ui.label(
                                            egui::RichText::new(self.i18n.tr("pex_help_math"))
                                                .strong(),
                                        );
                                        ui.label(self.i18n.tr("pex_help_math_val"));
                                        ui.end_row();

                                        ui.label(
                                            egui::RichText::new(
                                                self.i18n.tr("pex_help_example_title"),
                                            )
                                            .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(
                                                self.i18n.tr("pex_help_example_val"),
                                            )
                                            .monospace(),
                                        );
                                        ui.end_row();
                                    });
                            });
                        ui.separator();
                    }

                    let entry_count = self.pex.entries.len();
                    let mut remove_idx: Option<usize> = None;

                    for i in 0..entry_count {
                        self.sync_entry_from_text_if_needed(i);
                        ui.add_space(8.0);
                        let entry_id = format!("pex_entry_{}", i);
                        egui::Frame::none()
                            .fill(ui.visuals().faint_bg_color)
                            .inner_margin(10.0)
                            .rounding(8.0)
                            .stroke(egui::Stroke::new(
                                1.0,
                                ui.visuals().widgets.noninteractive.bg_stroke.color,
                            ))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.pex.entries[i].enabled, "");
                                    ui.label(
                                        egui::RichText::new(format!("#{}", i + 1))
                                            .strong()
                                            .size(14.0),
                                    );

                                    if let Some(model) = &self.pex.entries[i].wizard_model {
                                        ui.label(
                                            egui::RichText::new(model.format_label())
                                                .small()
                                                .monospace(),
                                        );
                                    }

                                    let status = self.entry_validation_text(i);
                                    let status_color = if status.starts_with('✅') {
                                        egui::Color32::from_rgb(80, 200, 80)
                                    } else {
                                        egui::Color32::from_rgb(255, 100, 100)
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(status).color(status_color),
                                        )
                                        .wrap(true),
                                    );

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if entry_count > 1 {
                                                if ui
                                                    .small_button("🗑")
                                                    .on_hover_text(self.i18n.tr("remove"))
                                                    .clicked()
                                                {
                                                    self.pex.confirm_delete = Some(i);
                                                }
                                            }
                                            if ui
                                                .small_button("⛶")
                                                .on_hover_text(self.i18n.tr("fullscreen"))
                                                .clicked()
                                            {
                                                self.pex.fullscreen_entry = Some(i);
                                            }
                                        },
                                    );
                                });

                                ui.add_space(6.0);
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("pex_command_preview"))
                                        .strong(),
                                );
                                let mut preview_text = self.pex.entries[i].command.clone();
                                ui.add(
                                    egui::TextEdit::multiline(&mut preview_text)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(3)
                                        .interactive(false)
                                        .code_editor(),
                                );

                                ui.add_space(6.0);
                                egui::Grid::new(format!("{}_grid", entry_id))
                                    .num_columns(2)
                                    .spacing([8.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(self.i18n.tr("pex_start_tick"));
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.pex.entries[i].start_tick,
                                            )
                                            .speed(1.0)
                                            .clamp_range(0.0..=100000.0_f32)
                                            .suffix(" tick"),
                                        );
                                        ui.end_row();

                                        ui.label(self.i18n.tr("pex_position"));
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.pex.entries[i].position[0],
                                                )
                                                .speed(0.1)
                                                .prefix("X:"),
                                            );
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.pex.entries[i].position[1],
                                                )
                                                .speed(0.1)
                                                .prefix("Y:"),
                                            );
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.pex.entries[i].position[2],
                                                )
                                                .speed(0.1)
                                                .prefix("Z:"),
                                            );
                                        });
                                        ui.end_row();

                                        ui.label(self.i18n.tr("pex_duration"));
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.pex.entries[i].duration_override,
                                            )
                                            .speed(1.0)
                                            .clamp_range(0.0..=100000.0_f32)
                                            .suffix(" tick"),
                                        );
                                        ui.end_row();
                                    });

                                ui.add_space(6.0);
                                ui.separator();
                                ui.add_space(4.0);

                                let entry = &mut self.pex.entries[i];
                                Self::show_texture_animation_editor(
                                    ui,
                                    self.i18n.tr("pex_texture_animation"),
                                    self.i18n.tr("pex_texture_interval"),
                                    self.i18n.tr("pex_texture_sequence"),
                                    self.i18n.tr("pex_add_texture"),
                                    self.i18n.tr("pex_reset_default_textures"),
                                    &mut entry.textures,
                                    &mut entry.texture_interval,
                                );
                            });
                    }

                    if let Some(idx) = self.pex.confirm_delete {
                        egui::Window::new(self.i18n.tr("pex_confirm_delete_title"))
                            .collapsible(false)
                            .resizable(false)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                ui.label(self.i18n.tr("pex_confirm_delete_msg"));
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if ui.button(self.i18n.tr("yes")).clicked() {
                                        remove_idx = Some(idx);
                                        self.pex.confirm_delete = None;
                                    }
                                    if ui.button(self.i18n.tr("no")).clicked() {
                                        self.pex.confirm_delete = None;
                                    }
                                });
                            });
                    }

                    if let Some(idx) = remove_idx {
                        self.pex.entries.remove(idx);
                        self.pex.preview_frames = None;
                    }

                    ui.add_space(12.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 32.0],
                            egui::Button::new(
                                egui::RichText::new(format!(
                                    "➕ {}",
                                    self.i18n.tr("pex_add_command")
                                ))
                                .size(14.0),
                            ),
                        )
                        .clicked()
                    {
                        self.pex.entries.push(PexCommandEntry::default());
                    }

                    ui.add_space(16.0);
                    ui.separator();

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

                    if let Some(ref msg) = self.pex.status_msg {
                        ui.add_space(8.0);
                        let color = if msg.starts_with('✅') {
                            egui::Color32::from_rgb(80, 200, 80)
                        } else {
                            egui::Color32::from_rgb(255, 100, 100)
                        };
                        ui.colored_label(color, msg.as_str());
                    }

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
                        let duration_secs = total_frames as f64 / self.pex.preview_fps as f64;

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

                                ui.label(self.i18n.tr("duration"));
                                ui.label(format!("{:.2}s", duration_secs));
                                ui.end_row();

                                ui.label(self.i18n.tr("fps"));
                                ui.label(format!("{}", self.pex.preview_fps));
                                ui.end_row();

                                ui.label(self.i18n.tr("pex_entries_count"));
                                ui.label(format!("{}", self.pex.entries.len()));
                                ui.end_row();
                            });
                    }
                });
            });

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
                                        egui::Slider::new(&mut f, 0..=max_frame)
                                            .show_value(true)
                                            .trailing_fill(true),
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

    fn show_particleex_fullscreen_editor(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        close: &mut bool,
    ) {
        self.sync_entry_from_text_if_needed(index);

        ui.horizontal(|ui| {
            ui.heading(format!(
                "#{} {}",
                index + 1,
                self.i18n.tr("pex_command_editor_title")
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(format!("x {}", self.i18n.tr("close"))).clicked() {
                    *close = true;
                }
            });
        });
        ui.separator();

        {
            let entry = &mut self.pex.entries[index];
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut entry.editor_mode,
                    ParticleexEditorMode::Wizard,
                    self.i18n.tr("pex_editor_wizard"),
                );
                ui.selectable_value(
                    &mut entry.editor_mode,
                    ParticleexEditorMode::Text,
                    self.i18n.tr("pex_editor_text"),
                );
                if let Some(model) = &entry.wizard_model {
                    ui.separator();
                    ui.label(egui::RichText::new(model.format_label()).monospace());
                }
            });
        }
        ui.add_space(6.0);

        let status_text = self.entry_validation_text(index);
        let status_color = if status_text.starts_with('✅') {
            egui::Color32::from_rgb(80, 200, 80)
        } else {
            egui::Color32::from_rgb(255, 100, 100)
        };
        ui.colored_label(status_color, status_text);
        ui.add_space(8.0);

        let editor_mode = self.pex.entries[index].editor_mode;
        match editor_mode {
            ParticleexEditorMode::Wizard => self.show_particleex_wizard_editor(ui, index),
            ParticleexEditorMode::Text => self.show_particleex_text_editor(ui, index),
        }
    }

    fn show_particleex_text_editor(&mut self, ui: &mut egui::Ui, index: usize) {
        let entry = &mut self.pex.entries[index];
        let avail = ui.available_size();
        let response = ui.add(
            egui::TextEdit::multiline(&mut entry.command)
                .desired_width(avail.x)
                .desired_rows(((avail.y - 30.0) / 16.0).max(12.0) as usize)
                .code_editor()
                .hint_text(self.i18n.tr("pex_command_hint")),
        );
        if response.changed() {
            match particleex::parse_command_model(&entry.command) {
                Ok(model) => {
                    entry.wizard_model = Some(model);
                    entry.parse_error = None;
                }
                Err(err) => {
                    entry.parse_error = Some(err);
                }
            }
        }
    }

    fn show_particleex_wizard_editor(&mut self, ui: &mut egui::Ui, index: usize) {
        let entry = &mut self.pex.entries[index];
        if entry.wizard_model.is_none() {
            match particleex::parse_command_model(&entry.command) {
                Ok(model) => entry.wizard_model = Some(model),
                Err(_) => entry.wizard_model = Some(ParticleexCommand::default()),
            }
        }

        let Some(model) = entry.wizard_model.as_mut() else {
            return;
        };

        let mut wizard_changed = false;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new(self.i18n.tr("pex_wizard_command_format")).strong());
                let before = model.format;
                egui::ComboBox::from_label(self.i18n.tr("pex_wizard_format_label"))
                    .selected_text(model.format.as_str())
                    .show_ui(ui, |ui| {
                        for format in ParticleexCommandFormat::ALL {
                            ui.selectable_value(&mut model.format, format, format.as_str());
                        }
                    });
                if model.format != before {
                    let preserved_prefix = model.prefix;
                    let preserved_particle = model.particle_name.clone();
                    let preserved_center = model.center.clone();
                    let preserved_velocity = model.base_velocity.clone();
                    *model = ParticleexCommand::for_format(model.format);
                    model.prefix = preserved_prefix;
                    model.particle_name = preserved_particle;
                    model.center = preserved_center;
                    model.base_velocity = preserved_velocity;
                    wizard_changed = true;
                }
                wizard_changed |= ui.text_edit_singleline(&mut model.particle_name).changed();
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new(self.i18n.tr("pex_wizard_center_velocity")).strong());
                ui.horizontal(|ui| {
                    ui.label(self.i18n.tr("pex_wizard_center"));
                    wizard_changed |= ui.text_edit_singleline(&mut model.center[0]).changed();
                    wizard_changed |= ui.text_edit_singleline(&mut model.center[1]).changed();
                    wizard_changed |= ui.text_edit_singleline(&mut model.center[2]).changed();
                });
                ui.horizontal(|ui| {
                    ui.label(self.i18n.tr("pex_wizard_velocity"));
                    wizard_changed |= ui
                        .text_edit_singleline(&mut model.base_velocity[0])
                        .changed();
                    wizard_changed |= ui
                        .text_edit_singleline(&mut model.base_velocity[1])
                        .changed();
                    wizard_changed |= ui
                        .text_edit_singleline(&mut model.base_velocity[2])
                        .changed();
                });
            });

            if let Some(color) = model.color.as_mut() {
                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.label(egui::RichText::new(self.i18n.tr("pex_wizard_color")).strong());
                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("pex_wizard_rgba"));
                        wizard_changed |= ui.text_edit_singleline(&mut color[0]).changed();
                        wizard_changed |= ui.text_edit_singleline(&mut color[1]).changed();
                        wizard_changed |= ui.text_edit_singleline(&mut color[2]).changed();
                        wizard_changed |= ui.text_edit_singleline(&mut color[3]).changed();
                    });
                });
            }

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new(self.i18n.tr("pex_wizard_mode_params")).strong());
                if model.format.is_normal() || model.format.is_conditional() {
                    if let Some(range) = model.range.as_mut() {
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("pex_wizard_range"));
                            wizard_changed |= ui.text_edit_singleline(&mut range[0]).changed();
                            wizard_changed |= ui.text_edit_singleline(&mut range[1]).changed();
                            wizard_changed |= ui.text_edit_singleline(&mut range[2]).changed();
                        });
                    }
                }

                if model.format.is_normal() {
                    if let Some(count) = model.count.as_mut() {
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("count"));
                            wizard_changed |= ui.text_edit_singleline(count).changed();
                        });
                    }
                }

                if model.format.is_conditional() {
                    if let Some(expr) = model.condition_expr.as_mut() {
                        ui.label(self.i18n.tr("pex_wizard_condition_expr"));
                        wizard_changed |= ui
                            .add(
                                egui::TextEdit::multiline(expr)
                                    .desired_rows(4)
                                    .code_editor(),
                            )
                            .changed();
                    }
                }

                if !model.format.is_normal() && !model.format.is_conditional() {
                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("pex_wizard_t_range_step"));
                        if let Some(v) = model.t_begin.as_mut() {
                            wizard_changed |= ui.text_edit_singleline(v).changed();
                        }
                        if let Some(v) = model.t_end.as_mut() {
                            wizard_changed |= ui.text_edit_singleline(v).changed();
                        }
                        if let Some(v) = model.t_step.as_mut() {
                            wizard_changed |= ui.text_edit_singleline(v).changed();
                        }
                    });
                    if let Some(expr) = model.shape_expr.as_mut() {
                        ui.label(self.i18n.tr("pex_wizard_shape_expr"));
                        wizard_changed |= ui
                            .add(
                                egui::TextEdit::multiline(expr)
                                    .desired_rows(5)
                                    .code_editor(),
                            )
                            .changed();
                    }
                    if model.format.is_animated() {
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("pex_wizard_count_per_tick"));
                            if let Some(v) = model.count_per_tick.as_mut() {
                                wizard_changed |= ui.text_edit_singleline(v).changed();
                            }
                        });
                    }
                } else if model.format.is_conditional() {
                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("pex_wizard_step"));
                        if let Some(v) = model.t_step.as_mut() {
                            wizard_changed |= ui.text_edit_singleline(v).changed();
                        }
                    });
                }

                ui.horizontal(|ui| {
                    ui.label(self.i18n.tr("pex_wizard_lifespan"));
                    if let Some(v) = model.lifespan.as_mut() {
                        wizard_changed |= ui.text_edit_singleline(v).changed();
                    }
                });

                ui.label(self.i18n.tr("pex_wizard_speed_expr"));
                if let Some(expr) = model.speed_expr.as_mut() {
                    wizard_changed |= ui
                        .add(
                            egui::TextEdit::multiline(expr)
                                .desired_rows(4)
                                .code_editor(),
                        )
                        .changed();
                }

                if !model.format.is_normal() && !model.format.is_conditional() {
                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("pex_wizard_speed_step"));
                        if let Some(v) = model.speed_step.as_mut() {
                            wizard_changed |= ui.text_edit_singleline(v).changed();
                        }
                    });
                }
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new(self.i18n.tr("pex_command_preview")).strong());
                let mut preview = particleex::format_command_model(model);
                ui.add(
                    egui::TextEdit::multiline(&mut preview)
                        .desired_rows(4)
                        .interactive(false)
                        .code_editor(),
                );
            });
        });

        if wizard_changed {
            entry.command = particleex::format_command_model(model);
            entry.parse_error = particleex::parse_command_model(&entry.command).err();
        }
    }

    fn sync_entry_from_text_if_needed(&mut self, index: usize) {
        let entry = &mut self.pex.entries[index];
        if entry.wizard_model.is_none() {
            match particleex::parse_command_model(&entry.command) {
                Ok(model) => {
                    entry.wizard_model = Some(model);
                    entry.parse_error = None;
                }
                Err(err) => {
                    entry.parse_error = Some(err);
                }
            }
        }
    }

    fn entry_validation_text(&self, index: usize) -> String {
        let entry = &self.pex.entries[index];
        if let Some(model) = &entry.wizard_model {
            if let Ok(info) = particleex::validate_command_model(model) {
                return info;
            }
        }
        if let Some(err) = &entry.parse_error {
            return format!("❌ {}", err);
        }
        match particleex::validate_command(&entry.command) {
            Ok(info) => info,
            Err(err) => err,
        }
    }

    fn compile_particleex(&mut self) {
        let entries: Vec<CompileEntry> = self
            .pex
            .entries
            .iter()
            .filter(|e| e.enabled && !e.command.trim().is_empty())
            .map(|e| CompileEntry {
                command: e.command.clone(),
                start_tick: e.start_tick as f64,
                position: [
                    e.position[0] as f64,
                    e.position[1] as f64,
                    e.position[2] as f64,
                ],
                duration_override: e.duration_override as f64,
                textures: e.textures.clone(),
                texture_interval: e.texture_interval,
            })
            .collect();

        if entries.is_empty() {
            self.pex.status_msg = Some(format!("❌ {}", self.i18n.tr("pex_no_enabled_commands")));
            return;
        }

        match particleex::compile_entries(&entries) {
            Ok((frames, fps, textures)) => {
                let frame_count = frames.len();
                let duration = frame_count as f64 / fps as f64;
                self.pex.preview_frames = Some(frames);
                self.pex.preview_fps = fps;
                self.pex.preview_frame_idx = 0;
                self.pex.preview_playing = true;
                self.pex.preview_textures = Some(textures);
                self.pex.status_msg = Some(format!(
                    "✅ {} {} {} ({:.1}s)",
                    self.i18n.tr("particleex_compiled"),
                    frame_count,
                    self.i18n.tr("frame"),
                    duration,
                ));
            }
            Err(e) => {
                self.pex.status_msg = Some(format!("❌ {}", e));
            }
        }
    }

    pub(crate) fn export_particleex_nbl(&mut self) {
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

        let raw_textures = self.pex.preview_textures.clone().unwrap_or_default();
        let textures = self.build_texture_entries(&raw_textures);

        let (bbox_min, bbox_max) = player::recalculate_bbox(&frames);
        let header = NblHeader {
            version: 1,
            target_fps: self.pex.preview_fps,
            total_frames: frames.len() as u32,
            texture_count: textures.len() as u16,
            attributes: 0x03,
            bbox_min,
            bbox_max,
        };

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
