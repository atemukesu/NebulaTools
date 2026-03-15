use super::app::{CreatorPreset, NebulaToolsApp};
use crate::player::{recalculate_bbox, NblHeader, Particle};
use eframe::egui;

fn apply_euler_rotation(mut x: f32, mut y: f32, mut z: f32, rot: [f32; 3]) -> (f32, f32, f32) {
    let (sx, cx) = rot[0].to_radians().sin_cos();
    let (sy, cy) = rot[1].to_radians().sin_cos();
    let (sz, cz) = rot[2].to_radians().sin_cos();

    // Rx
    let y1 = y * cx - z * sx;
    let z1 = y * sx + z * cx;
    y = y1;
    z = z1;

    // Ry
    let x1 = x * cy + z * sy;
    let z2 = -x * sy + z * cy;
    x = x1;
    z = z2;

    // Rz
    let x2 = x * cz - y * sz;
    let y2 = x * sz + y * cz;
    x = x2;
    y = y2;

    (x, y, z)
}

impl NebulaToolsApp {
    pub(crate) fn show_creator_workflow(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("creator_side_panel")
            .width_range(300.0..=480.0)
            .show(ctx, |ui: &mut egui::Ui| {
                egui::ScrollArea::vertical()
                    .id_source("creator_scroll")
                    .show(ui, |ui: &mut egui::Ui| {
                        ui.add_space(10.0);
                        ui.heading(self.i18n.tr("creator_mode"));
                        ui.add_space(10.0);

                        ui.group(|ui: &mut egui::Ui| {
                            ui.label(self.i18n.tr("preset"));
                            let selected_text = match self.creator.selected_preset {
                                CreatorPreset::Butterfly => {
                                    format!("🦋 {}", self.i18n.tr("butterfly"))
                                }
                            };
                            egui::ComboBox::from_id_source("creator_preset_combo")
                                .selected_text(selected_text)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.creator.selected_preset,
                                        CreatorPreset::Butterfly,
                                        format!("🦋 {}", self.i18n.tr("butterfly")),
                                    );
                                });
                        });

                        ui.add_space(10.0);

                        match self.creator.selected_preset {
                            CreatorPreset::Butterfly => {
                                // === Basic Settings ===
                                ui.group(|ui: &mut egui::Ui| {
                                    ui.label(
                                        egui::RichText::new(self.i18n.tr("butterfly_settings"))
                                            .strong(),
                                    );
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.creator.butterfly_count,
                                            100..=50000,
                                        )
                                        .text(self.i18n.tr("count")),
                                    );
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(self.i18n.tr("cr_flap_mode"));
                                        ui.radio_value(
                                            &mut self.creator.flap_mode,
                                            0,
                                            self.i18n.tr("cr_flap_continuous"),
                                        );
                                        ui.radio_value(
                                            &mut self.creator.flap_mode,
                                            1,
                                            self.i18n.tr("cr_flap_schedule"),
                                        );
                                    });

                                    if self.creator.flap_mode == 0 {
                                        ui.add(
                                            egui::Slider::new(
                                                &mut self.creator.butterfly_speed,
                                                0.1..=5.0,
                                            )
                                            .text(self.i18n.tr("flutter_speed")),
                                        );
                                    } else {
                                        // Schedule mode
                                        ui.small(self.i18n.tr("cr_flap_schedule_desc"));
                                        ui.add_space(4.0);
                                        if ui
                                            .button(format!(
                                                "📂 {}",
                                                self.i18n.tr("cr_flap_import")
                                            ))
                                            .clicked()
                                        {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .add_filter("Text", &["txt"])
                                                .pick_file()
                                            {
                                                match std::fs::read_to_string(&path) {
                                                    Ok(content) => {
                                                        let mut times: Vec<f32> = Vec::new();
                                                        let mut valid = true;
                                                        for line in content.lines() {
                                                            let trimmed = line.trim();
                                                            if trimmed.is_empty() {
                                                                continue;
                                                            }
                                                            match trimmed.parse::<f32>() {
                                                                Ok(v) => times.push(v),
                                                                Err(_) => {
                                                                    valid = false;
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        if valid && !times.is_empty() {
                                                            let count = times.len();
                                                            times.sort_by(|a, b| {
                                                                a.partial_cmp(b).unwrap()
                                                            });
                                                            self.creator.flap_schedule = times;
                                                            self.creator.flap_schedule_status =
                                                                Some(format!(
                                                                    "✅ {} {} {}",
                                                                    self.i18n
                                                                        .tr("cr_flap_valid_fmt"),
                                                                    count,
                                                                    self.i18n
                                                                        .tr("cr_flap_time_points")
                                                                ));
                                                        } else if times.is_empty() && valid {
                                                            self.creator.flap_schedule_status =
                                                                Some(format!(
                                                                    "⚠ {}",
                                                                    self.i18n
                                                                        .tr("cr_flap_empty_file")
                                                                ));
                                                        } else {
                                                            self.creator.flap_schedule_status =
                                                                Some(format!(
                                                                    "❌ {}",
                                                                    self.i18n
                                                                        .tr("cr_flap_invalid_fmt")
                                                                ));
                                                        }
                                                    }
                                                    Err(e) => {
                                                        self.creator.flap_schedule_status =
                                                            Some(format!("❌ {}", e));
                                                    }
                                                }
                                            }
                                        }
                                        if let Some(status) =
                                            &self.creator.flap_schedule_status
                                        {
                                            ui.add_space(2.0);
                                            ui.label(status.as_str());
                                        }
                                    }
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.creator.butterfly_size,
                                            0.01..=1.0,
                                        )
                                        .text(self.i18n.tr("particle_size")),
                                    );
                                    ui.add(
                                        egui::Slider::new(&mut self.creator.point_size, 0.01..=1.0)
                                            .text(self.i18n.tr("point_size")),
                                    );
                                });

                                ui.add_space(6.0);

                                // === Per-Part Color Settings ===
                                ui.group(|ui: &mut egui::Ui| {
                                    ui.label(
                                        egui::RichText::new(self.i18n.tr("cr_butterfly_colors"))
                                            .strong(),
                                    );
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(format!("{}:", self.i18n.tr("cr_upper_wing")));
                                        ui.color_edit_button_rgb(
                                            &mut self.creator.color_upper_wing,
                                        );
                                    });
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(format!("{}:", self.i18n.tr("cr_lower_wing")));
                                        ui.color_edit_button_rgb(
                                            &mut self.creator.color_lower_wing,
                                        );
                                    });
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(format!("{}:", self.i18n.tr("cr_wing_edge")));
                                        ui.color_edit_button_rgb(&mut self.creator.color_wing_edge);
                                    });
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(format!("{}:", self.i18n.tr("cr_body")));
                                        ui.color_edit_button_rgb(&mut self.creator.color_body);
                                    });
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(format!("{}:", self.i18n.tr("cr_antennae")));
                                        ui.color_edit_button_rgb(&mut self.creator.color_antennae);
                                    });
                                });

                                ui.add_space(6.0);

                                // === Rotation / Orientation ===
                                ui.group(|ui: &mut egui::Ui| {
                                    ui.label(
                                        egui::RichText::new(self.i18n.tr("cr_orientation"))
                                            .strong(),
                                    );
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        ui.label(self.i18n.tr("cr_rotation"));
                                        ui.add(
                                            egui::DragValue::new(&mut self.creator.rotation[0])
                                                .speed(1.0)
                                                .suffix("°"),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut self.creator.rotation[1])
                                                .speed(1.0)
                                                .suffix("°"),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut self.creator.rotation[2])
                                                .speed(1.0)
                                                .suffix("°"),
                                        );
                                    });

                                    egui::ComboBox::from_id_source("cr_rot_preset")
                                        .selected_text(self.i18n.tr("preset"))
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    false,
                                                    self.i18n.tr("facing_z_pos"),
                                                )
                                                .clicked()
                                            {
                                                self.creator.rotation = [0.0, 0.0, 0.0];
                                            }
                                            if ui
                                                .selectable_label(
                                                    false,
                                                    self.i18n.tr("facing_z_neg"),
                                                )
                                                .clicked()
                                            {
                                                self.creator.rotation = [0.0, 180.0, 0.0];
                                            }
                                            if ui
                                                .selectable_label(
                                                    false,
                                                    self.i18n.tr("facing_x_pos"),
                                                )
                                                .clicked()
                                            {
                                                self.creator.rotation = [0.0, -90.0, 0.0];
                                            }
                                            if ui
                                                .selectable_label(
                                                    false,
                                                    self.i18n.tr("facing_x_neg"),
                                                )
                                                .clicked()
                                            {
                                                self.creator.rotation = [0.0, 90.0, 0.0];
                                            }
                                            if ui
                                                .selectable_label(
                                                    false,
                                                    format!(
                                                        "{} (Y-up)",
                                                        self.i18n.tr("facing_y_pos")
                                                    ),
                                                )
                                                .clicked()
                                            {
                                                self.creator.rotation = [-90.0, 0.0, 0.0];
                                            }
                                            if ui
                                                .selectable_label(
                                                    false,
                                                    self.i18n.tr("facing_y_neg"),
                                                )
                                                .clicked()
                                            {
                                                self.creator.rotation = [90.0, 0.0, 0.0];
                                            }
                                        });
                                });

                                ui.add_space(6.0);

                                // === Trail Settings ===
                                ui.group(|ui: &mut egui::Ui| {
                                    ui.label(
                                        egui::RichText::new(self.i18n.tr("cr_trail_settings"))
                                            .strong(),
                                    );
                                    ui.checkbox(
                                        &mut self.creator.trail_enabled,
                                        self.i18n.tr("cr_trail_enabled"),
                                    );
                                    if self.creator.trail_enabled {
                                        ui.horizontal(|ui: &mut egui::Ui| {
                                            ui.label(self.i18n.tr("cr_trail_gravity"));
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.creator.trail_gravity[0],
                                                )
                                                .speed(0.01)
                                                .prefix("X: "),
                                            );
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.creator.trail_gravity[1],
                                                )
                                                .speed(0.01)
                                                .prefix("Y: "),
                                            );
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut self.creator.trail_gravity[2],
                                                )
                                                .speed(0.01)
                                                .prefix("Z: "),
                                            );
                                        });
                                        ui.add(
                                            egui::Slider::new(
                                                &mut self.creator.trail_duration,
                                                0.05..=3.0,
                                            )
                                            .text(self.i18n.tr("cr_trail_duration")),
                                        );
                                        ui.add(
                                            egui::Slider::new(
                                                &mut self.creator.trail_opacity,
                                                0.0..=1.0,
                                            )
                                            .text(self.i18n.tr("cr_trail_opacity")),
                                        );
                                    }
                                });

                                ui.add_space(6.0);

                                // === Velocity Expression ===
                                ui.group(|ui: &mut egui::Ui| {
                                    ui.label(
                                        egui::RichText::new(self.i18n.tr("velocity_expr")).strong(),
                                    );
                                    let editor_id =
                                        ui.make_persistent_id("creator_velocity_editor");
                                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(
                                                &mut self.creator.velocity_expr,
                                            )
                                            .id(editor_id)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(3)
                                            .lock_focus(true)
                                            .hint_text("vx = cos(t*0.1); vy = sin(t*0.1); ..."),
                                        );
                                    });
                                    ui.add_space(4.0);
                                    ui.collapsing(
                                        self.i18n.tr("expr_help"),
                                        |ui: &mut egui::Ui| {
                                            ui.small(self.i18n.tr("expr_funcs_desc"));
                                            ui.add_space(4.0);
                                            ui.group(|ui: &mut egui::Ui| {
                                                ui.label(
                                                    egui::RichText::new(self.i18n.tr("expr_vars"))
                                                        .strong(),
                                                );
                                                ui.small(self.i18n.tr("expr_vars_desc"));
                                                ui.add_space(4.0);
                                                ui.horizontal_wrapped(|ui: &mut egui::Ui| {
                                                    let vars = [
                                                        "x", "y", "z", "vx", "vy", "vz", "id", "t",
                                                    ];
                                                    for v in vars {
                                                        if ui
                                                            .button(
                                                                egui::RichText::new(v).monospace(),
                                                            )
                                                            .clicked()
                                                        {
                                                            self.creator.velocity_expr.push_str(v);
                                                        }
                                                    }
                                                });
                                            });
                                            ui.add_space(4.0);
                                            ui.group(|ui: &mut egui::Ui| {
                                                ui.label(
                                                    egui::RichText::new(self.i18n.tr("expr_funcs"))
                                                        .strong(),
                                                );
                                                ui.horizontal_wrapped(|ui: &mut egui::Ui| {
                                                    let funcs = [
                                                        "sin()", "cos()", "tan()", "abs()",
                                                        "random()", "pow()", "sqrt()", "lerp()",
                                                        "clamp()",
                                                    ];
                                                    for f in funcs {
                                                        if ui
                                                            .button(
                                                                egui::RichText::new(f).monospace(),
                                                            )
                                                            .clicked()
                                                        {
                                                            self.creator.velocity_expr.push_str(f);
                                                        }
                                                    }
                                                });
                                            });
                                        },
                                    );
                                });
                            }
                        }

                        ui.add_space(20.0);

                        ui.group(|ui: &mut egui::Ui| {
                            ui.label(self.i18n.tr("duration"));
                            ui.add(
                                egui::Slider::new(&mut self.creator.duration_secs, 1.0..=600.0)
                                    .text(self.i18n.tr("duration_s")),
                            );
                            ui.add(
                                egui::Slider::new(&mut self.creator.target_fps, 1..=120)
                                    .text(self.i18n.tr("fps")),
                            );
                        });

                        ui.add_space(20.0);

                        if ui
                            .add_sized(
                                [ui.available_width(), 40.0],
                                egui::Button::new(
                                    egui::RichText::new(format!(
                                        "✨ {}",
                                        self.i18n.tr("generate_preview")
                                    ))
                                    .size(18.0),
                                ),
                            )
                            .clicked()
                        {
                            self.generate_butterfly_preset();
                        }

                        if let Some(msg) = &self.creator.status_msg {
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new(msg).color(egui::Color32::GREEN));
                        }
                    });
            });

        let frames_len = self
            .creator
            .preview_frames
            .as_ref()
            .map(|f| f.len())
            .unwrap_or(0);

        egui::CentralPanel::default().show(ctx, |ui| {
            if frames_len > 0 {
                let frame_idx = self.creator.preview_frame_idx as usize;

                let render_data = if let Some(frames) = &self.creator.preview_frames {
                    if let Some(frame_data) = frames.get(frame_idx) {
                        Some(self.prepare_render_data_from(frame_data))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(data) = render_data {
                    self.paint_3d_viewport(ui, ctx, &data);

                    if self.creator.preview_playing {
                        let dt = ctx.input(|i| i.stable_dt);
                        self.creator.preview_timer += dt;
                        let frame_dur = 1.0 / self.creator.target_fps as f32;
                        if self.creator.preview_timer >= frame_dur {
                            self.creator.preview_timer -= frame_dur;
                            self.creator.preview_frame_idx =
                                (self.creator.preview_frame_idx + 1) % frames_len as i32;
                        }
                        ctx.request_repaint();
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(if self.creator.preview_playing {
                                self.i18n.tr("pause")
                            } else {
                                self.i18n.tr("play")
                            })
                            .clicked()
                        {
                            self.creator.preview_playing = !self.creator.preview_playing;
                        }

                        let mut idx = self.creator.preview_frame_idx as usize;
                        if ui
                            .add(
                                egui::Slider::new(&mut idx, 0..=frames_len.saturating_sub(1))
                                    .show_value(false)
                                    .trailing_fill(true),
                            )
                            .changed()
                        {
                            self.creator.preview_frame_idx = idx as i32;
                            self.creator.preview_playing = false;
                        }
                        ui.label(format!("{}/{}", idx, frames_len));
                    });
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(self.i18n.tr("click_generate_hint"));
                });
            }
        });
    }

    fn generate_butterfly_preset(&mut self) {
        let target_fps = self.creator.target_fps;
        let total_frames = (self.creator.duration_secs * target_fps as f32) as u32;
        let count = self.creator.butterfly_count;
        let rotation = self.creator.rotation;
        let point_size = self.creator.point_size;

        // Per-part colors (convert to u8)
        let to_rgba = |c: [f32; 3], a: u8| -> [u8; 4] {
            [
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                a,
            ]
        };
        let col_upper = to_rgba(self.creator.color_upper_wing, 255);
        let col_lower = to_rgba(self.creator.color_lower_wing, 255);
        let col_body = to_rgba(self.creator.color_body, 255);
        let col_antennae = to_rgba(self.creator.color_antennae, 255);
        let col_edge = to_rgba(self.creator.color_wing_edge, 255);

        // Trail configuration
        let trail_enabled = self.creator.trail_enabled;
        let trail_grav = self.creator.trail_gravity;
        let trail_dur = self.creator.trail_duration;
        let trail_opacity = self.creator.trail_opacity;

        // Velocity expression
        let stmts = crate::particleex::compile_expr(&self.creator.velocity_expr);

        // Particle distribution:
        //  ~60% upper wing, ~20% lower wing, ~10% body, ~5% antennae, ~5% wing edge
        let n_upper = (count as f32 * 0.55) as u32;
        let n_lower = (count as f32 * 0.20) as u32;
        let n_body = (count as f32 * 0.10) as u32;
        let n_antennae = (count as f32 * 0.05) as u32;
        let n_edge = count.saturating_sub(n_upper + n_lower + n_body + n_antennae);

        let trail_frames = if trail_enabled {
            (trail_dur * target_fps as f32).max(1.0) as u32
        } else {
            0
        };

        let mut frames: Vec<Vec<Particle>> = Vec::with_capacity(total_frames as usize);
        // Store base particles (without trails) separately to prevent recursive explosion
        let mut base_frames: Vec<Vec<Particle>> = Vec::with_capacity(total_frames as usize);

        for f in 0..total_frames {
            let mut particles = Vec::with_capacity(count as usize * 2);
            let time = f as f32 / target_fps as f32;

            // Compute flap factor based on mode
            // flutter_t is a continuous phase value fed into sin() for wing oscillation
            let flutter_t = if self.creator.flap_mode == 0 {
                // Continuous mode: original speed-based flutter
                time * self.creator.butterfly_speed * 10.0
            } else {
                // Schedule mode: flap with a burst of wing beats at each time point
                let schedule = &self.creator.flap_schedule;
                if schedule.is_empty() {
                    0.0 // No schedule loaded, wings stay flat
                } else {
                    // Envelope half-width (seconds): how long each flap burst lasts
                    let burst_half = 0.3f32;
                    // Number of wing beats per burst
                    let flap_freq = 8.0f32; // beats/sec within the burst

                    let mut amplitude = 0.0f32;
                    let mut phase = 0.0f32;

                    for &st in schedule.iter() {
                        let dt = time - st; // signed: negative = before event
                        if dt.abs() < burst_half {
                            // Smooth cosine envelope [0..1]
                            let t_norm = dt.abs() / burst_half;
                            let env = 0.5 * (1.0 + (t_norm * std::f32::consts::PI).cos());
                            if env > amplitude {
                                amplitude = env;
                                // Phase: oscillate fast within the burst window
                                // dt goes from -burst_half..+burst_half
                                // we want a sin that starts at 0 at the start of burst
                                phase = (dt + burst_half) * flap_freq * std::f32::consts::TAU;
                            }
                        }
                    }
                    // Return phase scaled by amplitude: when amplitude=0 wings are flat,
                    // when amplitude=1 wings oscillate at full swing
                    phase * amplitude
                }
            };
            let mut pid: i32 = 0;

            // ── Upper Wing (Fay's butterfly curve, main lobes) ──
            for i in 0..n_upper {
                let t_val = (i as f32 / n_upper as f32) * std::f32::consts::TAU;

                let petal =
                    t_val.cos().exp() - 2.0 * (4.0 * t_val).cos() - (t_val / 12.0).sin().powi(5);

                let mut x = t_val.sin() * petal;
                let mut y = t_val.cos() * petal;

                // Only take the upper half of the curve (the bigger lobes)
                let is_upper = y > -0.5;

                // Scale to Y-up
                x *= self.creator.butterfly_size * 2.0;
                y *= self.creator.butterfly_size * 2.0;

                // 3D flapping
                let flap = (flutter_t + (x.abs() * 0.5)).sin();
                let z = x.abs() * flap * 0.8;

                let (rx, ry, rz) = apply_euler_rotation(x, y, z, rotation);

                let color = if is_upper { col_upper } else { col_lower };

                particles.push(Particle {
                    id: pid,
                    pos: [rx, ry, rz],
                    color,
                    size: point_size,
                    tex_id: 0,
                    seq_index: 0,
                });
                pid += 1;
            }

            // ── Lower Wing (smaller inner lobes) ──
            for i in 0..n_lower {
                let t_val = (i as f32 / n_lower as f32) * std::f32::consts::TAU;

                // Smaller version of the curve for the inner wing pattern
                let petal =
                    t_val.cos().exp() - 2.0 * (4.0 * t_val).cos() - (t_val / 12.0).sin().powi(5);

                let x = t_val.sin() * petal * self.creator.butterfly_size * 1.2;
                let y = t_val.cos() * petal * self.creator.butterfly_size * 1.2;

                let flap = (flutter_t + (x.abs() * 0.5)).sin();
                let z = x.abs() * flap * 0.6;

                let (rx, ry, rz) = apply_euler_rotation(x, y, z, rotation);

                particles.push(Particle {
                    id: pid,
                    pos: [rx, ry, rz],
                    color: col_lower,
                    size: point_size * 0.85,
                    tex_id: 0,
                    seq_index: 0,
                });
                pid += 1;
            }

            // ── Wing Edge (perimeter highlight) ──
            for i in 0..n_edge {
                let t_val = (i as f32 / n_edge as f32) * std::f32::consts::TAU;

                let petal =
                    t_val.cos().exp() - 2.0 * (4.0 * t_val).cos() - (t_val / 12.0).sin().powi(5);

                // Place at the edge with slight scale push
                let scale = self.creator.butterfly_size * 2.1;
                let x = t_val.sin() * petal * scale;
                let y = t_val.cos() * petal * scale;

                let flap = (flutter_t + (x.abs() * 0.5)).sin();
                let z = x.abs() * flap * 0.85;

                let (rx, ry, rz) = apply_euler_rotation(x, y, z, rotation);

                particles.push(Particle {
                    id: pid,
                    pos: [rx, ry, rz],
                    color: col_edge,
                    size: point_size * 0.7,
                    tex_id: 0,
                    seq_index: 0,
                });
                pid += 1;
            }

            // ── Body (elongated ellipse along Y axis) ──
            for i in 0..n_body {
                let t_norm = i as f32 / n_body as f32;
                let body_y = (t_norm * 2.0 - 1.0) * self.creator.butterfly_size * 3.5;
                // Slightly tapered x
                let taper = 1.0 - (t_norm * 2.0 - 1.0).abs();
                let body_x = taper * self.creator.butterfly_size * 0.15;
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let bx = body_x * rng.gen_range(-1.0f32..1.0);

                let (rx, ry, rz) = apply_euler_rotation(bx, body_y, 0.0, rotation);

                particles.push(Particle {
                    id: pid,
                    pos: [rx, ry, rz],
                    color: col_body,
                    size: point_size * 1.2,
                    tex_id: 0,
                    seq_index: 0,
                });
                pid += 1;
            }

            // ── Antennae (two curving lines from head) ──
            let half_ant = n_antennae / 2;
            for side in 0..2 {
                for i in 0..half_ant {
                    let t_norm = i as f32 / half_ant as f32;
                    let sign = if side == 0 { 1.0f32 } else { -1.0f32 };

                    // Curl outward and upward
                    let ax =
                        sign * t_norm * self.creator.butterfly_size * 1.5 * (1.0 + t_norm * 0.5);
                    let ay = self.creator.butterfly_size * 3.5
                        + t_norm * self.creator.butterfly_size * 2.0;
                    let az = t_norm * t_norm * self.creator.butterfly_size * 0.5;

                    let (rx, ry, rz) = apply_euler_rotation(ax, ay, az, rotation);

                    // Tip of antennae is slightly more opaque / brighter
                    let alpha = ((0.5 + t_norm * 0.5) * 255.0) as u8;
                    let mut c = col_antennae;
                    c[3] = alpha;

                    particles.push(Particle {
                        id: pid,
                        pos: [rx, ry, rz],
                        color: c,
                        size: point_size * 0.6,
                        tex_id: 0,
                        seq_index: 0,
                    });
                    pid += 1;
                }
            }

            // ── Apply Velocity Expression ──
            if let Some(ref s) = stmts {
                let mut pex_ctx = crate::particleex::ExprContext::new();
                let t64 = time as f64;
                for p in particles.iter_mut() {
                    pex_ctx.set("t", crate::particleex::Value::Num(t64));
                    pex_ctx.set("x", crate::particleex::Value::Num(p.pos[0] as f64));
                    pex_ctx.set("y", crate::particleex::Value::Num(p.pos[1] as f64));
                    pex_ctx.set("z", crate::particleex::Value::Num(p.pos[2] as f64));
                    pex_ctx.set("id", crate::particleex::Value::Num(p.id as f64));
                    pex_ctx.set("vx", crate::particleex::Value::Num(0.0));
                    pex_ctx.set("vy", crate::particleex::Value::Num(0.0));
                    pex_ctx.set("vz", crate::particleex::Value::Num(0.0));

                    crate::particleex::exec_stmts(s, &mut pex_ctx);

                    let vx = pex_ctx.get("vx").as_num() as f32;
                    let vy = pex_ctx.get("vy").as_num() as f32;
                    let vz = pex_ctx.get("vz").as_num() as f32;

                    p.pos[0] += vx * time;
                    p.pos[1] += vy * time;
                    p.pos[2] += vz * time;
                }
            }

            // Save base particles (without trails) for trail sourcing
            base_frames.push(particles.clone());

            // ── Add Trail Particles (from previous frames' BASE particles only) ──
            if trail_enabled && trail_frames > 0 && f > 0 {
                let start = if f > trail_frames {
                    f - trail_frames
                } else {
                    0
                };
                for tf in start..f {
                    let age = (f - tf) as f32 / trail_frames as f32;
                    let alpha_factor = (1.0 - age) * trail_opacity;
                    let dt_trail = (f - tf) as f32 / target_fps as f32;

                    if let Some(prev_base) = base_frames.get(tf as usize) {
                        for pp in prev_base.iter() {
                            // Only include a subset of trail particles across frames
                            // to avoid O(n*k) explosion
                            if pp.id % (trail_frames as i32).max(1) != 0 {
                                continue;
                            }
                            let mut tp = pp.clone();
                            // Apply gravity offset to trail particles
                            tp.pos[0] += trail_grav[0] * dt_trail * dt_trail * 0.5;
                            tp.pos[1] += trail_grav[1] * dt_trail * dt_trail * 0.5;
                            tp.pos[2] += trail_grav[2] * dt_trail * dt_trail * 0.5;
                            tp.color[3] = (tp.color[3] as f32 * alpha_factor) as u8;
                            tp.size *= (1.0 - age * 0.5).max(0.1);
                            tp.id = pid;
                            pid += 1;
                            particles.push(tp);
                        }
                    }
                }
            }

            frames.push(particles);
        }

        self.creator.preview_frames = Some(frames);
        self.creator.preview_frame_idx = 0;
        self.creator.preview_playing = true;
        self.creator.status_msg = Some(self.i18n.tr("gen_success").to_string());
    }

    pub(crate) fn export_creator_nbl(&mut self) {
        if let Some(frames) = &self.creator.preview_frames {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Nebula", &["nbl"][..])
                .set_file_name("butterfly_preset.nbl")
                .save_file()
            {
                let (mut bbox_min, mut bbox_max) = recalculate_bbox(frames);

                // Ensure a minimum AABB size to prevent culling by downstream renderers
                let min_half_extent = 1.0f32; // minimum ±1.0 on each axis
                for i in 0..3 {
                    let center = (bbox_min[i] + bbox_max[i]) * 0.5;
                    let half = (bbox_max[i] - bbox_min[i]) * 0.5;
                    if half < min_half_extent {
                        bbox_min[i] = center - min_half_extent;
                        bbox_max[i] = center + min_half_extent;
                    }
                }

                let header = NblHeader {
                    version: 1,
                    target_fps: self.creator.target_fps,
                    total_frames: frames.len() as u32,
                    texture_count: 0,
                    attributes: 3, // 1 (has_alpha) + 2 (has_size)
                    bbox_min,
                    bbox_max,
                };

                let empty_textures: Vec<crate::player::TextureEntry> = Vec::new();
                match self
                    .player
                    .save_file(&path, &header, &empty_textures, frames)
                {
                    Ok(_) => {
                        self.creator.status_msg = Some(self.i18n.tr("export_success").to_string())
                    }
                    Err(e) => {
                        self.creator.status_msg =
                            Some(format!("{}{}", self.i18n.tr("export_failed"), e))
                    }
                }
            }
        }
    }
}
