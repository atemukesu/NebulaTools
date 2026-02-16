use super::app::{NebulaToolsApp, PexCommandEntry};
use crate::particleex::{self, CompileEntry};
use crate::player::{self, NblHeader, TextureEntry};
use eframe::egui;

impl NebulaToolsApp {
    pub(crate) fn show_particleex_workflow(&mut self, ctx: &egui::Context) {
        // Playback logic
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

        // ─── Fullscreen editor overlay ───
        if let Some(fs_idx) = self.pex.fullscreen_entry {
            if fs_idx < self.pex.entries.len() {
                let mut close = false;
                egui::Area::new(egui::Id::new("pex_fullscreen"))
                    .fixed_pos(egui::pos2(0.0, 0.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        let screen = ctx.screen_rect();
                        ui.allocate_exact_size(screen.size(), egui::Sense::hover());

                        let panel_rect = screen.shrink(40.0);
                        ui.painter()
                            .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(200));
                        ui.painter()
                            .rect_filled(panel_rect, 12.0, egui::Color32::from_gray(30));

                        ui.allocate_ui_at_rect(panel_rect.shrink(16.0), |ui| {
                            ui.horizontal(|ui| {
                                ui.heading(format!("#{} Command Editor", fs_idx + 1));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("✕ Close").clicked() {
                                            close = true;
                                        }
                                    },
                                );
                            });
                            ui.separator();
                            ui.add_space(8.0);

                            // Syntax hint
                            let cmd_text = &self.pex.entries[fs_idx].command;
                            if !cmd_text.trim().is_empty() {
                                let hint = particleex::validate_command(cmd_text);
                                match hint {
                                    Ok(info) => ui
                                        .colored_label(egui::Color32::from_rgb(80, 200, 80), &info),
                                    Err(err) => ui.colored_label(
                                        egui::Color32::from_rgb(255, 100, 100),
                                        &err,
                                    ),
                                };
                                ui.add_space(4.0);
                            }

                            let avail = ui.available_size();
                            let text_edit =
                                egui::TextEdit::multiline(&mut self.pex.entries[fs_idx].command)
                                    .desired_width(avail.x)
                                    .desired_rows(((avail.y - 30.0) / 16.0).max(10.0) as usize)
                                    .code_editor()
                                    .hint_text("particleex parameter ...");
                            ui.add(text_edit);
                        });
                    });
                if close {
                    self.pex.fullscreen_entry = None;
                }
                return; // Don't render anything else while fullscreen
            } else {
                self.pex.fullscreen_entry = None;
            }
        }

        // ─── Side Panel: Command Entries ───
        egui::SidePanel::left("particleex_side")
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.heading(self.i18n.tr("particleex_title"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("❓").on_hover_text(self.i18n.tr("particleex_hint")).clicked() {
                                self.pex.show_help = !self.pex.show_help;
                            }
                        });
                    });
                    ui.separator();

                    // Help panel
                    if self.pex.show_help {
                        ui.add_space(4.0);
                        egui::Frame::none()
                            .fill(egui::Color32::from_gray(25))
                            .inner_margin(8.0)
                            .rounding(6.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("📖 Syntax Reference").strong().size(14.0));
                                ui.add_space(4.0);
                                ui.label("Commands: normal, conditional, parameter, polar-parameter,\nrgba-parameter, tick-parameter, rgba-tick-polar-parameter ...");
                                ui.add_space(4.0);
                                ui.label("Variables: x y z vx vy vz cr cg cb alpha mpsize age t s1 s2 dis destory");
                                ui.add_space(4.0);
                                ui.label("Functions: sin cos tan asin acos atan atan2 pow sqrt exp log\nfloor ceil round abs min max random lerp clamp ...");
                                ui.add_space(4.0);
                                ui.label("Example:\nparticleex parameter end_rod ~ ~ ~ 1 1 1 1 0 0 0 -10 10 'x=t;y=sin(t)' 0.1 200");
                            });
                        ui.separator();
                    }

                    // ─── Entry list ───
                    let entry_count = self.pex.entries.len();
                    let mut remove_idx: Option<usize> = None;

                    for i in 0..entry_count {
                        ui.add_space(8.0);
                        let entry_id = format!("pex_entry_{}", i);
                        egui::Frame::none()
                            .fill(ui.visuals().faint_bg_color)
                            .inner_margin(10.0)
                            .rounding(8.0)
                            .stroke(egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color))
                            .show(ui, |ui| {
                                // Header row
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.pex.entries[i].enabled, "");
                                    ui.label(egui::RichText::new(format!("#{}", i + 1)).strong().size(14.0));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if entry_count > 1 {
                                            if ui.small_button("🗑").on_hover_text("Remove").clicked() {
                                                self.pex.confirm_delete = Some(i);
                                            }
                                        }
                                        if ui.small_button("⛶").on_hover_text("Fullscreen").clicked() {
                                            self.pex.fullscreen_entry = Some(i);
                                        }
                                    });
                                });

                                ui.add_space(4.0);

                                // Command text
                                let text_edit = egui::TextEdit::multiline(&mut self.pex.entries[i].command)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(4)
                                    .code_editor()
                                    .hint_text("particleex ...");
                                ui.add(text_edit);

                                // Inline validation
                                let cmd_text = &self.pex.entries[i].command;
                                if !cmd_text.trim().is_empty() {
                                    let hint = particleex::validate_command(cmd_text);
                                    match hint {
                                        Ok(info) => { ui.colored_label(egui::Color32::from_rgb(80, 200, 80), &info); }
                                        Err(err) => { ui.colored_label(egui::Color32::from_rgb(255, 100, 100), &err); }
                                    };
                                }

                                ui.add_space(6.0);

                                // Timing & overrides
                                egui::Grid::new(format!("{}_grid", entry_id))
                                    .num_columns(2)
                                    .spacing([8.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label(self.i18n.tr("pex_start_tick"));
                                        ui.add(egui::DragValue::new(&mut self.pex.entries[i].start_tick)
                                            .speed(1.0).clamp_range(0.0..=100000.0_f32).suffix(" tick"));
                                        ui.end_row();

                                        ui.label(self.i18n.tr("pex_position"));
                                        ui.horizontal(|ui| {
                                            ui.add(egui::DragValue::new(&mut self.pex.entries[i].position[0]).speed(0.1).prefix("X:"));
                                            ui.add(egui::DragValue::new(&mut self.pex.entries[i].position[1]).speed(0.1).prefix("Y:"));
                                            ui.add(egui::DragValue::new(&mut self.pex.entries[i].position[2]).speed(0.1).prefix("Z:"));
                                        });
                                        ui.end_row();

                                        ui.label(self.i18n.tr("pex_duration"));
                                        ui.add(egui::DragValue::new(&mut self.pex.entries[i].duration_override)
                                            .speed(1.0).clamp_range(0.0..=100000.0_f32).suffix(" tick"));
                                        ui.end_row();
                                    });
                            });
                    }

                    // Delete confirmation dialog
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

                    // Remove entry
                    if let Some(idx) = remove_idx {
                        self.pex.entries.remove(idx);
                        // Clear frames to force recompile
                        self.pex.preview_frames = None;
                    }

                    // Add button
                    ui.add_space(12.0);
                    if ui.add_sized(
                        [ui.available_width(), 32.0],
                        egui::Button::new(egui::RichText::new(format!("➕ {}", self.i18n.tr("pex_add_command"))).size(14.0)),
                    ).clicked() {
                        self.pex.entries.push(PexCommandEntry::default());
                    }

                    ui.add_space(16.0);
                    ui.separator();

                    // ─── Action Buttons ───
                    ui.add_space(8.0);
                    if ui.add_sized(
                        [ui.available_width(), 36.0],
                        egui::Button::new(egui::RichText::new(self.i18n.tr("particleex_compile")).strong().size(16.0)),
                    ).clicked() {
                        self.compile_particleex();
                    }

                    ui.add_space(8.0);
                    if ui.add_sized(
                        [ui.available_width(), 36.0],
                        egui::Button::new(egui::RichText::new(self.i18n.tr("export_nbl")).strong().size(16.0)),
                    ).clicked() {
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
                        ui.label(egui::RichText::new(self.i18n.tr("particleex_stats")).strong().size(15.0));
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

                                ui.label("FPS");
                                ui.label(format!("{}", self.pex.preview_fps));
                                ui.end_row();

                                ui.label(self.i18n.tr("pex_entries_count"));
                                ui.label(format!("{}", self.pex.entries.len()));
                                ui.end_row();
                            });
                    }
                });
            });

        // ─── Bottom Panel: Playback ───
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

        // ─── Central Panel: 3D Preview ───
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
            })
            .collect();

        if entries.is_empty() {
            self.pex.status_msg = Some("❌ No enabled commands".into());
            return;
        }

        match particleex::compile_entries(&entries) {
            Ok((frames, fps)) => {
                let frame_count = frames.len();
                let duration = frame_count as f64 / fps as f64;
                self.pex.preview_frames = Some(frames);
                self.pex.preview_fps = fps;
                self.pex.preview_frame_idx = 0;
                self.pex.preview_playing = true;
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
