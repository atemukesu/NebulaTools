use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

const MAGIC: &[u8; 8] = b"NEBULAFX";

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct NblHeader {
    pub version: u16,
    pub target_fps: u16,
    pub total_frames: u32,
    pub texture_count: u16,
    pub attributes: u16,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TextureEntry {
    pub path: String,
    pub rows: u8,
    pub cols: u8,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Particle {
    pub id: i32,
    pub pos: [f32; 3],
    pub color: [u8; 4],
    pub size: f32,
    pub tex_id: u8,
    pub seq_index: u8,
}

pub struct PlayerState {
    pub file: Option<File>,
    pub header: Option<NblHeader>,
    pub textures: Vec<TextureEntry>,
    pub frame_indices: Vec<(u64, u32)>,
    pub keyframe_indices: Vec<u32>,

    pub current_frame_idx: i32,
    pub particles: HashMap<i32, Particle>,
    pub is_playing: bool,
    pub frame_timer: f32,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            file: None,
            header: None,
            textures: Vec::new(),
            frame_indices: Vec::new(),
            keyframe_indices: Vec::new(),
            current_frame_idx: -1,
            particles: HashMap::new(),
            is_playing: false,
            frame_timer: 0.0,
        }
    }
}

impl PlayerState {
    pub fn load_file(&mut self, path: PathBuf) -> Result<()> {
        let mut f = File::open(path)?;

        let mut magic = [0u8; 8];
        f.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(anyhow!("Invalid NBL file: Magic mismatch"));
        }

        let version = f.read_u16::<LittleEndian>()?;
        let target_fps = f.read_u16::<LittleEndian>()?;
        let total_frames = f.read_u32::<LittleEndian>()?;
        let texture_count = f.read_u16::<LittleEndian>()?;
        let attributes = f.read_u16::<LittleEndian>()?;

        let mut bbox_min = [0.0; 3];
        f.read_f32_into::<LittleEndian>(&mut bbox_min)?;
        let mut bbox_max = [0.0; 3];
        f.read_f32_into::<LittleEndian>(&mut bbox_max)?;

        f.seek(SeekFrom::Current(4))?; // Reserved

        self.header = Some(NblHeader {
            version,
            target_fps,
            total_frames,
            texture_count,
            attributes,
            bbox_min,
            bbox_max,
        });

        self.textures.clear();
        for _ in 0..texture_count {
            let path_len = f.read_u16::<LittleEndian>()?;
            let mut path_bytes = vec![0u8; path_len as usize];
            f.read_exact(&mut path_bytes)?;
            let path = String::from_utf8(path_bytes).unwrap_or_else(|_| "Invalid UTF8".into());
            let rows = f.read_u8()?;
            let cols = f.read_u8()?;
            self.textures.push(TextureEntry { path, rows, cols });
        }

        self.frame_indices.clear();
        for _ in 0..total_frames {
            let offset = f.read_u64::<LittleEndian>()?;
            let size = f.read_u32::<LittleEndian>()?;
            self.frame_indices.push((offset, size));
        }

        self.keyframe_indices.clear();
        let k_count = f.read_u32::<LittleEndian>()?;
        for _ in 0..k_count {
            self.keyframe_indices.push(f.read_u32::<LittleEndian>()?);
        }

        self.file = Some(f);
        self.current_frame_idx = -1;
        self.particles.clear();
        self.is_playing = false;

        if total_frames > 0 {
            self.seek_to(0)?;
        }

        Ok(())
    }

    pub fn seek_to(&mut self, target_frame: u32) -> Result<()> {
        if self.file.is_none() {
            return Ok(());
        }
        let total = self.header.as_ref().unwrap().total_frames;
        if target_frame >= total {
            return Ok(());
        }

        let mut start_frame = 0;
        for &kf in &self.keyframe_indices {
            if kf <= target_frame {
                start_frame = kf;
            } else {
                break;
            }
        }

        if (target_frame as i32) != self.current_frame_idx + 1 || (target_frame == start_frame) {
            self.particles.clear();
            for f in start_frame..=target_frame {
                self.process_frame(f)?;
            }
        } else {
            self.process_frame(target_frame)?;
        }

        self.current_frame_idx = target_frame as i32;
        Ok(())
    }

    fn process_frame(&mut self, frame_idx: u32) -> Result<()> {
        let (offset, size) = self.frame_indices[frame_idx as usize];
        let file = self.file.as_mut().unwrap();

        file.seek(SeekFrom::Start(offset))?;
        let mut compressed = vec![0u8; size as usize];
        file.read_exact(&mut compressed)?;

        let raw_data = zstd::decode_all(Cursor::new(compressed))?;
        let mut cursor = Cursor::new(raw_data);

        let frame_type = cursor.read_u8()?;
        let particle_count = cursor.read_u32::<LittleEndian>()? as usize;

        match frame_type {
            0 => self.parse_i_frame(&mut cursor, particle_count),
            1 => self.parse_p_frame(&mut cursor, particle_count),
            _ => Err(anyhow!("Unknown frame type: {}", frame_type)),
        }
    }

    fn parse_i_frame(&mut self, r: &mut Cursor<Vec<u8>>, count: usize) -> Result<()> {
        let mut px = vec![0.0; count];
        r.read_f32_into::<LittleEndian>(&mut px)?;
        let mut py = vec![0.0; count];
        r.read_f32_into::<LittleEndian>(&mut py)?;
        let mut pz = vec![0.0; count];
        r.read_f32_into::<LittleEndian>(&mut pz)?;

        let mut cr = vec![0u8; count];
        r.read_exact(&mut cr)?;
        let mut cg = vec![0u8; count];
        r.read_exact(&mut cg)?;
        let mut cb = vec![0u8; count];
        r.read_exact(&mut cb)?;
        let mut ca = vec![0u8; count];
        r.read_exact(&mut ca)?;

        let mut sizes = vec![0u16; count];
        r.read_u16_into::<LittleEndian>(&mut sizes)?;

        let mut tex_ids = vec![0u8; count];
        r.read_exact(&mut tex_ids)?;
        let mut seq_indices = vec![0u8; count];
        r.read_exact(&mut seq_indices)?;

        let mut p_ids = vec![0i32; count];
        r.read_i32_into::<LittleEndian>(&mut p_ids)?;

        self.particles.clear();
        for i in 0..count {
            self.particles.insert(
                p_ids[i],
                Particle {
                    id: p_ids[i],
                    pos: [px[i], py[i], pz[i]],
                    color: [cr[i], cg[i], cb[i], ca[i]],
                    size: sizes[i] as f32 / 100.0,
                    tex_id: tex_ids[i],
                    seq_index: seq_indices[i],
                },
            );
        }
        Ok(())
    }

    fn parse_p_frame(&mut self, r: &mut Cursor<Vec<u8>>, count: usize) -> Result<()> {
        let mut dx = vec![0i16; count];
        r.read_i16_into::<LittleEndian>(&mut dx)?;
        let mut dy = vec![0i16; count];
        r.read_i16_into::<LittleEndian>(&mut dy)?;
        let mut dz = vec![0i16; count];
        r.read_i16_into::<LittleEndian>(&mut dz)?;

        let mut dr = vec![0i8; count];
        r.read_exact(bytemuck::cast_slice_mut(&mut dr))?;
        let mut dg = vec![0i8; count];
        r.read_exact(bytemuck::cast_slice_mut(&mut dg))?;
        let mut db = vec![0i8; count];
        r.read_exact(bytemuck::cast_slice_mut(&mut db))?;
        let mut da = vec![0i8; count];
        r.read_exact(bytemuck::cast_slice_mut(&mut da))?;

        let mut d_sizes = vec![0i16; count];
        r.read_i16_into::<LittleEndian>(&mut d_sizes)?;

        let mut d_tex = vec![0i8; count];
        r.read_exact(bytemuck::cast_slice_mut(&mut d_tex))?;
        let mut d_seq = vec![0i8; count];
        r.read_exact(bytemuck::cast_slice_mut(&mut d_seq))?;

        let mut p_ids = vec![0i32; count];
        r.read_i32_into::<LittleEndian>(&mut p_ids)?;

        let mut current_frame_ids = std::collections::HashSet::new();
        for i in 0..count {
            let id = p_ids[i];
            current_frame_ids.insert(id);

            let delta_pos = [
                dx[i] as f32 / 1000.0,
                dy[i] as f32 / 1000.0,
                dz[i] as f32 / 1000.0,
            ];
            let delta_col = [dr[i], dg[i], db[i], da[i]];
            let delta_size = d_sizes[i] as f32 / 100.0;

            self.particles
                .entry(id)
                .and_modify(|p| {
                    p.pos[0] += delta_pos[0];
                    p.pos[1] += delta_pos[1];
                    p.pos[2] += delta_pos[2];
                    p.color[0] = p.color[0].saturating_add_signed(delta_col[0]);
                    p.color[1] = p.color[1].saturating_add_signed(delta_col[1]);
                    p.color[2] = p.color[2].saturating_add_signed(delta_col[2]);
                    p.color[3] = p.color[3].saturating_add_signed(delta_col[3]);
                    p.size += delta_size;
                    p.tex_id = p.tex_id.wrapping_add(d_tex[i] as u8);
                    p.seq_index = p.seq_index.wrapping_add(d_seq[i] as u8);
                })
                .or_insert_with(|| Particle {
                    id,
                    pos: delta_pos,
                    color: [
                        0u8.saturating_add_signed(delta_col[0]),
                        0u8.saturating_add_signed(delta_col[1]),
                        0u8.saturating_add_signed(delta_col[2]),
                        0u8.saturating_add_signed(delta_col[3]),
                    ],
                    size: delta_size,
                    tex_id: 0u8.wrapping_add(d_tex[i] as u8),
                    seq_index: 0u8.wrapping_add(d_seq[i] as u8),
                });
        }
        self.particles.retain(|k, _| current_frame_ids.contains(k));
        Ok(())
    }

    /// Decode every frame in the file into a Vec of particle snapshots.
    /// Each entry is a Vec<Particle> representing the state at that frame.
    pub fn decode_all_frames(&mut self) -> Result<Vec<Vec<Particle>>> {
        let total = self
            .header
            .as_ref()
            .ok_or(anyhow!("No header"))?
            .total_frames;
        let mut all_frames: Vec<Vec<Particle>> = Vec::with_capacity(total as usize);

        // Reset state before decoding
        self.particles.clear();
        self.current_frame_idx = -1;

        for frame_idx in 0..total {
            self.process_frame(frame_idx)?;
            let snapshot: Vec<Particle> = self.particles.values().cloned().collect();
            all_frames.push(snapshot);
            self.current_frame_idx = frame_idx as i32;
        }

        // Reset playback state after full decode
        self.particles.clear();
        self.current_frame_idx = -1;
        if total > 0 {
            self.seek_to(0)?;
        }

        Ok(all_frames)
    }

    /// Write a complete NBL file from frame snapshots.
    /// Uses I-Frames only for simplicity and maximum compatibility.
    pub fn save_file(
        &self,
        path: &PathBuf,
        header: &NblHeader,
        textures: &[TextureEntry],
        frames: &[Vec<Particle>],
    ) -> Result<()> {
        let mut f = File::create(path)?;

        // 1. Header (48 bytes)
        f.write_all(MAGIC)?;
        f.write_u16::<LittleEndian>(header.version)?;
        f.write_u16::<LittleEndian>(header.target_fps)?;
        f.write_u32::<LittleEndian>(frames.len() as u32)?;
        f.write_u16::<LittleEndian>(textures.len() as u16)?;
        f.write_u16::<LittleEndian>(header.attributes)?;
        for v in &header.bbox_min {
            f.write_f32::<LittleEndian>(*v)?;
        }
        for v in &header.bbox_max {
            f.write_f32::<LittleEndian>(*v)?;
        }
        f.write_all(&[0u8; 4])?; // Reserved

        // 2. Texture block
        for tex in textures {
            let path_bytes = tex.path.as_bytes();
            f.write_u16::<LittleEndian>(path_bytes.len() as u16)?;
            f.write_all(path_bytes)?;
            f.write_u8(tex.rows)?;
            f.write_u8(tex.cols)?;
        }

        // 3. Encode all frames to compressed blobs
        let mut compressed_blobs: Vec<Vec<u8>> = Vec::with_capacity(frames.len());
        for frame_particles in frames {
            let raw = encode_i_frame(frame_particles);
            let compressed = zstd::encode_all(Cursor::new(&raw), 3)?;
            compressed_blobs.push(compressed);
        }

        // 4. Calculate offsets for the Frame Index Table
        // Header (48) + Texture block size + Frame Index Table size + Keyframe Index Table size
        let mut tex_block_size: usize = 0;
        for tex in textures {
            tex_block_size += 2 + tex.path.as_bytes().len() + 1 + 1;
        }

        let frame_index_table_size = frames.len() * 12; // 8 (u64 offset) + 4 (u32 size) per frame

        // All frames are keyframes (I-Frames)
        let keyframe_count = frames.len() as u32;
        let keyframe_index_table_size = 4 + frames.len() * 4; // u32 count + u32 * N

        let data_start = 48 + tex_block_size + frame_index_table_size + keyframe_index_table_size;

        // Build frame index entries
        let mut current_offset = data_start;
        let mut index_entries: Vec<(u64, u32)> = Vec::with_capacity(frames.len());
        for blob in &compressed_blobs {
            index_entries.push((current_offset as u64, blob.len() as u32));
            current_offset += blob.len();
        }

        // 5. Write Frame Index Table
        for (offset, size) in &index_entries {
            f.write_u64::<LittleEndian>(*offset)?;
            f.write_u32::<LittleEndian>(*size)?;
        }

        // 6. Write Keyframe Index Table (all frames are keyframes)
        f.write_u32::<LittleEndian>(keyframe_count)?;
        for i in 0..frames.len() {
            f.write_u32::<LittleEndian>(i as u32)?;
        }

        // 7. Write compressed frame data
        for blob in &compressed_blobs {
            f.write_all(blob)?;
        }

        f.flush()?;
        Ok(())
    }
}

/// Encode a single frame snapshot as an I-Frame (uncompressed raw bytes).
fn encode_i_frame(particles: &[Particle]) -> Vec<u8> {
    let n = particles.len();
    // Header: 1 byte FrameType + 4 bytes ParticleCount
    // Payload: see spec
    let payload_size = 5 + n * 4 * 3 + n * 4 + n * 2 + n + n + n * 4;
    let mut buf = Vec::with_capacity(payload_size);

    buf.push(0u8); // FrameType = I-Frame
    let _ = buf.write_u32::<LittleEndian>(n as u32);

    // SoA: X, Y, Z
    for p in particles {
        let _ = buf.write_f32::<LittleEndian>(p.pos[0]);
    }
    for p in particles {
        let _ = buf.write_f32::<LittleEndian>(p.pos[1]);
    }
    for p in particles {
        let _ = buf.write_f32::<LittleEndian>(p.pos[2]);
    }

    // SoA: R, G, B, A
    for p in particles {
        buf.push(p.color[0]);
    }
    for p in particles {
        buf.push(p.color[1]);
    }
    for p in particles {
        buf.push(p.color[2]);
    }
    for p in particles {
        buf.push(p.color[3]);
    }

    // Sizes (u16, scaled by 100)
    for p in particles {
        let size_u16 = (p.size * 100.0).round().clamp(0.0, 65535.0) as u16;
        let _ = buf.write_u16::<LittleEndian>(size_u16);
    }

    // Texture IDs
    for p in particles {
        buf.push(p.tex_id);
    }

    // Sequence Indices
    for p in particles {
        buf.push(p.seq_index);
    }

    // Particle IDs
    for p in particles {
        let _ = buf.write_i32::<LittleEndian>(p.id);
    }

    buf
}

// ======== Edit Transform Functions ========

/// Change the target FPS without modifying frame data.
pub fn edit_change_fps(header: &mut NblHeader, new_fps: u16) {
    header.target_fps = new_fps;
}

/// Interpolate frames to change animation speed while keeping FPS constant.
/// speed_factor > 1.0 = faster (fewer frames), speed_factor < 1.0 = slower (more frames).
pub fn edit_interpolate_frames(frames: &[Vec<Particle>], speed_factor: f32) -> Vec<Vec<Particle>> {
    if frames.is_empty() || speed_factor <= 0.0 {
        return frames.to_vec();
    }

    let new_count = ((frames.len() as f32) / speed_factor).round().max(1.0) as usize;
    let mut result = Vec::with_capacity(new_count);

    for i in 0..new_count {
        let src_pos = (i as f32) * (frames.len() as f32 - 1.0) / (new_count as f32 - 1.0).max(1.0);
        let idx_a = (src_pos.floor() as usize).min(frames.len() - 1);
        let idx_b = (idx_a + 1).min(frames.len() - 1);
        let t = src_pos - idx_a as f32;

        if idx_a == idx_b || t < 0.001 {
            result.push(frames[idx_a].clone());
        } else {
            result.push(lerp_particles(&frames[idx_a], &frames[idx_b], t));
        }
    }

    result
}

/// Linearly interpolate between two particle snapshots.
fn lerp_particles(a: &[Particle], b: &[Particle], t: f32) -> Vec<Particle> {
    let a_map: HashMap<i32, &Particle> = a.iter().map(|p| (p.id, p)).collect();
    let b_map: HashMap<i32, &Particle> = b.iter().map(|p| (p.id, p)).collect();

    let mut result = Vec::new();

    // Interpolate particles present in both frames
    for pa in a {
        if let Some(pb) = b_map.get(&pa.id) {
            result.push(Particle {
                id: pa.id,
                pos: [
                    pa.pos[0] + (pb.pos[0] - pa.pos[0]) * t,
                    pa.pos[1] + (pb.pos[1] - pa.pos[1]) * t,
                    pa.pos[2] + (pb.pos[2] - pa.pos[2]) * t,
                ],
                color: [
                    lerp_u8(pa.color[0], pb.color[0], t),
                    lerp_u8(pa.color[1], pb.color[1], t),
                    lerp_u8(pa.color[2], pb.color[2], t),
                    lerp_u8(pa.color[3], pb.color[3], t),
                ],
                size: pa.size + (pb.size - pa.size) * t,
                tex_id: if t < 0.5 { pa.tex_id } else { pb.tex_id },
                seq_index: if t < 0.5 { pa.seq_index } else { pb.seq_index },
            });
        } else if t < 0.5 {
            // Particle only in frame A, keep if closer to A
            result.push(pa.clone());
        }
    }

    // Particles only in frame B, add if closer to B
    if t >= 0.5 {
        for pb in b {
            if !a_map.contains_key(&pb.id) {
                result.push(pb.clone());
            }
        }
    }

    result
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// Scale particle sizes across all frames by a multiplier.
pub fn edit_scale_size(frames: &mut [Vec<Particle>], factor: f32) {
    for frame in frames.iter_mut() {
        for p in frame.iter_mut() {
            p.size *= factor;
        }
    }
}

/// Set all particle sizes to a uniform value across all frames.
pub fn edit_uniform_size(frames: &mut [Vec<Particle>], size: f32) {
    for frame in frames.iter_mut() {
        for p in frame.iter_mut() {
            p.size = size;
        }
    }
}

/// Adjust brightness (RGB) and opacity (Alpha) of all particles.
pub fn edit_adjust_color(frames: &mut [Vec<Particle>], brightness: f32, opacity: f32) {
    for frame in frames.iter_mut() {
        for p in frame.iter_mut() {
            p.color[0] = (p.color[0] as f32 * brightness).round().clamp(0.0, 255.0) as u8;
            p.color[1] = (p.color[1] as f32 * brightness).round().clamp(0.0, 255.0) as u8;
            p.color[2] = (p.color[2] as f32 * brightness).round().clamp(0.0, 255.0) as u8;
            p.color[3] = (p.color[3] as f32 * opacity).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Translate all particle positions by an offset.
pub fn edit_translate(frames: &mut [Vec<Particle>], offset: [f32; 3]) {
    for frame in frames.iter_mut() {
        for p in frame.iter_mut() {
            p.pos[0] += offset[0];
            p.pos[1] += offset[1];
            p.pos[2] += offset[2];
        }
    }
}

/// Scale all particle positions by a factor (relative to origin).
pub fn edit_scale_position(frames: &mut [Vec<Particle>], scale: f32) {
    for frame in frames.iter_mut() {
        for p in frame.iter_mut() {
            p.pos[0] *= scale;
            p.pos[1] *= scale;
            p.pos[2] *= scale;
        }
    }
}

/// Recalculate the AABB bounding box from frame data.
pub fn recalculate_bbox(frames: &[Vec<Particle>]) -> ([f32; 3], [f32; 3]) {
    let mut bbox_min = [f32::MAX; 3];
    let mut bbox_max = [f32::MIN; 3];
    for frame in frames {
        for p in frame {
            for i in 0..3 {
                bbox_min[i] = bbox_min[i].min(p.pos[i]);
                bbox_max[i] = bbox_max[i].max(p.pos[i]);
            }
        }
    }
    if bbox_min[0] == f32::MAX {
        bbox_min = [0.0; 3];
        bbox_max = [0.0; 3];
    }
    (bbox_min, bbox_max)
}

/// Trim frames to a sub-range [start, end] (inclusive).
pub fn edit_trim_frames(frames: &[Vec<Particle>], start: usize, end: usize) -> Vec<Vec<Particle>> {
    let end = end.min(frames.len().saturating_sub(1));
    let start = start.min(end);
    frames[start..=end].to_vec()
}

/// Encode a P-Frame: delta between prev_particles and cur_particles.
/// Uses zero-basis principle for newly spawned particles.
fn encode_p_frame(prev_particles: &[Particle], cur_particles: &[Particle]) -> Vec<u8> {
    let prev_map: HashMap<i32, &Particle> = prev_particles.iter().map(|p| (p.id, p)).collect();
    let n = cur_particles.len();

    let payload_size = 5 + n * 2 * 3 + n * 4 + n * 2 + n + n + n * 4;
    let mut buf = Vec::with_capacity(payload_size);

    buf.push(1u8); // FrameType = P-Frame
    let _ = buf.write_u32::<LittleEndian>(n as u32);

    // SoA: dX, dY, dZ (int16, * 1000)
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_x = prev.map_or(0.0, |pp| pp.pos[0]);
        let dx = ((p.pos[0] - prev_x) * 1000.0)
            .round()
            .clamp(-32768.0, 32767.0) as i16;
        let _ = buf.write_i16::<LittleEndian>(dx);
    }
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_y = prev.map_or(0.0, |pp| pp.pos[1]);
        let dy = ((p.pos[1] - prev_y) * 1000.0)
            .round()
            .clamp(-32768.0, 32767.0) as i16;
        let _ = buf.write_i16::<LittleEndian>(dy);
    }
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_z = prev.map_or(0.0, |pp| pp.pos[2]);
        let dz = ((p.pos[2] - prev_z) * 1000.0)
            .round()
            .clamp(-32768.0, 32767.0) as i16;
        let _ = buf.write_i16::<LittleEndian>(dz);
    }

    // SoA: dR, dG, dB, dA (int8)
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_r = prev.map_or(0u8, |pp| pp.color[0]);
        let dr = (p.color[0] as i16 - prev_r as i16).clamp(-128, 127) as i8;
        buf.push(dr as u8);
    }
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_g = prev.map_or(0u8, |pp| pp.color[1]);
        let dg = (p.color[1] as i16 - prev_g as i16).clamp(-128, 127) as i8;
        buf.push(dg as u8);
    }
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_b = prev.map_or(0u8, |pp| pp.color[2]);
        let db = (p.color[2] as i16 - prev_b as i16).clamp(-128, 127) as i8;
        buf.push(db as u8);
    }
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_a = prev.map_or(0u8, |pp| pp.color[3]);
        let da = (p.color[3] as i16 - prev_a as i16).clamp(-128, 127) as i8;
        buf.push(da as u8);
    }

    // SoA: dSize (int16, * 100)
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_size = prev.map_or(0.0, |pp| pp.size);
        let ds = ((p.size - prev_size) * 100.0)
            .round()
            .clamp(-32768.0, 32767.0) as i16;
        let _ = buf.write_i16::<LittleEndian>(ds);
    }

    // SoA: dTexID (int8)
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_tex = prev.map_or(0u8, |pp| pp.tex_id);
        let dt = (p.tex_id as i16 - prev_tex as i16).clamp(-128, 127) as i8;
        buf.push(dt as u8);
    }

    // SoA: dSeq (int8)
    for p in cur_particles {
        let prev = prev_map.get(&p.id);
        let prev_seq = prev.map_or(0u8, |pp| pp.seq_index);
        let ds = (p.seq_index as i16 - prev_seq as i16).clamp(-128, 127) as i8;
        buf.push(ds as u8);
    }

    // Particle IDs
    for p in cur_particles {
        let _ = buf.write_i32::<LittleEndian>(p.id);
    }

    buf
}

/// Write a complete NBL file using I/P frame compression.
/// - `keyframe_interval`: number of frames between I-Frames (0 = all I-Frames).
/// - `zstd_level`: Zstd compression level (1-5).
pub fn save_file_compressed(
    path: &PathBuf,
    header: &NblHeader,
    textures: &[TextureEntry],
    frames: &[Vec<Particle>],
    keyframe_interval: u32,
    zstd_level: i32,
) -> Result<()> {
    let mut f = File::create(path)?;

    // 1. Header (48 bytes)
    f.write_all(MAGIC)?;
    f.write_u16::<LittleEndian>(header.version)?;
    f.write_u16::<LittleEndian>(header.target_fps)?;
    f.write_u32::<LittleEndian>(frames.len() as u32)?;
    f.write_u16::<LittleEndian>(textures.len() as u16)?;
    f.write_u16::<LittleEndian>(header.attributes)?;
    for v in &header.bbox_min {
        f.write_f32::<LittleEndian>(*v)?;
    }
    for v in &header.bbox_max {
        f.write_f32::<LittleEndian>(*v)?;
    }
    f.write_all(&[0u8; 4])?; // Reserved

    // 2. Texture block
    for tex in textures {
        let path_bytes = tex.path.as_bytes();
        f.write_u16::<LittleEndian>(path_bytes.len() as u16)?;
        f.write_all(path_bytes)?;
        f.write_u8(tex.rows)?;
        f.write_u8(tex.cols)?;
    }

    // 3. Determine which frames are keyframes
    let effective_interval = if keyframe_interval == 0 {
        1
    } else {
        keyframe_interval
    };
    let mut keyframe_frame_indices: Vec<u32> = Vec::new();
    for i in 0..frames.len() {
        if (i as u32) % effective_interval == 0 {
            keyframe_frame_indices.push(i as u32);
        }
    }

    // 4. Encode all frames to compressed blobs
    let mut compressed_blobs: Vec<Vec<u8>> = Vec::with_capacity(frames.len());
    for (i, frame_particles) in frames.iter().enumerate() {
        let is_keyframe = keyframe_frame_indices.contains(&(i as u32));
        let raw = if is_keyframe || i == 0 {
            encode_i_frame(frame_particles)
        } else {
            encode_p_frame(&frames[i - 1], frame_particles)
        };
        let compressed = zstd::encode_all(Cursor::new(&raw), zstd_level)?;
        compressed_blobs.push(compressed);
    }

    // 5. Calculate offsets
    let mut tex_block_size: usize = 0;
    for tex in textures {
        tex_block_size += 2 + tex.path.as_bytes().len() + 1 + 1;
    }

    let frame_index_table_size = frames.len() * 12;
    let keyframe_count = keyframe_frame_indices.len();
    let keyframe_index_table_size = 4 + keyframe_count * 4;

    let data_start = 48 + tex_block_size + frame_index_table_size + keyframe_index_table_size;

    let mut current_offset = data_start;
    let mut index_entries: Vec<(u64, u32)> = Vec::with_capacity(frames.len());
    for blob in &compressed_blobs {
        index_entries.push((current_offset as u64, blob.len() as u32));
        current_offset += blob.len();
    }

    // 6. Write Frame Index Table
    for (offset, size) in &index_entries {
        f.write_u64::<LittleEndian>(*offset)?;
        f.write_u32::<LittleEndian>(*size)?;
    }

    // 7. Write Keyframe Index Table
    f.write_u32::<LittleEndian>(keyframe_count as u32)?;
    for &kf_idx in &keyframe_frame_indices {
        f.write_u32::<LittleEndian>(kf_idx)?;
    }

    // 8. Write compressed frame data
    for blob in &compressed_blobs {
        f.write_all(blob)?;
    }

    f.flush()?;
    Ok(())
}
