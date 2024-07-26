use lazy_static::lazy_static;
use lightning_model::data::TREE;
use lightning_model::tree::{self, Class, Node, NodeType, Rect, Sprite};
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
                n => (0..*n).map(|i| (360 * i) / n).collect(),
            };
            angles.into_iter().map(|a| (a as f32).to_radians()).collect()
        });
    }
    ret
}

lazy_static! {
    static ref ORBIT_ANGLES: Vec<Vec<f32>> = calc_angles();
}

pub fn node_pos(node: &Node) -> (f32, f32) {
    let group = node.group.unwrap();
    let orbit = node.orbit.unwrap() as usize;
    let angle = ORBIT_ANGLES[orbit][node.orbit_index.unwrap() as usize];
    let orbit_radius = TREE.constants.orbit_radii[orbit];

    (
        TREE.groups[&group].x + (angle.sin() * orbit_radius as f32),
        TREE.groups[&group].y.neg() + (angle.cos() * orbit_radius as f32),
    )
}

/// Normalize tree coords to GL normalized coords
fn norm(x: f32, y: f32) -> (f32, f32) {
    (x / 12500.0, y / 12500.0)
}

/// Normalize sprite coords to GL texture coords
fn norm_tex(x: u16, y: u16, w: u16, h: u16) -> (f32, f32) {
    (x as f32 / w as f32, y as f32 / h as f32)
}

pub fn get_rect(node: &Node) -> Option<(&'static tree::Rect, &'static tree::Sprite)> {
    let (key, icon): (&str, &str) = match node.node_type() {
        NodeType::Normal | NodeType::AscendancyNormal | NodeType::JewelSocket => ("normalInactive", &node.icon),
        NodeType::Notable | NodeType::AscendancyNotable => ("notableInactive", &node.icon),
        NodeType::Keystone => ("keystoneInactive", &node.icon),
        NodeType::Mastery => ("masteryConnected", node.inactive_icon.as_ref().unwrap()),
    };
    let sprite = &TREE.sprites[key];
    let rect = sprite.coords.get(icon)?;
    Some((rect, sprite))
}

#[derive(Default)]
pub struct DrawData {
    pub vertices: Vec<(f32, f32)>,
    pub tex_coords: Vec<(f32, f32)>,
    pub indices: Vec<u16>,
}

impl DrawData {
    pub fn append(&mut self, x: f32, y: f32, rect: &tree::Rect, sprite: &tree::Sprite, vflip: bool, scale: f32) {
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

/// Very simple straight connectors. todo: arcs
fn connector_gl(x1: f32, y1: f32, x2: f32, y2: f32, w: f32, rect: &Rect, sprite: &Sprite, dd: &mut DrawData) {
    let (vx, vy) = (x2 - x1, y2 - y1);
    let (px, py) = (vy, -vx);
    let len = (px * px + (py * py)).sqrt();
    let (nx, ny) = (px / len, py / len);
    dd.vertices.extend([
        norm(x1 - nx * w, y1 - ny * w),
        norm(x1 + nx * w, y1 + ny * w),
        norm(x2 + nx * w, y2 + ny * w),
        norm(x2 - nx * w, y2 - ny * w),
    ]);
    dd.tex_coords.extend([
        norm_tex(rect.x, rect.y, sprite.w, sprite.h),
        norm_tex(rect.x + rect.w, rect.y + rect.h, sprite.w, sprite.h),
        norm_tex(rect.x, rect.y + rect.h, sprite.w, sprite.h),
        norm_tex(rect.x + rect.w, rect.y, sprite.w, sprite.h),
    ]);

    let start = dd.vertices.len() as u16 - 4;
    dd.indices
        .extend([start, start + 1, start + 2, start + 3, start, start + 2]);
}

pub fn connectors_gl_inactive() -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["line"];
    let rect = &sprite.coords["LineConnectorNormal"];

    for node in TREE.nodes.values().filter(|n| {
        n.group.is_some()
            && (!n.name.starts_with("Path of the") || n.ascendancy_name.is_none())
            && n.class_start_index.is_none()
    }) {
        let (x1, y1) = node_pos(node);
        for out in node
            .out
            .iter()
            .flatten()
            .map(|id| &TREE.nodes[id])
            .filter(|n| !n.is_ascendancy_start && !n.is_mastery && n.class_start_index.is_none())
        {
            let (x2, y2) = node_pos(out);
            connector_gl(x1, y1, x2, y2, 10.0, rect, sprite, &mut dd);
        }
    }
    dd
}

pub fn connectors_gl(nodes: &[u16], rect: &Rect, w: f32) -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["line"];

    for node in nodes.iter().map(|id| &TREE.nodes[id]).filter(|n| {
        n.group.is_some()
            && (!n.name.starts_with("Path of the") || n.ascendancy_name.is_none())
            && n.class_start_index.is_none()
    }) {
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
            connector_gl(x1, y1, x2, y2, w, rect, sprite, &mut dd);
        }
    }
    dd
}
const ACTIVE_STRINGS: [&str; 6] = [
    "AscendancyFrameSmallAllocated",
    "AscendancyFrameLargeAllocated",
    "PSSkillFrameActive",
    "NotableFrameAllocated",
    "KeystoneFrameAllocated",
    "JewelFrameAllocated",
];

const INACTIVE_STRINGS: [&str; 6] = [
    "AscendancyFrameSmallNormal",
    "AscendancyFrameLargeNormal",
    "PSSkillFrame",
    "NotableFrameUnallocated",
    "KeystoneFrameUnallocated",
    "JewelFrameUnallocated",
];

fn node_gl(
    node: &Node,
    dd_nodes: &mut DrawData,
    dd_frames: &mut DrawData,
    dd_masteries: &mut DrawData,
    dd_asc_frames: &mut DrawData,
    is_active: bool,
    is_hovered: bool,
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
            dd_masteries.append(x, y, rect, sprite, false, 2.0);
        }
        NodeType::AscendancyNormal | NodeType::AscendancyNotable => {
            if !is_hovered {
                dd_nodes.append(x, y, rect, sprite, false, 2.0);
            }
            let sprite = &TREE.sprites["ascendancy"];
            let rect = match node.node_type() {
                NodeType::AscendancyNormal => &sprite.coords[icon_strings[0]],
                NodeType::AscendancyNotable => &sprite.coords[icon_strings[1]],
                _ => panic!("No frame"),
            };
            dd_asc_frames.append(x, y, rect, sprite, false, 2.0);
        }
        NodeType::JewelSocket => {
            let sprite = &TREE.sprites["frame"];
            let rect = &sprite.coords[icon_strings[5]];
            dd_frames.append(x, y, rect, sprite, false, 2.0);
        }
        _ => {
            if !is_hovered {
                dd_nodes.append(x, y, rect, sprite, false, 2.0);
            }
            let sprite = &TREE.sprites["frame"];
            let rect = match node.node_type() {
                NodeType::Normal => &sprite.coords[icon_strings[2]],
                NodeType::Notable => &sprite.coords[icon_strings[3]],
                NodeType::Keystone => &sprite.coords[icon_strings[4]],
                _ => panic!("No frame"),
            };
            dd_frames.append(x, y, rect, sprite, false, 2.0);
        }
    }
}

/// Nodes, Frames and Masteries
pub fn nodes_gl() -> [DrawData; 4] {
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
            false,
        );
    }
    [dd_nodes, dd_frames, dd_masteries, dd_asc_frames]
}

/// Player-selected Nodes, Frames and Masteries
pub fn nodes_gl_active(nodes: &[u16], hovered: Option<&u16>) -> [DrawData; 4] {
    let mut dd_nodes = DrawData::default();
    let mut dd_frames = DrawData::default();
    let mut dd_masteries = DrawData::default();
    let mut dd_asc_frames = DrawData::default();

    for node in nodes
        .iter()
        .map(|id| &TREE.nodes[id])
        .filter(|n| n.class_start_index.is_none())
    {
        node_gl(
            node,
            &mut dd_nodes,
            &mut dd_frames,
            &mut dd_masteries,
            &mut dd_asc_frames,
            true,
            false,
        );
    }

    if let Some(id) = hovered {
        node_gl(
            &TREE.nodes[id],
            &mut dd_nodes,
            &mut dd_frames,
            &mut dd_masteries,
            &mut dd_asc_frames,
            true,
            true,
        );
    }

    [dd_nodes, dd_frames, dd_masteries, dd_asc_frames]
}

fn get_class_coords(class: Class) -> &'static str {
    match class {
        Class::Witch => "centerwitch",
        Class::Templar => "centertemplar",
        Class::Shadow => "centershadow",
        Class::Scion => "centerscion",
        Class::Ranger => "centerranger",
        Class::Marauder => "centermarauder",
        Class::Duelist => "centerduelist",
    }
}

pub fn class_start_gl(class: Class) -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["startNode"];
    for node in TREE.nodes.values().filter(|n| n.class_start_index.is_some()) {
        let rect = if class as i32 == node.class_start_index.unwrap() {
            &sprite.coords[get_class_coords(class)]
        } else {
            &sprite.coords["PSStartNodeBackgroundInactive"]
        };
        let (x, y) = node_pos(node);
        dd.append(x, y, rect, sprite, false, 2.7);
    }
    dd
}

pub fn group_background_gl() -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["groupBackground"];
    for group in TREE.groups.values().filter(|g| g.background.is_some()) {
        let background = group.background.as_ref().unwrap();
        let rect = match sprite.coords.get(&background.image) {
            None => continue,
            Some(res) => res,
        };

        let x = group.x;
        let mut y = group.y.neg();
        if background.is_half_image.is_some() {
            // Need to draw upper half and then bottom half (vertically flipped)
            // todo: fix seams that appear sometimes
            y += rect.h as f32;
            dd.append(x, y, rect, sprite, false, 2.0);
            y -= (rect.h as f32 - 1.0) * 2.0;
            dd.append(x, y, rect, sprite, true, 2.0);
        } else {
            dd.append(x, y, rect, sprite, false, 2.0);
        }
    }
    dd
}

pub fn ascendancies_gl() -> DrawData {
    let mut dd = DrawData::default();
    for node in TREE.nodes.values().filter(|n| n.is_ascendancy_start) {
        let sprite = &TREE.sprites["ascendancyBackground"];
        let key = &("Classes".to_string() + node.ascendancy_name.as_ref().unwrap());
        if let Some(rect) = sprite.coords.get(key) {
            let (x, y) = node_pos(node);
            dd.append(x, y, rect, sprite, false, 2.5);
        }
    }
    dd
}
