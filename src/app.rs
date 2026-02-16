use crate::i18n::{I18nManager, Language};
use crate::player::{self, NblHeader, Particle, PlayerState};
use crate::renderer::ParticleRenderer;
use eframe::{
    egui, egui_glow,
    glow::{self, HasContext},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{Arc, Mutex};

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum AppMode {
    Edit,
    Preview,
}

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub lang: Language,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            lang: Language::ChineseSimplified,
        }
    }
}

pub struct CameraState {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: [f32; 3],
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            yaw: std::f32::consts::FRAC_PI_4,
            pitch: 0.5,
            distance: 20.0,
            target: [0.0, 0.0, 0.0],
        }
    }
}

pub struct EditState {
    pub new_fps: u16,
    pub speed_factor: f32,
    pub speed_mode: u8, // 0=FPS only, 1=Interp, 2=Both
    pub size_mode: u8,  // 0=Scale, 1=Uniform
    pub size_scale: f32,
    pub size_uniform: f32,
    pub brightness: f32,
    pub opacity: f32,
    pub translate: [f32; 3],
    pub pos_scale: f32,
    pub trim_start: u32,
    pub trim_end: u32,
    pub status_msg: Option<String>,
    pub decoded_frames: Option<Vec<Vec<Particle>>>,
    pub edited_header: Option<NblHeader>,
}

impl Default for EditState {
    fn default() -> Self {
        Self {
            new_fps: 30,
            speed_factor: 1.0,
            speed_mode: 0,
            size_mode: 0,
            size_scale: 1.0,
            size_uniform: 0.5,
            brightness: 1.0,
            opacity: 1.0,
            translate: [0.0; 3],
            pos_scale: 1.0,
            trim_start: 0,
            trim_end: 0,
            status_msg: None,
            decoded_frames: None,
            edited_header: None,
        }
    }
}

pub struct NebulaToolsApp {
    pub player: PlayerState,
    pub config: AppConfig,
    pub i18n: I18nManager,
    pub error_msg: Option<String>,
    pub camera: CameraState,
    pub renderer: Arc<Mutex<Option<ParticleRenderer>>>,
    pub show_grid: bool,
    pub mode: AppMode,
    pub scrub_frame: Option<u32>,
    pub fps_counter: f32,
    pub fps_display: f32,
    pub fps_timer: f32,
    pub edit: EditState,
}

impl NebulaToolsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let _ = egui_chinese_font::setup_chinese_fonts(&cc.egui_ctx);

        let config = match fs::read_to_string("config.json") {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        };

        let i18n = I18nManager::new(config.lang);

        Self {
            player: PlayerState::default(),
            config,
            i18n,
            error_msg: None,
            camera: CameraState::default(),
            renderer: Arc::new(Mutex::new(None)),
            show_grid: true,
            mode: AppMode::Preview,
            scrub_frame: None,
            fps_counter: 0.0,
            fps_display: 0.0,
            fps_timer: 0.0,
            edit: EditState::default(),
        }
    }

    fn save_config(&self) {
        if let Ok(s) = serde_json::to_string(&self.config) {
            let _ = fs::write("config.json", s);
        }
    }

    fn update_lang(&mut self, lang: Language) {
        self.config.lang = lang;
        self.i18n.active_lang = lang;
        self.save_config();
    }

    fn prepare_render_data(&self) -> Vec<f32> {
        let count = self.player.particles.len();
        let mut data = Vec::with_capacity(count * 8);
        for p in self.player.particles.values() {
            data.push(p.pos[0]);
            data.push(p.pos[1]);
            data.push(p.pos[2]);
            data.push(p.color[0] as f32 / 255.0);
            data.push(p.color[1] as f32 / 255.0);
            data.push(p.color[2] as f32 / 255.0);
            data.push(p.color[3] as f32 / 255.0);
            data.push(p.size);
        }
        data
    }

    fn handle_import(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .pick_file()
        {
            match self.player.load_file(path) {
                Ok(_) => {
                    self.error_msg = None;
                }
                Err(e) => self.error_msg = Some(format!("Load Failed: {}", e)),
            }
        }
    }
}

impl eframe::App for NebulaToolsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.player.header.is_none() {
            self.show_welcome_screen(ctx);
            return;
        }

        // --- Logic: Playback ---
        if self.mode == AppMode::Preview && self.player.is_playing {
            if let Some(header) = &self.player.header {
                let dt = ctx.input(|i| i.stable_dt);
                self.player.frame_timer += dt;
                let frame_dur = 1.0 / header.target_fps as f32;
                if self.player.frame_timer >= frame_dur {
                    self.player.frame_timer -= frame_dur;
                    let next_frame = self.player.current_frame_idx + 1;
                    if (next_frame as u32) < header.total_frames {
                        let _ = self.player.seek_to(next_frame as u32);
                    } else {
                        self.player.is_playing = false;
                    }
                }
                ctx.request_repaint();
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(self.i18n.tr("file"), |ui| {
                    if ui.button(self.i18n.tr("import")).clicked() {
                        self.handle_import();
                        ui.close_menu();
                    }
                });

                egui::ComboBox::from_id_source("top_lang_combo")
                    .selected_text(self.config.lang.display_name())
                    .show_ui(ui, |ui| {
                        let current_lang = self.config.lang;
                        if ui
                            .selectable_label(
                                current_lang == Language::ChineseSimplified,
                                Language::ChineseSimplified.display_name(),
                            )
                            .clicked()
                        {
                            self.update_lang(Language::ChineseSimplified);
                        }
                        if ui
                            .selectable_label(
                                current_lang == Language::English,
                                Language::English.display_name(),
                            )
                            .clicked()
                        {
                            self.update_lang(Language::English);
                        }
                    });

                ui.separator();
                ui.selectable_value(&mut self.mode, AppMode::Edit, self.i18n.tr("edit_mode"));
                ui.selectable_value(
                    &mut self.mode,
                    AppMode::Preview,
                    self.i18n.tr("preview_mode"),
                );
            });
        });

        if self.mode == AppMode::Preview {
            self.show_preview_workflow(ctx);
        } else {
            self.show_edit_workflow(ctx);
        }
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            if let Some(renderer) = self.renderer.lock().unwrap().take() {
                renderer.destroy(gl);
            }
        }
    }
}

impl NebulaToolsApp {
    fn show_welcome_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                egui::ComboBox::from_id_source("welcome_lang")
                    .selected_text(self.config.lang.display_name())
                    .show_ui(ui, |ui| {
                        let current_lang = self.config.lang;
                        if ui
                            .selectable_label(
                                current_lang == Language::ChineseSimplified,
                                Language::ChineseSimplified.display_name(),
                            )
                            .clicked()
                        {
                            self.update_lang(Language::ChineseSimplified);
                        }
                        if ui
                            .selectable_label(
                                current_lang == Language::English,
                                Language::English.display_name(),
                            )
                            .clicked()
                        {
                            self.update_lang(Language::English);
                        }
                    });
            });

            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.2);
                ui.heading(
                    egui::RichText::new(format!(
                        "{} NebulaTools v{}",
                        self.i18n.tr("welcome"),
                        env!("CARGO_PKG_VERSION")
                    ))
                    .size(40.0)
                    .strong(),
                );
                ui.add_space(40.0);
                ui.horizontal(|ui| {
                    let total_width = 420.0;
                    ui.add_space((ui.available_width() - total_width) / 2.0);
                    let btn_size = egui::vec2(200.0, 60.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(
                                egui::RichText::new(format!(
                                    "📂 {}",
                                    self.i18n.tr("open_existing")
                                ))
                                .size(20.0),
                            )
                            .rounding(8.0),
                        )
                        .clicked()
                    {
                        self.handle_import();
                    }
                    ui.add_space(20.0);
                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(
                                egui::RichText::new(format!("✨ {}", self.i18n.tr("create_new")))
                                    .size(20.0),
                            )
                            .rounding(8.0),
                        )
                        .clicked()
                    {}
                });
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(20.0);
                    ui.colored_label(
                        egui::Color32::GRAY,
                        format!("{}: Atemukesu", self.i18n.tr("author")),
                    );
                });
            });
        });
    }

    fn show_preview_workflow(&mut self, ctx: &egui::Context) {
        // --- Side Panel: Left ---
        egui::SidePanel::left("metadata_side")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.heading(self.i18n.tr("metadata"));
                ui.separator();

                if let Some(header) = &self.player.header {
                    ui.vertical_centered_justified(|ui: &mut egui::Ui| {
                        egui::Grid::new("meta_grid")
                            .num_columns(2)
                            .spacing([12.0, 6.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(self.i18n.tr("version"));
                                ui.label(
                                    egui::RichText::new(format!("v{}", header.version)).strong(),
                                );
                                ui.end_row();

                                ui.label(self.i18n.tr("fps"));
                                ui.label(
                                    egui::RichText::new(format!("{}", header.target_fps)).strong(),
                                );
                                ui.end_row();

                                ui.label(self.i18n.tr("total_frames"));
                                ui.label(
                                    egui::RichText::new(format!("{}", header.total_frames))
                                        .strong(),
                                );
                                ui.end_row();

                                if header.target_fps > 0 {
                                    ui.label(self.i18n.tr("duration"));
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:.2} s",
                                            header.total_frames as f32 / header.target_fps as f32
                                        ))
                                        .strong(),
                                    );
                                    ui.end_row();
                                }

                                ui.label(self.i18n.tr("keyframe_count"));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}",
                                        self.player.keyframe_indices.len()
                                    ))
                                    .strong(),
                                );
                                ui.end_row();

                                ui.label(self.i18n.tr("textures"));
                                ui.label(
                                    egui::RichText::new(format!("{}", header.texture_count))
                                        .strong(),
                                );
                                ui.end_row();
                            });
                    });

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(self.i18n.tr("attributes")).strong());
                    ui.horizontal(|ui| {
                        let has_alpha = (header.attributes & 0x01) != 0;
                        let has_size = (header.attributes & 0x02) != 0;

                        ui.set_enabled(false);
                        ui.checkbox(&mut has_alpha.clone(), self.i18n.tr("has_alpha"));
                        ui.checkbox(&mut has_size.clone(), self.i18n.tr("has_size"));
                    });

                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(self.i18n.tr("bbox")).strong());
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Min:");
                                ui.label(format!(
                                    "{:.2}, {:.2}, {:.2}",
                                    header.bbox_min[0], header.bbox_min[1], header.bbox_min[2]
                                ));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Max:");
                                ui.label(format!(
                                    "{:.2}, {:.2}, {:.2}",
                                    header.bbox_max[0], header.bbox_max[1], header.bbox_max[2]
                                ));
                            });
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();

                    egui::CollapsingHeader::new(self.i18n.tr("texture_list"))
                        .default_open(false)
                        .show(ui, |ui| {
                            for (i, tex) in self.player.textures.iter().enumerate() {
                                ui.label(format!("{}: {}", i, tex.path));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  Rows: {}, Cols: {}",
                                        tex.rows, tex.cols
                                    ))
                                    .weak(),
                                );
                            }
                        });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.label(format!(
                        "{}: {}",
                        self.i18n.tr("particle_count"),
                        self.player.particles.len()
                    ));
                }

                if let Some(err) = &self.error_msg {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, err);
                }
            });

        // --- Bottom Panel: New Slider Logic ---
        egui::TopBottomPanel::bottom("playback_strip")
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    // 1. Playback buttons (Fixed width)
                    let play_label = if self.player.is_playing {
                        self.i18n.tr("pause")
                    } else {
                        self.i18n.tr("play")
                    };
                    if ui.button(play_label).clicked() {
                        self.player.is_playing = !self.player.is_playing;
                    }
                    if ui.button(self.i18n.tr("stop")).clicked() {
                        self.player.is_playing = false;
                        let _ = self.player.seek_to(0);
                    }
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if let Some(header) = &self.player.header {
                        let max_frame = header.total_frames.saturating_sub(1);

                        // 使用 scrub_frame 代理视觉上的当前帧，解决拖拽冲突
                        let mut visual_frame = self
                            .scrub_frame
                            .unwrap_or(self.player.current_frame_idx.max(0) as u32);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut self.show_grid, "Grid");
                            ui.add_space(8.0);
                            ui.label(format!("/ {}", max_frame));

                            // 1. 当前帧 DragValue
                            let drag_res = ui.add(
                                egui::DragValue::new(&mut visual_frame)
                                    .clamp_range(0..=max_frame)
                                    .speed(1.0),
                            );

                            // 2. 进度条
                            ui.add_space(8.0);
                            let slider_width = ui.available_width() - 8.0;
                            let slider_res = ui.add_sized(
                                [slider_width, ui.spacing().interact_size.y],
                                egui::Slider::new(&mut visual_frame, 0..=max_frame)
                                    .show_value(false),
                            );

                            // --- 状态检测 ---
                            let is_scrubbing = slider_res.dragged() || drag_res.dragged();
                            let stop_scrubbing = slider_res.drag_stopped()
                                || drag_res.drag_stopped()
                                || drag_res.lost_focus();

                            if is_scrubbing {
                                self.player.is_playing = false;
                                self.scrub_frame = Some(visual_frame);

                                // 连续帧微调优化：P帧更新很快，保持实时性
                                if visual_frame == (self.player.current_frame_idx + 1) as u32 {
                                    let _ = self.player.seek_to(visual_frame);
                                }
                            }

                            if stop_scrubbing {
                                let _ = self.player.seek_to(visual_frame);
                                self.scrub_frame = None;
                            }
                        });
                    }
                });
                ui.add_space(6.0);
            });

        // --- Central Panel ---
        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            if response.dragged_by(egui::PointerButton::Primary) {
                let d = response.drag_delta();
                self.camera.yaw -= d.x * 0.01;
                self.camera.pitch += d.y * 0.01;
                self.camera.pitch = self.camera.pitch.clamp(-1.5, 1.5);
            }
            if response.hovered() {
                let s = ctx.input(|i| i.smooth_scroll_delta.y);
                self.camera.distance = (self.camera.distance - s * 0.1).clamp(1.0, 1000.0);
            }
            let particles_data = self.prepare_render_data();
            let aspect = rect.width() / rect.height();
            let mvp = self.calculate_mvp(aspect);
            let renderer_ref = self.renderer.clone();
            let show_grid = self.show_grid;
            let rect_height = rect.height();

            // Calculate mathematically correct scaling factor for point size
            // PointSize (pixels) = (SizeInBlocks * FocalLength * ViewportHeightPixels) / (2.0 * Distance)
            let fov_y = 45.0f32.to_radians();
            let focal_length = 1.0 / (fov_y / 2.0).tan();

            let callback = egui_glow::CallbackFn::new(move |info, painter| {
                let mut lock = renderer_ref.lock().unwrap();
                if lock.is_none() {
                    *lock = Some(ParticleRenderer::new(painter.gl()));
                }
                if let Some(r) = lock.as_ref() {
                    let physical_height = rect_height * info.pixels_per_point;
                    let scaling = (focal_length * physical_height) / 2.0;

                    unsafe {
                        painter.gl().clear_color(0.0, 0.0, 0.0, 1.0);
                        r.paint(painter.gl(), mvp, &particles_data, scaling, show_grid);
                    }
                }
            });
            ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
            ui.painter().add(egui::PaintCallback {
                rect,
                callback: Arc::new(callback),
            });

            // --- Overlay Logic ---
            let dt = ctx.input(|i| i.stable_dt);
            if dt > 0.0 {
                self.fps_counter = self.fps_counter * 0.9 + (1.0 / dt) * 0.1;
                self.fps_timer += dt;
                if self.fps_timer >= 0.25 {
                    self.fps_display = self.fps_counter;
                    self.fps_timer = 0.0;
                }
            }

            let _response = ui.allocate_rect(rect, egui::Sense::hover());
            let painter = ui.painter_at(rect);

            let overlay_pos = rect.left_top() + egui::vec2(10.0, 10.0);
            let mut info_text =
                format!("{}: {:.1}\n", self.i18n.tr("render_fps"), self.fps_display);
            info_text.push_str(&format!(
                "{}: {}",
                self.i18n.tr("particle_count"),
                self.player.particles.len()
            ));

            painter.text(
                overlay_pos,
                egui::Align2::LEFT_TOP,
                info_text,
                egui::FontId::proportional(16.0),
                egui::Color32::from_white_alpha(180),
            );
        });
    }

    fn show_edit_workflow(&mut self, ctx: &egui::Context) {
        // Ensure decoded frame data is ready when entering edit mode
        if self.edit.decoded_frames.is_none() && self.player.header.is_some() {
            match self.player.decode_all_frames() {
                Ok(frames) => {
                    let header = self.player.header.clone().unwrap();
                    self.edit.trim_end = header.total_frames.saturating_sub(1);
                    self.edit.new_fps = header.target_fps;
                    self.edit.edited_header = Some(header);
                    self.edit.decoded_frames = Some(frames);
                    self.edit.status_msg = None;
                }
                Err(e) => {
                    self.edit.status_msg = Some(format!("Decode failed: {}", e));
                }
            }
        }

        egui::SidePanel::left("edit_side")
            .resizable(true)
            .default_width(360.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    ui.heading(self.i18n.tr("edit_tools"));
                    ui.separator();

                    if self.player.header.is_none() {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(self.i18n.tr("no_file_loaded")).italics());
                        return;
                    }

                    // ===== 1. Animation Speed =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_anim_speed"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(self.i18n.tr("edit_anim_speed_desc")).weak());
                        ui.add_space(6.0);

                        ui.radio_value(
                            &mut self.edit.speed_mode,
                            0,
                            self.i18n.tr("speed_mode_fps_only"),
                        );
                        if self.edit.speed_mode == 0 {
                            ui.indent("fps_only_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("speed_mode_fps_only_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("new_fps"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.new_fps)
                                            .clamp_range(1..=240)
                                            .speed(1.0),
                                    );
                                });
                            });
                        }

                        ui.add_space(4.0);
                        ui.radio_value(
                            &mut self.edit.speed_mode,
                            1,
                            self.i18n.tr("speed_mode_interp"),
                        );
                        if self.edit.speed_mode == 1 {
                            ui.indent("interp_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("speed_mode_interp_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("speed_factor"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.speed_factor)
                                            .clamp_range(0.1..=10.0)
                                            .speed(0.05)
                                            .fixed_decimals(2),
                                    );
                                });
                                if let Some(ref frames) = self.edit.decoded_frames {
                                    let new_count = ((frames.len() as f32) / self.edit.speed_factor)
                                        .round()
                                        as usize;
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "  {} → {} {}",
                                            frames.len(),
                                            new_count,
                                            self.i18n.tr("frame")
                                        ))
                                        .weak(),
                                    );
                                }
                            });
                        }

                        ui.add_space(4.0);
                        ui.radio_value(
                            &mut self.edit.speed_mode,
                            2,
                            self.i18n.tr("speed_mode_both"),
                        );
                        if self.edit.speed_mode == 2 {
                            ui.indent("both_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("speed_mode_both_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("new_fps"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.new_fps)
                                            .clamp_range(1..=240)
                                            .speed(1.0),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("speed_factor"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.speed_factor)
                                            .clamp_range(0.1..=10.0)
                                            .speed(0.05)
                                            .fixed_decimals(2),
                                    );
                                });
                            });
                        }

                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_speed_edit();
                        }
                    });

                    ui.add_space(6.0);

                    // ===== 2. Particle Size =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_particle_size"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(self.i18n.tr("edit_particle_size_desc")).weak(),
                        );
                        ui.add_space(6.0);

                        ui.radio_value(
                            &mut self.edit.size_mode,
                            0,
                            self.i18n.tr("size_mode_scale"),
                        );
                        if self.edit.size_mode == 0 {
                            ui.indent("size_scale_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("size_mode_scale_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("scale_factor"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.size_scale)
                                            .clamp_range(0.01..=100.0)
                                            .speed(0.05)
                                            .fixed_decimals(2),
                                    );
                                });
                            });
                        }

                        ui.add_space(4.0);
                        ui.radio_value(
                            &mut self.edit.size_mode,
                            1,
                            self.i18n.tr("size_mode_uniform"),
                        );
                        if self.edit.size_mode == 1 {
                            ui.indent("size_uniform_indent", |ui| {
                                ui.label(
                                    egui::RichText::new(self.i18n.tr("size_mode_uniform_desc"))
                                        .weak()
                                        .small(),
                                );
                                ui.horizontal(|ui| {
                                    ui.label(self.i18n.tr("uniform_size"));
                                    ui.add(
                                        egui::DragValue::new(&mut self.edit.size_uniform)
                                            .clamp_range(0.0..=655.0)
                                            .speed(0.01)
                                            .fixed_decimals(2),
                                    );
                                });
                            });
                        }

                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_size_edit();
                        }
                    });

                    ui.add_space(6.0);

                    // ===== 3. Color Adjustment =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_color"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(self.i18n.tr("edit_color_desc")).weak());
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("brightness_factor"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.brightness)
                                    .clamp_range(0.0..=5.0)
                                    .speed(0.01)
                                    .fixed_decimals(2),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("opacity_factor"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.opacity)
                                    .clamp_range(0.0..=5.0)
                                    .speed(0.01)
                                    .fixed_decimals(2),
                            );
                        });

                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_color_edit();
                        }
                    });

                    ui.add_space(6.0);

                    // ===== 4. Position Transform =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_transform"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(self.i18n.tr("edit_transform_desc")).weak());
                        ui.add_space(6.0);

                        ui.label(self.i18n.tr("translate_offset"));
                        ui.horizontal(|ui| {
                            ui.label("X:");
                            ui.add(
                                egui::DragValue::new(&mut self.edit.translate[0])
                                    .speed(0.1)
                                    .fixed_decimals(2),
                            );
                            ui.label("Y:");
                            ui.add(
                                egui::DragValue::new(&mut self.edit.translate[1])
                                    .speed(0.1)
                                    .fixed_decimals(2),
                            );
                            ui.label("Z:");
                            ui.add(
                                egui::DragValue::new(&mut self.edit.translate[2])
                                    .speed(0.1)
                                    .fixed_decimals(2),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("position_scale"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.pos_scale)
                                    .clamp_range(0.01..=100.0)
                                    .speed(0.01)
                                    .fixed_decimals(2),
                            );
                        });

                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_transform_edit();
                        }
                    });

                    ui.add_space(6.0);

                    // ===== 5. Trim Frame Range =====
                    egui::CollapsingHeader::new(
                        egui::RichText::new(self.i18n.tr("edit_trim"))
                            .strong()
                            .size(15.0),
                    )
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(self.i18n.tr("edit_trim_desc")).weak());
                        ui.add_space(6.0);

                        let max_frame = self
                            .edit
                            .decoded_frames
                            .as_ref()
                            .map(|f| f.len().saturating_sub(1) as u32)
                            .unwrap_or(0);
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("trim_start"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.trim_start)
                                    .clamp_range(0..=max_frame)
                                    .speed(1.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(self.i18n.tr("trim_end"));
                            ui.add(
                                egui::DragValue::new(&mut self.edit.trim_end)
                                    .clamp_range(0..=max_frame)
                                    .speed(1.0),
                            );
                        });

                        if let Some(ref frames) = self.edit.decoded_frames {
                            let start = self.edit.trim_start as usize;
                            let end =
                                (self.edit.trim_end as usize).min(frames.len().saturating_sub(1));
                            if end >= start {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "  → {} {}",
                                        end - start + 1,
                                        self.i18n.tr("frame")
                                    ))
                                    .weak(),
                                );
                            }
                        }

                        ui.add_space(6.0);
                        if ui
                            .button(
                                egui::RichText::new(format!("▶ {}", self.i18n.tr("apply")))
                                    .strong(),
                            )
                            .clicked()
                        {
                            self.apply_trim_edit();
                        }
                    });

                    ui.add_space(20.0);
                    ui.separator();

                    // ===== Save Button =====
                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 36.0],
                            egui::Button::new(
                                egui::RichText::new(self.i18n.tr("save_file"))
                                    .strong()
                                    .size(16.0),
                            ),
                        )
                        .clicked()
                    {
                        self.save_edited_file();
                    }

                    // ===== Status Message =====
                    if let Some(ref msg) = self.edit.status_msg {
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

        // Central panel: summary of current edit state
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.25);

                if let Some(ref header) = self.edit.edited_header {
                    ui.heading(
                        egui::RichText::new(self.i18n.tr("edit_mode"))
                            .size(28.0)
                            .strong(),
                    );
                    ui.add_space(20.0);

                    let frame_count = self
                        .edit
                        .decoded_frames
                        .as_ref()
                        .map(|f| f.len())
                        .unwrap_or(0);
                    let duration = if header.target_fps > 0 {
                        frame_count as f32 / header.target_fps as f32
                    } else {
                        0.0
                    };

                    egui::Grid::new("edit_summary_grid")
                        .num_columns(2)
                        .spacing([20.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(self.i18n.tr("fps"));
                            ui.label(
                                egui::RichText::new(format!("{}", header.target_fps)).strong(),
                            );
                            ui.end_row();

                            ui.label(self.i18n.tr("total_frames"));
                            ui.label(egui::RichText::new(format!("{}", frame_count)).strong());
                            ui.end_row();

                            ui.label(self.i18n.tr("duration"));
                            ui.label(egui::RichText::new(format!("{:.2} s", duration)).strong());
                            ui.end_row();

                            ui.label(self.i18n.tr("bbox"));
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
                } else {
                    ui.heading(self.i18n.tr("no_file_loaded"));
                }
            });
        });
    }

    fn apply_speed_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            if let Some(ref mut header) = self.edit.edited_header {
                match self.edit.speed_mode {
                    0 => {
                        // FPS only
                        player::edit_change_fps(header, self.edit.new_fps);
                    }
                    1 => {
                        // Interpolate only
                        let new_frames =
                            player::edit_interpolate_frames(frames, self.edit.speed_factor);
                        *frames = new_frames;
                    }
                    2 => {
                        // Both
                        player::edit_change_fps(header, self.edit.new_fps);
                        let new_frames =
                            player::edit_interpolate_frames(frames, self.edit.speed_factor);
                        *frames = new_frames;
                    }
                    _ => {}
                }
                header.total_frames = frames.len() as u32;
                self.edit.trim_end = header.total_frames.saturating_sub(1);
                self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
            }
        }
    }

    fn apply_size_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            match self.edit.size_mode {
                0 => player::edit_scale_size(frames, self.edit.size_scale),
                1 => player::edit_uniform_size(frames, self.edit.size_uniform),
                _ => {}
            }
            self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
        }
    }

    fn apply_color_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            player::edit_adjust_color(frames, self.edit.brightness, self.edit.opacity);
            // Reset to 1.0 after applying so user can chain edits
            self.edit.brightness = 1.0;
            self.edit.opacity = 1.0;
            self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
        }
    }

    fn apply_transform_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            if self.edit.translate != [0.0; 3] {
                player::edit_translate(frames, self.edit.translate);
                self.edit.translate = [0.0; 3];
            }
            if (self.edit.pos_scale - 1.0).abs() > 0.001 {
                player::edit_scale_position(frames, self.edit.pos_scale);
                self.edit.pos_scale = 1.0;
            }
            // Recalculate bounding box
            if let Some(ref mut header) = self.edit.edited_header {
                let (bmin, bmax) = player::recalculate_bbox(frames);
                header.bbox_min = bmin;
                header.bbox_max = bmax;
            }
            self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
        }
    }

    fn apply_trim_edit(&mut self) {
        if let Some(ref mut frames) = self.edit.decoded_frames {
            let new_frames = player::edit_trim_frames(
                frames,
                self.edit.trim_start as usize,
                self.edit.trim_end as usize,
            );
            *frames = new_frames;
            if let Some(ref mut header) = self.edit.edited_header {
                header.total_frames = frames.len() as u32;
                let (bmin, bmax) = player::recalculate_bbox(frames);
                header.bbox_min = bmin;
                header.bbox_max = bmax;
            }
            self.edit.trim_start = 0;
            self.edit.trim_end = frames.len().saturating_sub(1) as u32;
            self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
        }
    }

    fn save_edited_file(&mut self) {
        let frames = match self.edit.decoded_frames {
            Some(ref f) => f.clone(),
            None => {
                self.edit.status_msg = Some(self.i18n.tr("apply_failed").to_string());
                return;
            }
        };
        let header = match self.edit.edited_header {
            Some(ref h) => h.clone(),
            None => {
                self.edit.status_msg = Some(self.i18n.tr("apply_failed").to_string());
                return;
            }
        };
        let textures = self.player.textures.clone();

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .set_file_name("output.nbl")
            .save_file()
        {
            match self.player.save_file(&path, &header, &textures, &frames) {
                Ok(_) => {
                    self.edit.status_msg = Some(self.i18n.tr("apply_success").to_string());
                }
                Err(e) => {
                    self.edit.status_msg = Some(format!("{}: {}", self.i18n.tr("apply_failed"), e));
                }
            }
        }
    }

    fn calculate_mvp(&self, aspect: f32) -> [f32; 16] {
        let view = self.calculate_view_matrix();
        let proj = self.calculate_projection_matrix(aspect);
        multiply_matrices(proj, view)
    }

    fn calculate_view_matrix(&self) -> [f32; 16] {
        let cos_p = self.camera.pitch.cos();
        let sin_p = self.camera.pitch.sin();
        let cos_y = self.camera.yaw.cos();
        let sin_y = self.camera.yaw.sin();
        let eye = [
            self.camera.target[0] + self.camera.distance * cos_p * sin_y,
            self.camera.target[1] + self.camera.distance * sin_p,
            self.camera.target[2] + self.camera.distance * cos_p * cos_y,
        ];
        look_at(eye, self.camera.target, [0.0, 1.0, 0.0])
    }

    fn calculate_projection_matrix(&self, aspect: f32) -> [f32; 16] {
        perspective(45.0f32.to_radians(), aspect, 0.1, 5000.0)
    }
}

// --- Matrix Utilities ---
fn look_at(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> [f32; 16] {
    let f = normalize([center[0] - eye[0], center[1] - eye[1], center[2] - eye[2]]);
    let s = normalize(cross(f, up));
    let u = cross(s, f);
    [
        s[0],
        u[0],
        -f[0],
        0.0,
        s[1],
        u[1],
        -f[1],
        0.0,
        s[2],
        u[2],
        -f[2],
        0.0,
        -dot(s, eye),
        -dot(u, eye),
        dot(f, eye),
        1.0,
    ]
}

fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fov / 2.0).tan();
    [
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        f,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) / (near - far),
        -1.0,
        0.0,
        0.0,
        (2.0 * far * near) / (near - far),
        0.0,
    ]
}

fn multiply_matrices(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut out = [0.0; 16];
    for r in 0..4 {
        for c in 0..4 {
            for k in 0..4 {
                out[r + c * 4] += a[r + k * 4] * b[k + c * 4];
            }
        }
    }
    out
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0; 3]
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
