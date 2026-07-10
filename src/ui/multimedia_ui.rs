use crate::player::{NblHeader, Particle};
use crate::ui::app::{
    build_texture_entries, MultimediaThreadProgress, MultimediaThreadStatus, NebulaToolsApp,
};
use ab_glyph::{Font, PxScale, ScaleFont};
use eframe::egui;
use image::{DynamicImage, GenericImageView};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

const HIGH_PARTICLE_WARNING_THRESHOLD: usize = 100_000;

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

struct VideoProbeInfo {
    width: u32,
    height: u32,
    duration: f32,
}

struct ScreenPixel {
    idx: usize,
    px: f32,
    py: f32,
    pz: f32,
    id: i32,
    ox: f32,
    oy: f32,
    oz: f32,
}

struct VideoParticleGenerator {
    screen_pixels: Vec<ScreenPixel>,
    pex_ctx: crate::particleex::ExprContext,
    stmts: Option<Vec<crate::particleex::Stmt>>,
    brightness_threshold: f32,
    point_size: f32,
    target_fps: u16,
    frame_count: usize,
}

impl VideoParticleGenerator {
    fn new(
        width: u32,
        height: u32,
        target_fps: u16,
        density: f32,
        brightness_threshold: f32,
        particle_size: f32,
        point_size: f32,
        rotation: [f32; 3],
        velocity_expr: &str,
        start_frame: u32,
    ) -> Self {
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let copies_per_pixel = if density >= 1.0 {
            density.floor() as u32
        } else {
            1u32
        };

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut screen_pixels = Vec::new();
        let mut fixed_pid: i32 = 0;

        for y in 0..height {
            for x in 0..width {
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
                        ox: 0.0,
                        oy: 0.0,
                        oz: 0.0,
                    });
                    fixed_pid += 1;
                }
            }
        }

        Self {
            screen_pixels,
            pex_ctx: crate::particleex::ExprContext::new(),
            stmts: crate::particleex::compile_expr(velocity_expr),
            brightness_threshold,
            point_size,
            target_fps,
            frame_count: start_frame as usize,
        }
    }

    fn next_frame(&mut self, buffer: &[u8]) -> Vec<Particle> {
        let mut frame_particles = Vec::with_capacity(self.screen_pixels.len());
        let t = self.frame_count as f64 / self.target_fps as f64;

        for sp in &mut self.screen_pixels {
            let r = buffer[sp.idx];
            let g = buffer[sp.idx + 1];
            let b = buffer[sp.idx + 2];
            let luma = (r as f32 * 0.299 + g as f32 * 0.587 + b as f32 * 0.114) / 255.0;
            if luma < self.brightness_threshold {
                continue;
            }

            self.pex_ctx.set("t", crate::particleex::Value::Num(t));
            self.pex_ctx
                .set("x", crate::particleex::Value::Num((sp.px + sp.ox) as f64));
            self.pex_ctx
                .set("y", crate::particleex::Value::Num((sp.py + sp.oy) as f64));
            self.pex_ctx
                .set("z", crate::particleex::Value::Num((sp.pz + sp.oz) as f64));
            self.pex_ctx
                .set("cr", crate::particleex::Value::Num(r as f64 / 255.0));
            self.pex_ctx
                .set("cg", crate::particleex::Value::Num(g as f64 / 255.0));
            self.pex_ctx
                .set("cb", crate::particleex::Value::Num(b as f64 / 255.0));
            self.pex_ctx
                .set("alpha", crate::particleex::Value::Num(1.0));
            self.pex_ctx.set(
                "mpsize",
                crate::particleex::Value::Num(self.point_size as f64),
            );
            self.pex_ctx.set("vx", crate::particleex::Value::Num(0.0));
            self.pex_ctx.set("vy", crate::particleex::Value::Num(0.0));
            self.pex_ctx.set("vz", crate::particleex::Value::Num(0.0));
            self.pex_ctx
                .set("destroy", crate::particleex::Value::Num(0.0));
            self.pex_ctx
                .set("id", crate::particleex::Value::Num(sp.id as f64));

            if let Some(ref s) = self.stmts {
                crate::particleex::exec_stmts(s, &mut self.pex_ctx);
            }

            sp.ox += self.pex_ctx.get("vx").as_num() as f32;
            sp.oy += self.pex_ctx.get("vy").as_num() as f32;
            sp.oz += self.pex_ctx.get("vz").as_num() as f32;

            if self.pex_ctx.get("destroy").as_num() >= 1.0 {
                continue;
            }

            let final_r = (self.pex_ctx.get("cr").as_num().clamp(0.0, 1.0) * 255.0) as u8;
            let final_g = (self.pex_ctx.get("cg").as_num().clamp(0.0, 1.0) * 255.0) as u8;
            let final_b = (self.pex_ctx.get("cb").as_num().clamp(0.0, 1.0) * 255.0) as u8;
            let final_a = (self.pex_ctx.get("alpha").as_num().clamp(0.0, 1.0) * 255.0) as u8;
            let final_size = self.pex_ctx.get("mpsize").as_num() as f32;

            frame_particles.push(Particle {
                id: sp.id,
                pos: [sp.px + sp.ox, sp.py + sp.oy, sp.pz + sp.oz],
                color: [final_r, final_g, final_b, final_a],
                size: final_size,
                tex_id: 0,
                seq_index: 0,
            });
        }

        self.frame_count += 1;
        frame_particles
    }
}

fn probe_video_info(media_path: &str) -> anyhow::Result<VideoProbeInfo> {
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
            media_path,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run ffprobe: {}", e))?;

    if !probe.status.success() {
        let stderr = String::from_utf8_lossy(&probe.stderr);
        return Err(anyhow::anyhow!(
            "ffprobe failed with status {}: {}",
            probe.status,
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&probe.stdout);
    let parts: Vec<&str> = stdout.trim().split(',').collect();
    if parts.len() < 3 {
        return Err(anyhow::anyhow!(
            "ffprobe returned unexpected stream info: {}",
            stdout.trim()
        ));
    }

    let width = parts[0]
        .parse::<u32>()
        .map_err(|e| anyhow::anyhow!("Invalid ffprobe width '{}': {}", parts[0], e))?;
    let height = parts[1]
        .parse::<u32>()
        .map_err(|e| anyhow::anyhow!("Invalid ffprobe height '{}': {}", parts[1], e))?;
    let duration = parts[2]
        .parse::<f32>()
        .map_err(|e| anyhow::anyhow!("Invalid ffprobe duration '{}': {}", parts[2], e))?;

    Ok(VideoProbeInfo {
        width,
        height,
        duration,
    })
}

fn split_frame_ranges(total_frames: u32, chunk_count: usize) -> Vec<(u32, u32)> {
    if total_frames == 0 || chunk_count == 0 {
        return Vec::new();
    }

    let chunk_count = chunk_count.min(total_frames as usize).max(1);
    let base = total_frames / chunk_count as u32;
    let remainder = total_frames % chunk_count as u32;
    let mut start = 0u32;
    let mut ranges = Vec::with_capacity(chunk_count);

    for idx in 0..chunk_count {
        let len = base + if idx < remainder as usize { 1 } else { 0 };
        let end = start + len;
        ranges.push((start, end));
        start = end;
    }

    ranges
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
                            ui.separator();
                            Self::show_texture_animation_editor(
                                ui,
                                self.i18n.tr("pex_texture_animation"),
                                self.i18n.tr("pex_texture_interval"),
                                self.i18n.tr("pex_texture_sequence"),
                                self.i18n.tr("pex_add_texture"),
                                self.i18n.tr("pex_reset_default_textures"),
                                &mut self.multimedia.texture_animation.textures,
                                &mut self.multimedia.texture_animation.texture_interval,
                            );
                        });

                        ui.add_space(8.0);
                        match self.multimedia.mode {
                            2 if self.multimedia.last_source_size.is_none() => {
                                ui.label(format!(
                                    "{}: {}",
                                    self.i18n.tr("estimated_count"),
                                    self.i18n.tr("select_video_for_estimate")
                                ));
                            }
                            _ => {
                                let est_count = self.estimate_multimedia_particles();
                                ui.label(format!(
                                    "{}: {}",
                                    self.i18n.tr("estimated_count"),
                                    est_count
                                ));
                                if est_count >= HIGH_PARTICLE_WARNING_THRESHOLD {
                                    ui.colored_label(
                                        egui::Color32::from_rgb(255, 80, 80),
                                        self.i18n.tr("high_particle_count_warning"),
                                    );
                                }
                            }
                        }
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
                if let Some((
                    ref progress_arc,
                    ref status_arc,
                    ref done_arc,
                    ref frames_arc,
                    ref thread_arc,
                )) = self.multimedia.video_compile_shared
                {
                    if let Ok(pct) = progress_arc.lock() {
                        self.multimedia.processing_progress = Some(*pct);
                    }
                    if let Ok(thread_progress) = thread_arc.lock() {
                        self.multimedia.thread_progress = thread_progress.clone();
                    }
                    let is_done = done_arc.lock().map(|d| *d).unwrap_or(false);
                    if is_done {
                        self.multimedia.is_processing = false;
                        self.multimedia.processing_progress = None;
                        if let Ok(status) = status_arc.lock() {
                            self.multimedia.status_msg = status.clone();
                        }
                        let compiled_frames = frames_arc
                            .lock()
                            .ok()
                            .and_then(|mut frames_lock| frames_lock.take());
                        if let Some(mut frames) = compiled_frames {
                            self.apply_texture_animation_to_frames(
                                &mut frames,
                                &self.multimedia.texture_animation.textures,
                                self.multimedia.texture_animation.texture_interval,
                            );
                            let saved_path = self.multimedia.preview_output_path.clone();
                            self.finalize_multimedia_preview_from_frames(
                                frames,
                                saved_path.as_deref().map(std::path::Path::new),
                            );
                        } else if let Some(saved_path) = self.multimedia.preview_output_path.clone() {
                            match self.load_preview_frames_from_nbl(std::path::Path::new(&saved_path)) {
                                Ok(frames) => {
                                    self.finalize_multimedia_preview_from_frames(
                                        frames,
                                        Some(std::path::Path::new(&saved_path)),
                                    );
                                }
                                Err(e) => {
                                    self.multimedia.status_msg = Some(format!(
                                        "{} {}",
                                        self.i18n.tr("multimedia_preview_load_failed"),
                                        e
                                    ));
                                }
                            }
                        }
                        self.multimedia.thread_progress.clear();
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
                        if !self.multimedia.thread_progress.is_empty() {
                            ui.add_space(12.0);
                            ui.group(|ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("export_thread_status"))
                                        .strong(),
                                );
                                for (idx, thread) in
                                    self.multimedia.thread_progress.iter().enumerate()
                                {
                                    let total =
                                        thread.end_frame.saturating_sub(thread.start_frame).max(1);
                                    let done = thread
                                        .current_frame
                                        .saturating_sub(thread.start_frame)
                                        .min(total);
                                    let pct = done as f32 / total as f32;
                                    ui.label(format!(
                                        "{} {} [{}..{}) - {}",
                                        self.i18n.tr("export_thread"),
                                        idx + 1,
                                        thread.start_frame,
                                        thread.end_frame,
                                        self.describe_thread_status(thread.status)
                                    ));
                                    ui.add(
                                        egui::ProgressBar::new(pct)
                                            .desired_width(280.0)
                                            .text(format!("{}/{}", done, total)),
                                    );
                                }
                            });
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
        let total = (w * h) as f32;
        let factor = if density < 1.0 {
            density
        } else {
            density.floor()
        };
        (total * factor) as usize
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
                    self.multimedia.last_source_size = None;
                    match probe_video_info(&path.to_string_lossy()) {
                        Ok(info) => {
                            self.multimedia.last_source_size = Some([info.width, info.height]);
                        }
                        Err(e) => {
                            self.multimedia.status_msg = Some(format!("Video probe failed: {}", e));
                        }
                    }
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
            ui.label(self.i18n.tr("export_threads"));
            ui.add(
                egui::DragValue::new(&mut self.multimedia.export_threads)
                    .clamp_range(1..=64)
                    .speed(1.0),
            );
        });
        ui.small(self.i18n.tr("export_threads_hint"));
    }

    fn describe_thread_status(&self, status: MultimediaThreadStatus) -> &'static str {
        match status {
            MultimediaThreadStatus::Waiting => self.i18n.tr("export_status_waiting"),
            MultimediaThreadStatus::Decoding => self.i18n.tr("export_status_decoding"),
            MultimediaThreadStatus::Generating => self.i18n.tr("export_status_generating"),
            MultimediaThreadStatus::Encoded => self.i18n.tr("export_status_encoded"),
            MultimediaThreadStatus::Merging => self.i18n.tr("export_status_merging"),
            MultimediaThreadStatus::Done => self.i18n.tr("export_status_done"),
        }
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

    fn load_preview_frames_from_nbl(
        &mut self,
        path: &std::path::Path,
    ) -> anyhow::Result<Vec<Vec<Particle>>> {
        let mut player = crate::player::PlayerState::default();
        player.load_file(path.to_path_buf())?;
        let total_frames = player
            .header
            .as_ref()
            .map(|header| header.total_frames)
            .unwrap_or(0);
        let mut frames = Vec::with_capacity(total_frames as usize);
        for frame_idx in 0..total_frames {
            player.seek_to(frame_idx)?;
            let mut frame_particles: Vec<Particle> = player.particles.values().cloned().collect();
            frame_particles.sort_unstable_by_key(|particle| particle.id);
            frames.push(frame_particles);
        }
        Ok(frames)
    }

    fn save_preview_frames_to_nbl(
        &mut self,
        path: &std::path::Path,
        frames: &[Vec<Particle>],
    ) -> anyhow::Result<Vec<Vec<Particle>>> {
        let (bbox_min, bbox_max) = crate::player::recalculate_bbox(frames);
        let textures = build_texture_entries(&self.multimedia.texture_animation.textures);
        let header = NblHeader {
            version: 1,
            target_fps: self.multimedia.target_fps,
            total_frames: frames.len() as u32,
            texture_count: textures.len() as u16,
            attributes: 0x03,
            bbox_min,
            bbox_max,
        };
        let path_buf = path.to_path_buf();
        self.player.save_file(&path_buf, &header, &textures, frames)?;
        self.load_preview_frames_from_nbl(path)
    }

    fn choose_multimedia_preview_path(&mut self) -> Option<std::path::PathBuf> {
        let mut dialog = rfd::FileDialog::new().add_filter("Nebula", &["nbl"]);
        if let Some(existing) = &self.multimedia.preview_output_path {
            dialog = dialog.set_file_name(
                std::path::Path::new(existing)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("multimedia_preview.nbl"),
            );
        } else {
            dialog = dialog.set_file_name("multimedia_preview.nbl");
        }
        let path = dialog.save_file()?;
        self.multimedia.preview_output_path = Some(path.to_string_lossy().to_string());
        Some(path)
    }

    fn finalize_multimedia_preview_from_frames(
        &mut self,
        frames: Vec<Vec<Particle>>,
        saved_path: Option<&std::path::Path>,
    ) {
        if let Some(path) = saved_path {
            self.multimedia.preview_output_path = Some(path.to_string_lossy().to_string());
        }
        self.multimedia.preview_frames = Some(frames);
        self.multimedia.status_msg = Some(format!(
            "{}: {}",
            self.i18n.tr("multimedia_preview_ready"),
            self.multimedia
                .preview_output_path
                .as_deref()
                .unwrap_or(self.i18n.tr("multimedia_preview_no_path"))
        ));
        self.multimedia.preview_playing = true;
        self.multimedia.preview_frame_idx = 0;
        self.multimedia.preview_timer = 0.0;
    }

    fn compile_multimedia_preview(&mut self, ctx: &egui::Context, source_only: bool) {
        self.multimedia.status_msg = Some(
            if source_only {
                self.i18n.tr("multimedia_refreshing_source")
            } else {
                self.i18n.tr("multimedia_compiling_preview")
            }
            .to_string(),
        );

        let mode = self.multimedia.mode;
        self.multimedia.source_image_preview = None;

        let preview_path = if source_only {
            None
        } else {
            match self.choose_multimedia_preview_path() {
                Some(path) => Some(path),
                None => {
                    self.multimedia.status_msg = Some(
                        self.i18n.tr("multimedia_preview_save_cancelled").to_string(),
                    );
                    return;
                }
            }
        };

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
                    self.multimedia.status_msg =
                        Some(self.i18n.tr("multimedia_source_preview_updated").to_string());
                    return;
                }

                if let Some(preview_path) = preview_path.as_deref() {
                    self.compile_video_preview_via_nbl(ctx, preview_path.to_path_buf());
                }
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
                self.multimedia.status_msg =
                    Some(self.i18n.tr("multimedia_source_preview_updated").to_string());
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

            let copies_per_pixel = if density >= 1.0 {
                density.floor() as u32
            } else {
                1u32
            };
            use rand::Rng;
            let mut rng = rand::thread_rng();

            for y in 0..height {
                for x in 0..width {
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

            if let Some(preview_path) = preview_path.as_deref() {
                match self.save_preview_frames_to_nbl(preview_path, &frames) {
                    Ok(preview_frames) => {
                        self.finalize_multimedia_preview_from_frames(
                            preview_frames,
                            Some(preview_path),
                        );
                    }
                    Err(e) => {
                        self.multimedia.status_msg = Some(format!(
                            "{} {}",
                            self.i18n.tr("multimedia_preview_nbl_failed"),
                            e
                        ));
                    }
                }
            }
        }
    }

    fn compile_video_preview_via_nbl(
        &mut self,
        ctx: &egui::Context,
        path: std::path::PathBuf,
    ) {
        self.export_video_nbl_streaming(ctx, path);
    }

    fn export_video_nbl_streaming(&mut self, ctx: &egui::Context, path: std::path::PathBuf) {
        let media_path = match &self.multimedia.media_path {
            Some(p) => p.clone(),
            None => {
                self.multimedia.status_msg = Some("No Video selected".into());
                return;
            }
        };

        self.multimedia.is_processing = true;
        self.multimedia.status_msg = Some("Exporting video to NBL...".into());
        self.multimedia.processing_progress = Some(0.0);
        self.multimedia.thread_progress.clear();

        let target_fps = self.multimedia.target_fps;
        let density = self.multimedia.density.max(0.000001);
        let brightness_threshold = self.multimedia.brightness_threshold;
        let particle_size = self.multimedia.particle_size;
        let point_size = self.multimedia.point_size;
        let rotation = self.multimedia.rotation;
        let velocity_expr = self.multimedia.velocity_expr.clone();
        let export_threads = self.multimedia.export_threads.max(1);

        let shared_progress = Arc::new(Mutex::new(0.0f32));
        let shared_status = Arc::new(Mutex::new(None::<String>));
        let shared_done = Arc::new(Mutex::new(false));
        let shared_frames = Arc::new(Mutex::new(None::<Vec<Vec<Particle>>>));
        let shared_threads = Arc::new(Mutex::new(Vec::<MultimediaThreadProgress>::new()));

        let status_clone = shared_status.clone();
        let done_clone = shared_done.clone();

        self.multimedia.video_compile_shared = Some((
            shared_progress.clone(),
            shared_status,
            shared_done,
            shared_frames,
            shared_threads.clone(),
        ));

        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            let probe = match probe_video_info(&media_path) {
                Ok(info) => info,
                Err(e) => {
                    *status_clone.lock().unwrap() = Some(format!("Video probe failed: {}", e));
                    *done_clone.lock().unwrap() = true;
                    ctx_clone.request_repaint();
                    return;
                }
            };
            let total_frames = (probe.duration * target_fps as f32).ceil().max(1.0) as u32;
            let frame_size = (probe.width * probe.height * 3) as usize;
            let ranges = split_frame_ranges(total_frames, export_threads);
            let keyframe_interval = target_fps.max(1) as u32;

            if let Ok(mut threads) = shared_threads.lock() {
                *threads = ranges
                    .iter()
                    .map(|(start, end)| MultimediaThreadProgress {
                        start_frame: *start,
                        end_frame: *end,
                        current_frame: *start,
                        status: MultimediaThreadStatus::Waiting,
                    })
                    .collect();
            }
            ctx_clone.request_repaint();

            let mut worker_handles = Vec::new();
            for (worker_idx, (start_frame, end_frame)) in ranges.iter().copied().enumerate() {
                let media_path = media_path.clone();
                let velocity_expr = velocity_expr.clone();
                let thread_progress = shared_threads.clone();
                let shared_progress_worker = shared_progress.clone();
                let ctx_worker = ctx_clone.clone();

                worker_handles.push(std::thread::spawn(
                    move || -> anyhow::Result<crate::player::ExportChunkResult> {
                        if let Ok(mut progress) = thread_progress.lock() {
                            if let Some(entry) = progress.get_mut(worker_idx) {
                                entry.status = MultimediaThreadStatus::Decoding;
                            }
                        }

                        let child = Command::new("ffmpeg")
                            .args([
                                "-ss",
                                &format!("{:.6}", start_frame as f64 / target_fps as f64),
                                "-i",
                                &media_path,
                                "-frames:v",
                                &(end_frame - start_frame).to_string(),
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
                            .spawn()?;

                        let mut child = child;
                        let mut stdout = child.stdout.take().expect("Failed to open stdout");
                        let mut buffer = vec![0u8; frame_size];
                        let mut generator = VideoParticleGenerator::new(
                            probe.width,
                            probe.height,
                            target_fps,
                            density,
                            brightness_threshold,
                            particle_size,
                            point_size,
                            rotation,
                            &velocity_expr,
                            start_frame,
                        );
                        let player = crate::player::PlayerState::default();

                        let mut provider = |frame_idx: u32| -> anyhow::Result<Vec<Particle>> {
                            if let Ok(mut progress) = thread_progress.lock() {
                                if let Some(entry) = progress.get_mut(worker_idx) {
                                    entry.status = MultimediaThreadStatus::Generating;
                                    entry.current_frame = frame_idx;
                                }
                            }
                            let completed_frames = if let Ok(progress) = thread_progress.lock() {
                                progress
                                    .iter()
                                    .map(|entry| {
                                        entry
                                            .current_frame
                                            .saturating_sub(entry.start_frame)
                                            .min(entry.end_frame.saturating_sub(entry.start_frame))
                                    })
                                    .sum::<u32>()
                            } else {
                                0
                            };
                            if let Ok(mut pct) = shared_progress_worker.lock() {
                                *pct = (completed_frames as f32 / total_frames as f32).min(1.0);
                            }
                            ctx_worker.request_repaint();
                            if stdout.read_exact(&mut buffer).is_err() {
                                return Err(anyhow::anyhow!(
                                    "Failed: video ended before frame {} could be decoded.",
                                    frame_idx
                                ));
                            }
                            Ok(generator.next_frame(&buffer))
                        };

                        let chunk = player.build_export_chunk(
                            start_frame,
                            end_frame,
                            start_frame,
                            keyframe_interval,
                            &mut provider,
                        )?;

                        let wait_status = child.wait()?;
                        if !wait_status.success() {
                            return Err(anyhow::anyhow!(
                                "ffmpeg chunk worker exited with status {} for frames [{}..{})",
                                wait_status,
                                start_frame,
                                end_frame
                            ));
                        }
                        if let Ok(mut progress) = thread_progress.lock() {
                            if let Some(entry) = progress.get_mut(worker_idx) {
                                entry.current_frame = end_frame;
                                entry.status = MultimediaThreadStatus::Encoded;
                            }
                        }
                        let completed_frames = if let Ok(progress) = thread_progress.lock() {
                            progress
                                .iter()
                                .map(|entry| {
                                    entry
                                        .current_frame
                                        .saturating_sub(entry.start_frame)
                                        .min(entry.end_frame.saturating_sub(entry.start_frame))
                                })
                                .sum::<u32>()
                        } else {
                            0
                        };
                        if let Ok(mut pct) = shared_progress_worker.lock() {
                            *pct = (completed_frames as f32 / total_frames as f32).min(1.0);
                        }
                        ctx_worker.request_repaint();
                        Ok(chunk)
                    },
                ));
            }

            let mut chunks = Vec::new();
            for handle in worker_handles {
                match handle.join() {
                    Ok(Ok(chunk)) => {
                        chunks.push(chunk);
                        let completed_frames: u32 =
                            chunks.iter().map(|c| c.end_frame - c.start_frame).sum();
                        if let Ok(mut pct) = shared_progress.lock() {
                            *pct = (completed_frames as f32 / total_frames as f32).min(1.0);
                        }
                        ctx_clone.request_repaint();
                    }
                    Ok(Err(e)) => {
                        *status_clone.lock().unwrap() = Some(format!("Export failed: {}", e));
                        *done_clone.lock().unwrap() = true;
                        ctx_clone.request_repaint();
                        return;
                    }
                    Err(_) => {
                        *status_clone.lock().unwrap() =
                            Some("Export failed: worker thread panicked".into());
                        *done_clone.lock().unwrap() = true;
                        ctx_clone.request_repaint();
                        return;
                    }
                }
            }

            if let Ok(mut progress) = shared_threads.lock() {
                for entry in progress.iter_mut() {
                    entry.status = MultimediaThreadStatus::Merging;
                    entry.current_frame = entry.end_frame;
                }
            }

            let header = NblHeader {
                version: 1,
                target_fps,
                total_frames,
                texture_count: 0,
                attributes: 0x03,
                bbox_min: [0.0; 3],
                bbox_max: [0.0; 3],
            };
            let player = crate::player::PlayerState::default();
            let save_result = player.write_chunked_nbl(&path, &header, &[], total_frames, chunks);

            match save_result {
                Ok(_) => {
                    *shared_progress.lock().unwrap() = 1.0;
                    if let Ok(mut progress) = shared_threads.lock() {
                        for entry in progress.iter_mut() {
                            entry.status = MultimediaThreadStatus::Done;
                        }
                    }
                    *status_clone.lock().unwrap() = Some(format!(
                        "Export success! {} frames with {} threads.",
                        total_frames, export_threads
                    ));
                }
                Err(e) => {
                    *status_clone.lock().unwrap() = Some(format!("Export failed: {}", e));
                }
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
