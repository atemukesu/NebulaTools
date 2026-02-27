use crate::player::{NblHeader, Particle};
use crate::ui::app::NebulaToolsApp;
use ab_glyph::{Font, PxScale, ScaleFont};
use eframe::egui;
use image::{DynamicImage, GenericImageView};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

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
                            // Intro/Outro only for text and image modes
                            if self.multimedia.mode != 2 {
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("intro_duration"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.multimedia.intro_duration)
                                            .speed(0.1)
                                            .clamp_range(0.0..=f32::MAX),
                                    );
                                });
                                // Intro Preset
                                let cur_intro =
                                    self.i18n.tr(self.multimedia.intro_preset.i18n_key());
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
                                    for (idx, (key, _default, min, max)) in info.iter().enumerate()
                                    {
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
                                            .clamp_range(0.0..=f32::MAX),
                                    );
                                });
                                // Outro Preset
                                let cur_outro =
                                    self.i18n.tr(self.multimedia.outro_preset.i18n_key());
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
                                    for (idx, (key, _default, min, max)) in info.iter().enumerate()
                                    {
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
                            }
                            self.show_expression_editor(ui);
                            ui.separator();
                            self.show_multimedia_common_settings(ui);
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
                                .button(format!("🔄 {}", self.i18n.tr("refresh_source")))
                                .clicked()
                            {
                                self.compile_multimedia_preview(ctx, true);
                            }
                            if ui
                                .button(format!("▶ {}", self.i18n.tr("compile_preview")))
                                .clicked()
                                && !self.multimedia.is_processing
                            {
                                self.compile_multimedia_preview(ctx, false);
                            }
                            if ui
                                .button(format!("💾 {}", self.i18n.tr("export_nbl")))
                                .clicked()
                            {
                                self.export_multimedia_nbl(ctx);
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
                // Poll shared state from video compile thread
                if let Some((ref progress_arc, ref status_arc, ref done_arc, ref frames_arc)) =
                    self.multimedia.video_compile_shared
                {
                    if let Ok(pct) = progress_arc.lock() {
                        self.multimedia.processing_progress = Some(*pct);
                    }
                    let is_done = done_arc.lock().map(|d| *d).unwrap_or(false);
                    if is_done {
                        self.multimedia.is_processing = false;
                        self.multimedia.processing_progress = None;
                        if let Ok(status) = status_arc.lock() {
                            self.multimedia.status_msg = status.clone();
                        }
                        // Pick up compiled frames
                        if let Ok(mut frames_lock) = frames_arc.lock() {
                            if let Some(frames) = frames_lock.take() {
                                self.multimedia.preview_frames = Some(frames);
                                self.multimedia.preview_playing = true;
                                self.multimedia.preview_frame_idx = 0;
                            }
                        }
                        self.multimedia.video_compile_shared = None;
                    } else {
                        ctx.request_repaint();
                    }
                }

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

    fn count_particles(&self, w: u32, h: u32, density: f32) -> usize {
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
        let nx = w / step;
        let ny = h / step;
        (nx * ny * copies_per_pixel) as usize
    }

    fn estimate_multimedia_particles(&self) -> usize {
        let density = self.multimedia.density.max(0.000001);

        match self.multimedia.mode {
            0 => {
                let char_count = self
                    .multimedia
                    .text_input
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .count();
                let pixels_per_char =
                    (self.multimedia.font_size * self.multimedia.font_size * 0.3) as u32;

                let effective_density = if density < 1.0 { density } else { 1.0 };
                let copies = if density >= 1.0 {
                    density.floor() as u32
                } else {
                    1u32
                };

                (char_count as f32 * pixels_per_char as f32 * effective_density * copies as f32)
                    as usize
            }
            1 => {
                let [w, h] = self.multimedia.last_source_size.unwrap_or([1920, 1080]);
                self.count_particles(w, h, density)
            }
            2 => {
                let [w, h] = self.multimedia.last_source_size.unwrap_or([1280, 720]);
                self.count_particles(w, h, density)
            }
            _ => 0,
        }
    }

    fn show_multimedia_common_settings(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.label(egui::RichText::new(self.i18n.tr("common_settings")).strong());

            ui.horizontal(|ui| {
                ui.label(self.i18n.tr("particle_size"));
                ui.add(
                    egui::DragValue::new(&mut self.multimedia.particle_size)
                        .speed(0.001)
                        .max_decimals(6)
                        .clamp_range(0.000001..=f32::MAX),
                );
            });

            ui.horizontal(|ui| {
                ui.label(self.i18n.tr("point_size"));
                ui.add(
                    egui::DragValue::new(&mut self.multimedia.point_size)
                        .speed(0.0001)
                        .max_decimals(6)
                        .clamp_range(0.000001..=f32::MAX),
                );
            });

            ui.horizontal(|ui| {
                ui.label(self.i18n.tr("density"));
                ui.add(
                    egui::DragValue::new(&mut self.multimedia.density)
                        .speed(0.001)
                        .max_decimals(6)
                        .clamp_range(0.000001..=f32::MAX),
                );
            });

            ui.horizontal(|ui| {
                ui.label(self.i18n.tr("brightness_threshold"));
                ui.add(egui::Slider::new(
                    &mut self.multimedia.brightness_threshold,
                    0.0..=1.0,
                ));
            });

            ui.horizontal(|ui| {
                ui.label(self.i18n.tr("cr_rotation"));
                ui.add(egui::DragValue::new(&mut self.multimedia.rotation[0]).speed(1.0));
                ui.add(egui::DragValue::new(&mut self.multimedia.rotation[1]).speed(1.0));
                ui.add(egui::DragValue::new(&mut self.multimedia.rotation[2]).speed(1.0));

                egui::ComboBox::from_id_source("mm_rot_preset")
                    .selected_text(self.i18n.tr("preset"))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(false, self.i18n.tr("facing_z_pos"))
                            .clicked()
                        {
                            self.multimedia.rotation = [0.0, 0.0, 0.0];
                        }
                        if ui
                            .selectable_label(false, self.i18n.tr("facing_z_neg"))
                            .clicked()
                        {
                            self.multimedia.rotation = [0.0, 180.0, 0.0];
                        }
                        if ui
                            .selectable_label(false, self.i18n.tr("facing_x_pos"))
                            .clicked()
                        {
                            self.multimedia.rotation = [0.0, -90.0, 0.0];
                        }
                        if ui
                            .selectable_label(false, self.i18n.tr("facing_x_neg"))
                            .clicked()
                        {
                            self.multimedia.rotation = [0.0, 90.0, 0.0];
                        }
                        if ui
                            .selectable_label(false, self.i18n.tr("facing_y_pos"))
                            .clicked()
                        {
                            self.multimedia.rotation = [-90.0, 0.0, 0.0];
                        }
                        if ui
                            .selectable_label(false, self.i18n.tr("facing_y_neg"))
                            .clicked()
                        {
                            self.multimedia.rotation = [90.0, 0.0, 0.0];
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label(self.i18n.tr("duration_s"));
                ui.add(egui::DragValue::new(&mut self.multimedia.duration_secs).speed(0.1));
            });
        });
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

    fn compile_multimedia_preview(&mut self, ctx: &egui::Context, source_only: bool) {
        self.multimedia.status_msg = Some(
            if source_only {
                "Refreshing source..."
            } else {
                "Compiling preview..."
            }
            .to_string(),
        );

        let mode = self.multimedia.mode;
        self.multimedia.source_image_preview = None;

        let mut img: Option<DynamicImage> = None;

        if mode == 0 {
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
                    let lines: Vec<&str> = text.lines().collect();

                    let ascent = scale_font.ascent().ceil() as u32;
                    let descent = scale_font.descent().floor() as i32;
                    let line_height = (ascent as i32 - descent).abs() as u32;
                    let line_gap: u32 = (line_height as f32 * 0.2).ceil() as u32;

                    let mut max_w: u32 = 1;
                    for line in &lines {
                        let mut w: f32 = 0.0;
                        let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
                        for ch in line.chars() {
                            let glyph_id = scale_font.glyph_id(ch);
                            if let Some(prev) = prev_glyph {
                                w += scale_font.kern(prev, glyph_id);
                            }
                            w += scale_font.h_advance(glyph_id);
                            prev_glyph = Some(glyph_id);
                        }
                        max_w = max_w.max(w.ceil() as u32);
                    }

                    let pad = (self.multimedia.font_size as u32).max(1);
                    let canvas_w = max_w + pad * 4;
                    let canvas_h = (lines.len() as u32 * line_height)
                        + ((lines.len() as u32).saturating_sub(1) * line_gap)
                        + pad * 4;
                    let mut text_img = image::RgbaImage::new(canvas_w, canvas_h);

                    let mut cur_y = (pad * 2) as i32;
                    for line in lines {
                        if !line.is_empty() {
                            imageproc::drawing::draw_text_mut(
                                &mut text_img,
                                image::Rgba([255, 255, 255, 255]),
                                (pad * 2) as i32,
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
            // Video Mode: Extract first frame for source preview
            if let Some(path) = &self.multimedia.media_path {
                let output = Command::new("ffmpeg")
                    .args([
                        "-i",
                        path,
                        "-vframes",
                        "1",
                        "-f",
                        "image2pipe",
                        "-vcodec",
                        "png",
                        "-",
                    ])
                    .output();

                if let Ok(out) = output {
                    if let Ok(loaded) = image::load_from_memory(&out.stdout) {
                        // Show source preview from first frame
                        let size = [loaded.width() as usize, loaded.height() as usize];
                        self.multimedia.last_source_size = Some([loaded.width(), loaded.height()]);
                        let pixels = loaded.to_rgba8();
                        let color_img = egui::ColorImage::from_rgba_unmultiplied(
                            size,
                            pixels.as_flat_samples().as_slice(),
                        );
                        self.multimedia.source_image_preview = Some(ctx.load_texture(
                            "multimedia_source_preview",
                            color_img,
                            Default::default(),
                        ));
                    }
                }

                if source_only {
                    self.multimedia.status_msg = Some("Source preview updated".into());
                    return;
                }

                // Compile all video frames in background thread
                self.compile_video_all_frames(ctx);
                return;
            } else {
                self.multimedia.status_msg = Some("No Video selected".into());
                return;
            }
        }

        if let Some(img) = img {
            let size = [img.width() as usize, img.height() as usize];
            let pixels = img.to_rgba8();
            let color_img =
                egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_flat_samples().as_slice());
            self.multimedia.source_image_preview =
                Some(ctx.load_texture("multimedia_source_preview", color_img, Default::default()));

            if source_only {
                self.multimedia.status_msg = Some("Source preview updated".into());
                return;
            }

            let mut base_particles = Vec::new();
            let (width, height) = img.dimensions();
            self.multimedia.last_source_size = Some([width, height]);
            let cx = width as f32 / 2.0;
            let cy = height as f32 / 2.0;
            let dist_scale = self.multimedia.particle_size;
            let density = self.multimedia.density.max(0.000001);

            let mut id: i32 = 0;
            let step = if mode == 0 {
                1u32
            } else if density < 1.0 {
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

                    let is_filtered = if mode == 0 {
                        pixel[3] < 128
                    } else {
                        let luma = (pixel[0] as f32 * 0.299
                            + pixel[1] as f32 * 0.587
                            + pixel[2] as f32 * 0.114)
                            / 255.0;
                        pixel[3] == 0 || luma < self.multimedia.brightness_threshold
                    };

                    if is_filtered {
                        continue;
                    }

                    if mode != 0 && density < 1.0 && rng.gen::<f32>() > density {
                        continue;
                    }

                    for c in 0..copies_per_pixel {
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
                        let (px, py, pz) =
                            apply_euler_rotation(px, py, 0.0, self.multimedia.rotation);
                        base_particles.push(Particle {
                            id,
                            pos: [px, py, pz],
                            color: [pixel[0], pixel[1], pixel[2], pixel[3]],
                            size: self.multimedia.point_size,
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

            let stmts = crate::particleex::compile_expr(&self.multimedia.velocity_expr);

            let mut frames = Vec::with_capacity(total_frames);
            let mut runtime_particles = base_particles.clone();

            let mut pex_ctx = crate::particleex::ExprContext::new();

            for f_idx in 0..total_frames {
                let t = f_idx as f64 / self.multimedia.target_fps as f64;

                for p in runtime_particles.iter_mut() {
                    pex_ctx.set("t", crate::particleex::Value::Num(t));
                    pex_ctx.set("x", crate::particleex::Value::Num(p.pos[0] as f64));
                    pex_ctx.set("y", crate::particleex::Value::Num(p.pos[1] as f64));
                    pex_ctx.set("z", crate::particleex::Value::Num(p.pos[2] as f64));
                    pex_ctx.set(
                        "cr",
                        crate::particleex::Value::Num(p.color[0] as f64 / 255.0),
                    );
                    pex_ctx.set(
                        "cg",
                        crate::particleex::Value::Num(p.color[1] as f64 / 255.0),
                    );
                    pex_ctx.set(
                        "cb",
                        crate::particleex::Value::Num(p.color[2] as f64 / 255.0),
                    );
                    pex_ctx.set(
                        "alpha",
                        crate::particleex::Value::Num(p.color[3] as f64 / 255.0),
                    );
                    pex_ctx.set("mpsize", crate::particleex::Value::Num(p.size as f64));
                    pex_ctx.set("vx", crate::particleex::Value::Num(0.0));
                    pex_ctx.set("vy", crate::particleex::Value::Num(0.0));
                    pex_ctx.set("vz", crate::particleex::Value::Num(0.0));
                    pex_ctx.set("destroy", crate::particleex::Value::Num(0.0));

                    if let Some(ref s) = stmts {
                        crate::particleex::exec_stmts(s, &mut pex_ctx);
                    }

                    if pex_ctx.get("destroy").as_num() >= 1.0 {
                        p.color[3] = 0;
                    }

                    let vx = pex_ctx.get("vx").as_num() as f32;
                    let vy = pex_ctx.get("vy").as_num() as f32;
                    let vz = pex_ctx.get("vz").as_num() as f32;

                    p.pos[0] += vx;
                    p.pos[1] += vy;
                    p.pos[2] += vz;

                    p.color[0] = (pex_ctx.get("cr").as_num().clamp(0.0, 1.0) * 255.0) as u8;
                    p.color[1] = (pex_ctx.get("cg").as_num().clamp(0.0, 1.0) * 255.0) as u8;
                    p.color[2] = (pex_ctx.get("cb").as_num().clamp(0.0, 1.0) * 255.0) as u8;
                    p.color[3] = (pex_ctx.get("alpha").as_num().clamp(0.0, 1.0) * 255.0) as u8;
                    p.size = pex_ctx.get("mpsize").as_num() as f32;
                }

                let mut frame_particles = runtime_particles.clone();
                use crate::ui::app::{IntroPreset, OutroPreset};

                if f_idx < intro_frames && intro_frames > 0 {
                    let t_raw = f_idx as f32 / intro_frames as f32;
                    let t = t_raw * t_raw * (3.0 - 2.0 * t_raw);
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
                            let spread = (1.0 - t) * p0;
                            for (i, p) in frame_particles.iter_mut().enumerate() {
                                let angle = (i as f32 * 2.39996) * std::f32::consts::PI;
                                p.pos[0] += angle.cos() * spread;
                                p.pos[1] += angle.sin() * spread;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        IntroPreset::SlideUp => {
                            let offset = (1.0 - t) * -p0;
                            for p in frame_particles.iter_mut() {
                                p.pos[1] += offset;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        IntroPreset::ZoomIn => {
                            let scale_factor = 1.0 + (1.0 - t) * (p0 - 1.0);
                            for p in frame_particles.iter_mut() {
                                p.size *= scale_factor;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        IntroPreset::SpinIn => {
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
                    let t = t_raw * (2.0 - t_raw);
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
                            let spread = (1.0 - t) * p0;
                            for (i, p) in frame_particles.iter_mut().enumerate() {
                                let angle = (i as f32 * 2.39996) * std::f32::consts::PI;
                                p.pos[0] += angle.cos() * spread;
                                p.pos[1] += angle.sin() * spread;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        OutroPreset::SlideDown => {
                            let offset = (1.0 - t) * p0;
                            for p in frame_particles.iter_mut() {
                                p.pos[1] -= offset;
                                p.color[3] = (p.color[3] as f32 * t) as u8;
                            }
                        }
                        OutroPreset::Explode => {
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

    pub fn export_multimedia_nbl(&mut self, _ctx: &egui::Context) {
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
                    Err(e) => self.multimedia.status_msg = Some(format!("Export failed: {}", e)),
                }
            }
        } else {
            self.multimedia.status_msg = Some("No compiled preview. Please compile first.".into());
        }
    }

    fn compile_video_all_frames(&mut self, ctx: &egui::Context) {
        let media_path = match &self.multimedia.media_path {
            Some(p) => p.clone(),
            None => {
                self.multimedia.status_msg = Some("No video selected!".into());
                return;
            }
        };

        self.multimedia.is_processing = true;
        self.multimedia.status_msg = Some("Compiling video frames...".into());
        self.multimedia.processing_progress = Some(0.0);

        let target_fps = self.multimedia.target_fps;
        let density = self.multimedia.density.max(0.000001);
        let brightness_threshold = self.multimedia.brightness_threshold;
        let particle_size = self.multimedia.particle_size;
        let point_size = self.multimedia.point_size;
        let rotation = self.multimedia.rotation;
        let velocity_expr = self.multimedia.velocity_expr.clone();

        // Shared state for thread communication
        let shared_progress = Arc::new(Mutex::new(0.0f32));
        let shared_status = Arc::new(Mutex::new(None::<String>));
        let shared_done = Arc::new(Mutex::new(false));
        let shared_frames = Arc::new(Mutex::new(None::<Vec<Vec<Particle>>>));

        let progress_clone = shared_progress.clone();
        let status_clone = shared_status.clone();
        let done_clone = shared_done.clone();
        let frames_clone = shared_frames.clone();

        self.multimedia.video_compile_shared =
            Some((shared_progress, shared_status, shared_done, shared_frames));

        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let probe = Command::new("ffprobe")
                .args([
                    "-v",
                    "error",
                    "-select_streams",
                    "v:0",
                    "-show_entries",
                    "stream=width,height,duration",
                    "-of",
                    "csv=p=0",
                    &media_path,
                ])
                .output();

            let (width, height, duration) = if let Ok(out) = probe {
                let s = String::from_utf8_lossy(&out.stdout);
                let parts: Vec<&str> = s.trim().split(',').collect();
                if parts.len() >= 3 {
                    (
                        parts[0].parse::<u32>().unwrap_or(1280),
                        parts[1].parse::<u32>().unwrap_or(720),
                        parts[2].parse::<f32>().unwrap_or(10.0),
                    )
                } else {
                    (1280, 720, 10.0)
                }
            } else {
                (1280, 720, 10.0)
            };

            let child = Command::new("ffmpeg")
                .args([
                    "-i",
                    &media_path,
                    "-f",
                    "image2pipe",
                    "-vcodec",
                    "rawvideo",
                    "-pix_fmt",
                    "rgb24",
                    "-r",
                    &target_fps.to_string(),
                    "-",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn();

            let mut child = match child {
                Ok(c) => c,
                Err(e) => {
                    *status_clone.lock().unwrap() = Some(format!(
                        "Failed to start FFmpeg: {}. Is FFmpeg installed?",
                        e
                    ));
                    *done_clone.lock().unwrap() = true;
                    ctx_clone.request_repaint();
                    return;
                }
            };

            let mut stdout = child.stdout.take().expect("Failed to open stdout");
            let frame_size = (width * height * 3) as usize;
            let mut buffer = vec![0u8; frame_size];

            let mut frames: Vec<Vec<Particle>> = Vec::new();
            let mut frame_count = 0usize;
            let total_est_frames = (duration * target_fps as f32).ceil().max(1.0) as usize;

            let cx = width as f32 / 2.0;
            let cy = height as f32 / 2.0;

            let step = if density < 1.0 {
                (1.0 / density).ceil() as usize
            } else {
                1usize
            };
            let copies_per_pixel = if density >= 1.0 {
                density.floor() as u32
            } else {
                1u32
            };

            let stmts = crate::particleex::compile_expr(&velocity_expr);

            use rand::Rng;
            let mut rng = rand::thread_rng();

            struct ScreenPixel {
                idx: usize,
                px: f32,
                py: f32,
                pz: f32,
                id: i32,
            }

            let mut screen_pixels = Vec::new();
            let mut fixed_pid: i32 = 0;
            // Pre-calculate screen layout to ensure stable PIDs and positions across frames
            for y in (0..height).step_by(step) {
                for x in (0..width).step_by(step) {
                    if density < 1.0 && rng.gen::<f32>() > density {
                        continue;
                    }
                    let idx = ((y * width + x) * 3) as usize;
                    for c in 0..copies_per_pixel {
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
                        let px = (x as f32 + jx - cx) * particle_size;
                        let py = -(y as f32 + jy - cy) * particle_size;
                        let (px, py, pz) = apply_euler_rotation(px, py, 0.0, rotation);

                        screen_pixels.push(ScreenPixel {
                            idx,
                            px,
                            py,
                            pz,
                            id: fixed_pid,
                        });
                        fixed_pid += 1;
                    }
                }
            }

            let mut pex_ctx = crate::particleex::ExprContext::new();

            while stdout.read_exact(&mut buffer).is_ok() {
                let mut frame_particles = Vec::with_capacity(screen_pixels.len());
                let t = frame_count as f64 / target_fps as f64;

                for sp in &screen_pixels {
                    let r = buffer[sp.idx];
                    let g = buffer[sp.idx + 1];
                    let b = buffer[sp.idx + 2];
                    let luma = (r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114) / 255.0;
                    if luma < brightness_threshold {
                        continue;
                    }

                    // Run velocity_expr per particle, cr/cg/cb override video pixel
                    pex_ctx.set("t", crate::particleex::Value::Num(t));
                    pex_ctx.set("x", crate::particleex::Value::Num(sp.px as f64));
                    pex_ctx.set("y", crate::particleex::Value::Num(sp.py as f64));
                    pex_ctx.set("z", crate::particleex::Value::Num(sp.pz as f64));
                    pex_ctx.set("cr", crate::particleex::Value::Num(r as f64 / 255.0));
                    pex_ctx.set("cg", crate::particleex::Value::Num(g as f64 / 255.0));
                    pex_ctx.set("cb", crate::particleex::Value::Num(b as f64 / 255.0));
                    pex_ctx.set("alpha", crate::particleex::Value::Num(1.0));
                    pex_ctx.set("mpsize", crate::particleex::Value::Num(point_size as f64));
                    pex_ctx.set("vx", crate::particleex::Value::Num(0.0));
                    pex_ctx.set("vy", crate::particleex::Value::Num(0.0));
                    pex_ctx.set("vz", crate::particleex::Value::Num(0.0));
                    pex_ctx.set("destroy", crate::particleex::Value::Num(0.0));
                    pex_ctx.set("id", crate::particleex::Value::Num(sp.id as f64));

                    if let Some(ref s) = stmts {
                        crate::particleex::exec_stmts(s, &mut pex_ctx);
                    }

                    if pex_ctx.get("destroy").as_num() >= 1.0 {
                        continue; // skip destroyed particles
                    }

                    let final_x = sp.px + pex_ctx.get("vx").as_num() as f32;
                    let final_y = sp.py + pex_ctx.get("vy").as_num() as f32;
                    let final_z = pex_ctx.get("vz").as_num() as f32;
                    let final_r = (pex_ctx.get("cr").as_num().clamp(0.0, 1.0) * 255.0) as u8;
                    let final_g = (pex_ctx.get("cg").as_num().clamp(0.0, 1.0) * 255.0) as u8;
                    let final_b = (pex_ctx.get("cb").as_num().clamp(0.0, 1.0) * 255.0) as u8;
                    let final_a = (pex_ctx.get("alpha").as_num().clamp(0.0, 1.0) * 255.0) as u8;
                    let final_size = pex_ctx.get("mpsize").as_num() as f32;

                    frame_particles.push(Particle {
                        id: sp.id,
                        pos: [final_x, final_y, final_z],
                        color: [final_r, final_g, final_b, final_a],
                        size: final_size,
                        tex_id: 0,
                        seq_index: 0,
                    });
                }
                frames.push(frame_particles);
                frame_count += 1;

                // Update progress
                let pct = (frame_count as f32 / total_est_frames as f32).min(1.0);
                *progress_clone.lock().unwrap() = pct;
                ctx_clone.request_repaint();
            }

            let _ = child.kill();

            if !frames.is_empty() {
                *status_clone.lock().unwrap() =
                    Some(format!("Compilation Success! {} frames.", frames.len()));
                *frames_clone.lock().unwrap() = Some(frames);
            } else {
                *status_clone.lock().unwrap() =
                    Some("Failed: no frames decoded from video.".into());
            }

            *done_clone.lock().unwrap() = true;
            ctx_clone.request_repaint();
        });
    }

    fn show_expression_editor(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label(self.i18n.tr("velocity_expr"));
            let editor_id = ui.make_persistent_id("velocity_script_editor");
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.multimedia.velocity_expr)
                        .id(editor_id)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(4)
                        .lock_focus(true)
                        .hint_text("vx = cos(t*0.1); vy = sin(t*0.1); ..."),
                );
            });

            ui.add_space(4.0);
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
