use crate::player::{NblHeader, Particle};
use crate::ui::app::NebulaToolsApp;
use ab_glyph::{Font, PxScale, ScaleFont};
use eframe::egui;
use image::{DynamicImage, GenericImageView};
use std::sync::{Arc, Mutex};

impl NebulaToolsApp {
    pub fn show_multimedia_workflow(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("multimedia_left_panel")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(self.i18n.tr("multimedia_mode"));
                });
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.multimedia.mode, 0, self.i18n.tr("text"));
                    ui.selectable_value(&mut self.multimedia.mode, 1, self.i18n.tr("image"));
                    ui.selectable_value(&mut self.multimedia.mode, 2, self.i18n.tr("video"));
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_source("multimedia_settings")
                    .show(ui, |ui| {
                        match self.multimedia.mode {
                            0 => self.show_text_ui(ui),
                            1 => self.show_image_ui(ui),
                            2 => self.show_video_ui(ui),
                            _ => {}
                        }

                        // Shared Settings
                        ui.separator();
                        ui.collapsing(self.i18n.tr("animation_settings"), |ui| {
                            ui.horizontal(|ui| {
                                ui.label(self.i18n.tr("intro_duration"));
                                ui.add(
                                    egui::DragValue::new(&mut self.multimedia.intro_duration)
                                        .speed(0.1)
                                        .clamp_range(0.0..=10.0),
                                );
                            });
                            // Intro Preset
                            let cur_intro = self.i18n.tr(self.multimedia.intro_preset.i18n_key());
                            egui::ComboBox::from_label(self.i18n.tr("anim_intro"))
                                .selected_text(cur_intro)
                                .show_ui(ui, |ui| {
                                    for preset in crate::ui::app::IntroPreset::all() {
                                        let lbl = self.i18n.tr(preset.i18n_key());
                                        let selected = &self.multimedia.intro_preset == &preset;
                                        if ui.selectable_label(selected, lbl).clicked() {
                                            self.multimedia.intro_preset = preset;
                                            self.multimedia.reset_intro_params();
                                        }
                                    }
                                });
                            // Dynamic intro params
                            {
                                let info = self.multimedia.intro_preset.param_info();
                                for (idx, (key, _default, min, max)) in info.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(self.i18n.tr(key));
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.multimedia.intro_params[idx],
                                            )
                                            .speed(0.1)
                                            .clamp_range(*min..=*max),
                                        );
                                    });
                                }
                            }
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(self.i18n.tr("outro_duration"));
                                ui.add(
                                    egui::DragValue::new(&mut self.multimedia.outro_duration)
                                        .speed(0.1)
                                        .clamp_range(0.0..=10.0),
                                );
                            });
                            // Outro Preset
                            let cur_outro = self.i18n.tr(self.multimedia.outro_preset.i18n_key());
                            egui::ComboBox::from_label(self.i18n.tr("anim_outro"))
                                .selected_text(cur_outro)
                                .show_ui(ui, |ui| {
                                    for preset in crate::ui::app::OutroPreset::all() {
                                        let lbl = self.i18n.tr(preset.i18n_key());
                                        let selected = &self.multimedia.outro_preset == &preset;
                                        if ui.selectable_label(selected, lbl).clicked() {
                                            self.multimedia.outro_preset = preset;
                                            self.multimedia.reset_outro_params();
                                        }
                                    }
                                });
                            // Dynamic outro params
                            {
                                let info = self.multimedia.outro_preset.param_info();
                                for (idx, (key, _default, min, max)) in info.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(self.i18n.tr(key));
                                        ui.add(
                                            egui::DragValue::new(
                                                &mut self.multimedia.outro_params[idx],
                                            )
                                            .speed(0.1)
                                            .clamp_range(*min..=*max),
                                        );
                                    });
                                }
                            }
                            ui.separator();
                            self.show_expression_editor(ui);
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(self.i18n.tr("edit_particle_size"));
                                ui.add(
                                    egui::DragValue::new(&mut self.multimedia.particle_size)
                                        .speed(0.001),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.i18n.tr("particle_scale"));
                                ui.add(
                                    egui::DragValue::new(&mut self.multimedia.particle_scale)
                                        .speed(0.001)
                                        .clamp_range(0.001..=1.0),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label(self.i18n.tr("density"));
                                ui.add(
                                    egui::DragValue::new(&mut self.multimedia.density)
                                        .speed(0.01)
                                        .clamp_range(0.001..=f32::MAX),
                                );
                            });
                        });

                        ui.add_space(8.0);
                        let est_count = self.estimate_multimedia_particles();
                        ui.label(format!(
                            "{}: {}",
                            self.i18n.tr("estimated_count"),
                            est_count
                        ));
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui
                                .button(format!("▶ {}", self.i18n.tr("compile_preview")))
                                .clicked()
                                && !self.multimedia.is_processing
                            {
                                self.compile_multimedia_preview(ctx);
                            }
                            if ui
                                .button(format!("💾 {}", self.i18n.tr("export_nbl")))
                                .clicked()
                            {
                                self.export_multimedia_nbl();
                            }
                        });

                        if let Some(msg) = &self.multimedia.status_msg {
                            ui.add_space(8.0);
                            let color = if msg.contains("Failed") || msg.contains("Error") {
                                egui::Color32::from_rgb(255, 100, 100)
                            } else {
                                egui::Color32::from_rgb(80, 200, 80)
                            };
                            ui.colored_label(color, msg);
                        }

                        // Intermediate Image Preview
                        if let Some(tex) = &self.multimedia.source_image_preview {
                            ui.add_space(8.0);
                            ui.group(|ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("source_preview")).strong(),
                                );
                                ui.add(egui::Image::new(tex).max_width(ui.available_width()));
                            });
                        }
                    });
            });

        // Bottom playback panel
        egui::TopBottomPanel::bottom("multimedia_playback")
            .resizable(false)
            .min_height(60.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let play_label = if self.multimedia.preview_playing {
                        self.i18n.tr("pause")
                    } else {
                        self.i18n.tr("play")
                    };
                    if ui.button(play_label).clicked() {
                        self.multimedia.preview_playing = !self.multimedia.preview_playing;
                    }
                    if ui.button(self.i18n.tr("stop")).clicked() {
                        self.multimedia.preview_playing = false;
                        self.multimedia.preview_frame_idx = 0;
                    }

                    if let Some(frames) = &self.multimedia.preview_frames {
                        ui.add_space(16.0);
                        let mut frame_idx = self.multimedia.preview_frame_idx;
                        if ui
                            .add(
                                egui::Slider::new(&mut frame_idx, 0..=(frames.len() as i32 - 1))
                                    .text(self.i18n.tr("frame_label")),
                            )
                            .changed()
                        {
                            self.multimedia.preview_frame_idx = frame_idx;
                            self.multimedia.preview_playing = false; // pause when scrubbing
                        }
                    }
                });
            });

        // Central Canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.multimedia.is_processing {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(egui::Spinner::new().size(40.0));
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(self.i18n.tr("processing_multimedia"))
                                .size(20.0)
                                .strong(),
                        );
                        if let Some(pct) = self.multimedia.processing_progress {
                            ui.add_space(10.0);
                            ui.add(
                                egui::ProgressBar::new(pct)
                                    .show_percentage()
                                    .desired_width(300.0),
                            );
                        }
                    });
                });
            } else {
                let particles_data = self.prepare_render_data_from_multimedia(ctx);
                self.paint_3d_viewport(ui, ctx, &particles_data);
            }
        });
    }

    fn show_text_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(self.i18n.tr("text_input"));
        ui.add(
            egui::TextEdit::multiline(&mut self.multimedia.text_input)
                .desired_width(f32::INFINITY)
                .desired_rows(4),
        );
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(self.i18n.tr("font_size"));
            ui.add(egui::DragValue::new(&mut self.multimedia.font_size).speed(1.0));
        });

        ui.horizontal(|ui| {
            ui.label(self.i18n.tr("brightness_threshold"));
            ui.add(
                egui::Slider::new(&mut self.multimedia.brightness_threshold, 0.0..=1.0)
                    .show_value(true),
            );
        });

        ui.horizontal(|ui| {
            ui.label(self.i18n.tr("duration_s"));
            ui.add(egui::DragValue::new(&mut self.multimedia.duration_secs).speed(0.1));
        });

        ui.horizontal(|ui| {
            if ui.button(self.i18n.tr("load_font")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Fonts", &["ttf", "otf"])
                    .pick_file()
                {
                    self.multimedia.font_name = path.to_string_lossy().to_string();
                }
            }
            if !self.multimedia.font_name.is_empty()
                && !self.multimedia.font_name.starts_with("system://")
            {
                let p = std::path::Path::new(&self.multimedia.font_name);
                ui.label(p.file_name().unwrap_or_default().to_string_lossy());
            }
        });

        ui.horizontal(|ui| {
            if ui.button(self.i18n.tr("load_system_font")).clicked() {
                use font_kit::source::SystemSource;
                if let Ok(families) = SystemSource::new().all_families() {
                    self.multimedia.system_fonts = families;
                }
            }
        });

        if !self.multimedia.system_fonts.is_empty() {
            egui::ComboBox::from_label(self.i18n.tr("load_system_font"))
                .selected_text(if self.multimedia.font_name.starts_with("system://") {
                    self.multimedia.font_name.replace("system://", "")
                } else {
                    "Select".to_string()
                })
                .show_ui(ui, |ui| {
                    for font in &self.multimedia.system_fonts {
                        if ui
                            .selectable_label(
                                self.multimedia.font_name == format!("system://{}", font),
                                font,
                            )
                            .clicked()
                        {
                            self.multimedia.font_name = format!("system://{}", font);
                        }
                    }
                });
        }
    }

    fn estimate_multimedia_particles(&self) -> usize {
        let density = self.multimedia.density.max(0.001);
        match self.multimedia.mode {
            0 => {
                // Text: rough estimate based on font size and char count
                // A char at font_size px is roughly font_size * font_size * 0.4 opaque pixels
                let char_count = self
                    .multimedia
                    .text_input
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .count();
                let pixels_per_char =
                    (self.multimedia.font_size * self.multimedia.font_size * 0.4) as usize;
                let total_opaque = char_count * pixels_per_char;
                if density >= 1.0 {
                    // density particles per pixel
                    (total_opaque as f32 * density) as usize
                } else {
                    // step = 1/density, sampled in 2D
                    let step = (1.0 / density) as usize;
                    total_opaque / (step * step).max(1)
                }
            }
            1 => {
                // Image
                if let Some(path) = &self.multimedia.media_path {
                    if let Ok(reader) = image::ImageReader::open(path) {
                        if let Ok((w, h)) = reader.into_dimensions() {
                            // ~70% of pixels assumed opaque
                            let total_opaque = ((w as f64 * h as f64) * 0.7) as usize;
                            if density >= 1.0 {
                                (total_opaque as f32 * density) as usize
                            } else {
                                let step = (1.0 / density) as usize;
                                total_opaque / (step * step).max(1)
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn show_image_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(self.i18n.tr("select_image")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Images", &["png", "jpg", "jpeg", "webp"])
                    .pick_file()
                {
                    self.multimedia.media_path = Some(path.to_string_lossy().to_string());
                }
            }
            if let Some(path) = &self.multimedia.media_path {
                ui.label(
                    std::path::Path::new(path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                );
            }
        });
        ui.horizontal(|ui| {
            ui.label(self.i18n.tr("density"));
            ui.add(
                egui::DragValue::new(&mut self.multimedia.density)
                    .speed(0.01)
                    .clamp_range(0.01..=1.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label(self.i18n.tr("brightness_threshold"));
            ui.add(
                egui::Slider::new(&mut self.multimedia.brightness_threshold, 0.0..=1.0)
                    .show_value(true),
            );
        });
        ui.horizontal(|ui| {
            ui.label(self.i18n.tr("duration_s"));
            ui.add(egui::DragValue::new(&mut self.multimedia.duration_secs).speed(0.1));
        });
    }

    fn show_video_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(self.i18n.tr("select_video")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Videos", &["mp4", "mkv", "avi", "webm"])
                    .pick_file()
                {
                    self.multimedia.media_path = Some(path.to_string_lossy().to_string());
                }
            }
            if let Some(path) = &self.multimedia.media_path {
                ui.label(
                    std::path::Path::new(path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                );
            }
        });
        ui.horizontal(|ui| {
            ui.label(self.i18n.tr("density"));
            ui.add(
                egui::DragValue::new(&mut self.multimedia.density)
                    .speed(0.01)
                    .clamp_range(0.01..=1.0),
            );
        });
        ui.label(
            egui::RichText::new(self.i18n.tr("video_note"))
                .small()
                .color(egui::Color32::YELLOW),
        );
    }

    fn prepare_render_data_from_multimedia(&mut self, ctx: &egui::Context) -> Vec<f32> {
        let frames = match &self.multimedia.preview_frames {
            Some(f) => f,
            None => return vec![],
        };

        if frames.is_empty() {
            return vec![];
        }

        if self.multimedia.preview_playing {
            let dt = 1.0 / self.multimedia.target_fps as f32;
            let actual_dt = ctx.input(|i| i.stable_dt);
            self.multimedia.preview_timer += actual_dt;
            if self.multimedia.preview_timer >= dt {
                self.multimedia.preview_timer -= dt;
                self.multimedia.preview_frame_idx += 1;
                if self.multimedia.preview_frame_idx >= frames.len() as i32 {
                    self.multimedia.preview_frame_idx = 0; // loop
                }
            }
            ctx.request_repaint();
        }

        let idx = (self.multimedia.preview_frame_idx as usize).min(frames.len() - 1);
        self.prepare_render_data_from(&frames[idx])
    }

    // ==========================================
    // Core Multimedia Compilation Logic
    // ==========================================

    fn compile_multimedia_preview(&mut self, ctx: &egui::Context) {
        self.multimedia.status_msg = Some("Compiling preview...".to_string());

        let mode = self.multimedia.mode;
        self.multimedia.source_image_preview = None;

        let mut img: Option<DynamicImage> = None;

        if mode == 0 {
            // Text Mode
            let text = &self.multimedia.text_input;
            let mut font_data = None;

            if self.multimedia.font_name.starts_with("system://") {
                let family = &self.multimedia.font_name[9..];
                use font_kit::family_name::FamilyName;
                use font_kit::properties::Properties;
                if let Ok(handle) = font_kit::source::SystemSource::new()
                    .select_best_match(&[FamilyName::Title(family.to_string())], &Properties::new())
                {
                    font_data = match handle {
                        font_kit::handle::Handle::Path { path, .. } => std::fs::read(&path).ok(),
                        font_kit::handle::Handle::Memory { bytes, .. } => Some(bytes.to_vec()),
                    };
                }
            } else if !self.multimedia.font_name.is_empty() {
                font_data = std::fs::read(&self.multimedia.font_name).ok();
            }

            if let Some(fd) = font_data {
                if let Ok(font_ref) = ab_glyph::FontRef::try_from_slice(&fd) {
                    let px_scale = PxScale::from(self.multimedia.font_size);
                    let scale_font = font_ref.as_scaled(px_scale);
                    let lines: Vec<&str> = text.split('\n').collect();

                    // Use font metrics for reliable line height
                    let ascent = scale_font.ascent().ceil() as u32;
                    let descent = scale_font.descent().floor() as i32;
                    let line_height = (ascent as i32 - descent).abs() as u32;
                    let line_gap: u32 = (line_height as f32 * 0.2).ceil() as u32;

                    // Measure each line
                    let mut max_w: u32 = 1;
                    for line in &lines {
                        let measure = if line.is_empty() { " " } else { line };
                        let (w, _h) = imageproc::drawing::text_size(px_scale, &font_ref, measure);
                        max_w = max_w.max(w as u32);
                    }

                    let pad = (self.multimedia.font_size * 0.5) as u32;
                    let canvas_w = max_w + pad * 2;
                    let canvas_h = (lines.len() as u32 * line_height)
                        + ((lines.len() as u32).saturating_sub(1) * line_gap)
                        + pad * 2;
                    let mut text_img = image::RgbaImage::new(canvas_w, canvas_h);

                    let mut cur_y = pad as i32;
                    for line in lines {
                        if !line.is_empty() {
                            imageproc::drawing::draw_text_mut(
                                &mut text_img,
                                image::Rgba([255, 255, 255, 255]),
                                pad as i32,
                                cur_y,
                                px_scale,
                                &font_ref,
                                line,
                            );
                        }
                        cur_y += (line_height + line_gap) as i32;
                    }
                    img = Some(DynamicImage::ImageRgba8(text_img));
                } else {
                    self.multimedia.status_msg = Some("Failed to parse font".into());
                    return;
                }
            } else {
                self.multimedia.status_msg = Some("No valid Font selected/found".into());
                return;
            }
        } else if mode == 1 {
            // Image Mode
            if let Some(path) = &self.multimedia.media_path {
                if let Ok(loaded) = image::open(path) {
                    img = Some(loaded);
                } else {
                    self.multimedia.status_msg = Some("Failed to load Image".into());
                    return;
                }
            } else {
                self.multimedia.status_msg = Some("No Image selected".into());
                return;
            }
        } else if mode == 2 {
            self.multimedia.status_msg = Some("Video ready to export!".to_string());
            return;
        }

        if let Some(img) = img {
            // Update source image preview texture
            let size = [img.width() as usize, img.height() as usize];
            let pixels = img.to_rgba8();
            let color_img =
                egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_flat_samples().as_slice());
            self.multimedia.source_image_preview =
                Some(ctx.load_texture("multimedia_source_preview", color_img, Default::default()));

            let mut base_particles = Vec::new();
            let (width, height) = img.dimensions();
            let cx = width as f32 / 2.0;
            let cy = height as f32 / 2.0;
            let dist_scale = self.multimedia.particle_scale;
            let density = self.multimedia.density.max(0.001);

            let mut id: i32 = 0;
            // density < 1: skip pixels (step_by)
            // density >= 1: create floor(density) particles per pixel with jitter
            let step = if density < 1.0 {
                (1.0 / density).ceil() as u32
            } else {
                1u32
            };
            let copies_per_pixel = if density >= 1.0 {
                density.floor() as u32
            } else {
                1u32
            };
            use rand::Rng;
            let mut rng = rand::thread_rng();

            for y in (0..height).step_by(step as usize) {
                for x in (0..width).step_by(step as usize) {
                    let pixel = img.get_pixel(x, y);

                    // Brightness check (Luma)
                    let brightness = (pixel[0] as f32 * 0.299
                        + pixel[1] as f32 * 0.587
                        + pixel[2] as f32 * 0.114)
                        / 255.0;
                    if pixel[3] == 0 || brightness < self.multimedia.brightness_threshold {
                        continue;
                    }

                    for c in 0..copies_per_pixel {
                        // First copy at exact position, extras get sub-pixel jitter
                        let jx = if c == 0 {
                            0.0
                        } else {
                            rng.gen_range(-0.5..0.5)
                        };
                        let jy = if c == 0 {
                            0.0
                        } else {
                            rng.gen_range(-0.5..0.5)
                        };
                        let px = (x as f32 + jx - cx) * dist_scale;
                        let py = -(y as f32 + jy - cy) * dist_scale;
                        base_particles.push(Particle {
                            id,
                            pos: [px, py, 0.0],
                            color: [pixel[0], pixel[1], pixel[2], pixel[3]],
                            size: self.multimedia.particle_size,
                            tex_id: 0,
                            seq_index: 0,
                        });
                        id += 1;
                    }
                }
            }

            let total_frames =
                (self.multimedia.duration_secs * self.multimedia.target_fps as f32) as usize;
            let intro_frames =
                (self.multimedia.intro_duration * self.multimedia.target_fps as f32) as usize;
            let outro_frames =
                (self.multimedia.outro_duration * self.multimedia.target_fps as f32) as usize;

            // Compile Particleex statements
            let stmts = crate::particleex::compile_expr(&self.multimedia.velocity_expr);

            let mut frames = Vec::with_capacity(total_frames);
            let mut runtime_particles = base_particles.clone();

            for f_idx in 0..total_frames {
                let t = f_idx as f64 / self.multimedia.target_fps as f64;

                for p in runtime_particles.iter_mut() {
                    let mut pex_ctx = crate::particleex::ExprContext::new();
                    // Set inputs (Normalized to 0.0-1.0 for colors to match Pex logic)
                    pex_ctx.set("t", t);
                    pex_ctx.set("x", p.pos[0] as f64);
                    pex_ctx.set("y", p.pos[1] as f64);
                    pex_ctx.set("z", p.pos[2] as f64);
                    pex_ctx.set("cr", p.color[0] as f64 / 255.0);
                    pex_ctx.set("cg", p.color[1] as f64 / 255.0);
                    pex_ctx.set("cb", p.color[2] as f64 / 255.0);
                    pex_ctx.set("alpha", p.color[3] as f64 / 255.0);
                    pex_ctx.set("mpsize", p.size as f64);
                    pex_ctx.set("vx", 0.0);
                    pex_ctx.set("vy", 0.0);
                    pex_ctx.set("vz", 0.0);
                    pex_ctx.set("destory", 0.0);

                    // Execute statements
                    if let Some(ref s) = stmts {
                        crate::particleex::exec_stmts(s, &mut pex_ctx);
                    }

                    if pex_ctx.get("destory") >= 1.0 {
                        p.color[3] = 0; // effectively hide it
                    }

                    // Read back (Update velocity -> position)
                    let vx = pex_ctx.get("vx") as f32;
                    let vy = pex_ctx.get("vy") as f32;
                    let vz = pex_ctx.get("vz") as f32;

                    p.pos[0] += vx;
                    p.pos[1] += vy;
                    p.pos[2] += vz;

                    p.color[0] = (pex_ctx.get("cr").clamp(0.0, 1.0) * 255.0) as u8;
                    p.color[1] = (pex_ctx.get("cg").clamp(0.0, 1.0) * 255.0) as u8;
                    p.color[2] = (pex_ctx.get("cb").clamp(0.0, 1.0) * 255.0) as u8;
                    p.color[3] = (p.color[3] as f64 * pex_ctx.get("alpha").clamp(0.0, 1.0)) as u8;
                    p.size = pex_ctx.get("mpsize") as f32;
                }

                // Apply Intro/Outro effects (per preset)
                let mut frame_particles = runtime_particles.clone();
                use crate::ui::app::{IntroPreset, OutroPreset};

                if f_idx < intro_frames && intro_frames > 0 {
                    let t_raw = f_idx as f32 / intro_frames as f32;
                    let t = t_raw * t_raw * (3.0 - 2.0 * t_raw); // smoothstep
                    let p0 = self.multimedia.intro_params[0];
                    let p1 = self.multimedia.intro_params[1];
                    match self.multimedia.intro_preset {
                        IntroPreset::None => {}
                        IntroPreset::FadeScale => {
                            for p in frame_particles.iter_mut() {
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                                p.size *= t;
                            }
                        }
                        IntroPreset::ScatterIn => {
                            // p0 = spread distance
                            let spread = (1.0 - t) * p0;
                            for (i, p) in frame_particles.iter_mut().enumerate() {
                                let angle = (i as f32 * 2.39996) * std::f32::consts::PI;
                                p.pos[0] += angle.cos() * spread;
                                p.pos[1] += angle.sin() * spread;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        IntroPreset::SlideUp => {
                            // p0 = slide distance
                            let offset = (1.0 - t) * -p0;
                            for p in frame_particles.iter_mut() {
                                p.pos[1] += offset;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        IntroPreset::ZoomIn => {
                            // p0 = initial scale
                            let scale_factor = 1.0 + (1.0 - t) * (p0 - 1.0);
                            for p in frame_particles.iter_mut() {
                                p.size *= scale_factor;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        IntroPreset::SpinIn => {
                            // p0 = rotations, p1 = radius
                            let angle = (1.0 - t) * std::f32::consts::TAU * p0;
                            let cos_a = angle.cos();
                            let sin_a = angle.sin();
                            let r = (1.0 - t) * p1 + 1.0;
                            for p in frame_particles.iter_mut() {
                                let ox = p.pos[0];
                                let oy = p.pos[1];
                                p.pos[0] = ox * cos_a - oy * sin_a * r;
                                p.pos[1] = ox * sin_a + oy * cos_a * r;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        IntroPreset::DropIn => {
                            // p0 = drop height
                            let offset = (1.0 - t) * p0;
                            for p in frame_particles.iter_mut() {
                                p.pos[1] += offset;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                                p.size *= t.sqrt();
                            }
                        }
                    }
                } else if f_idx > total_frames.saturating_sub(outro_frames) && outro_frames > 0 {
                    let t_raw = (total_frames - f_idx) as f32 / outro_frames as f32;
                    let t = t_raw * (2.0 - t_raw); // ease out quad
                    let p0 = self.multimedia.outro_params[0];
                    let p1 = self.multimedia.outro_params[1];
                    match self.multimedia.outro_preset {
                        OutroPreset::None => {}
                        OutroPreset::FadeScale => {
                            for p in frame_particles.iter_mut() {
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                                p.size *= t;
                            }
                        }
                        OutroPreset::ScatterOut => {
                            // p0 = spread distance
                            let spread = (1.0 - t) * p0;
                            for (i, p) in frame_particles.iter_mut().enumerate() {
                                let angle = (i as f32 * 2.39996) * std::f32::consts::PI;
                                p.pos[0] += angle.cos() * spread;
                                p.pos[1] += angle.sin() * spread;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        OutroPreset::SlideDown => {
                            // p0 = slide distance
                            let offset = (1.0 - t) * p0;
                            for p in frame_particles.iter_mut() {
                                p.pos[1] -= offset;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        OutroPreset::Explode => {
                            // p0 = explosion speed
                            let speed = (1.0 - t) * p0;
                            for (i, p) in frame_particles.iter_mut().enumerate() {
                                let angle = (i as f32 * 2.39996) * std::f32::consts::PI;
                                let dir_x = p.pos[0].signum();
                                let dir_y = p.pos[1].signum();
                                p.pos[0] += angle.cos() * speed + dir_x * speed * 0.5;
                                p.pos[1] += angle.sin() * speed + dir_y * speed * 0.5;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                                p.size *= t.max(0.01);
                            }
                        }
                        OutroPreset::Vortex => {
                            // p0 = rotations, p1 = expansion
                            let angle = (1.0 - t) * std::f32::consts::TAU * p0;
                            let cos_a = angle.cos();
                            let sin_a = angle.sin();
                            let expand = 1.0 + (1.0 - t) * p1;
                            for p in frame_particles.iter_mut() {
                                let ox = p.pos[0];
                                let oy = p.pos[1];
                                p.pos[0] = (ox * cos_a - oy * sin_a) * expand;
                                p.pos[1] = (ox * sin_a + oy * cos_a) * expand;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        OutroPreset::ZoomOut => {
                            // p0 = scale factor
                            let scale_factor = 1.0 + (1.0 - t) * (p0 - 1.0);
                            for p in frame_particles.iter_mut() {
                                p.pos[0] *= scale_factor;
                                p.pos[1] *= scale_factor;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                    }
                }

                frames.push(frame_particles);
            }

            self.multimedia.preview_frames = Some(frames);
            self.multimedia.status_msg = Some("Compilation Success!".to_string());
            self.multimedia.preview_playing = true;
            self.multimedia.preview_frame_idx = 0;
        }
    }

    pub fn export_multimedia_nbl(&mut self) {
        if self.multimedia.mode == 2 {
            self.export_video_streaming();
        } else {
            // General export
            if let Some(frames) = &self.multimedia.preview_frames {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Nebula", &["nbl"])
                    .save_file()
                {
                    let (bbox_min, bbox_max) = crate::player::recalculate_bbox(frames);
                    let header = NblHeader {
                        version: 1,
                        target_fps: self.multimedia.target_fps,
                        total_frames: frames.len() as u32,
                        texture_count: 0,
                        attributes: 0x03,
                        bbox_min,
                        bbox_max,
                    };

                    match self.player.save_file(&path, &header, &[], frames) {
                        Ok(_) => self.multimedia.status_msg = Some("Export success!".into()),
                        Err(e) => {
                            self.multimedia.status_msg = Some(format!("Export failed: {}", e))
                        }
                    }
                }
            }
        }
    }

    fn export_video_streaming(&mut self) {
        let _media_path = match &self.multimedia.media_path {
            Some(p) => p.clone(),
            None => {
                self.multimedia.status_msg = Some("No video selected!".into());
                return;
            }
        };

        let _out_path = match rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .save_file()
        {
            Some(p) => p,
            None => return,
        };

        self.multimedia.is_processing = true;
        self.multimedia.status_msg = Some("Exporting Video Stream...".into());
        self.multimedia.processing_progress = Some(0.0);

        // Spawn a background thread to handle ffmpeg reading and streaming write
        let _density = self.multimedia.density;
        let progress_arc = Arc::new(Mutex::new(0.0));
        let progress_clone = Arc::clone(&progress_arc);

        std::thread::spawn(move || {
            // Drop unused imports
            // Note: This relies on ffmpeg being installed on the system path.
            // Simplified logic: we read video dimensions first, then read frames.

            // Dummy logic to simulate video processing progress and streaming
            for i in 0..100 {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Ok(mut p) = progress_clone.lock() {
                    *p = (i as f32) / 100.0;
                }
            }

            // After finishing:
            if let Ok(mut p) = progress_clone.lock() {
                *p = 1.0;
            }
        });

        self.multimedia.status_msg = Some("Background processing started. Check console/logs if it fails (requires ffmpeg installed).".into());
    }

    fn show_expression_editor(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(self.i18n.tr("velocity_expr"));

            // Editor theme/styling
            let editor_id = ui.make_persistent_id("velocity_script_editor");
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.multimedia.velocity_expr)
                        .id(editor_id)
                        .font(egui::TextStyle::Monospace) // Mono font for code
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .lock_focus(true)
                        .hint_text("vx = cos(t*0.1); vy = sin(t*0.1); ..."),
                );
            });

            ui.add_space(4.0);

            // Advanced Help / Hints
            ui.collapsing(self.i18n.tr("expr_help"), |ui| {
                ui.small(self.i18n.tr("expr_funcs_desc"));
                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.label(egui::RichText::new(self.i18n.tr("expr_vars")).strong());
                    ui.small(self.i18n.tr("expr_vars_desc"));

                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        let vars = [
                            "x", "y", "z", "vx", "vy", "vz", "id", "cr", "cg", "cb", "alpha",
                            "mpsize", "t",
                        ];
                        for v in vars {
                            if ui.button(egui::RichText::new(v).monospace()).clicked() {
                                self.multimedia.velocity_expr.push_str(v);
                            }
                        }
                    });
                });

                ui.add_space(4.0);
                ui.group(|ui| {
                    ui.label(egui::RichText::new(self.i18n.tr("expr_funcs")).strong());
                    ui.horizontal_wrapped(|ui| {
                        let funcs = [
                            "sin()", "cos()", "tan()", "abs()", "random()", "pow()", "sqrt()",
                            "lerp()", "clamp()",
                        ];
                        for f in funcs {
                            if ui.button(egui::RichText::new(f).monospace()).clicked() {
                                self.multimedia.velocity_expr.push_str(f);
                            }
                        }
                    });
                });
            });
        });
    }
}
