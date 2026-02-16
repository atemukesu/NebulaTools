use crate::editor::EmitterConfig;
use crate::i18n::I18nManager;
use crate::player::{NblHeader, Particle, PlayerState};
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
    Create,
    Particleex,
}

#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub lang: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            lang: "en_US".into(),
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

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum EditTool {
    Speed,
    Size,
    Color,
    Transform,
    Trim,
    Compress,
}

pub struct EditState {
    pub selected_tool: EditTool,
    pub new_fps: u16,
    pub speed_factor: f32,
    pub speed_mode: u8,
    pub size_mode: u8,
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
    pub compress_keyframe_interval: u32,
    pub compress_zstd_level: i32,
}

impl Default for EditState {
    fn default() -> Self {
        Self {
            selected_tool: EditTool::Speed,
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
            compress_keyframe_interval: 60,
            compress_zstd_level: 3,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum CreatorPreset {
    Fireworks,
    Fountain,
    Spiral,
    Explosion,
    Snow,
    Custom,
}

pub struct CreatorState {
    pub preset: CreatorPreset,
    pub config: EmitterConfig,
    pub preview_frames: Option<Vec<Vec<Particle>>>,
    pub preview_playing: bool,
    pub preview_frame_idx: i32,
    pub preview_timer: f32,
    pub status_msg: Option<String>,
}

impl Default for CreatorState {
    fn default() -> Self {
        Self {
            preset: CreatorPreset::Fireworks,
            config: EmitterConfig::preset_fireworks(),
            preview_frames: None,
            preview_playing: false,
            preview_frame_idx: 0,
            preview_timer: 0.0,
            status_msg: None,
        }
    }
}

pub struct PexCommandEntry {
    pub command: String,
    pub start_tick: f32,
    pub position: [f32; 3],
    pub duration_override: f32, // 0 = use command's own lifespan
    pub enabled: bool,
}

impl Default for PexCommandEntry {
    fn default() -> Self {
        Self {
            command: String::new(),
            start_tick: 0.0,
            position: [0.0, 0.0, 0.0],
            duration_override: 0.0,
            enabled: true,
        }
    }
}

pub struct ParticleexState {
    pub entries: Vec<PexCommandEntry>,
    pub preview_frames: Option<Vec<Vec<Particle>>>,
    pub preview_playing: bool,
    pub preview_frame_idx: i32,
    pub preview_timer: f32,
    pub preview_fps: u16,
    pub status_msg: Option<String>,
    pub show_help: bool,
    pub fullscreen_entry: Option<usize>,
    pub confirm_delete: Option<usize>,
}

impl Default for ParticleexState {
    fn default() -> Self {
        Self {
            entries: vec![PexCommandEntry::default()],
            preview_frames: None,
            preview_playing: false,
            preview_frame_idx: 0,
            preview_timer: 0.0,
            preview_fps: 60,
            status_msg: None,
            show_help: false,
            fullscreen_entry: None,
            confirm_delete: None,
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
    pub creator: CreatorState,
    pub pex: ParticleexState,
}

impl NebulaToolsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let _ = egui_chinese_font::setup_chinese_fonts(&cc.egui_ctx);

        let config = match fs::read_to_string("config.json") {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        };

        let i18n = I18nManager::new(config.lang.clone());

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
            creator: CreatorState::default(),
            pex: ParticleexState::default(),
        }
    }

    pub fn save_config(&self) {
        if let Ok(s) = serde_json::to_string(&self.config) {
            let _ = fs::write("config.json", s);
        }
    }

    pub fn update_lang(&mut self, lang_id: String) {
        self.config.lang = lang_id.clone();
        self.i18n.active_lang = lang_id;
        self.save_config();
    }

    pub fn prepare_render_data(&self) -> Vec<f32> {
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

    /// Build render data from an arbitrary particle slice (for creator preview).
    pub fn prepare_render_data_from(&self, particles: &[Particle]) -> Vec<f32> {
        let mut data = Vec::with_capacity(particles.len() * 8);
        for p in particles {
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

    pub fn handle_import(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .pick_file()
        {
            match self.player.load_file(path) {
                Ok(_) => {
                    self.error_msg = None;
                    self.mode = AppMode::Preview;
                }
                Err(e) => self.error_msg = Some(format!("Load Failed: {}", e)),
            }
        }
    }

    pub fn calculate_mvp(&self, aspect: f32) -> [f32; 16] {
        let view = self.calculate_view_matrix();
        let proj = self.calculate_projection_matrix(aspect);
        crate::math::multiply_matrices(proj, view)
    }

    pub fn calculate_view_matrix(&self) -> [f32; 16] {
        let cos_p = self.camera.pitch.cos();
        let sin_p = self.camera.pitch.sin();
        let cos_y = self.camera.yaw.cos();
        let sin_y = self.camera.yaw.sin();
        let eye = [
            self.camera.target[0] + self.camera.distance * cos_p * sin_y,
            self.camera.target[1] + self.camera.distance * sin_p,
            self.camera.target[2] + self.camera.distance * cos_p * cos_y,
        ];
        crate::math::look_at(eye, self.camera.target, [0.0, 1.0, 0.0])
    }

    pub fn calculate_projection_matrix(&self, aspect: f32) -> [f32; 16] {
        crate::math::perspective(45.0f32.to_radians(), aspect, 0.1, 1000000.0)
    }

    /// Shared 3D viewport rendering (used by both preview and creator).
    pub fn paint_3d_viewport(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        particles_data: &[f32],
    ) {
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
            self.camera.distance = (self.camera.distance - s * 0.1).clamp(0.1, 1000000.0);
        }
        let aspect = rect.width() / rect.height();
        let mvp = self.calculate_mvp(aspect);
        let renderer_ref = self.renderer.clone();
        let show_grid = self.show_grid;
        let rect_height = rect.height();
        let data = particles_data.to_vec();

        let fov_y = 45.0f32.to_radians();
        let focal_length = 1.0 / (fov_y / 2.0).tan();

        let callback = egui_glow::CallbackFn::new(move |info, painter| {
            let mut lock = renderer_ref.lock().unwrap();
            if lock.is_none() {
                *lock = Some(crate::renderer::ParticleRenderer::new(painter.gl()));
            }
            if let Some(r) = lock.as_ref() {
                let physical_height = rect_height * info.pixels_per_point;
                let scaling = (focal_length * physical_height) / 2.0;
                unsafe {
                    painter.gl().clear_color(0.0, 0.0, 0.0, 1.0);
                    r.paint(painter.gl(), mvp, &data, scaling, show_grid);
                }
            }
        });
        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
        ui.painter().add(egui::PaintCallback {
            rect,
            callback: Arc::new(callback),
        });

        // FPS overlay
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
        let info_text = format!("FPS: {:.1}", self.fps_display);
        painter.text(
            overlay_pos,
            egui::Align2::LEFT_TOP,
            info_text,
            egui::FontId::proportional(16.0),
            egui::Color32::from_white_alpha(180),
        );

        // Particle count overlay (below FPS)
        let particle_count = particles_data.len() / 8;
        let particle_pos = rect.left_top() + egui::vec2(10.0, 30.0);
        painter.text(
            particle_pos,
            egui::Align2::LEFT_TOP,
            format!("Particles: {}", particle_count),
            egui::FontId::proportional(16.0),
            egui::Color32::from_white_alpha(180),
        );
    }
}

impl eframe::App for NebulaToolsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.player.header.is_none()
            && self.mode != AppMode::Create
            && self.mode != AppMode::Particleex
        {
            self.show_welcome_screen(ctx);
            return;
        }

        // Playback logic (preview mode only)
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
                if ui.button(self.i18n.tr("home")).clicked() {
                    self.mode = AppMode::Preview;
                    self.player.header = None;
                    self.pex.preview_frames = None;
                }
                ui.separator();

                ui.menu_button(self.i18n.tr("file"), |ui| {
                    if self.mode == AppMode::Create {
                        if ui.button(self.i18n.tr("export_nbl")).clicked() {
                            self.export_creator_nbl();
                            ui.close_menu();
                        }
                    } else if self.mode == AppMode::Particleex {
                        if ui.button(self.i18n.tr("export_nbl")).clicked() {
                            self.export_particleex_nbl();
                            ui.close_menu();
                        }
                    } else {
                        if ui.button(self.i18n.tr("import")).clicked() {
                            self.handle_import();
                            ui.close_menu();
                        }
                    }
                });

                egui::ComboBox::from_id_source("top_lang_combo")
                    .selected_text(self.i18n.get_lang_name(&self.config.lang))
                    .show_ui(ui, |ui| {
                        let available = self.i18n.available_langs.clone();
                        for lang_id in available {
                            let name = self.i18n.get_lang_name(&lang_id);
                            if ui
                                .selectable_label(self.config.lang == lang_id, name)
                                .clicked()
                            {
                                self.update_lang(lang_id);
                            }
                        }
                    });

                ui.separator();
                if self.player.header.is_some() {
                    ui.selectable_value(&mut self.mode, AppMode::Edit, self.i18n.tr("edit_mode"));
                    ui.selectable_value(
                        &mut self.mode,
                        AppMode::Preview,
                        self.i18n.tr("preview_mode"),
                    );
                } else if self.mode == AppMode::Create {
                    ui.selectable_value(&mut self.mode, AppMode::Create, self.i18n.tr("creator"));
                } else if self.mode == AppMode::Particleex {
                    ui.selectable_value(&mut self.mode, AppMode::Particleex, "Particleex");
                }
            });
        });

        match self.mode {
            AppMode::Preview => self.show_preview_workflow(ctx),
            AppMode::Edit => self.show_edit_workflow(ctx),
            AppMode::Create => self.show_creator_workflow(ctx),
            AppMode::Particleex => self.show_particleex_workflow(ctx),
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
