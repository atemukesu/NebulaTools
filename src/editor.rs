/// Particle animation creator / simulator.
/// Provides emitter presets and a simulation engine that generates
/// `Vec<Vec<Particle>>` frame snapshots ready for NBL export.
use crate::player::Particle;
use serde::{Deserialize, Serialize};

// ──── Emitter Shape ────

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmitterShape {
    Point,
    Sphere,
    Box,
    Ring,
    Cylinder,
    Cone,
    Torus,
}

impl EmitterShape {
    pub fn i18n_key(&self) -> &'static str {
        match self {
            Self::Point => "shape_point",
            Self::Sphere => "shape_sphere",
            Self::Box => "shape_box",
            Self::Ring => "shape_ring",
            Self::Cylinder => "shape_cylinder",
            Self::Cone => "shape_cone",
            Self::Torus => "shape_torus",
        }
    }

    pub const ALL: [EmitterShape; 7] = [
        EmitterShape::Point,
        EmitterShape::Sphere,
        EmitterShape::Box,
        EmitterShape::Ring,
        EmitterShape::Cylinder,
        EmitterShape::Cone,
        EmitterShape::Torus,
    ];
}

// ──── Emitter Config ────

#[derive(Clone, Serialize, Deserialize)]
pub struct EmitterConfig {
    // Shape
    pub shape: EmitterShape,
    pub shape_radius: f32,
    pub shape_box_size: [f32; 3],
    pub surface_only: bool, // Spawns on the surface of the shape instead of inside

    // Emission
    pub emission_rate: f32, // particles per second
    pub burst_count: u32,   // initial burst (frame 0)
    pub burst_only: bool,   // if true, only emit on frame 0

    // Particle lifetime
    pub lifetime_frames: u32,

    // Velocity
    pub speed_min: f32,
    pub speed_max: f32,
    pub direction: [f32; 3], // primary direction (normalized internally)
    pub spread: f32,         // cone half-angle in degrees

    // Physics
    pub gravity: f32,
    pub drag: f32, // velocity damping per frame (0 = no drag, 1 = full stop)

    // Appearance
    pub color_start: [u8; 4],
    pub color_end: [u8; 4],
    pub size_start: f32,
    pub size_end: f32,
    pub texture_id: u8,

    // Animation
    pub target_fps: u16,
    pub duration_secs: f32,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            shape: EmitterShape::Point,
            shape_radius: 1.0,
            shape_box_size: [2.0, 2.0, 2.0],
            surface_only: false,
            emission_rate: 50.0,
            burst_count: 0,
            burst_only: false,
            lifetime_frames: 60,
            speed_min: 2.0,
            speed_max: 5.0,
            direction: [0.0, 1.0, 0.0],
            spread: 30.0,
            gravity: -9.8,
            drag: 0.0,
            color_start: [255, 200, 50, 255],
            color_end: [255, 50, 20, 80],
            size_start: 0.3,
            size_end: 0.05,
            texture_id: 0,
            target_fps: 30,
            duration_secs: 3.0,
        }
    }
}

// ──── Presets ────

impl EmitterConfig {
    pub fn preset_fireworks() -> Self {
        Self {
            shape: EmitterShape::Point,
            emission_rate: 0.0,
            burst_count: 200,
            burst_only: true,
            lifetime_frames: 45,
            speed_min: 3.0,
            speed_max: 8.0,
            direction: [0.0, 1.0, 0.0],
            spread: 180.0,
            gravity: -6.0,
            drag: 0.02,
            color_start: [255, 220, 80, 255],
            color_end: [255, 50, 10, 0],
            size_start: 0.25,
            size_end: 0.02,
            target_fps: 30,
            duration_secs: 2.0,
            ..Default::default()
        }
    }

    pub fn preset_fountain() -> Self {
        Self {
            shape: EmitterShape::Point,
            emission_rate: 80.0,
            burst_count: 0,
            burst_only: false,
            lifetime_frames: 50,
            speed_min: 4.0,
            speed_max: 7.0,
            direction: [0.0, 1.0, 0.0],
            spread: 15.0,
            gravity: -9.8,
            drag: 0.0,
            color_start: [100, 180, 255, 230],
            color_end: [30, 80, 200, 40],
            size_start: 0.2,
            size_end: 0.08,
            target_fps: 30,
            duration_secs: 4.0,
            ..Default::default()
        }
    }

    pub fn preset_spiral() -> Self {
        Self {
            shape: EmitterShape::Ring,
            shape_radius: 0.5,
            emission_rate: 60.0,
            burst_count: 0,
            burst_only: false,
            lifetime_frames: 90,
            speed_min: 1.0,
            speed_max: 2.0,
            direction: [0.0, 1.0, 0.0],
            spread: 5.0,
            gravity: 0.0,
            drag: 0.0,
            color_start: [180, 100, 255, 255],
            color_end: [50, 200, 255, 100],
            size_start: 0.15,
            size_end: 0.05,
            target_fps: 30,
            duration_secs: 4.0,
            ..Default::default()
        }
    }

    pub fn preset_explosion() -> Self {
        Self {
            shape: EmitterShape::Sphere,
            shape_radius: 0.2,
            emission_rate: 0.0,
            burst_count: 500,
            burst_only: true,
            lifetime_frames: 30,
            speed_min: 5.0,
            speed_max: 15.0,
            direction: [0.0, 0.0, 0.0], // radial
            spread: 180.0,
            gravity: -2.0,
            drag: 0.05,
            color_start: [255, 255, 200, 255],
            color_end: [200, 80, 20, 0],
            size_start: 0.4,
            size_end: 0.02,
            target_fps: 30,
            duration_secs: 1.5,
            ..Default::default()
        }
    }

    pub fn preset_snow() -> Self {
        Self {
            shape: EmitterShape::Box,
            shape_box_size: [20.0, 0.5, 20.0],
            emission_rate: 40.0,
            burst_count: 0,
            burst_only: false,
            lifetime_frames: 120,
            speed_min: 0.5,
            speed_max: 1.5,
            direction: [0.0, -1.0, 0.0],
            spread: 10.0,
            gravity: -0.5,
            drag: 0.01,
            color_start: [240, 245, 255, 220],
            color_end: [200, 220, 255, 80],
            size_start: 0.1,
            size_end: 0.08,
            target_fps: 30,
            duration_secs: 5.0,
            ..Default::default()
        }
    }
}

// ──── Simulation ────

struct LiveParticle {
    id: i32,
    pos: [f32; 3],
    vel: [f32; 3],
    birth_frame: u32,
    lifetime: u32,
    color_start: [u8; 4],
    color_end: [u8; 4],
    size_start: f32,
    size_end: f32,
    tex_id: u8,
}

/// Simple deterministic PRNG (xorshift32) for reproducible particle generation.
struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Random float in [0, 1)
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    /// Random float in [min, max]
    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

pub fn interpolate_config(a: &EmitterConfig, b: &EmitterConfig, t: f32) -> EmitterConfig {
    EmitterConfig {
        shape: a.shape,
        shape_radius: a.shape_radius + (b.shape_radius - a.shape_radius) * t,
        shape_box_size: [
            a.shape_box_size[0] + (b.shape_box_size[0] - a.shape_box_size[0]) * t,
            a.shape_box_size[1] + (b.shape_box_size[1] - a.shape_box_size[1]) * t,
            a.shape_box_size[2] + (b.shape_box_size[2] - a.shape_box_size[2]) * t,
        ],
        surface_only: a.surface_only,
        emission_rate: a.emission_rate + (b.emission_rate - a.emission_rate) * t,
        burst_count: a.burst_count,
        burst_only: a.burst_only,
        lifetime_frames: a.lifetime_frames,
        speed_min: a.speed_min + (b.speed_min - a.speed_min) * t,
        speed_max: a.speed_max + (b.speed_max - a.speed_max) * t,
        direction: [
            a.direction[0] + (b.direction[0] - a.direction[0]) * t,
            a.direction[1] + (b.direction[1] - a.direction[1]) * t,
            a.direction[2] + (b.direction[2] - a.direction[2]) * t,
        ],
        spread: a.spread + (b.spread - a.spread) * t,
        gravity: a.gravity + (b.gravity - a.gravity) * t,
        drag: a.drag + (b.drag - a.drag) * t,
        color_start: [
            lerp_u8(a.color_start[0], b.color_start[0], t),
            lerp_u8(a.color_start[1], b.color_start[1], t),
            lerp_u8(a.color_start[2], b.color_start[2], t),
            lerp_u8(a.color_start[3], b.color_start[3], t),
        ],
        color_end: [
            lerp_u8(a.color_end[0], b.color_end[0], t),
            lerp_u8(a.color_end[1], b.color_end[1], t),
            lerp_u8(a.color_end[2], b.color_end[2], t),
            lerp_u8(a.color_end[3], b.color_end[3], t),
        ],
        size_start: a.size_start + (b.size_start - a.size_start) * t,
        size_end: a.size_end + (b.size_end - a.size_end) * t,
        texture_id: a.texture_id,
        target_fps: a.target_fps,
        duration_secs: a.duration_secs,
    }
}

pub fn get_config_at_frame(
    frame: u32,
    base: &EmitterConfig,
    keyframes: &[(u32, EmitterConfig)],
) -> EmitterConfig {
    if keyframes.is_empty() {
        return base.clone();
    }

    let mut prev = (0, base.clone());
    let mut next = None;

    for (kf_frame, kf_config) in keyframes {
        if frame < *kf_frame {
            next = Some((*kf_frame, kf_config.clone()));
            break;
        }
        prev = (*kf_frame, kf_config.clone());
    }

    if let Some((next_frame, next_config)) = next {
        if next_frame == prev.0 {
            return prev.1;
        }
        let t = (frame - prev.0) as f32 / (next_frame - prev.0) as f32;
        interpolate_config(&prev.1, &next_config, t)
    } else {
        prev.1
    }
}

/// Run the full particle simulation and return frame snapshots.
pub fn simulate(config: &EmitterConfig, keyframes: &[(u32, EmitterConfig)]) -> Vec<Vec<Particle>> {
    let total_frames = (config.duration_secs * config.target_fps as f32).ceil() as u32;
    let dt = 1.0 / config.target_fps as f32;
    let mut rng = SimpleRng::new(42);
    let mut next_id: i32 = 1;
    let mut live: Vec<LiveParticle> = Vec::new();
    let mut frames: Vec<Vec<Particle>> = Vec::with_capacity(total_frames as usize);
    let mut emit_accum: f32 = 0.0;

    for frame in 0..total_frames {
        let current_config = get_config_at_frame(frame, config, keyframes);

        // 1. Emit new particles
        let emit_count = if frame == 0 {
            current_config.burst_count
        } else if current_config.burst_only {
            0
        } else {
            emit_accum += current_config.emission_rate * dt;
            let n = emit_accum.floor() as u32;
            emit_accum -= n as f32;
            n
        };

        for _ in 0..emit_count {
            let pos = spawn_position(&current_config, &mut rng);
            let vel = spawn_velocity(&current_config, &mut rng, frame, total_frames);
            live.push(LiveParticle {
                id: next_id,
                pos,
                vel,
                birth_frame: frame,
                lifetime: current_config.lifetime_frames,
                color_start: current_config.color_start,
                color_end: current_config.color_end,
                size_start: current_config.size_start,
                size_end: current_config.size_end,
                tex_id: current_config.texture_id,
            });
            next_id += 1;
        }

        // 2. Update physics
        for p in live.iter_mut() {
            p.vel[1] += current_config.gravity * dt;
            if current_config.drag > 0.0 {
                let damp = 1.0 - current_config.drag;
                p.vel[0] *= damp;
                p.vel[1] *= damp;
                p.vel[2] *= damp;
            }
            p.pos[0] += p.vel[0] * dt;
            p.pos[1] += p.vel[1] * dt;
            p.pos[2] += p.vel[2] * dt;
        }

        // 3. Remove dead particles
        live.retain(|p| frame - p.birth_frame < p.lifetime);

        // 4. Snapshot
        let snapshot: Vec<Particle> = live
            .iter()
            .map(|p| {
                let age = (frame - p.birth_frame) as f32 / p.lifetime.max(1) as f32;
                let t = age.clamp(0.0, 1.0);
                Particle {
                    id: p.id,
                    pos: p.pos,
                    color: [
                        lerp_u8(p.color_start[0], p.color_end[0], t),
                        lerp_u8(p.color_start[1], p.color_end[1], t),
                        lerp_u8(p.color_start[2], p.color_end[2], t),
                        lerp_u8(p.color_start[3], p.color_end[3], t),
                    ],
                    size: p.size_start + (p.size_end - p.size_start) * t,
                    tex_id: p.tex_id,
                    seq_index: 0,
                }
            })
            .collect();
        frames.push(snapshot);
    }

    frames
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn spawn_position(config: &EmitterConfig, rng: &mut SimpleRng) -> [f32; 3] {
    match config.shape {
        EmitterShape::Point => [0.0, 0.0, 0.0],
        EmitterShape::Sphere => {
            let theta = rng.range_f32(0.0, std::f32::consts::TAU);
            let phi = rng.range_f32(0.0, std::f32::consts::PI);
            let r = if config.surface_only {
                config.shape_radius
            } else {
                rng.range_f32(0.0, config.shape_radius)
            };
            [
                r * phi.sin() * theta.cos(),
                r * phi.cos(),
                r * phi.sin() * theta.sin(),
            ]
        }
        EmitterShape::Box => {
            let s = config.shape_box_size;
            if config.surface_only {
                let face = rng.range_f32(0.0, 6.0) as u32;
                let mut p = [
                    rng.range_f32(-s[0] / 2.0, s[0] / 2.0),
                    rng.range_f32(-s[1] / 2.0, s[1] / 2.0),
                    rng.range_f32(-s[2] / 2.0, s[2] / 2.0),
                ];
                match face {
                    0 => p[0] = s[0] / 2.0,
                    1 => p[0] = -s[0] / 2.0,
                    2 => p[1] = s[1] / 2.0,
                    3 => p[1] = -s[1] / 2.0,
                    4 => p[2] = s[2] / 2.0,
                    _ => p[2] = -s[2] / 2.0,
                }
                p
            } else {
                [
                    rng.range_f32(-s[0] / 2.0, s[0] / 2.0),
                    rng.range_f32(-s[1] / 2.0, s[1] / 2.0),
                    rng.range_f32(-s[2] / 2.0, s[2] / 2.0),
                ]
            }
        }
        EmitterShape::Ring => {
            let angle = rng.range_f32(0.0, std::f32::consts::TAU);
            [
                config.shape_radius * angle.cos(),
                0.0,
                config.shape_radius * angle.sin(),
            ]
        }
        EmitterShape::Cylinder => {
            let theta = rng.range_f32(0.0, std::f32::consts::TAU);
            let r = if config.surface_only {
                config.shape_radius
            } else {
                rng.range_f32(0.0, config.shape_radius)
            };
            if config.surface_only && rng.next_f32() > 0.8 {
                // caps
                let r2 = rng.range_f32(0.0, config.shape_radius);
                let cap = if rng.next_f32() > 0.5 { 1.0 } else { -1.0 };
                [
                    r2 * theta.cos(),
                    cap * config.shape_box_size[1] / 2.0,
                    r2 * theta.sin(),
                ]
            } else {
                // tube
                let h = rng.range_f32(
                    -config.shape_box_size[1] / 2.0,
                    config.shape_box_size[1] / 2.0,
                );
                [r * theta.cos(), h, r * theta.sin()]
            }
        }
        EmitterShape::Cone => {
            let theta = rng.range_f32(0.0, std::f32::consts::TAU);
            let h = rng.range_f32(0.0, config.shape_box_size[1]);
            let r_max = ((config.shape_box_size[1] - h).max(0.0)
                / config.shape_box_size[1].max(0.001))
                * config.shape_radius;
            let r = if config.surface_only {
                r_max
            } else {
                rng.range_f32(0.0, r_max)
            };
            [r * theta.cos(), h, r * theta.sin()]
        }
        EmitterShape::Torus => {
            let theta = rng.range_f32(0.0, std::f32::consts::TAU);
            let phi = rng.range_f32(0.0, std::f32::consts::TAU);
            let tube_r = if config.surface_only {
                config.shape_box_size[0]
            } else {
                rng.range_f32(0.0, config.shape_box_size[0])
            };
            let r_main = config.shape_radius;
            [
                (r_main + tube_r * phi.cos()) * theta.cos(),
                tube_r * phi.sin(),
                (r_main + tube_r * phi.cos()) * theta.sin(),
            ]
        }
    }
}

fn spawn_velocity(
    config: &EmitterConfig,
    rng: &mut SimpleRng,
    frame: u32,
    total_frames: u32,
) -> [f32; 3] {
    let speed = rng.range_f32(config.speed_min, config.speed_max);

    // For spiral, rotate direction over time
    if config.shape == EmitterShape::Ring {
        let t = frame as f32 / total_frames.max(1) as f32;
        let angle = t * std::f32::consts::TAU * 3.0;
        let base_dir = [angle.cos() * 0.3, 1.0, angle.sin() * 0.3];
        let len =
            (base_dir[0] * base_dir[0] + base_dir[1] * base_dir[1] + base_dir[2] * base_dir[2])
                .sqrt();
        return [
            base_dir[0] / len * speed,
            base_dir[1] / len * speed,
            base_dir[2] / len * speed,
        ];
    }

    let dir = config.direction;
    let dir_len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();

    if dir_len < 0.001 || config.spread >= 179.0 {
        // Random direction (uniform sphere)
        let theta = rng.range_f32(0.0, std::f32::consts::TAU);
        let cos_phi = rng.range_f32(-1.0, 1.0);
        let sin_phi = (1.0 - cos_phi * cos_phi).sqrt();
        return [
            sin_phi * theta.cos() * speed,
            cos_phi * speed,
            sin_phi * theta.sin() * speed,
        ];
    }

    let nd = [dir[0] / dir_len, dir[1] / dir_len, dir[2] / dir_len];

    // Apply spread cone
    let spread_rad = config.spread.to_radians();
    let theta = rng.range_f32(0.0, std::f32::consts::TAU);
    let cos_a = rng.range_f32(spread_rad.cos(), 1.0);
    let sin_a = (1.0 - cos_a * cos_a).sqrt();

    // Build orthonormal basis around `nd`
    let up = if nd[1].abs() < 0.9 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let right = crate::math::normalize(crate::math::cross(up, nd));
    let actual_up = crate::math::cross(nd, right);

    [
        (nd[0] * cos_a + right[0] * sin_a * theta.cos() + actual_up[0] * sin_a * theta.sin())
            * speed,
        (nd[1] * cos_a + right[1] * sin_a * theta.cos() + actual_up[1] * sin_a * theta.sin())
            * speed,
        (nd[2] * cos_a + right[2] * sin_a * theta.cos() + actual_up[2] * sin_a * theta.sin())
            * speed,
    ]
}
