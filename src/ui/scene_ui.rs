use super::app::{
    NebulaToolsApp, ParameterConfig, SceneItem, SceneItemData, SceneShapeType, ShapeConfig,
};
use crate::editor::{self, EmitterConfig, EmitterShape};
use crate::particleex::{self, CompileEntry};
use crate::player::{self, NblHeader};
use eframe::egui;

impl NebulaToolsApp {
    pub fn show_scene_workflow(&mut self, ctx: &egui::Context) {
        // --- 1. 左侧面板：列表管理 + 全局动画设置 ---
        egui::SidePanel::left("scene_left_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(self.i18n.tr("scene_title"));
                });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui
                        .button(format!("➕ {}", self.i18n.tr("add_item")))
                        .clicked()
                    {
                        self.scene.items.push(SceneItem::default());
                        self.scene.selected_item = Some(self.scene.items.len() - 1);
                    }
                    if ui
                        .button(format!("▶ {}", self.i18n.tr("compile_scene")))
                        .clicked()
                    {
                        self.compile_scene();
                    }
                });

                ui.separator();

                // 场景条目列表
                let mut to_remove = None;
                egui::ScrollArea::vertical()
                    .id_source("items_list")
                    .show(ui, |ui| {
                        for (idx, item) in self.scene.items.iter_mut().enumerate() {
                            let is_selected = self.scene.selected_item == Some(idx);
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut item.enabled, "");
                                let label = format!("{}: {}", idx + 1, item.name);
                                if ui.selectable_label(is_selected, label).clicked() {
                                    self.scene.selected_item = Some(idx);
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("❌").clicked() {
                                            to_remove = Some(idx);
                                        }
                                    },
                                );
                            });
                        }
                    });

                if let Some(idx) = to_remove {
                    self.scene.items.remove(idx);
                    self.scene.selected_item = None;
                }

                // --- 重点：动画设置单列出来 ---
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(4.0);
                    ui.heading(format!("🎬 {}", self.i18n.tr("animation_settings")));
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("fps"));
                        ui.add(
                            egui::DragValue::new(&mut self.scene.preview_fps)
                                .speed(1.0)
                                .clamp_range(1..=120),
                        );
                    });

                    if let Some(msg) = &self.scene.status_msg {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(msg).color(egui::Color32::LIGHT_BLUE));
                    }
                    ui.add_space(10.0);
                });
            });

        // --- 2. 右侧面板：当前选中项详情编辑器 ---
        let selected_item = self.scene.selected_item;
        if let Some(idx) = selected_item {
            egui::SidePanel::right("scene_right_panel")
                .resizable(true)
                .default_width(320.0)
                .show(ctx, |ui| {
                    if idx < self.scene.items.len() {
                        self.show_scene_item_editor(ui, idx);
                    } else {
                        self.scene.selected_item = None;
                    }
                });
        }

        // --- 3. 底部预览播放器 ---
        if self.scene.preview_frames.is_some() {
            egui::TopBottomPanel::bottom("scene_playback").show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let play_label = if self.scene.preview_playing {
                        self.i18n.tr("pause")
                    } else {
                        self.i18n.tr("play")
                    };
                    if ui.button(play_label).clicked() {
                        self.scene.preview_playing = !self.scene.preview_playing;
                    }
                    if ui.button(self.i18n.tr("stop")).clicked() {
                        self.scene.preview_playing = false;
                        self.scene.preview_frame_idx = 0;
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if let Some(ref frames) = self.scene.preview_frames {
                        let max_frame = frames.len().saturating_sub(1) as i32;
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut self.show_grid, self.i18n.tr("grid"));
                            ui.add_space(8.0);
                            ui.label(format!("/ {}", max_frame));
                            let mut f = self.scene.preview_frame_idx;
                            ui.add_space(8.0);
                            let slider_width = ui.available_width() - 8.0;
                            if ui
                                .add_sized(
                                    [slider_width, ui.spacing().interact_size.y],
                                    egui::Slider::new(&mut f, 0..=max_frame)
                                        .show_value(true)
                                        .trailing_fill(true),
                                )
                                .changed()
                            {
                                self.scene.preview_frame_idx = f;
                            }
                        });
                    }
                });
                ui.add_space(6.0);
            });
        }

        // --- 4. 中央区域：3D 预览 ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let render_data = self.prepare_render_data_from_scene(ctx);
            // 这里我们始终显示 3D 视口作为背景
            self.paint_3d_viewport(ui, ctx, &render_data);
        });
    }

    fn show_scene_item_editor(&mut self, ui: &mut egui::Ui, idx: usize) {
        let item = &mut self.scene.items[idx];
        ui.heading(format!("{} #{}", self.i18n.tr("scene_item"), idx + 1));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut item.name);
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(self.i18n.tr("start_tick"));
            ui.add(egui::DragValue::new(&mut item.start_tick).speed(1.0));
        });
        ui.add_space(4.0);
        ui.label(self.i18n.tr("position"));
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut item.position[0])
                    .speed(0.1)
                    .prefix("X:"),
            );
            ui.add(
                egui::DragValue::new(&mut item.position[1])
                    .speed(0.1)
                    .prefix("Y:"),
            );
            ui.add(
                egui::DragValue::new(&mut item.position[2])
                    .speed(0.1)
                    .prefix("Z:"),
            );
        });

        ui.separator();

        egui::ComboBox::from_label(self.i18n.tr("scene_type"))
            .selected_text(match item.data {
                SceneItemData::Emitter(_) => self.i18n.tr("type_emitter"),
                SceneItemData::Parameter(ref p) => {
                    if p.is_polar {
                        self.i18n.tr("type_polar")
                    } else {
                        self.i18n.tr("type_parameter")
                    }
                }
                SceneItemData::Shape(_) => self.i18n.tr("type_shape"),
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(
                        matches!(item.data, SceneItemData::Emitter(_)),
                        self.i18n.tr("type_emitter"),
                    )
                    .clicked()
                {
                    item.data = SceneItemData::Emitter(EmitterConfig::default());
                }
                if ui
                    .selectable_label(
                        matches!(item.data, SceneItemData::Parameter(ref p) if !p.is_polar),
                        self.i18n.tr("type_parameter"),
                    )
                    .clicked()
                {
                    item.data = SceneItemData::Parameter(ParameterConfig {
                        is_polar: false,
                        ..Default::default()
                    });
                }
                if ui
                    .selectable_label(
                        matches!(item.data, SceneItemData::Parameter(ref p) if p.is_polar),
                        self.i18n.tr("type_polar"),
                    )
                    .clicked()
                {
                    item.data = SceneItemData::Parameter(ParameterConfig {
                        is_polar: true,
                        ..Default::default()
                    });
                }
                if ui
                    .selectable_label(
                        matches!(item.data, SceneItemData::Shape(_)),
                        self.i18n.tr("type_shape"),
                    )
                    .clicked()
                {
                    item.data = SceneItemData::Shape(ShapeConfig::default());
                }
            });

        ui.add_space(8.0);

        match &mut item.data {
            SceneItemData::Emitter(config) => {
                Self::show_emitter_config_ui(&self.i18n, ui, config);
            }
            SceneItemData::Parameter(config) => {
                Self::show_parameter_config_ui(&self.i18n, ui, config);
            }
            SceneItemData::Shape(config) => {
                Self::show_shape_config_ui(&self.i18n, ui, config);
            }
        }
    }

    fn show_parameter_config_ui(
        i18n: &crate::i18n::I18nManager,
        ui: &mut egui::Ui,
        config: &mut ParameterConfig,
    ) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(i18n.tr("center"));
                ui.add(
                    egui::DragValue::new(&mut config.center[0])
                        .speed(0.1)
                        .prefix("X:"),
                );
                ui.add(
                    egui::DragValue::new(&mut config.center[1])
                        .speed(0.1)
                        .prefix("Y:"),
                );
                ui.add(
                    egui::DragValue::new(&mut config.center[2])
                        .speed(0.1)
                        .prefix("Z:"),
                );
            });
            ui.horizontal(|ui| {
                ui.label(i18n.tr("color"));
                let mut c = egui::Color32::from_rgba_unmultiplied(
                    config.color[0],
                    config.color[1],
                    config.color[2],
                    config.color[3],
                );
                if ui.color_edit_button_srgba(&mut c).changed() {
                    config.color = [c.r(), c.g(), c.b(), c.a()];
                }
            });
            ui.horizontal(|ui| {
                ui.label(i18n.tr("initial_velocity"));
                ui.add(
                    egui::DragValue::new(&mut config.velocity[0])
                        .speed(0.1)
                        .prefix("X:"),
                );
                ui.add(
                    egui::DragValue::new(&mut config.velocity[1])
                        .speed(0.1)
                        .prefix("Y:"),
                );
                ui.add(
                    egui::DragValue::new(&mut config.velocity[2])
                        .speed(0.1)
                        .prefix("Z:"),
                );
            });
            ui.horizontal(|ui| {
                ui.label(i18n.tr("t_begin"));
                ui.add(egui::DragValue::new(&mut config.t_begin).speed(0.1));
                ui.label(i18n.tr("t_end"));
                ui.add(egui::DragValue::new(&mut config.t_end).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label(i18n.tr("t_step"));
                ui.add(
                    egui::DragValue::new(&mut config.t_step)
                        .speed(0.01)
                        .clamp_range(0.001..=10.0),
                );
            });
            ui.add_space(4.0);
            ui.label(i18n.tr("expr"));
            ui.add(
                egui::TextEdit::multiline(&mut config.expr)
                    .font(egui::FontId::monospace(14.0))
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(4.0);
            ui.label(i18n.tr("velocity_expr"));
            ui.add(
                egui::TextEdit::multiline(&mut config.velocity_expr)
                    .font(egui::FontId::monospace(14.0))
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(i18n.tr("lifespan"));
                ui.add(egui::DragValue::new(&mut config.lifespan).speed(1.0));
            });
        });
    }

    fn show_shape_config_ui(
        i18n: &crate::i18n::I18nManager,
        ui: &mut egui::Ui,
        config: &mut ShapeConfig,
    ) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(
                    &mut config.shape_type,
                    SceneShapeType::Line,
                    i18n.tr("shape_line"),
                );
                ui.selectable_value(
                    &mut config.shape_type,
                    SceneShapeType::Plane,
                    i18n.tr("shape_plane"),
                );
                ui.selectable_value(
                    &mut config.shape_type,
                    SceneShapeType::Sphere,
                    i18n.tr("shape_sphere"),
                );
                ui.selectable_value(
                    &mut config.shape_type,
                    SceneShapeType::Cube,
                    i18n.tr("shape_box"),
                );
                ui.selectable_value(
                    &mut config.shape_type,
                    SceneShapeType::Cylinder,
                    i18n.tr("shape_cylinder"),
                );
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(i18n.tr("origin"));
                ui.add(
                    egui::DragValue::new(&mut config.origin[0])
                        .speed(0.1)
                        .prefix("X:"),
                );
                ui.add(
                    egui::DragValue::new(&mut config.origin[1])
                        .speed(0.1)
                        .prefix("Y:"),
                );
                ui.add(
                    egui::DragValue::new(&mut config.origin[2])
                        .speed(0.1)
                        .prefix("Z:"),
                );
            });
            if config.shape_type == SceneShapeType::Line {
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("end_pos"));
                    ui.add(
                        egui::DragValue::new(&mut config.end_pos[0])
                            .speed(0.1)
                            .prefix("X:"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut config.end_pos[1])
                            .speed(0.1)
                            .prefix("Y:"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut config.end_pos[2])
                            .speed(0.1)
                            .prefix("Z:"),
                    );
                });
            } else if config.shape_type == SceneShapeType::Plane
                || config.shape_type == SceneShapeType::Cube
                || config.shape_type == SceneShapeType::Cylinder
            {
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("size"));
                    ui.add(
                        egui::DragValue::new(&mut config.size[0])
                            .speed(0.1)
                            .prefix("W/X:"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut config.size[1])
                            .speed(0.1)
                            .prefix("H/Y:"),
                    );
                    if config.shape_type == SceneShapeType::Cube {
                        ui.add(
                            egui::DragValue::new(&mut config.size[2])
                                .speed(0.1)
                                .prefix("D/Z:"),
                        );
                    }
                });
            }
            if config.shape_type == SceneShapeType::Sphere
                || config.shape_type == SceneShapeType::Cylinder
            {
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("radius"));
                    ui.add(egui::DragValue::new(&mut config.radius).speed(0.1));
                });
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(i18n.tr("count"));
                ui.add(
                    egui::DragValue::new(&mut config.count)
                        .speed(1.0)
                        .clamp_range(1..=5000),
                );
            });
            ui.add_space(4.0);
            ui.label(i18n.tr("velocity_expr"));
            ui.add(
                egui::TextEdit::multiline(&mut config.velocity_expr)
                    .font(egui::FontId::monospace(14.0))
                    .desired_width(f32::INFINITY),
            );

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(i18n.tr("color"));
                let mut c = egui::Color32::from_rgba_unmultiplied(
                    config.color[0],
                    config.color[1],
                    config.color[2],
                    config.color[3],
                );
                if ui.color_edit_button_srgba(&mut c).changed() {
                    config.color = [c.r(), c.g(), c.b(), c.a()];
                }
            });
            ui.horizontal(|ui| {
                ui.label(i18n.tr("lifespan"));
                ui.add(egui::DragValue::new(&mut config.lifespan).speed(1.0));
            });
        });
    }

    fn show_emitter_config_ui(
        i18n: &crate::i18n::I18nManager,
        ui: &mut egui::Ui,
        config: &mut EmitterConfig,
    ) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.collapsing(i18n.tr("shape"), |ui| {
                egui::ComboBox::from_label(i18n.tr("shape"))
                    .selected_text(i18n.tr(config.shape.i18n_key()))
                    .show_ui(ui, |ui| {
                        for shape in EmitterShape::ALL {
                            ui.selectable_value(
                                &mut config.shape,
                                shape,
                                i18n.tr(shape.i18n_key()),
                            );
                        }
                    });

                match config.shape {
                    EmitterShape::Sphere | EmitterShape::Ring => {
                        ui.horizontal(|ui| {
                            ui.label(i18n.tr("radius"));
                            ui.add(egui::DragValue::new(&mut config.shape_radius).speed(0.1));
                        });
                    }
                    EmitterShape::Box
                    | EmitterShape::Cone
                    | EmitterShape::Cylinder
                    | EmitterShape::Torus => {
                        if config.shape == EmitterShape::Cylinder
                            || config.shape == EmitterShape::Cone
                            || config.shape == EmitterShape::Torus
                        {
                            ui.horizontal(|ui| {
                                ui.label(i18n.tr("radius"));
                                ui.add(egui::DragValue::new(&mut config.shape_radius).speed(0.1));
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.label(i18n.tr("size"));
                            ui.add(egui::DragValue::new(&mut config.shape_box_size[0]).speed(0.1));
                            ui.add(egui::DragValue::new(&mut config.shape_box_size[1]).speed(0.1));
                            ui.add(egui::DragValue::new(&mut config.shape_box_size[2]).speed(0.1));
                        });
                    }
                    _ => {}
                }

                if config.shape != EmitterShape::Point && config.shape != EmitterShape::Ring {
                    ui.checkbox(&mut config.surface_only, "Surface Only");
                }
            });

            ui.collapsing(i18n.tr("emitter_config"), |ui| {
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("emission_rate"));
                    ui.add(egui::DragValue::new(&mut config.emission_rate).speed(1.0));
                });
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("burst_count"));
                    ui.add(egui::DragValue::new(&mut config.burst_count).speed(1.0));
                });
                ui.checkbox(&mut config.burst_only, i18n.tr("burst_only"));
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("particle_lifetime"));
                    ui.add(egui::DragValue::new(&mut config.lifetime_frames).speed(1.0));
                });
            });

            ui.collapsing(i18n.tr("playback"), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Min Speed");
                    ui.add(egui::DragValue::new(&mut config.speed_min).speed(0.1));
                    ui.label("Max Speed");
                    ui.add(egui::DragValue::new(&mut config.speed_max).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("direction"));
                    ui.add(egui::DragValue::new(&mut config.direction[0]).speed(0.01));
                    ui.add(egui::DragValue::new(&mut config.direction[1]).speed(0.01));
                    ui.add(egui::DragValue::new(&mut config.direction[2]).speed(0.01));
                });
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("spread"));
                    ui.add(egui::DragValue::new(&mut config.spread).speed(1.0));
                });
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("gravity"));
                    ui.add(egui::DragValue::new(&mut config.gravity).speed(0.1));
                });
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("drag"));
                    ui.add(
                        egui::DragValue::new(&mut config.drag)
                            .speed(0.001)
                            .clamp_range(0.0..=1.0),
                    );
                });
            });

            ui.collapsing(i18n.tr("metadata"), |ui| {
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("color_start"));
                    let mut color = egui::Color32::from_rgba_unmultiplied(
                        config.color_start[0],
                        config.color_start[1],
                        config.color_start[2],
                        config.color_start[3],
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        config.color_start = [color.r(), color.g(), color.b(), color.a()];
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("color_end"));
                    let mut color = egui::Color32::from_rgba_unmultiplied(
                        config.color_end[0],
                        config.color_end[1],
                        config.color_end[2],
                        config.color_end[3],
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        config.color_end = [color.r(), color.g(), color.b(), color.a()];
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(i18n.tr("size_start"));
                    ui.add(egui::DragValue::new(&mut config.size_start).speed(0.01));
                    ui.label(i18n.tr("size_end"));
                    ui.add(egui::DragValue::new(&mut config.size_end).speed(0.01));
                });
            });
        });
    }

    fn compile_scene(&mut self) {
        let mut all_frames: Vec<Vec<player::Particle>> = Vec::new();
        let mut compile_entries = Vec::new();

        for item in &self.scene.items {
            if !item.enabled {
                continue;
            }

            match &item.data {
                SceneItemData::Emitter(config) => {
                    let frames = editor::simulate(config, &[]);
                    let offset = item.start_tick as usize;
                    let mut shifted_frames = vec![vec![]; offset];
                    for f in frames {
                        let mut snapshot = f;
                        for p in &mut snapshot {
                            p.pos[0] += item.position[0];
                            p.pos[1] += item.position[1];
                            p.pos[2] += item.position[2];
                        }
                        shifted_frames.push(snapshot);
                    }
                    all_frames = self.merge_frames(all_frames, shifted_frames);
                }
                SceneItemData::Parameter(config) => {
                    let cmd_type = if config.is_polar {
                        "polarparameter"
                    } else {
                        "parameter"
                    };
                    let cmd = format!(
                        "particleex {} end_rod {} {} {} {} {} {} {} {} {} {} {} {} '{}' {} {} '{}'",
                        cmd_type,
                        config.center[0],
                        config.center[1],
                        config.center[2],
                        config.color[0],
                        config.color[1],
                        config.color[2],
                        config.color[3],
                        config.velocity[0],
                        config.velocity[1],
                        config.velocity[2],
                        config.t_begin,
                        config.t_end,
                        config.expr,
                        config.t_step,
                        config.lifespan,
                        config.velocity_expr
                    );
                    compile_entries.push(CompileEntry {
                        command: cmd,
                        start_tick: item.start_tick as f64,
                        position: [
                            item.position[0] as f64,
                            item.position[1] as f64,
                            item.position[2] as f64,
                        ],
                        duration_override: 0.0,
                    });
                }
                SceneItemData::Shape(config) => {
                    let t_begin = 0.0;
                    let t_end = 1.0;
                    let t_step = 1.0 / (config.count as f64).max(1.0);
                    let shape_expr = match config.shape_type {
                        SceneShapeType::Line => {
                            let dx = config.end_pos[0] - config.origin[0];
                            let dy = config.end_pos[1] - config.origin[1];
                            let dz = config.end_pos[2] - config.origin[2];
                            format!(
                                "x={}+t*{}; y={}+t*{}; z={}+t*{}",
                                config.origin[0], dx, config.origin[1], dy, config.origin[2], dz
                            )
                        }
                        SceneShapeType::Plane => {
                            format!(
                                "x={}+(t*{count} % 1.0)*{}; y={}+floor(t*{count})/{count}.0*{}; z={}",
                                config.origin[0],
                                config.size[0],
                                config.origin[1],
                                config.size[1],
                                config.origin[2],
                                count = config.count.max(1)
                            )
                        }
                        SceneShapeType::Sphere => {
                            let radius = config.radius;
                            format!(
                                "x={}+{}*cos(t*{Math_PI}*2)*sin(t*{Math_PI}); y={}+{}*cos(t*{Math_PI}); z={}+{}*sin(t*{Math_PI}*2)*sin(t*{Math_PI})",
                                config.origin[0], radius,
                                config.origin[1], radius,
                                config.origin[2], radius,
                                Math_PI = std::f32::consts::PI
                            )
                        }
                        SceneShapeType::Cube => {
                            format!(
                                "x={}+{}-2.0*{}*abs(sin(t*{Math_PI}*3.14)); y={}+{}-2.0*{}*abs(sin(t*{Math_PI}*1.57)); z={}+{}-2.0*{}*abs(sin(t*{Math_PI}*0.75))",
                                config.origin[0], config.size[0] / 2.0, config.size[0] / 2.0,
                                config.origin[1], config.size[1] / 2.0, config.size[1] / 2.0,
                                config.origin[2], config.size[2] / 2.0, config.size[2] / 2.0,
                                Math_PI = std::f32::consts::PI
                            )
                        }
                        SceneShapeType::Cylinder => {
                            let radius = config.radius;
                            format!(
                                "x={}+{}*cos(t*{Math_PI}*10); y={}+(t-0.5)*{}; z={}+{}*sin(t*{Math_PI}*10)",
                                config.origin[0], radius,
                                config.origin[1], config.size[1],
                                config.origin[2], radius,
                                Math_PI = std::f32::consts::PI
                            )
                        }
                    };
                    let cmd = format!(
                        "particleex parameter end_rod 0 0 0 {} {} {} {} 0 0 0 {} {} '{}' {} {} '{}'",
                        config.color[0], config.color[1], config.color[2], config.color[3],
                        t_begin, t_end, shape_expr, t_step, config.lifespan, config.velocity_expr
                    );
                    compile_entries.push(CompileEntry {
                        command: cmd,
                        start_tick: item.start_tick as f64,
                        position: [
                            item.position[0] as f64,
                            item.position[1] as f64,
                            item.position[2] as f64,
                        ],
                        duration_override: 0.0,
                    });
                }
            }
        }

        if !compile_entries.is_empty() {
            if let Ok((frames, _fps)) = particleex::compile_entries(&compile_entries) {
                // 如果是 PEX 命令，我们尊重它的 FPS，但场景预览以全局 FPS 为准
                all_frames = self.merge_frames(all_frames, frames);
            }
        }

        self.scene.preview_frames = Some(all_frames);
        self.scene.preview_playing = true;
        self.scene.preview_frame_idx = 0;
        self.scene.status_msg = Some(self.i18n.tr("scene_compiled").to_string());
    }

    fn merge_frames(
        &self,
        mut a: Vec<Vec<player::Particle>>,
        b: Vec<Vec<player::Particle>>,
    ) -> Vec<Vec<player::Particle>> {
        let max_len = a.len().max(b.len());
        if a.len() < max_len {
            a.resize(max_len, vec![]);
        }
        for (i, b_frame) in b.into_iter().enumerate() {
            if i < a.len() {
                a[i].extend(b_frame);
            } else {
                a.push(b_frame);
            }
        }
        a
    }

    fn prepare_render_data_from_scene(&mut self, ctx: &egui::Context) -> Vec<f32> {
        let frames = match &self.scene.preview_frames {
            Some(f) => f,
            None => return vec![],
        };

        if frames.is_empty() {
            return vec![];
        }

        if self.scene.preview_playing {
            let dt = 1.0 / self.scene.preview_fps as f32;
            let actual_dt = ctx.input(|i| i.stable_dt);
            self.scene.preview_timer += actual_dt;
            if self.scene.preview_timer >= dt {
                self.scene.preview_timer -= dt;
                self.scene.preview_frame_idx += 1;
                if self.scene.preview_frame_idx >= frames.len() as i32 {
                    self.scene.preview_frame_idx = 0;
                }
            }
            ctx.request_repaint();
        }

        let idx = self.scene.preview_frame_idx as usize;
        if idx < frames.len() {
            self.prepare_render_data_from(&frames[idx])
        } else {
            vec![]
        }
    }

    pub fn export_scene_nbl(&mut self) {
        let frames = match &self.scene.preview_frames {
            Some(f) => f,
            None => {
                self.scene.status_msg = Some("Compile first!".into());
                return;
            }
        };

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .save_file()
        {
            let (bbox_min, bbox_max) = player::recalculate_bbox(frames);
            let header = NblHeader {
                version: 1,
                total_frames: frames.len() as u32,
                target_fps: self.scene.preview_fps,
                texture_count: 0,
                attributes: 0x03,
                bbox_min,
                bbox_max,
            };

            match self.player.save_file(&path, &header, &[], frames) {
                Ok(_) => self.scene.status_msg = Some("Export successful".into()),
                Err(e) => self.scene.status_msg = Some(format!("Export failed: {}", e)),
            }
        }
    }
}
