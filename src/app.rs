use crate::i18n::{I18nManager, Language};
use crate::player::PlayerState;
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
    pub side_panel_width: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            lang: Language::ChineseSimplified,
            side_panel_width: 320.0,
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

pub struct NebulaToolsApp {
    pub player: PlayerState,
    pub config: AppConfig,
    pub i18n: I18nManager,
    pub error_msg: Option<String>,
    pub camera: CameraState,
    pub renderer: Arc<Mutex<Option<ParticleRenderer>>>,
    pub show_grid: bool,
    pub mode: AppMode,
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

                if self.mode == AppMode::Preview {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.checkbox(&mut self.show_grid, "Grid");
                    });
                }
            });
        });

        let panel_id = if self.mode == AppMode::Preview {
            "preview_side"
        } else {
            "edit_side"
        };
        egui::SidePanel::left(panel_id)
            .resizable(true)
            .width_range(300.0..=600.0)
            .default_width(self.config.side_panel_width)
            .show(ctx, |ui| {
                let w = ui.available_width();
                if (w - self.config.side_panel_width).abs() > 1.0 {
                    self.config.side_panel_width = w;
                    self.save_config();
                }

                ui.add_space(8.0);
                if let Some(header) = self.player.header.clone() {
                    if self.mode == AppMode::Preview {
                        ui.heading(self.i18n.tr("playback"));
                        ui.horizontal(|ui| {
                            let label = if self.player.is_playing {
                                self.i18n.tr("pause")
                            } else {
                                self.i18n.tr("play")
                            };
                            if ui.button(label).clicked() {
                                self.player.is_playing = !self.player.is_playing;
                            }
                            if ui.button(self.i18n.tr("stop")).clicked() {
                                self.player.is_playing = false;
                                let _ = self.player.seek_to(0);
                            }
                        });
                        let mut frame = self.player.current_frame_idx.max(0) as u32;
                        if ui
                            .add(
                                egui::Slider::new(
                                    &mut frame,
                                    0..=header.total_frames.saturating_sub(1),
                                )
                                .text(self.i18n.tr("frame")),
                            )
                            .changed()
                        {
                            self.player.is_playing = false;
                            let _ = self.player.seek_to(frame);
                        }
                        ui.label(format!(
                            "{}: {}",
                            self.i18n.tr("particle_count"),
                            self.player.particles.len()
                        ));
                        ui.separator();
                    } else {
                        ui.heading(self.i18n.tr("edit_mode"));
                        ui.label("Properties Editor");
                        ui.separator();
                    }
                    ui.collapsing(self.i18n.tr("metadata"), |ui| {
                        ui.label(format!("{}: {}", self.i18n.tr("version"), header.version));
                        ui.label(format!("{}: {}", self.i18n.tr("fps"), header.target_fps));
                        ui.label(format!(
                            "{}: {}",
                            self.i18n.tr("total_frames"),
                            header.total_frames
                        ));
                    });
                }
                if let Some(err) = &self.error_msg {
                    ui.colored_label(egui::Color32::RED, err);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.mode == AppMode::Preview {
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
                let mvp = self.calculate_mvp(rect.width() / rect.height());
                let renderer_ref = self.renderer.clone();
                let show_grid = self.show_grid;
                let rect_height = rect.height();
                let callback = egui_glow::CallbackFn::new(move |info, painter| {
                    let mut lock = renderer_ref.lock().unwrap();
                    if lock.is_none() {
                        *lock = Some(ParticleRenderer::new(painter.gl()));
                    }
                    if let Some(r) = lock.as_ref() {
                        let scaling = rect_height * info.pixels_per_point * 1.2;
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
                ui.put(
                    egui::Rect::from_min_size(
                        rect.min + egui::vec2(10.0, 10.0),
                        egui::vec2(200.0, 50.0),
                    ),
                    |ui: &mut egui::Ui| {
                        ui.colored_label(egui::Color32::WHITE, self.i18n.tr("preview_3d"));
                        ui.colored_label(egui::Color32::GRAY, self.i18n.tr("preview_hint"));
                        ui.label("")
                    },
                );
            } else {
                ui.centered_and_justified(|ui| {
                    ui.heading("Editing Canvas (WIP)");
                    ui.label("Visual editing environment will appear here.");
                });
            }
        });
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
                            egui::Button::new(format!("📂 {}", self.i18n.tr("open_existing")))
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
                            egui::Button::new(format!("✨ {}", self.i18n.tr("create_new")))
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
