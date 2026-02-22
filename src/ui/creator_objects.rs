use super::app::{
    CreatorObjectData, ImageTextConfig, NebulaToolsApp, ParameterConfig, SceneShapeType,
    ShapeConfig,
};
use super::creator::{color_f32_to_u8, color_u8_to_f32};
use crate::editor::{EmitterConfig, EmitterShape};
use eframe::egui;

impl NebulaToolsApp {
    /// Right-side panel editor for a single creator object.
    pub(crate) fn show_creator_object_editor(&mut self, ui: &mut egui::Ui, idx: usize) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let obj = &mut self.creator.objects[idx];
            ui.heading(format!("{} #{}", self.i18n.tr("cr_object_props"), idx + 1));
            ui.add_space(6.0);

            // Name
            ui.horizontal(|ui| {
                ui.label(self.i18n.tr("cr_object_name"));
                ui.text_edit_singleline(&mut obj.name);
            });
            ui.add_space(8.0);

            // --- Transform Section ---
            ui.collapsing(
                egui::RichText::new(self.i18n.tr("cr_transform")).strong(),
                |ui| {
                    ui.label(self.i18n.tr("position"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut obj.position[0])
                                .speed(0.1)
                                .prefix("X:"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut obj.position[1])
                                .speed(0.1)
                                .prefix("Y:"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut obj.position[2])
                                .speed(0.1)
                                .prefix("Z:"),
                        );
                    });

                    ui.label(self.i18n.tr("cr_rotation"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut obj.rotation[0])
                                .speed(1.0)
                                .suffix("°")
                                .prefix("X:"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut obj.rotation[1])
                                .speed(1.0)
                                .suffix("°")
                                .prefix("Y:"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut obj.rotation[2])
                                .speed(1.0)
                                .suffix("°")
                                .prefix("Z:"),
                        );
                    });

                    ui.label(self.i18n.tr("cr_scale"));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut obj.scale[0])
                                .speed(0.01)
                                .prefix("X:"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut obj.scale[1])
                                .speed(0.01)
                                .prefix("Y:"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut obj.scale[2])
                                .speed(0.01)
                                .prefix("Z:"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(self.i18n.tr("cr_alpha"));
                        ui.add(
                            egui::DragValue::new(&mut obj.alpha)
                                .speed(0.01)
                                .clamp_range(0.0..=1.0),
                        );
                    });
                },
            );

            ui.add_space(4.0);

            // Velocity expression
            ui.collapsing(
                egui::RichText::new(self.i18n.tr("velocity_expr")).strong(),
                |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut obj.velocity_expr)
                            .font(egui::FontId::monospace(13.0))
                            .desired_width(f32::INFINITY)
                            .desired_rows(3),
                    );
                },
            );

            ui.add_space(4.0);

            // Keyframe info
            ui.collapsing(
                egui::RichText::new(format!(
                    "🔑 {} ({})",
                    self.i18n.tr("cr_keyframe_add"),
                    obj.keyframes.len()
                ))
                .strong(),
                |ui| {
                    if obj.keyframes.is_empty() {
                        ui.label(egui::RichText::new("No keyframes set").weak().italics());
                    } else {
                        let frames: Vec<u32> = obj.keyframes.keys().copied().collect();
                        for f in &frames {
                            ui.horizontal(|ui| {
                                ui.label(format!("Frame {}", f));
                                if ui.small_button("🗑").clicked() {
                                    obj.keyframes.remove(f);
                                }
                            });
                        }
                    }
                },
            );

            ui.separator();
            ui.add_space(4.0);

            // --- Type-specific data editor ---
            match &mut obj.data {
                CreatorObjectData::Emitter(config) => {
                    Self::show_creator_emitter_ui(&self.i18n, ui, config);
                }
                CreatorObjectData::Shape(config) => {
                    Self::show_creator_shape_ui(&self.i18n, ui, config);
                }
                CreatorObjectData::Parameter(config) => {
                    Self::show_creator_parameter_ui(&self.i18n, ui, config);
                }
                CreatorObjectData::ImageText(config) => {
                    Self::show_creator_imagetext_ui(&self.i18n, ui, config);
                }
            }
        });
    }

    // ─── Emitter Editor ───
    fn show_creator_emitter_ui(
        i18n: &crate::i18n::I18nManager,
        ui: &mut egui::Ui,
        config: &mut EmitterConfig,
    ) {
        ui.label(
            egui::RichText::new(format!("✨ {}", i18n.tr("emitter_config")))
                .strong()
                .size(14.0),
        );
        ui.add_space(4.0);

        // Shape
        ui.horizontal(|ui| {
            ui.label(i18n.tr("shape"));
            egui::ComboBox::from_id_source("cr_emitter_shape")
                .selected_text(i18n.tr(config.shape.i18n_key()))
                .show_ui(ui, |ui| {
                    for s in EmitterShape::ALL {
                        ui.selectable_value(&mut config.shape, s, i18n.tr(s.i18n_key()));
                    }
                });
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
                    ui.label(i18n.tr("box_size"));
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

        ui.add_space(4.0);

        // Emission
        ui.horizontal(|ui| {
            ui.label(i18n.tr("emission_rate"));
            ui.add(
                egui::DragValue::new(&mut config.emission_rate)
                    .speed(1.0)
                    .suffix(" /s"),
            );
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("burst_count"));
            ui.add(egui::DragValue::new(&mut config.burst_count).speed(1.0));
        });
        ui.checkbox(&mut config.burst_only, i18n.tr("burst_only"));

        ui.horizontal(|ui| {
            ui.label(i18n.tr("particle_lifetime"));
            ui.add(
                egui::DragValue::new(&mut config.lifetime_frames)
                    .speed(1.0)
                    .suffix(" frames"),
            );
        });

        ui.add_space(4.0);

        // Velocity
        ui.horizontal(|ui| {
            ui.label(i18n.tr("initial_velocity"));
            ui.add(
                egui::DragValue::new(&mut config.speed_min)
                    .speed(0.1)
                    .prefix("min:"),
            );
            ui.add(
                egui::DragValue::new(&mut config.speed_max)
                    .speed(0.1)
                    .prefix("max:"),
            );
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("direction"));
            ui.add(
                egui::DragValue::new(&mut config.direction[0])
                    .speed(0.05)
                    .prefix("X:"),
            );
            ui.add(
                egui::DragValue::new(&mut config.direction[1])
                    .speed(0.05)
                    .prefix("Y:"),
            );
            ui.add(
                egui::DragValue::new(&mut config.direction[2])
                    .speed(0.05)
                    .prefix("Z:"),
            );
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("spread"));
            ui.add(
                egui::DragValue::new(&mut config.spread)
                    .clamp_range(0.0..=360.0)
                    .suffix("°"),
            );
        });

        ui.add_space(4.0);

        // Physics
        ui.horizontal(|ui| {
            ui.label(i18n.tr("gravity"));
            ui.add(egui::DragValue::new(&mut config.gravity).speed(0.1));
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("drag"));
            ui.add(
                egui::DragValue::new(&mut config.drag)
                    .speed(0.005)
                    .fixed_decimals(3),
            );
        });

        ui.add_space(4.0);

        // Colors & Size
        ui.horizontal(|ui| {
            ui.label(i18n.tr("color_start"));
            let mut c = color_u8_to_f32(config.color_start);
            if ui.color_edit_button_rgba_unmultiplied(&mut c).changed() {
                config.color_start = color_f32_to_u8(c);
            }
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("color_end"));
            let mut c = color_u8_to_f32(config.color_end);
            if ui.color_edit_button_rgba_unmultiplied(&mut c).changed() {
                config.color_end = color_f32_to_u8(c);
            }
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("size_start"));
            ui.add(egui::DragValue::new(&mut config.size_start).speed(0.01));
            ui.label(i18n.tr("size_end"));
            ui.add(egui::DragValue::new(&mut config.size_end).speed(0.01));
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(i18n.tr("anim_duration"));
            ui.add(
                egui::DragValue::new(&mut config.duration_secs)
                    .speed(0.1)
                    .suffix(" s"),
            );
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("target_fps_setting"));
            ui.add(
                egui::DragValue::new(&mut config.target_fps)
                    .clamp_range(1..=1000)
                    .speed(1.0),
            );
        });
    }

    // ─── Shape (Geometric) Editor ───
    fn show_creator_shape_ui(
        i18n: &crate::i18n::I18nManager,
        ui: &mut egui::Ui,
        config: &mut ShapeConfig,
    ) {
        ui.label(
            egui::RichText::new(format!("🔷 {}", i18n.tr("type_shape")))
                .strong()
                .size(14.0),
        );
        ui.add_space(4.0);

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
                ui.add(egui::DragValue::new(&mut config.size[0]).speed(0.1));
                ui.add(egui::DragValue::new(&mut config.size[1]).speed(0.1));
                if config.shape_type == SceneShapeType::Cube {
                    ui.add(egui::DragValue::new(&mut config.size[2]).speed(0.1));
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

        ui.horizontal(|ui| {
            ui.label(i18n.tr("count"));
            ui.add(
                egui::DragValue::new(&mut config.count)
                    .speed(1.0)
                    .clamp_range(1..=100000),
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
            ui.label(i18n.tr("lifespan"));
            ui.add(egui::DragValue::new(&mut config.lifespan).speed(1.0));
        });
    }

    // ─── Parameter / Polar Editor ───
    fn show_creator_parameter_ui(
        i18n: &crate::i18n::I18nManager,
        ui: &mut egui::Ui,
        config: &mut ParameterConfig,
    ) {
        let type_label = if config.is_polar {
            format!("🌀 {}", i18n.tr("type_polar"))
        } else {
            format!("📐 {}", i18n.tr("type_parameter"))
        };
        ui.label(egui::RichText::new(type_label).strong().size(14.0));
        ui.add_space(4.0);

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
                .font(egui::FontId::monospace(13.0))
                .desired_width(f32::INFINITY)
                .desired_rows(3),
        );

        ui.add_space(4.0);
        ui.label(i18n.tr("velocity_expr"));
        ui.add(
            egui::TextEdit::multiline(&mut config.velocity_expr)
                .font(egui::FontId::monospace(13.0))
                .desired_width(f32::INFINITY)
                .desired_rows(3),
        );

        ui.horizontal(|ui| {
            ui.label(i18n.tr("lifespan"));
            ui.add(egui::DragValue::new(&mut config.lifespan).speed(1.0));
        });
    }

    // ─── Image / Text Editor ───
    fn show_creator_imagetext_ui(
        i18n: &crate::i18n::I18nManager,
        ui: &mut egui::Ui,
        config: &mut ImageTextConfig,
    ) {
        let type_label = if config.is_text {
            format!("🔤 {}", i18n.tr("text"))
        } else {
            format!("🖼 {}", i18n.tr("image"))
        };
        ui.label(egui::RichText::new(type_label).strong().size(14.0));
        ui.add_space(4.0);

        if config.is_text {
            // Text input
            ui.label(i18n.tr("text_input"));
            ui.add(
                egui::TextEdit::multiline(&mut config.text_input)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(i18n.tr("font_size"));
                ui.add(egui::DragValue::new(&mut config.font_size).speed(1.0));
            });

            ui.horizontal(|ui| {
                if ui.button(i18n.tr("load_font")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Fonts", &["ttf", "otf"])
                        .pick_file()
                    {
                        config.font_name = path.to_string_lossy().to_string();
                    }
                }
                if !config.font_name.is_empty() && !config.font_name.starts_with("system://") {
                    let p = std::path::Path::new(&config.font_name);
                    ui.label(p.file_name().unwrap_or_default().to_string_lossy());
                }
            });

            ui.horizontal(|ui| {
                if ui.button(i18n.tr("load_system_font")).clicked() {
                    use font_kit::source::SystemSource;
                    if let Ok(families) = SystemSource::new().all_families() {
                        config.system_fonts = families;
                    }
                }
            });

            if !config.system_fonts.is_empty() {
                egui::ComboBox::from_id_source("cr_system_font")
                    .selected_text(if config.font_name.starts_with("system://") {
                        config.font_name.replace("system://", "")
                    } else {
                        "Select".to_string()
                    })
                    .show_ui(ui, |ui| {
                        for font in config.system_fonts.clone() {
                            if ui
                                .selectable_label(
                                    config.font_name == format!("system://{}", font),
                                    &font,
                                )
                                .clicked()
                            {
                                config.font_name = format!("system://{}", font);
                            }
                        }
                    });
            }
        } else {
            // Image input
            ui.horizontal(|ui| {
                if ui.button(i18n.tr("select_image")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                        .pick_file()
                    {
                        config.media_path = Some(path.to_string_lossy().to_string());
                    }
                }
                if let Some(path) = &config.media_path {
                    ui.label(
                        std::path::Path::new(path)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy(),
                    );
                }
            });
        }

        ui.add_space(4.0);

        // Common image/text settings
        ui.horizontal(|ui| {
            ui.label(i18n.tr("particle_size"));
            ui.add(
                egui::DragValue::new(&mut config.particle_size)
                    .speed(0.001)
                    .max_decimals(6)
                    .clamp_range(0.000001..=f32::MAX),
            );
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("point_size"));
            ui.add(
                egui::DragValue::new(&mut config.point_size)
                    .speed(0.0001)
                    .max_decimals(6)
                    .clamp_range(0.000001..=f32::MAX),
            );
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("density"));
            ui.add(
                egui::DragValue::new(&mut config.density)
                    .speed(0.001)
                    .max_decimals(6)
                    .clamp_range(0.000001..=f32::MAX),
            );
        });
        ui.horizontal(|ui| {
            ui.label(i18n.tr("brightness_threshold"));
            ui.add(egui::Slider::new(
                &mut config.brightness_threshold,
                0.0..=1.0,
            ));
        });
    }
}
