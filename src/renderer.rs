use eframe::glow::{self, HasContext};

pub struct ParticleRenderer {
    program: glow::Program,
    line_program: glow::Program,
    vbo: glow::Buffer,
    vao: glow::VertexArray,
    line_vbo: glow::Buffer,
    line_vao: glow::VertexArray,
}

impl ParticleRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            // --- Particle Shader ---
            let program = create_program(
                gl,
                r#"#version 330 core
                layout (location = 0) in vec3 a_pos;
                layout (location = 1) in vec4 a_color;
                layout (location = 2) in float a_size;
                uniform mat4 u_mvp;
                uniform float u_scaling;
                out vec4 v_color;
                void main() {
                    gl_Position = u_mvp * vec4(a_pos, 1.0);
                    // 核心修复：确保计算出的点大小至少为 1.2 像素
                    gl_PointSize = max((a_size * u_scaling) / gl_Position.w, 1.2);
                    v_color = a_color;
                }"#,
                r#"#version 330 core
                in vec4 v_color;
                out vec4 f_color;
                void main() {
                    float dist = distance(gl_PointCoord, vec2(0.5));
                    if (dist > 0.5) discard;
                    f_color = v_color;
                }"#,
            );

            // --- Line Shader (for Grid/Axes) ---
            let line_program = create_program(
                gl,
                r#"#version 330 core
                layout (location = 0) in vec3 a_pos;
                layout (location = 1) in vec4 a_color;
                uniform mat4 u_mvp;
                out vec4 v_color;
                void main() {
                    gl_Position = u_mvp * vec4(a_pos, 1.0);
                    v_color = a_color;
                }"#,
                r#"#version 330 core
                in vec4 v_color;
                out vec4 f_color;
                void main() {
                    f_color = v_color;
                }"#,
            );

            let vbo = gl.create_buffer().unwrap();
            let vao = gl.create_vertex_array().unwrap();
            let line_vbo = gl.create_buffer().unwrap();
            let line_vao = gl.create_vertex_array().unwrap();

            Self {
                program,
                line_program,
                vbo,
                vao,
                line_vbo,
                line_vao,
            }
        }
    }

    pub unsafe fn paint(
        &self,
        gl: &glow::Context,
        mvp: [f32; 16],
        particles: &[f32],
        scaling: f32,
        grid_enabled: bool,
    ) {
        gl.enable(glow::DEPTH_TEST);
        gl.enable(glow::BLEND);
        gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        gl.enable(glow::PROGRAM_POINT_SIZE);

        // 1. Draw Grid & Axes
        if grid_enabled {
            self.draw_grid_and_axes(gl, mvp);
        }

        // 2. Draw Particles
        if !particles.is_empty() {
            gl.use_program(Some(self.program));
            let mvp_loc = gl.get_uniform_location(self.program, "u_mvp");
            gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, &mvp);
            let scale_loc = gl.get_uniform_location(self.program, "u_scaling");
            gl.uniform_1_f32(scale_loc.as_ref(), scaling);

            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(particles),
                glow::DYNAMIC_DRAW,
            );

            // Pos
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 8 * 4, 0);
            // Color
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 8 * 4, 3 * 4);
            // Size
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 1, glow::FLOAT, false, 8 * 4, 7 * 4);

            gl.draw_arrays(glow::POINTS, 0, (particles.len() / 8) as i32);
        }

        // 3. Draw Compass (Direction Indicator) in corner
        self.draw_compass(gl, mvp);
    }

    unsafe fn draw_compass(&self, gl: &glow::Context, mvp: [f32; 16]) {
        // We create a small MVP for the compass
        let mut compass_mvp = mvp;
        // Zero out translation to keep it rotated but centered
        compass_mvp[12] = 0.85; // Move to bottom right
        compass_mvp[13] = -0.85;
        compass_mvp[14] = 0.0;

        // Scale down the rotation part
        for i in 0..11 {
            if i % 4 != 3 {
                compass_mvp[i] *= 0.1;
            }
        }

        let mut lines = Vec::new();
        let len = 1.0;
        // X - Red
        lines.extend_from_slice(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
        lines.extend_from_slice(&[len, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
        // Y - Green
        lines.extend_from_slice(&[0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0]);
        lines.extend_from_slice(&[0.0, len, 0.0, 0.0, 1.0, 0.0, 1.0]);
        // Z - Blue
        lines.extend_from_slice(&[0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0]);
        lines.extend_from_slice(&[0.0, 0.0, len, 0.0, 0.0, 1.0, 1.0]);

        gl.use_program(Some(self.line_program));
        let loc = gl.get_uniform_location(self.line_program, "u_mvp");
        gl.uniform_matrix_4_f32_slice(loc.as_ref(), false, &compass_mvp);

        gl.disable(glow::DEPTH_TEST);
        gl.bind_vertex_array(Some(self.line_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.line_vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&lines),
            glow::STREAM_DRAW,
        );
        gl.draw_arrays(glow::LINES, 0, 6);
    }

    unsafe fn draw_grid_and_axes(&self, gl: &glow::Context, mvp: [f32; 16]) {
        let mut lines = Vec::new();

        // Grid (XZ plane)
        let grid_size = 10;
        let step = 1.0;
        let grid_color = [0.3, 0.3, 0.3, 0.5];

        for i in -grid_size..=grid_size {
            let f = i as f32 * step;
            // X lines
            lines.extend_from_slice(&[f, 0.0, -grid_size as f32 * step]);
            lines.extend_from_slice(&grid_color);
            lines.extend_from_slice(&[f, 0.0, grid_size as f32 * step]);
            lines.extend_from_slice(&grid_color);
            // Z lines
            lines.extend_from_slice(&[-grid_size as f32 * step, 0.0, f]);
            lines.extend_from_slice(&grid_color);
            lines.extend_from_slice(&[grid_size as f32 * step, 0.0, f]);
            lines.extend_from_slice(&grid_color);
        }

        // Axes
        let axis_len = 5.0;
        // X - Red
        lines.extend_from_slice(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
        lines.extend_from_slice(&[axis_len, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]);
        // Y - Green
        lines.extend_from_slice(&[0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0]);
        lines.extend_from_slice(&[0.0, axis_len, 0.0, 0.0, 1.0, 0.0, 1.0]);
        // Z - Blue
        lines.extend_from_slice(&[0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0]);
        lines.extend_from_slice(&[0.0, 0.0, axis_len, 0.0, 0.0, 1.0, 1.0]);

        gl.use_program(Some(self.line_program));
        let mvp_loc = gl.get_uniform_location(self.line_program, "u_mvp");
        gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, &mvp);

        gl.bind_vertex_array(Some(self.line_vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.line_vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&lines),
            glow::STREAM_DRAW,
        );

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 7 * 4, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, 7 * 4, 3 * 4);

        gl.draw_arrays(glow::LINES, 0, (lines.len() / 7) as i32);
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_program(self.line_program);
            gl.delete_buffer(self.vbo);
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.line_vbo);
            gl.delete_vertex_array(self.line_vao);
        }
    }
}

unsafe fn create_program(gl: &glow::Context, v_src: &str, f_src: &str) -> glow::Program {
    let program = gl.create_program().expect("Cannot create program");
    let vs = gl
        .create_shader(glow::VERTEX_SHADER)
        .expect("Cannot create vs");
    gl.shader_source(vs, v_src);
    gl.compile_shader(vs);
    if !gl.get_shader_compile_status(vs) {
        panic!("VS Error: {}", gl.get_shader_info_log(vs));
    }

    let fs = gl
        .create_shader(glow::FRAGMENT_SHADER)
        .expect("Cannot create fs");
    gl.shader_source(fs, f_src);
    gl.compile_shader(fs);
    if !gl.get_shader_compile_status(fs) {
        panic!("FS Error: {}", gl.get_shader_info_log(fs));
    }

    gl.attach_shader(program, vs);
    gl.attach_shader(program, fs);
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!("Link Error: {}", gl.get_program_info_log(program));
    }

    gl.delete_shader(vs);
    gl.delete_shader(fs);
    program
}
