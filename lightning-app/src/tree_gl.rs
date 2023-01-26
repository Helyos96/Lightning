use crate::gui::State;
use glow::HasContext;
use lazy_static::lazy_static;
use lightning_model::build::Build;
use lightning_model::data::TREE;
use lightning_model::tree::{self, Node, NodeType, PassiveTree, Rect, Sprite};
use rustc_hash::FxHashMap;
use std::fs::File;
use std::ops::Neg;

fn calc_angles() -> Vec<Vec<f32>> {
    let mut ret = vec![];
    for skills in &TREE.constants.skills_per_orbit {
        ret.push({
            let angles = match skills {
                16 => vec![0, 30, 45, 60, 90, 120, 135, 150, 180, 210, 225, 240, 270, 300, 315, 330],
                40 => vec![
                    0, 10, 20, 30, 40, 45, 50, 60, 70, 80, 90, 100, 110, 120, 130, 135, 140, 150, 160, 170, 180, 190,
                    200, 210, 220, 225, 230, 240, 250, 260, 270, 280, 290, 300, 310, 315, 320, 330, 340, 350,
                ],
                n => (0..*n).into_iter().map(|i| (360 * i) / n).collect(),
            };
            angles.into_iter().map(|a| (a as f32).to_radians()).collect()
        });
    }
    ret
}

lazy_static! {
    static ref ORBIT_ANGLES: Vec<Vec<f32>> = calc_angles();
}

fn node_pos(node: &Node) -> (f32, f32) {
    let group = node.group.unwrap();
    let orbit = node.orbit.unwrap() as usize;
    let angle = ORBIT_ANGLES[orbit][node.orbit_index.unwrap() as usize];
    let orbit_radius = TREE.constants.orbit_radii[orbit];

    (
        TREE.groups[&group].x + (angle.sin() * orbit_radius as f32) + TREE.min_x.abs() as f32,
        TREE.groups[&group].y.neg() + (angle.cos() * orbit_radius as f32) + TREE.min_y.abs() as f32,
    )
}

/// Normalize tree coords to GL normalized coords
fn norm(x: f32, y: f32) -> (f32, f32) {
    (x / 12500.0 - 1.0, y / 12500.0 - 1.0)
}

/// Normalize sprite coords to GL texture coords
fn norm_tex(x: u16, y: u16, w: u16, h: u16) -> (f32, f32) {
    (x as f32 / w as f32, y as f32 / h as f32)
}

fn get_rect(node: &Node) -> Option<(&'static tree::Rect, &'static tree::Sprite)> {
    let (key, icon): (&str, &str) = match node.node_type() {
        NodeType::Normal | NodeType::AscendancyNormal => ("normalInactive", &node.icon),
        NodeType::Notable | NodeType::AscendancyNotable => ("notableInactive", &node.icon),
        NodeType::Keystone => ("keystoneInactive", &node.icon),
        NodeType::Mastery => ("masteryConnected", node.inactive_icon.as_ref().unwrap()),
    };
    let sprite = &TREE.sprites[key];
    let rect = sprite.coords.get(icon)?;
    Some((rect, sprite))
}

#[derive(Default)]
struct DrawData {
    vertices: Vec<(f32, f32)>,
    tex_coords: Vec<(f32, f32)>,
    indices: Vec<u16>,
}

impl DrawData {
    fn append(&mut self, x: f32, y: f32, rect: &tree::Rect, sprite: &tree::Sprite, vflip: bool, scale: f32) {
        self.vertices.extend([
            norm(x - (rect.w as f32 * scale) / 2.0, y - (rect.h as f32 * scale) / 2.0), // Bottom Left
            norm(x - (rect.w as f32 * scale) / 2.0, y + (rect.h as f32 * scale) / 2.0), // Top Left
            norm(x + (rect.w as f32 * scale) / 2.0, y + (rect.h as f32 * scale) / 2.0), // Top Right
            norm(x + (rect.w as f32 * scale) / 2.0, y - (rect.h as f32 * scale) / 2.0), // Bottom Right
        ]);

        if vflip {
            self.tex_coords.extend([
                norm_tex(rect.x, rect.y, sprite.w, sprite.h),
                norm_tex(rect.x, rect.y + rect.h, sprite.w, sprite.h),
                norm_tex(rect.x + rect.w, rect.y + rect.h, sprite.w, sprite.h),
                norm_tex(rect.x + rect.w, rect.y, sprite.w, sprite.h),
            ]);
        } else {
            self.tex_coords.extend([
                norm_tex(rect.x, rect.y + rect.h, sprite.w, sprite.h),
                norm_tex(rect.x, rect.y, sprite.w, sprite.h),
                norm_tex(rect.x + rect.w, rect.y, sprite.w, sprite.h),
                norm_tex(rect.x + rect.w, rect.y + rect.h, sprite.w, sprite.h),
            ]);
        }

        let start = self.vertices.len() as u16 - 4;
        self.indices
            .extend([start, start + 1, start + 2, start + 3, start, start + 2]);
    }
}

fn connector_gl(x1: f32, y1: f32, x2: f32, y2: f32, rect: &Rect, sprite: &Sprite, dd: &mut DrawData) {
    dd.vertices.extend([
        // todo: better than this +5 / -5. Some angles don't render.
        norm(x1 - 5.0, y1 + 5.0),
        norm(x1 + 5.0, y1 - 5.0),
        norm(x2 + 5.0, y2 - 5.0),
        norm(x2 - 5.0, y2 + 5.0),
    ]);
    dd.tex_coords.extend([
        norm_tex(rect.x, rect.y + rect.h, sprite.w, sprite.h),
        norm_tex(rect.x, rect.y, sprite.w, sprite.h),
        norm_tex(rect.x + rect.w, rect.y, sprite.w, sprite.h),
        norm_tex(rect.x + rect.w, rect.y + rect.h, sprite.w, sprite.h),
    ]);

    let start = dd.vertices.len() as u16 - 4;
    dd.indices
        .extend([start, start + 1, start + 2, start + 3, start, start + 2]);
}

/// Very simple straight connectors. todo: arcs
fn connectors_gl() -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["line"];
    let rect = &sprite.coords["LineConnectorNormal"];

    for node in TREE
        .nodes
        .values()
        .filter(|n| n.group.is_some() && !n.name.starts_with("Path of the") && n.class_start_index.is_none())
    {
        let (x1, y1) = node_pos(node);
        for out in node
            .out
            .iter()
            .flatten()
            .map(|id| &TREE.nodes[id])
            .filter(|n| !n.is_ascendancy_start && !n.is_mastery && n.class_start_index.is_none())
        {
            let (x2, y2) = node_pos(out);
            connector_gl(x1, y1, x2, y2, rect, sprite, &mut dd);
        }
    }
    dd
}

fn connectors_gl_active(nodes: &[u16]) -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["line"];
    let rect = &sprite.coords["LineConnectorActive"];

    for node in nodes
        .iter()
        .map(|id| &TREE.nodes[id])
        .filter(|n| n.group.is_some() && !n.name.starts_with("Path of the") && n.class_start_index.is_none())
    {
        let (x1, y1) = node_pos(node);
        for out in node
            .out
            .iter()
            .flatten()
            .filter(|id| nodes.contains(id))
            .map(|id| &TREE.nodes[id])
            .filter(|n| !n.is_ascendancy_start && !n.is_mastery && n.class_start_index.is_none())
        {
            let (x2, y2) = node_pos(out);
            connector_gl(x1, y1, x2, y2, rect, sprite, &mut dd);
        }
    }
    dd
}
const ACTIVE_STRINGS: [&str; 5] = [
    "AscendancyFrameSmallAllocated",
    "AscendancyFrameLargeAllocated",
    "PSSkillFrameActive",
    "NotableFrameAllocated",
    "KeystoneFrameAllocated",
];

const INACTIVE_STRINGS: [&str; 5] = [
    "AscendancyFrameSmallNormal",
    "AscendancyFrameLargeNormal",
    "PSSkillFrame",
    "NotableFrameUnallocated",
    "KeystoneFrameUnallocated",
];

fn node_gl(
    node: &Node,
    dd_nodes: &mut DrawData,
    dd_frames: &mut DrawData,
    dd_masteries: &mut DrawData,
    dd_asc_frames: &mut DrawData,
    is_active: bool,
) {
    let icon_strings = match is_active {
        true => &ACTIVE_STRINGS,
        false => &INACTIVE_STRINGS,
    };

    let (rect, sprite) = match get_rect(node) {
        None => {
            println!("No rect for node {}", node.name);
            return;
        }
        Some(res) => res,
    };

    let (x, y) = node_pos(node);

    match node.node_type() {
        NodeType::Mastery => {
            dd_masteries.append(x, y, rect, sprite, false, 1.0);
        }
        NodeType::AscendancyNormal | NodeType::AscendancyNotable => {
            dd_nodes.append(x, y, rect, sprite, false, 2.0);
            let sprite = &TREE.sprites["ascendancy"];
            let rect = match node.node_type() {
                NodeType::AscendancyNormal => &sprite.coords[icon_strings[0]],
                NodeType::AscendancyNotable => &sprite.coords[icon_strings[1]],
                _ => panic!("No frame"),
            };
            dd_asc_frames.append(x, y, rect, sprite, false, 2.0);
        }
        _ => {
            dd_nodes.append(x, y, rect, sprite, false, 1.0);
            let sprite = &TREE.sprites["frame"];
            let rect = match node.node_type() {
                NodeType::Normal => &sprite.coords[icon_strings[2]],
                NodeType::Notable => &sprite.coords[icon_strings[3]],
                NodeType::Keystone => &sprite.coords[icon_strings[4]],
                _ => panic!("No frame"),
            };
            dd_frames.append(x, y, rect, sprite, false, 1.0);
        }
    }
}
/// Nodes, Frames and Masteries
fn nodes_gl() -> [DrawData; 4] {
    let mut dd_nodes = DrawData::default();
    let mut dd_frames = DrawData::default();
    let mut dd_masteries = DrawData::default();
    let mut dd_asc_frames = DrawData::default();

    for node in TREE
        .nodes
        .values()
        .filter(|n| n.group.is_some() && n.class_start_index.is_none())
    {
        node_gl(
            node,
            &mut dd_nodes,
            &mut dd_frames,
            &mut dd_masteries,
            &mut dd_asc_frames,
            false,
        );
    }
    [dd_nodes, dd_frames, dd_masteries, dd_asc_frames]
}

/// Player-selected Nodes, Frames and Masteries
fn nodes_gl_active(tree: &PassiveTree) -> [DrawData; 4] {
    let mut dd_nodes = DrawData::default();
    let mut dd_frames = DrawData::default();
    let mut dd_masteries = DrawData::default();
    let mut dd_asc_frames = DrawData::default();

    for node in tree.nodes.iter().map(|id| &TREE.nodes[id]) {
        node_gl(
            node,
            &mut dd_nodes,
            &mut dd_frames,
            &mut dd_masteries,
            &mut dd_asc_frames,
            true,
        );
    }

    [dd_nodes, dd_frames, dd_masteries, dd_asc_frames]
}

fn group_background_gl() -> DrawData {
    let mut dd = DrawData::default();
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
            // todo: fix seams that appear sometimes
            y += rect.h as f32 / 2.0;
            dd.append(x, y, rect, sprite, false, 1.0);
            y -= rect.h as f32;
            dd.append(x, y, rect, sprite, true, 1.0);
        } else {
            dd.append(x, y, rect, sprite, false, 1.0);
        }
    }
    let sprite = &TREE.sprites["startNode"];
    let rect = &sprite.coords["PSStartNodeBackgroundInactive"];
    for node in TREE.nodes.values().filter(|n| n.class_start_index.is_some()) {
        let (x, y) = node_pos(node);
        dd.append(x, y, rect, sprite, false, 2.5);
    }
    dd
}

fn ascendancies_gl() -> DrawData {
    let mut dd = DrawData::default();
    for node in TREE.nodes.values().filter(|n| n.is_ascendancy_start) {
        let sprite = &TREE.sprites["ascendancyBackground"];
        let rect = &sprite.coords[&("Classes".to_string() + node.ascendancy_name.as_ref().unwrap())];
        let (x, y) = node_pos(node);
        dd.append(x, y, rect, sprite, false, 2.5);
    }
    dd
}

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

void main() {
  color = texture( myTextureSampler, UV ).rgba;
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
    draw_data: FxHashMap<String, GlDrawData>,
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
        let data = connectors_gl();
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
        // todo destroy buffers
    }

    pub fn draw(&mut self, tree: &PassiveTree, gl: &glow::Context, zoom: f32, translate: (i32, i32)) {
        const REDRAW: [&str; 5] = [
            "nodes_active",
            "frames_active",
            "masteries_active",
            "ascendancy_frames_active",
            "connectors_active",
        ];
        for dd in self.draw_data.iter_mut().filter(|(k, _v)| REDRAW.contains(&k.as_str())) {
            dd.1.destroy(gl);
        }
        let data = nodes_gl_active(tree);
        self.draw_data
            .insert("nodes_active".to_string(), GlDrawData::new(gl, &data[0]));
        self.draw_data
            .insert("frames_active".to_string(), GlDrawData::new(gl, &data[1]));
        self.draw_data
            .insert("masteries_active".to_string(), GlDrawData::new(gl, &data[2]));
        self.draw_data
            .insert("ascendancy_frames_active".to_string(), GlDrawData::new(gl, &data[3]));
        let data = connectors_gl_active(&tree.nodes);
        self.draw_data
            .insert("connectors_active".to_string(), GlDrawData::new(gl, &data));

        let draw_order = [
            ("background", "group-background-3.dds"),
            ("ascendancy_background", "ascendancy-background-3.dds"),
            ("connectors", "line-3.dds"),
            ("connectors_active", "line-3.dds"),
            ("nodes", "skills-disabled-3.dds"),
            ("nodes_active", "skills-3.dds"),
            ("frames", "frame-3.dds"),
            ("frames_active", "frame-3.dds"),
            ("ascendancy_frames", "ascendancy-3.dds"),
            ("ascendancy_frames_active", "ascendancy-3.dds"),
            ("masteries", "mastery-disabled-3.dds"),
            ("masteries_active", "mastery-connected-3.dds"),
        ];
        unsafe {
            let mut viewport = [0; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
            let aspect_ratio = viewport[2] as f32 / viewport[3] as f32;
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.use_program(self.program);
            let scale = glam::Mat4::from_scale(glam::Vec3::new(zoom, zoom, 0.0));
            let ortho = glam::Mat4::orthographic_rh_gl(-aspect_ratio, aspect_ratio, -1.0, 1.0, -1.0, 1.0);
            let translate = glam::Mat4::from_translation(glam::Vec3::new(
                translate.0 as f32 / 12500.0,
                translate.1 as f32 / 12500.0,
                0.0,
            ));
            gl.uniform_matrix_4_f32_slice(
                self.uniform_zoom.as_ref(),
                false,
                &(scale * ortho * translate).to_cols_array(),
            );

            for to_draw in draw_order {
                gl.bind_vertex_array(self.draw_data[to_draw.0].vao);
                gl.bind_texture(glow::TEXTURE_2D, Some(self.textures[to_draw.1].gl_texture));
                gl.draw_elements(glow::TRIANGLES, self.draw_data[to_draw.0].len, glow::UNSIGNED_SHORT, 0);
            }
        }
    }
}
