use lightning_model::data::TREE;
use lightning_model::build::Build;
use lightning_model::tree;
use lazy_static::lazy_static;
use rustc_hash::FxHashMap;
use crate::gui::State;
use glow::HasContext;
use std::fs::File;
use std::ops::Neg;

fn calc_angles() -> Vec<Vec<f32>> {
    let mut ret = vec![];
    for skills in &TREE.constants.skills_per_orbit {
        ret.push({
            let angles = match skills {
                16 => vec![0, 30, 45, 60, 90, 120, 135, 150, 180, 210, 225, 240, 270, 300, 315, 330],
                40 => vec![0, 10, 20, 30, 40, 45, 50, 60, 70, 80, 90, 100, 110, 120, 130, 135, 140, 150, 160, 170, 180, 190, 200, 210, 220, 225, 230, 240, 250, 260, 270, 280, 290, 300, 310, 315, 320, 330, 340, 350],
                n => (0..*n).into_iter().map(|i| (360 * i) / n).collect(), 
            };
            angles.into_iter().map(|a| (a as f32).to_radians()).collect()
        });
    }
    ret
}

#[derive(Copy,Clone,Eq,PartialEq)]
enum NodeType {
    Normal,
    Notable,
    Keystone,
    Mastery,
}

fn get_rect(mut icon: &str, typ: NodeType) -> Option<(&'static tree::Rect, &'static tree::Sprite)> {
    let key = match typ {
        NodeType::Normal => "normalActive",
        NodeType::Notable => "notableActive",
        NodeType::Keystone => "keystoneActive",
        NodeType::Mastery => "masteryConnected",
    };
    let sprite = &TREE.sprites[key];
    let rect = sprite.coords.get(icon)?;
    Some((rect, sprite))
}

fn append_to(x: f32, y: f32, rect: &tree::Rect, sprite: &tree::Sprite, vertices: &mut Vec<(f32, f32)>, tex_coords: &mut Vec<(f32, f32)>, indices: &mut Vec<u16>, vflip: bool) {
    vertices.extend([
        norm(x - rect.w as f32 / 2.0, y - rect.h as f32 / 2.0), // Bottom Left
        norm(x - rect.w as f32 / 2.0, y + rect.h as f32 / 2.0), // Top Left
        norm(x + rect.w as f32 / 2.0, y + rect.h as f32 / 2.0), // Top Right
        norm(x + rect.w as f32 / 2.0, y - rect.h as f32 / 2.0), // Bottom Right
    ]);

    if vflip {
        tex_coords.extend([
            norm_tex(rect.x as f32, rect.y as f32, sprite.w as f32, sprite.h as f32),
            norm_tex(rect.x as f32, (rect.y + rect.h) as f32, sprite.w as f32, sprite.h as f32),
            norm_tex((rect.x + rect.w) as f32, (rect.y + rect.h) as f32, sprite.w as f32, sprite.h as f32),
            norm_tex((rect.x + rect.w) as f32, rect.y as f32, sprite.w as f32, sprite.h as f32),
        ]);
    } else {
        tex_coords.extend([
            norm_tex(rect.x as f32, (rect.y + rect.h) as f32, sprite.w as f32, sprite.h as f32),
            norm_tex(rect.x as f32, rect.y as f32, sprite.w as f32, sprite.h as f32),
            norm_tex((rect.x + rect.w) as f32, rect.y as f32, sprite.w as f32, sprite.h as f32),
            norm_tex((rect.x + rect.w) as f32, (rect.y + rect.h) as f32, sprite.w as f32, sprite.h as f32),
        ]);
    }

    let start = vertices.len() as u16 - 4;
    indices.extend([start, start + 1, start + 2, start + 3, start, start + 2]);
}

fn nodes_gl() -> [(Vec<(f32,f32)>, Vec<(f32,f32)>, Vec<u16>); 3] {
    let mut vertices = vec![];
    let mut tex_coords = vec![];
    let mut indices = vec![];
    let mut vertices_frames = vec![];
    let mut tex_coords_frames = vec![];
    let mut indices_frames = vec![];
    let mut vertices_masteries = vec![];
    let mut tex_coords_masteries = vec![];
    let mut indices_masteries = vec![];
    let orbit_angles = calc_angles();

    for node in TREE.nodes.values().filter(|n| n.group.is_some()) {
        let typ = {
            if node.is_notable {
                NodeType::Notable
            } else if node.is_keystone {
                NodeType::Keystone
            } else if node.is_mastery {
                NodeType::Mastery
            } else {
                NodeType::Normal
            }
        };
        let icon = match typ {
            NodeType::Mastery => node.inactive_icon.as_ref().unwrap(),
            _ => &node.icon,
        };
        let (rect,sprite) = match get_rect(icon, typ) {
            None => { println!("No rect for {}", icon); continue },
            Some(res) => res,
        };
        let group = node.group.unwrap();
        let orbit = node.orbit.unwrap() as usize;
        let angle = orbit_angles[orbit][node.orbit_index.unwrap() as usize];
        let orbit_radius = TREE.constants.orbit_radii[orbit];

        let x = TREE.groups[&group].x + (angle.sin() * orbit_radius as f32) + TREE.min_x.abs() as f32;
        let y = TREE.groups[&group].y.neg() + (angle.cos() * orbit_radius as f32) + TREE.min_y.abs() as f32;

        if typ == NodeType::Mastery {
            append_to(x, y, &rect, &sprite, &mut vertices_masteries, &mut tex_coords_masteries, &mut indices_masteries, false);
        } else {
            append_to(x, y, &rect, &sprite, &mut vertices, &mut tex_coords, &mut indices, false);
            let sprite = &TREE.sprites["frame"];
            let rect = match typ {
                NodeType::Normal => &sprite.coords["PSSkillFrame"],
                NodeType::Notable => &sprite.coords["NotableFrameUnallocated"],
                NodeType::Keystone => &sprite.coords["KeystoneFrameUnallocated"],
                NodeType::Mastery => panic!("No frame for masteries"),
            };
            append_to(x, y, &rect, &sprite, &mut vertices_frames, &mut tex_coords_frames, &mut indices_frames, false);
        }

        if let Some(out) = &node.out {
            for _out_id in out {
                // todo connectors
            }
        }
    }
    [
        (vertices, tex_coords, indices),
        (vertices_frames, tex_coords_frames, indices_frames),
        (vertices_masteries, tex_coords_masteries, indices_masteries),
    ]
}

fn group_background_gl() -> (Vec<(f32,f32)>, Vec<(f32,f32)>, Vec<u16>) {
    let mut vertices = vec![];
    let mut tex_coords = vec![];
    let mut indices = vec![];
    let sprite = &TREE.sprites["groupBackground"];
    for group in TREE.groups.values().filter(|g| g.background.is_some()) {
        let background = group.background.as_ref().unwrap();
        let rect = match sprite.coords.get(&background.image) {
            None => continue,
            Some(res) => res,
        };

        let x = group.x + TREE.min_x.abs() as f32;
        let mut y = group.y.neg() + TREE.min_y.abs() as f32;
        if background.is_half_image.is_some() {
            // Need to draw upper half and then bottom half (vertically flipped)
            y += rect.h as f32 / 2.0;
            append_to(x, y, &rect, &sprite, &mut vertices, &mut tex_coords, &mut indices, false);
            y -= rect.h as f32;
            append_to(x, y, &rect, &sprite, &mut vertices, &mut tex_coords, &mut indices, true);
        } else {
            append_to(x, y, &rect, &sprite, &mut vertices, &mut tex_coords, &mut indices, false);
        }
    }
    (vertices, tex_coords, indices)
}

/// Normalize tree coords to GL normalized coords
fn norm(mut x: f32, mut y: f32) -> (f32, f32) {
    x /= 12500.0;
    y /= 12500.0;
    x -= 1.0;
    y -= 1.0;
    (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0))
}

/// Normalize sprite coords to GL texture coords
fn norm_tex(mut x: f32, mut y: f32, w: f32, h: f32) -> (f32, f32) {
    x /= w;
    y /= h;
    (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))
}

fn load_texture(img: &ddsfile::Dds, gl: &glow::Context) -> glow::Texture {
    unsafe {
        let tex = gl.create_texture().unwrap();

        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.compressed_tex_image_2d(glow::TEXTURE_2D, 0, glow::COMPRESSED_RGBA_BPTC_UNORM as i32, img.get_width() as i32, img.get_height() as i32, 0, img.data.len() as i32, &img.data);
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

void main() {
  color = texture( myTextureSampler, UV ).rgba;
}
"#;

#[derive(Default)]
pub struct DrawData {
    vao: Option<glow::VertexArray>,
    vbo: Option<glow::Buffer>,
    tbo: Option<glow::Buffer>,
    idx: Option<glow::Buffer>,
    len: i32,
}

impl DrawData {
    fn new(gl: &glow::Context, vertices: &[(f32,f32)], tex_coords: &[(f32,f32)], indices: &[u16]) -> DrawData {
        unsafe {
            let vao = Some(gl.create_vertex_array().unwrap());
            gl.bind_vertex_array(vao);

            // Vertices
            let vbo = Some(gl.create_buffer().unwrap());
            gl.bind_buffer(glow::ARRAY_BUFFER, vbo);
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                std::slice::from_raw_parts(vertices.as_ptr() as *const u8, vertices.len() * 8),
                glow::STATIC_DRAW
            );
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(0);

            // Texture coords
            let tbo = Some(gl.create_buffer().unwrap());
            gl.bind_buffer(glow::ARRAY_BUFFER, tbo);
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                std::slice::from_raw_parts(tex_coords.as_ptr() as *const u8, vertices.len() * 8),
                glow::STATIC_DRAW
            );
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 0, 0);
            gl.enable_vertex_attrib_array(1);

            // Index buffer
            let idx = Some(gl.create_buffer().unwrap());
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, idx);
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                std::slice::from_raw_parts(indices.as_ptr() as *const u8, indices.len() * 2),
                glow::STATIC_DRAW
            );

            DrawData { vao, vbo, tbo, idx, len: indices.len() as i32 }
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
    draw_data: FxHashMap<String, DrawData>,
}

impl TreeGl {
    fn init_shaders(&mut self, gl: &glow::Context) {
        let mut shaders = [
            (glow::VERTEX_SHADER, VERTEX_SHADER_SOURCE, 0),
            (glow::FRAGMENT_SHADER, FRAGMENT_SHADER_SOURCE, 0),
        ];

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            for (kind, source, handle) in &mut shaders {
                let shader = gl.create_shader(*kind).expect("Cannot create shader");
                gl.shader_source(shader, source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                *handle = shader;
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for &(_, _, shader) in &shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            self.uniform_zoom = gl.get_uniform_location(program, "ZOOM");
            self.program = Some(program);
        }
    }

    pub fn init(&mut self, gl: &glow::Context) {
        let mut textures = FxHashMap::default();

        for sprite in TREE.sprites.values() {
            if textures.contains_key(&sprite.filename) {
                continue;
            }
            let img = ddsfile::Dds::read(File::open("assets/".to_string() + &sprite.filename).unwrap()).unwrap();
            textures.insert(sprite.filename.clone(), Texture {
                gl_texture: load_texture(&img, gl),
                w: img.get_width() as i32,
                h: img.get_height() as i32,
            });
        }

        self.textures = textures;

        let data = nodes_gl();
        self.draw_data.insert("nodes".to_string(), DrawData::new(gl, &data[0].0, &data[0].1, &data[0].2));
        self.draw_data.insert("frames".to_string(), DrawData::new(gl, &data[1].0, &data[1].1, &data[1].2));
        self.draw_data.insert("masteries".to_string(), DrawData::new(gl, &data[2].0, &data[2].1, &data[2].2));
        let (vertices, tex_coords, indices) = group_background_gl();
        self.draw_data.insert("background".to_string(), DrawData::new(gl, &vertices, &tex_coords, &indices));
        self.init_shaders(gl);
    }

    pub fn destroy(&mut self, gl: &glow::Context) {
        for tex in self.textures.values() {
            unsafe { gl.delete_texture(tex.gl_texture); }
        }
        self.textures.clear();
        // todo destroy buffers
    }

    pub fn draw(&mut self/*, state: &State*/, gl: &glow::Context, zoom: f32, translate: (i32, i32)) {
        let draw_order = [
            ("background", "group-background-3.dds"),
            ("nodes", "skills-3.dds"),
            ("frames", "frame-3.dds"),
            ("masteries", "mastery-connected-3.dds"),
        ];
        /*// Uncomment block to recompute draw buffers every frame
        for dd in self.draw_data.values_mut() {
            dd.destroy(gl);
        }
        let data = nodes_gl();
        self.draw_data.insert("nodes".to_string(), DrawData::new(gl, &data[0].0, &data[0].1, &data[0].2));
        self.draw_data.insert("frames".to_string(), DrawData::new(gl, &data[1].0, &data[1].1, &data[1].2));
        let (vertices, tex_coords, indices) = group_background_gl();
        self.draw_data.insert("background".to_string(), DrawData::new(gl, &vertices, &tex_coords, &indices));*/
        unsafe {
            let mut viewport = [0; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
            let aspect_ratio = viewport[2] as f32 / viewport[3] as f32;
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.use_program(self.program);
            let scale = glam::Mat4::from_scale(glam::Vec3::new(zoom, zoom, 0.0));
            let ortho = glam::Mat4::orthographic_rh_gl(-aspect_ratio, aspect_ratio, -1.0, 1.0, -1.0, 1.0);
            let translate = glam::Mat4::from_translation(glam::Vec3::new(translate.0 as f32 / 12500.0, translate.1 as f32 / 12500.0, 0.0));
            gl.uniform_matrix_4_f32_slice(self.uniform_zoom.as_ref(), false, &(scale * ortho * translate).to_cols_array());

            for to_draw in draw_order {
                gl.bind_vertex_array(self.draw_data[to_draw.0].vao);
                gl.bind_texture(glow::TEXTURE_2D, Some(self.textures[to_draw.1].gl_texture));
                gl.draw_elements(glow::TRIANGLES, self.draw_data[to_draw.0].len, glow::UNSIGNED_SHORT, 0);
            }
        }
    }
}

