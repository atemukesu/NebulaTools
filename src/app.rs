use crate::i18n::{tr, Language};
use crate::player::PlayerState;
use crate::renderer::ParticleRenderer;
use eframe::{egui, egui_glow, glow};
use std::sync::{Arc, Mutex};

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
    pub lang: Language,
    pub error_msg: Option<String>,
    pub camera: CameraState,
    pub renderer: Arc<Mutex<Option<ParticleRenderer>>>,
    pub show_grid: bool,
}

impl NebulaToolsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let _ = egui_chinese_font::setup_chinese_fonts(&cc.egui_ctx);
        Self {
            player: PlayerState::default(),
            lang: Language::ChineseSimplified,
            error_msg: None,
            camera: CameraState::default(),
            renderer: Arc::new(Mutex::new(None)),
            show_grid: true,
        }
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
}

impl eframe::App for NebulaToolsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.player.is_playing {
            if let Some(header) = &self.player.header {
                let dt = ctx.input(|i| i.stable_dt);
                self.player.frame_timer += dt;
                let frame_dur = 1.0 / header.target_fps as f32;
                if self.player.frame_timer >= frame_dur {
                    self.player.frame_timer -= frame_dur;
                    let next_frame = self.player.current_frame_idx + 1;
                    if (next_frame as u32) < header.total_frames {
                        if let Err(e) = self.player.seek_to(next_frame as u32) {
                            self.player.is_playing = false;
                            self.error_msg = Some(format!("Playback Error: {}", e));
                        }
                    } else {
                        self.player.is_playing = false;
                    }
                    ctx.request_repaint();
                } else {
                    ctx.request_repaint();
                }
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(tr(self.lang, "file"), |ui| {
                    if ui.button(tr(self.lang, "import")).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Nebula", &["nbl"])
                            .pick_file()
                        {
                            match self.player.load_file(path) {
                                Ok(_) => {
                                    self.error_msg = None;
                                    if let Some(h) = &self.player.header {
                                        self.camera.target = [
                                            (h.bbox_min[0] + h.bbox_max[0]) * 0.5,
                                            (h.bbox_min[1] + h.bbox_max[1]) * 0.5,
                                            (h.bbox_min[2] + h.bbox_max[2]) * 0.5,
                                        ];
                                    }
                                }
                                Err(e) => self.error_msg = Some(format!("Load Failed: {}", e)),
                            }
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button(tr(self.lang, "language"), |ui| {
                    if ui
                        .selectable_label(self.lang == Language::ChineseSimplified, "简体中文")
                        .clicked()
                    {
                        self.lang = Language::ChineseSimplified;
                        ui.close_menu();
                    }
                    if ui
                        .selectable_label(self.lang == Language::English, "English")
                        .clicked()
                    {
                        self.lang = Language::English;
                        ui.close_menu();
                    }
                });

                ui.separator();
                ui.checkbox(&mut self.show_grid, "Grid");
            });
        });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(8.0);
                    if let Some(header) = self.player.header.clone() {
                        ui.heading(tr(self.lang, "playback"));
                        ui.horizontal(|ui| {
                            let label = if self.player.is_playing {
                                tr(self.lang, "pause")
                            } else {
                                tr(self.lang, "play")
                            };
                            if ui.button(label).clicked() {
                                self.player.is_playing = !self.player.is_playing;
                            }
                            if ui.button(tr(self.lang, "stop")).clicked() {
                                self.player.is_playing = false;
                                let _ = self.player.seek_to(0);
                            }
                        });

                        let mut frame = self.player.current_frame_idx.max(0) as u32;
                        let max_frame = header.total_frames.saturating_sub(1);
                        if ui
                            .add(
                                egui::Slider::new(&mut frame, 0..=max_frame)
                                    .text(tr(self.lang, "frame")),
                            )
                            .changed()
                        {
                            self.player.is_playing = false;
                            let _ = self.player.seek_to(frame);
                        }
                        ui.label(format!(
                            "{}: {}",
                            tr(self.lang, "particle_count"),
                            self.player.particles.len()
                        ));

                        ui.separator();
                        ui.collapsing(tr(self.lang, "metadata"), |ui| {
                            ui.label(format!("{}: {}", tr(self.lang, "version"), header.version));
                            ui.label(format!("{}: {}", tr(self.lang, "fps"), header.target_fps));
                            ui.label(format!(
                                "{}: {}",
                                tr(self.lang, "total_frames"),
                                header.total_frames
                            ));
                        });
                    } else {
                        ui.label("No file loaded");
                    }

                    if let Some(err) = &self.error_msg {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());

            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = response.drag_delta();
                self.camera.yaw += delta.x * 0.01;
                self.camera.pitch += delta.y * 0.01;
                self.camera.pitch = self.camera.pitch.clamp(-1.5, 1.5);
            }
            if response.hovered() {
                let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
                self.camera.distance -= scroll * 0.1;
                self.camera.distance = self.camera.distance.clamp(1.0, 1000.0);
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
                if let Some(renderer) = lock.as_ref() {
                    let px_height = rect_height * info.pixels_per_point;
                    let scaling = px_height * 1.2;
                    unsafe {
                        renderer.paint(painter.gl(), mvp, &particles_data, scaling, show_grid);
                    }
                }
            });

            ui.painter().add(egui::PaintCallback {
                rect,
                callback: Arc::new(callback),
            });

            // Overlay Text
            ui.put(
                egui::Rect::from_min_size(
                    rect.min + egui::vec2(10.0, 10.0),
                    egui::vec2(200.0, 50.0),
                ),
                |ui: &mut egui::Ui| {
                    ui.colored_label(egui::Color32::WHITE, tr(self.lang, "preview_3d"));
                    ui.colored_label(egui::Color32::GRAY, tr(self.lang, "preview_hint"));
                    ui.label("")
                },
            );
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
