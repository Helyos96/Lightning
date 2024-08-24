pub mod draw_data;
pub mod hover;

use draw_data::*;
use glow::HasContext;
use lightning_model::data::TREE;
use lightning_model::tree::PassiveTree;
use rustc_hash::FxHashMap;
use std::fs::File;

fn load_texture(img: &ddsfile::Dds, gl: &glow::Context) -> glow::Texture {
    unsafe {
        let tex = gl.create_texture().unwrap();

        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.compressed_tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::COMPRESSED_RGBA_BPTC_UNORM as i32,
            img.get_width() as i32,
            img.get_height() as i32,
            0,
            img.data.len() as i32,
            &img.data,
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);

        tex
    }
}

#[derive(Debug)]
struct Texture {
    gl_texture: glow::Texture,
    w: i32,
    h: i32,
}

const VERTEX_SHADER_SOURCE: &str = r#"
#version 330 core

layout(location = 0) in vec2 vertexPosition_modelspace;
layout(location = 1) in vec2 vertexUV;

out vec2 UV;

uniform mat4 ZOOM;

void main() {
    gl_Position = ZOOM * vec4(vertexPosition_modelspace, 0, 1);
    UV = vertexUV;
}
"#;

const FRAGMENT_SHADER_SOURCE: &str = r#"
#version 330 core

in vec2 UV;
out vec4 color;

uniform sampler2D myTextureSampler;
uniform vec4 tint;

void main() {
  color = (texture( myTextureSampler, UV ) * tint).rgba;
}
"#;

#[derive(Default)]
pub struct GlDrawData {
    vao: Option<glow::VertexArray>,
    vbo: Option<glow::Buffer>,
    tbo: Option<glow::Buffer>,
    idx: Option<glow::Buffer>,
    len: i32,
}

impl GlDrawData {
    fn new(gl: &glow::Context, dd: &DrawData) -> Self {
        unsafe {
            let vao = Some(gl.create_vertex_array().unwrap());
            gl.bind_vertex_array(vao);

            // Vertices
            let vbo = Some(gl.create_buffer().unwrap());
            gl.bind_buffer(glow::ARRAY_BUFFER, vbo);
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                std::slice::from_raw_parts(dd.vertices.as_ptr() as *const u8, dd.vertices.len() * 8),
                glow::STATIC_DRAW,
            );
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(0);

            // Texture coords
            let tbo = Some(gl.create_buffer().unwrap());
            gl.bind_buffer(glow::ARRAY_BUFFER, tbo);
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                std::slice::from_raw_parts(dd.tex_coords.as_ptr() as *const u8, dd.vertices.len() * 8),
                glow::STATIC_DRAW,
            );
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(1);

            // Index buffer
            let idx = Some(gl.create_buffer().unwrap());
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, idx);
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                std::slice::from_raw_parts(dd.indices.as_ptr() as *const u8, dd.indices.len() * 2),
                glow::STATIC_DRAW,
            );

            Self {
                vao,
                vbo,
                tbo,
                idx,
                len: dd.indices.len() as i32,
            }
        }
    }

    fn destroy(&mut self, gl: &glow::Context) {
        unsafe {
            if let Some(vao) = self.vao {
                gl.delete_vertex_array(vao);
                self.vao = None;
            }
            if let Some(buffer) = self.vbo {
                gl.delete_buffer(buffer);
                self.vbo = None;
            }
            if let Some(buffer) = self.tbo {
                gl.delete_buffer(buffer);
                self.tbo = None;
            }
            if let Some(buffer) = self.idx {
                gl.delete_buffer(buffer);
                self.idx = None;
            }
        }
    }
}

#[derive(Default)]
pub struct TreeGl {
    textures: FxHashMap<String, Texture>,
    program: Option<glow::Program>,
    uniform_zoom: Option<glow::UniformLocation>,
    uniform_tint: Option<glow::UniformLocation>,
    draw_data: FxHashMap<String, GlDrawData>,
}

impl TreeGl {
    fn init_shaders(&mut self, gl: &glow::Context) {
        let mut shaders = [
            (glow::VERTEX_SHADER, VERTEX_SHADER_SOURCE, None),
            (glow::FRAGMENT_SHADER, FRAGMENT_SHADER_SOURCE, None),
        ];

        unsafe {
            for (kind, source, handle) in &mut shaders {
                let shader = gl.create_shader(*kind).expect("Cannot create shader");
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }
                *handle = Some(shader);
            }

            let program = gl.create_program().expect("Cannot create program");
            gl.attach_shader(program, shaders[0].2.unwrap());
            gl.attach_shader(program, shaders[1].2.unwrap());
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            let uniform_zoom = gl.get_uniform_location(program, "ZOOM").unwrap();
            let uniform_tint = gl.get_uniform_location(program, "tint").unwrap();
            self.program = Some(program);
            self.uniform_zoom = Some(uniform_zoom);
            self.uniform_tint = Some(uniform_tint);

            for &(_, _, shader) in &shaders {
                gl.detach_shader(program, shader.unwrap());
                gl.delete_shader(shader.unwrap());
            }
        }
    }

    pub fn init(&mut self, gl: &glow::Context) {
        let mut textures = FxHashMap::default();

        for sprite in TREE.sprites.values() {
            if textures.contains_key(&sprite.filename) {
                continue;
            }
            let img = ddsfile::Dds::read(File::open("assets/".to_string() + &sprite.filename).unwrap()).unwrap();
            textures.insert(
                sprite.filename.clone(),
                Texture {
                    gl_texture: load_texture(&img, gl),
                    w: img.get_width() as i32,
                    h: img.get_height() as i32,
                },
            );
        }

        self.textures = textures;

        let data = nodes_gl();
        self.draw_data
            .insert("nodes".to_string(), GlDrawData::new(gl, &data[0]));
        self.draw_data
            .insert("frames".to_string(), GlDrawData::new(gl, &data[1]));
        self.draw_data
            .insert("masteries".to_string(), GlDrawData::new(gl, &data[2]));
        self.draw_data
            .insert("ascendancy_frames".to_string(), GlDrawData::new(gl, &data[3]));
        let data = group_background_gl();
        self.draw_data
            .insert("background".to_string(), GlDrawData::new(gl, &data));
        let data = connectors_gl_inactive();
        self.draw_data
            .insert("connectors".to_string(), GlDrawData::new(gl, &data));
        let data = ascendancies_gl();
        self.draw_data
            .insert("ascendancy_background".to_string(), GlDrawData::new(gl, &data));
        self.init_shaders(gl);
    }

    pub fn destroy(&mut self, gl: &glow::Context) {
        for tex in self.textures.values() {
            unsafe {
                gl.delete_texture(tex.gl_texture);
            }
        }
        self.textures.clear();
        for dd in self.draw_data.values_mut() {
            dd.destroy(gl);
        }
        self.draw_data.clear();
    }

    pub fn regen_active(&mut self, gl: &glow::Context, tree: &PassiveTree, path_hovered: &Option<Vec<u16>>, path_red: &Option<Vec<u16>>,) {
        const REDRAW: [&str; 12] = [
            "nodes_active",
            "frames_active",
            "masteries_active",
            "masteries_active_selected",
            "ascendancy_frames_active",
            "connectors_active",
            "connectors_hovered",
            "class_start",
            "connectors_red",
            "nodes_active_red",
            "frames_active_red",
            "ascendancy_frames_active_red",
        ];

        for &s in &REDRAW {
            if let Some(dd) = self.draw_data.get_mut(s) {
                dd.destroy(gl);
            }
        }

        let last_node = path_hovered.as_ref().map(|path| path.first().unwrap());
        let data = nodes_gl_active(&tree.nodes, last_node);
        self.draw_data
            .insert("nodes_active".to_string(), GlDrawData::new(gl, &data[0]));
        self.draw_data
            .insert("frames_active".to_string(), GlDrawData::new(gl, &data[1]));
        self.draw_data
            .insert("masteries_active".to_string(), GlDrawData::new(gl, &data[2]));
        self.draw_data
            .insert("masteries_active_selected".to_string(), GlDrawData::new(gl, &data[3]));
        self.draw_data
            .insert("ascendancy_frames_active".to_string(), GlDrawData::new(gl, &data[4]));
        self.draw_data
            .insert("connectors_active".to_string(), GlDrawData::new(gl, &connectors_gl(&tree.nodes, &TREE.sprites["line"].coords["LineConnectorActive"], 20.0)));
        self.draw_data
            .insert("class_start".to_string(), GlDrawData::new(gl, &class_start_gl(tree.class)));
        if let Some(path) = path_hovered {
            let data = connectors_gl(path, &TREE.sprites["line"].coords["LineConnectorActive"], 8.0);
            self.draw_data
                .insert("connectors_hovered".to_string(), GlDrawData::new(gl, &data));
        } else {
            self.draw_data.remove("connectors_hovered");
        }
        if let Some(path_red) = path_red {
            self.draw_data
                .insert("connectors_red".to_string(), GlDrawData::new(gl, &connectors_gl(path_red, &TREE.sprites["line"].coords["LineConnectorActive"], 20.0)));
            let data = nodes_gl_active(path_red, None);
            self.draw_data
                .insert("nodes_active_red".to_string(), GlDrawData::new(gl, &data[0]));
            self.draw_data
                .insert("frames_active_red".to_string(), GlDrawData::new(gl, &data[1]));
            self.draw_data
                .insert("ascendancy_frames_active_red".to_string(), GlDrawData::new(gl, &data[4]));
        }
    }

    pub fn draw(
        &mut self,
        tree: &PassiveTree,
        gl: &glow::Context,
        zoom: f32,
        translate: (f32, f32),
        path_hovered: &Option<Vec<u16>>,
        path_red: &Option<Vec<u16>>,
    ) {
        if !self.draw_data.contains_key("nodes_active") {
            self.regen_active(gl, tree, path_hovered, path_red);
        }

        // draw_data name ; texture file ; color tint factor
        const DRAW_ORDER: [(&str, &str, [f32; 4]); 17] = [
            ("background", "group-background-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("ascendancy_background", "ascendancy-background-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("connectors", "line-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("connectors_active", "line-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("connectors_hovered", "line-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("connectors_red", "line-3.dds", [1.0, 0.0, 0.0, 1.0]),
            ("nodes", "skills-disabled-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("nodes_active", "skills-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("frames", "frame-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("frames_active", "frame-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("frames_active_red", "frame-3.dds", [1.0, 0.0, 0.0, 1.0]),
            ("class_start", "group-background-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("ascendancy_frames", "ascendancy-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("ascendancy_frames_active", "ascendancy-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("masteries", "mastery-disabled-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("masteries_active", "mastery-connected-3.dds", [1.0, 1.0, 1.0, 1.0]),
            ("masteries_active_selected", "mastery-active-selected-3.dds", [1.0, 1.0, 1.0, 1.0]),
        ];

        unsafe {
            let mut viewport = [0; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
            let aspect_ratio = viewport[2] as f32 / viewport[3] as f32;
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            
            let scale = glam::Mat4::from_scale(glam::Vec3::new(zoom, zoom, 0.0));
            let ortho = glam::Mat4::orthographic_rh_gl(-aspect_ratio, aspect_ratio, -1.0, 1.0, -1.0, 1.0);
            let translate = glam::Mat4::from_translation(glam::Vec3::new(
                translate.0 as f32 / 12500.0,
                translate.1 as f32 / 12500.0,
                0.0,
            ));

            gl.use_program(self.program);
            gl.uniform_matrix_4_f32_slice(
                self.uniform_zoom.as_ref(),
                false,
                &(scale * ortho * translate).to_cols_array(),
            );

            for to_draw in DRAW_ORDER.iter().filter(|d| self.draw_data.contains_key(d.0)) {
                gl.uniform_4_f32(self.uniform_tint.as_ref(), to_draw.2[0], to_draw.2[1], to_draw.2[2], to_draw.2[3]);
                gl.bind_vertex_array(self.draw_data[to_draw.0].vao);
                gl.bind_texture(glow::TEXTURE_2D, Some(self.textures[to_draw.1].gl_texture));
                gl.draw_elements(glow::TRIANGLES, self.draw_data[to_draw.0].len, glow::UNSIGNED_SHORT, 0);
            }
        }
    }
}
