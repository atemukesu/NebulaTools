use super::app::{AnimKeyframe, CreatorObjectData, NebulaToolsApp};
use crate::editor;
use crate::particleex::{self, CompileEntry};
use crate::player::{self, NblHeader, Particle, TextureEntry};
use image::{DynamicImage, GenericImageView};

impl NebulaToolsApp {
    /// Compile all creator objects into preview frames.
    pub(crate) fn compile_creator(&mut self) {
        let fps = self.creator.target_fps;
        let total_frames = (self.creator.duration_secs * fps as f32).ceil() as u32;

        let mut all_frames: Vec<Vec<Particle>> = vec![vec![]; total_frames as usize];
        let mut compile_entries: Vec<CompileEntry> = Vec::new();
        let mut next_global_id: i32 = 1;

        for obj in &self.creator.objects {
            if !obj.enabled {
                continue;
            }

            match &obj.data {
                CreatorObjectData::Emitter(config) => {
                    let frames = editor::simulate(config, &[]);
                    let obj_frames = self.apply_object_animation(frames, obj, total_frames, fps);
                    all_frames = merge_frames(all_frames, obj_frames);
                }
                CreatorObjectData::Shape(config) => {
                    let t_begin = 0.0;
                    let t_end = 1.0;
                    let t_step = 1.0 / (config.count as f64).max(1.0);
                    let shape_expr = match config.shape_type {
                        super::app::SceneShapeType::Line => {
                            let dx = config.end_pos[0] - config.origin[0];
                            let dy = config.end_pos[1] - config.origin[1];
                            let dz = config.end_pos[2] - config.origin[2];
                            format!(
                                "x={}+t*{}; y={}+t*{}; z={}+t*{}",
                                config.origin[0], dx, config.origin[1], dy, config.origin[2], dz
                            )
                        }
                        super::app::SceneShapeType::Plane => {
                            format!(
                                "x={}+(t*{count} % 1.0)*{}; y={}+floor(t*{count})/{count}.0*{}; z={}",
                                config.origin[0], config.size[0],
                                config.origin[1], config.size[1],
                                config.origin[2],
                                count = config.count.max(1)
                            )
                        }
                        super::app::SceneShapeType::Sphere => {
                            let r = config.radius;
                            format!(
                                "x={}+{}*cos(t*{PI}*2)*sin(t*{PI}); y={}+{}*cos(t*{PI}); z={}+{}*sin(t*{PI}*2)*sin(t*{PI})",
                                config.origin[0], r,
                                config.origin[1], r,
                                config.origin[2], r,
                                PI = std::f32::consts::PI
                            )
                        }
                        super::app::SceneShapeType::Cube => {
                            format!(
                                "x={}+{}-2.0*{}*abs(sin(t*{PI}*3.14)); y={}+{}-2.0*{}*abs(sin(t*{PI}*1.57)); z={}+{}-2.0*{}*abs(sin(t*{PI}*0.75))",
                                config.origin[0], config.size[0]/2.0, config.size[0]/2.0,
                                config.origin[1], config.size[1]/2.0, config.size[1]/2.0,
                                config.origin[2], config.size[2]/2.0, config.size[2]/2.0,
                                PI = std::f32::consts::PI
                            )
                        }
                        super::app::SceneShapeType::Cylinder => {
                            let r = config.radius;
                            format!(
                                "x={}+{}*cos(t*{PI}*10); y={}+(t-0.5)*{}; z={}+{}*sin(t*{PI}*10)",
                                config.origin[0],
                                r,
                                config.origin[1],
                                config.size[1],
                                config.origin[2],
                                r,
                                PI = std::f32::consts::PI
                            )
                        }
                    };
                    let cmd = format!(
                        "particleex parameter end_rod 0 0 0 {} {} {} {} 0 0 0 {} {} '{}' {} {} '{}'",
                        config.color[0], config.color[1], config.color[2], config.color[3],
                        t_begin, t_end, shape_expr, t_step, config.lifespan, config.velocity_expr,
                    );
                    compile_entries.push(CompileEntry {
                        command: cmd,
                        start_tick: 0.0,
                        position: [
                            obj.position[0] as f64,
                            obj.position[1] as f64,
                            obj.position[2] as f64,
                        ],
                        duration_override: 0.0,
                    });
                }
                CreatorObjectData::Parameter(config) => {
                    let cmd_type = if config.is_polar {
                        "polarparameter"
                    } else {
                        "parameter"
                    };
                    let cmd =
                        format!(
                        "particleex {} end_rod {} {} {} {} {} {} {} {} {} {} {} {} '{}' {} {} '{}'",
                        cmd_type,
                        config.center[0], config.center[1], config.center[2],
                        config.color[0], config.color[1], config.color[2], config.color[3],
                        config.velocity[0], config.velocity[1], config.velocity[2],
                        config.t_begin, config.t_end,
                        config.expr, config.t_step, config.lifespan, config.velocity_expr,
                    );
                    compile_entries.push(CompileEntry {
                        command: cmd,
                        start_tick: 0.0,
                        position: [
                            obj.position[0] as f64,
                            obj.position[1] as f64,
                            obj.position[2] as f64,
                        ],
                        duration_override: 0.0,
                    });
                }
                CreatorObjectData::ImageText(config) => {
                    let particles =
                        self.compile_imagetext_to_particles(config, &mut next_global_id);
                    if particles.is_empty() {
                        continue;
                    }
                    // Build frames with velocity expression applied
                    let vel_stmts = particleex::compile_expr(&obj.velocity_expr);
                    let obj_total = total_frames as usize;
                    let mut runtime = particles.clone();
                    let mut img_frames = Vec::with_capacity(obj_total);

                    for f_idx in 0..obj_total {
                        let t = f_idx as f64 / fps as f64;
                        for p in runtime.iter_mut() {
                            let mut ctx = particleex::ExprContext::new();
                            ctx.set("t", t);
                            ctx.set("x", p.pos[0] as f64);
                            ctx.set("y", p.pos[1] as f64);
                            ctx.set("z", p.pos[2] as f64);
                            ctx.set("vx", 0.0);
                            ctx.set("vy", 0.0);
                            ctx.set("vz", 0.0);
                            ctx.set("cr", p.color[0] as f64 / 255.0);
                            ctx.set("cg", p.color[1] as f64 / 255.0);
                            ctx.set("cb", p.color[2] as f64 / 255.0);
                            ctx.set("alpha", p.color[3] as f64 / 255.0);
                            ctx.set("mpsize", p.size as f64);
                            ctx.set("destory", 0.0);

                            if let Some(ref s) = vel_stmts {
                                particleex::exec_stmts(s, &mut ctx);
                            }

                            if ctx.get("destory") >= 1.0 {
                                p.color[3] = 0;
                            }

                            p.pos[0] += ctx.get("vx") as f32;
                            p.pos[1] += ctx.get("vy") as f32;
                            p.pos[2] += ctx.get("vz") as f32;
                            p.color[0] = (ctx.get("cr").clamp(0.0, 1.0) * 255.0) as u8;
                            p.color[1] = (ctx.get("cg").clamp(0.0, 1.0) * 255.0) as u8;
                            p.color[2] = (ctx.get("cb").clamp(0.0, 1.0) * 255.0) as u8;
                            p.color[3] = (ctx.get("alpha").clamp(0.0, 1.0) * 255.0) as u8;
                            p.size = ctx.get("mpsize") as f32;
                        }
                        img_frames.push(runtime.clone());
                    }

                    let obj_frames =
                        self.apply_object_animation(img_frames, obj, total_frames, fps);
                    all_frames = merge_frames(all_frames, obj_frames);
                }
            }
        }

        // Compile PEX entries
        if !compile_entries.is_empty() {
            if let Ok((pex_frames, _fps)) = particleex::compile_entries(&compile_entries) {
                all_frames = merge_frames(all_frames, pex_frames);
            }
        }

        self.creator.preview_frames = Some(all_frames);
        self.creator.preview_playing = true;
        self.creator.preview_frame_idx = 0;
        self.creator.status_msg = Some(format!("✅ {}", self.i18n.tr("apply_success")));
    }

    /// Apply per-object keyframe animation (position/rotation/scale/alpha transform).
    fn apply_object_animation(
        &self,
        frames: Vec<Vec<Particle>>,
        obj: &super::app::CreatorObject,
        total_frames: u32,
        _fps: u16,
    ) -> Vec<Vec<Particle>> {
        let mut result = Vec::with_capacity(total_frames as usize);

        for frame_idx in 0..total_frames as usize {
            let source_idx = frame_idx.min(frames.len().saturating_sub(1));
            let source = if frame_idx < frames.len() {
                &frames[source_idx]
            } else {
                // Pad with last frame
                if frames.is_empty() {
                    &[] as &[Particle]
                } else {
                    &frames[frames.len() - 1]
                }
            };

            // Interpolate keyframes
            let kf = interpolate_keyframes(frame_idx as u32, obj);

            let cos_rx = (kf.rotation[0].to_radians()).cos();
            let sin_rx = (kf.rotation[0].to_radians()).sin();
            let cos_ry = (kf.rotation[1].to_radians()).cos();
            let sin_ry = (kf.rotation[1].to_radians()).sin();
            let cos_rz = (kf.rotation[2].to_radians()).cos();
            let sin_rz = (kf.rotation[2].to_radians()).sin();

            let snapshot: Vec<Particle> = source
                .iter()
                .map(|p| {
                    // Scale
                    let mut x = p.pos[0] * kf.scale[0];
                    let mut y = p.pos[1] * kf.scale[1];
                    let mut z = p.pos[2] * kf.scale[2];

                    // Rotate X
                    let y1 = y * cos_rx - z * sin_rx;
                    let z1 = y * sin_rx + z * cos_rx;
                    y = y1;
                    z = z1;

                    // Rotate Y
                    let x1 = x * cos_ry + z * sin_ry;
                    let z2 = -x * sin_ry + z * cos_ry;
                    x = x1;
                    z = z2;

                    // Rotate Z
                    let x2 = x * cos_rz - y * sin_rz;
                    let y2 = x * sin_rz + y * cos_rz;
                    x = x2;
                    y = y2;

                    // Translate
                    x += kf.position[0];
                    y += kf.position[1];
                    z += kf.position[2];

                    // Alpha
                    let a = (p.color[3] as f32 * kf.alpha).clamp(0.0, 255.0) as u8;

                    Particle {
                        id: p.id,
                        pos: [x, y, z],
                        color: [p.color[0], p.color[1], p.color[2], a],
                        size: p.size,
                        tex_id: p.tex_id,
                        seq_index: p.seq_index,
                    }
                })
                .collect();

            result.push(snapshot);
        }

        result
    }

    /// Convert ImageTextConfig to base particles.
    fn compile_imagetext_to_particles(
        &self,
        config: &super::app::ImageTextConfig,
        next_id: &mut i32,
    ) -> Vec<Particle> {
        let mut img: Option<DynamicImage> = None;

        if config.is_text {
            // Text → image
            let mut font_data = None;
            if config.font_name.starts_with("system://") {
                let family = &config.font_name[9..];
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
            } else if !config.font_name.is_empty() {
                font_data = std::fs::read(&config.font_name).ok();
            }

            if let Some(fd) = font_data {
                if let Ok(font_ref) = ab_glyph::FontRef::try_from_slice(&fd) {
                    use ab_glyph::{Font, PxScale, ScaleFont};
                    let px_scale = PxScale::from(config.font_size);
                    let scale_font = font_ref.as_scaled(px_scale);
                    let lines: Vec<&str> = config.text_input.lines().collect();

                    let ascent = scale_font.ascent().ceil() as u32;
                    let descent = scale_font.descent().floor() as i32;
                    let line_height = (ascent as i32 - descent).abs() as u32;
                    let line_gap = (line_height as f32 * 0.2).ceil() as u32;

                    let mut max_w: u32 = 1;
                    for line in &lines {
                        let mut w: f32 = 0.0;
                        let mut prev: Option<ab_glyph::GlyphId> = None;
                        for ch in line.chars() {
                            let gid = scale_font.glyph_id(ch);
                            if let Some(p) = prev {
                                w += scale_font.kern(p, gid);
                            }
                            w += scale_font.h_advance(gid);
                            prev = Some(gid);
                        }
                        max_w = max_w.max(w.ceil() as u32);
                    }

                    let pad = config.font_size as u32;
                    let cw = max_w + pad * 4;
                    let ch = (lines.len() as u32 * line_height)
                        + (lines.len() as u32).saturating_sub(1) * line_gap
                        + pad * 4;
                    let mut text_img = image::RgbaImage::new(cw, ch);

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
                }
            }
        } else {
            // Image load
            if let Some(path) = &config.media_path {
                if let Ok(loaded) = image::open(path) {
                    img = Some(loaded);
                }
            }
        }

        let Some(img) = img else {
            return vec![];
        };

        let (width, height) = img.dimensions();
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let dist_scale = config.particle_size;
        let density = config.density.max(0.000001);

        let step = if config.is_text {
            1u32
        } else if density < 1.0 {
            (1.0 / density).ceil() as u32
        } else {
            1u32
        };
        let copies = if density >= 1.0 {
            density.floor() as u32
        } else {
            1u32
        };

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut particles = Vec::new();

        for y in (0..height).step_by(step as usize) {
            for x in (0..width).step_by(step as usize) {
                let pixel = img.get_pixel(x, y);
                let is_filtered = if config.is_text {
                    pixel[3] < 128
                } else {
                    let luma = (pixel[0] as f32 * 0.299
                        + pixel[1] as f32 * 0.587
                        + pixel[2] as f32 * 0.114)
                        / 255.0;
                    pixel[3] == 0 || luma < config.brightness_threshold
                };
                if is_filtered {
                    continue;
                }
                if !config.is_text && density < 1.0 && rng.gen::<f32>() > density {
                    continue;
                }

                for c in 0..copies {
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
                    particles.push(Particle {
                        id: *next_id,
                        pos: [px, py, 0.0],
                        color: [pixel[0], pixel[1], pixel[2], pixel[3]],
                        size: config.point_size,
                        tex_id: 0,
                        seq_index: 0,
                    });
                    *next_id += 1;
                }
            }
        }

        particles
    }

    /// Export compiled creator to NBL.
    pub(crate) fn export_creator_nbl(&mut self) {
        if self.creator.preview_frames.is_none() {
            self.compile_creator();
        }

        let frames = match self.creator.preview_frames {
            Some(ref f) => f.clone(),
            None => {
                self.creator.status_msg = Some(self.i18n.tr("apply_failed").to_string());
                return;
            }
        };

        let (bbox_min, bbox_max) = player::recalculate_bbox(&frames);
        let header = NblHeader {
            version: 1,
            target_fps: self.creator.target_fps,
            total_frames: frames.len() as u32,
            texture_count: 0,
            attributes: 0x03,
            bbox_min,
            bbox_max,
        };
        let textures: Vec<TextureEntry> = vec![];

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Nebula", &["nbl"])
            .set_file_name("creator_export.nbl")
            .save_file()
        {
            match self.player.save_file(&path, &header, &textures, &frames) {
                Ok(_) => {
                    self.creator.status_msg = Some(format!("✅ {}", self.i18n.tr("apply_success")));
                }
                Err(e) => {
                    self.creator.status_msg =
                        Some(format!("{}: {}", self.i18n.tr("apply_failed"), e));
                }
            }
        }
    }
}

/// Interpolate keyframe animation for a given object at a given frame.
fn interpolate_keyframes(frame: u32, obj: &super::app::CreatorObject) -> AnimKeyframe {
    if obj.keyframes.is_empty() {
        return AnimKeyframe {
            position: obj.position,
            rotation: obj.rotation,
            scale: obj.scale,
            alpha: obj.alpha,
        };
    }

    let keys: Vec<u32> = obj.keyframes.keys().copied().collect();
    let first = keys[0];
    let last = *keys.last().unwrap();

    if frame <= first {
        return obj.keyframes[&first].clone();
    }
    if frame >= last {
        return obj.keyframes[&last].clone();
    }

    // Find surrounding keyframes
    let mut prev_f = first;
    let mut next_f = last;
    for &k in &keys {
        if k <= frame {
            prev_f = k;
        }
        if k > frame && k < next_f {
            next_f = k;
            break;
        }
    }

    if prev_f == next_f {
        return obj.keyframes[&prev_f].clone();
    }

    let a = &obj.keyframes[&prev_f];
    let b = &obj.keyframes[&next_f];
    let t = (frame - prev_f) as f32 / (next_f - prev_f) as f32;

    AnimKeyframe {
        position: [
            a.position[0] + (b.position[0] - a.position[0]) * t,
            a.position[1] + (b.position[1] - a.position[1]) * t,
            a.position[2] + (b.position[2] - a.position[2]) * t,
        ],
        rotation: [
            a.rotation[0] + (b.rotation[0] - a.rotation[0]) * t,
            a.rotation[1] + (b.rotation[1] - a.rotation[1]) * t,
            a.rotation[2] + (b.rotation[2] - a.rotation[2]) * t,
        ],
        scale: [
            a.scale[0] + (b.scale[0] - a.scale[0]) * t,
            a.scale[1] + (b.scale[1] - a.scale[1]) * t,
            a.scale[2] + (b.scale[2] - a.scale[2]) * t,
        ],
        alpha: a.alpha + (b.alpha - a.alpha) * t,
    }
}

/// Merge two frame arrays (extend short one, append particles).
fn merge_frames(mut a: Vec<Vec<Particle>>, b: Vec<Vec<Particle>>) -> Vec<Vec<Particle>> {
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
