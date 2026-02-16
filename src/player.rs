use anyhow::{anyhow, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;

const MAGIC: &[u8; 8] = b"NEBULAFX";

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

#[derive(Debug, Clone)]
pub struct TextureEntry {
    pub path: String,
    pub rows: u8,
    pub cols: u8,
}

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
}
