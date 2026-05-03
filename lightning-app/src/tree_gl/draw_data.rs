use lazy_static::lazy_static;
use lightning_model::build::{Build, Slot};
use lightning_model::calc::PowerReport;
use lightning_model::data::tree::{Ascendancy, Class, Node, NodeType, Rect, Sprite};
use lightning_model::data::TREE;
use std::collections::HashMap;
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
fn norm_tex(x: f32, y: f32, w: u16, h: u16) -> (f32, f32) {
    (x / w as f32, y / h as f32)
}

pub fn get_rect(node: &Node, active: bool) -> Option<(&'static Rect, &'static Sprite)> {
    let (key, icon): (&str, &str) = match node.node_type() {
        NodeType::Normal | NodeType::AscendancyNormal | NodeType::JewelSocket => ("normalInactive", &node.icon),
        NodeType::Notable | NodeType::AscendancyNotable => ("notableInactive", &node.icon),
        NodeType::Keystone => ("keystoneInactive", &node.icon),
        NodeType::Mastery => {
            match (node.is_tattoo, active) {
                (true, true) => ("masteryActiveSelected", &node.icon),
                (true, false) => ("masteryConnected", &node.icon),
                (false, true) =>("masteryActiveSelected", node.active_icon.as_ref().unwrap()),
                (false, false) => ("masteryConnected", node.inactive_icon.as_ref().unwrap()),
            }
        },
    };
    let sprite = &TREE.sprites[key];
    let rect = sprite.coords.get(icon)?;
    Some((rect, sprite))
}

#[derive(Default)]
pub struct DrawData {
    pub vertices: Vec<(f32, f32)>,
    pub tex_coords: Vec<(f32, f32)>,
    pub tints: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl DrawData {
    pub fn append_tint(&mut self, x: f32, y: f32, rect: &Rect, sprite: &Sprite, vflip: bool, scale: f32, color: [f32; 4]) {
        self.vertices.extend([
            norm(x - (rect.w as f32 * scale) / 2.0, y - (rect.h as f32 * scale) / 2.0), // Bottom Left
            norm(x - (rect.w as f32 * scale) / 2.0, y + (rect.h as f32 * scale) / 2.0), // Top Left
            norm(x + (rect.w as f32 * scale) / 2.0, y + (rect.h as f32 * scale) / 2.0), // Top Right
            norm(x + (rect.w as f32 * scale) / 2.0, y - (rect.h as f32 * scale) / 2.0), // Bottom Right
        ]);

        let x_left = rect.x as f32 + 0.5;
        let x_right = rect.x as f32 + rect.w as f32 - 0.5;
        let y_top = rect.y as f32 + 0.5;
        let y_bottom = rect.y as f32 + rect.h as f32 - 0.5;
        if vflip {
            self.tex_coords.extend([
                norm_tex(x_left, y_top, sprite.w, sprite.h),
                norm_tex(x_left, y_bottom, sprite.w, sprite.h),
                norm_tex(x_right, y_bottom, sprite.w, sprite.h),
                norm_tex(x_right, y_top, sprite.w, sprite.h),
            ]);
        } else {
            self.tex_coords.extend([
                norm_tex(x_left, y_bottom, sprite.w, sprite.h),
                norm_tex(x_left, y_top, sprite.w, sprite.h),
                norm_tex(x_right, y_top, sprite.w, sprite.h),
                norm_tex(x_right, y_bottom, sprite.w, sprite.h),
            ]);
        }

        self.tints.extend([color, color, color, color]);
        let start = self.vertices.len() as u32 - 4;
        self.indices
            .extend([start, start + 1, start + 2, start + 3, start, start + 2]);
    }

    pub fn append(&mut self, x: f32, y: f32, rect: &Rect, sprite: &Sprite, vflip: bool, scale: f32) {
        self.append_tint(x, y, rect, sprite, vflip, scale, [1.0, 1.0, 1.0, 1.0]);
    }
}

pub fn background_gl() -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["background"];

    dd.vertices.extend([
        norm(-12500.0, 12500.0),  // top left
        norm(-12500.0, -12500.0), // bottom left
        norm(12500.0, -12500.0), // bottom right
        norm(12500.0, 12500.0), // top right
    ]);

    dd.tex_coords.extend([
        (0.0, 0.0), // top left
        (0.0, (25000.0 / sprite.h as f32)), // bottom left
        ((25000.0 / sprite.w as f32), (25000.0 / sprite.h as f32)), // bottom right
        ((25000.0 / sprite.w as f32), 0.0), // top right
    ]);

    dd.tints.extend([[1.0, 1.0, 1.0, 1.0]; 4]);

    let start = dd.vertices.len() as u32 - 4;
    dd.indices
        .extend([start, start + 1, start + 2, start + 3, start, start + 2]);

    dd
}

pub fn search_highlights_gl(matched_node_ids: &[u32], nodes: &imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>) -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["frame"];
    let rect = &sprite.coords["NotableFrameAllocated"]; // Reuse frame sprite for search highlights

    for node in matched_node_ids.iter().map(|id| &nodes[id]) {
        let (x, y) = node_pos(node);
        let scale = if node.is_keystone {
            5.0
        } else if node.is_notable {
            4.2
        } else {
            3.5
        };
        dd.append(x, y, rect, sprite, false, scale);
    }
    dd
}

/// Bendy (arc) connector
fn arc_connector_gl(
    cx: f32, cy: f32,
    r: f32,
    a1: f32, a2: f32,
    w: f32,
    rect: &Rect, sprite: &Sprite, dd: &mut DrawData,
) {
    let mut diff = a2 - a1;
    while diff > std::f32::consts::PI { diff -= 2.0 * std::f32::consts::PI; }
    while diff < -std::f32::consts::PI { diff += 2.0 * std::f32::consts::PI; }

    let arc_length = diff.abs() * r;
    let segments = (arc_length / 5.0).ceil().max(3.0) as usize;

    let x_left = rect.x as f32 + 0.5;
    let x_right = rect.x as f32 + rect.w as f32 - 0.5;
    let y_top = rect.y as f32 + 0.5;
    let y_bottom = rect.y as f32 + rect.h as f32 - 0.5;

    let start_vertex = dd.vertices.len() as u32;

    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let angle = a1 + diff * t;

        let sin_a = angle.sin();
        let cos_a = angle.cos();

        dd.vertices.push(norm(cx + (r - w) * sin_a, cy + (r - w) * cos_a));
        dd.vertices.push(norm(cx + (r + w) * sin_a, cy + (r + w) * cos_a));

        let u = x_left + (x_right - x_left) * t;
        dd.tex_coords.push(norm_tex(u, y_bottom, sprite.w, sprite.h));
        dd.tex_coords.push(norm_tex(u, y_top, sprite.w, sprite.h));
        dd.tints.push([1.0, 1.0, 1.0, 1.0]);
        dd.tints.push([1.0, 1.0, 1.0, 1.0]);
    }

    for i in 0..segments as u32 {
        let base = start_vertex + i * 2;
        dd.indices.extend([
            base, base + 1, base + 2,
            base + 1, base + 3, base + 2
        ]);
    }
}

/// Very simple straight connector.
fn connector_gl(x1: f32, y1: f32, x2: f32, y2: f32, w: f32, rect: &Rect, sprite: &Sprite, dd: &mut DrawData) {
    let (vx, vy) = (x2 - x1, y2 - y1);
    let (px, py) = (vy, -vx);
    let len = (px * px + (py * py)).sqrt();
    let (nx, ny) = ((px / len) * w, (py / len) * w);
    dd.vertices.extend([
        norm(x1 - nx, y1 - ny),
        norm(x1 + nx, y1 + ny),
        norm(x2 + nx, y2 + ny),
        norm(x2 - nx, y2 - ny),
    ]);

    let x_left = rect.x as f32 + 0.5;
    let x_right = rect.x as f32 + rect.w as f32 - 0.5;
    let y_top = rect.y as f32 + 0.5;
    let y_bottom = rect.y as f32 + rect.h as f32 - 0.5;
    dd.tex_coords.extend([
        norm_tex(x_left, y_top, sprite.w, sprite.h), // top left
        norm_tex(x_left, y_bottom, sprite.w, sprite.h), // bottom left
        norm_tex(x_right, y_bottom, sprite.w, sprite.h), // bottom right
        norm_tex(x_right, y_top, sprite.w, sprite.h), // top right
    ]);

    dd.tints.extend([[1.0, 1.0, 1.0, 1.0]; 4]);

    let start = dd.vertices.len() as u32 - 4;
    dd.indices
        .extend([start, start + 1, start + 2, start + 3, start, start + 2]);
}

pub fn build_connection(
    node1: &Node,
    node2: &Node,
    w: f32,
    rect: &Rect,
    sprite: &Sprite,
    dd: &mut DrawData
) {
    let group1 = node1.group.unwrap();
    let group2 = node2.group.unwrap();

    let orbit1 = node1.orbit.unwrap() as usize;
    let orbit2 = node2.orbit.unwrap() as usize;

    if group1 == group2 && orbit1 == orbit2 {
        let orbit_radius = TREE.constants.orbit_radii[orbit1] as f32;
        let cx = TREE.groups[&group1].x;
        let cy = TREE.groups[&group1].y.neg(); // Match your negative Y logic

        let a1 = ORBIT_ANGLES[orbit1][node1.orbit_index.unwrap() as usize];
        let a2 = ORBIT_ANGLES[orbit2][node2.orbit_index.unwrap() as usize];

        arc_connector_gl(cx, cy, orbit_radius, a1, a2, w, rect, sprite, dd);
    } else {
        let (x1, y1) = node_pos(node1);
        let (x2, y2) = node_pos(node2);

        connector_gl(x1, y1, x2, y2, w, rect, sprite, dd);
    }
}

pub fn connectors_gl_inactive(nodes: &imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>) -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["line"];
    let rect = &sprite.coords["LineConnectorNormal"];

    for node in nodes.values().filter(|n| {
        n.group.is_some()
            && (!n.name.starts_with("Path of the") || n.ascendancy.is_none())
            && n.class_start_index.is_none()
            && !n.is_mastery
            && !n.is_proxy
    }) {
        for out in node
            .out
            .iter()
            .flatten()
            .map(|id| &nodes[id])
            .filter(|n| !n.is_ascendancy_start && !n.is_mastery && !n.is_proxy && n.class_start_index.is_none())
        {
            build_connection(node, out, 18.0, rect, sprite, &mut dd);
        }
    }
    dd
}

pub fn connectors_gl(nodes_id: &[u32], nodes: &imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>, rect: &Rect, w: f32) -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["line"];

    for node in nodes_id.iter().map(|id| &nodes[id]).filter(|n| {
        n.group.is_some()
            && (!n.name.starts_with("Path of the") || n.ascendancy.is_none())
            && n.class_start_index.is_none()
            && !n.is_mastery
    }) {
        for out in node
            .out
            .iter()
            .flatten()
            .filter(|id| nodes_id.contains(id))
            .map(|id| &nodes[id])
            .filter(|n| !n.is_ascendancy_start && !n.is_mastery && n.class_start_index.is_none())
        {
            build_connection(node, out, w, rect, sprite, &mut dd);
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
    "PSSkillFrameHighlighted",
    "NotableFrameCanAllocate",
    "KeystoneFrameUnallocated",
    "JewelFrameUnallocated",
];

fn node_gl(
    node: &Node,
    dd_nodes: &mut DrawData,
    dd_frames: &mut DrawData,
    dd_masteries: &mut DrawData,
    dd_asc_frames: &mut DrawData,
    dd_tattoo_active_effects: &mut DrawData,
    dd_mastery_active_effects: &mut DrawData,
    is_active: bool,
    is_hovered: bool,
    tint: [f32; 4]
) {
    let (rect, sprite) = match get_rect(node, is_active) {
        None => {
            println!("No rect for node {}", node.name);
            return;
        }
        Some(res) => res,
    };

    let (x, y) = node_pos(node);
    let icon_strings = match node.node_type() {
        NodeType::Mastery => {
            match is_active {
                true => &ACTIVE_STRINGS,
                false => &INACTIVE_STRINGS,
            }
        },
        _ => {
            if is_active || is_hovered {
                &ACTIVE_STRINGS
            } else {
                &INACTIVE_STRINGS
            }
        },
    };

    const SCALE: f32 = 2.5;
    match node.node_type() {
        NodeType::Mastery => {
            dd_masteries.append(x, y, rect, sprite, false, SCALE);
            if is_active &&
               let Some(active_effect) = &node.active_effect_image &&
               let Some(sprite) = TREE.sprites.get("masteryActiveEffect") &&
               let Some(rect) = sprite.coords.get(active_effect)
            {
                dd_mastery_active_effects.append(x, y, rect, sprite, false, SCALE);
            }
        }
        NodeType::AscendancyNormal | NodeType::AscendancyNotable => {
            if !is_hovered {
                dd_nodes.append_tint(x, y, rect, sprite, false, SCALE, tint);
            }
            let sprite = &TREE.sprites["ascendancy"];
            let rect = match node.node_type() {
                NodeType::AscendancyNormal => &sprite.coords[icon_strings[0]],
                NodeType::AscendancyNotable => &sprite.coords[icon_strings[1]],
                _ => panic!("No frame"),
            };
            dd_asc_frames.append_tint(x, y, rect, sprite, false, SCALE, tint);
        }
        NodeType::JewelSocket => {
            let sprite = &TREE.sprites["frame"];
            let rect = &sprite.coords[icon_strings[5]];
            dd_frames.append_tint(x, y, rect, sprite, false, SCALE, tint);
        }
        _ => {
            if !is_hovered {
                dd_nodes.append_tint(x, y, rect, sprite, false, SCALE, tint);
            }
            let sprite = &TREE.sprites["frame"];
            let rect = match node.node_type() {
                NodeType::Normal => &sprite.coords[icon_strings[2]],
                NodeType::Notable => &sprite.coords[icon_strings[3]],
                NodeType::Keystone => &sprite.coords[icon_strings[4]],
                _ => panic!("No frame"),
            };
            dd_frames.append_tint(x, y, rect, sprite, false, SCALE, tint);
            if (is_active || node.is_tattoo) &&
               let Some(active_effect) = &node.active_effect_image &&
               let Some(sprite) = TREE.sprites.get("tattooActiveEffect") &&
               let Some(rect) = sprite.coords.get(active_effect)
            {
                dd_tattoo_active_effects.append(x, y, rect, sprite, false, SCALE);
            }
        }
    }
}

/// Unallocated Nodes, Frames and Masteries (the entire tree pretty much)
pub fn nodes_gl(nodes: &imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>, power_report: Option<&PowerReport>) -> [DrawData; 5] {
    let mut dd_nodes = DrawData::default();
    let mut dd_frames = DrawData::default();
    let mut dd_masteries = DrawData::default();
    let mut dd_asc_frames = DrawData::default();
    let mut dd_tattoo_active_effects = DrawData::default();
    let mut dd_mastery_active_effects = DrawData::default();

    for node in nodes
        .values()
        .filter(|n| n.group.is_some() && n.class_start_index.is_none() && !n.is_proxy)
    {
        let (tint, mut active) = if let Some(power_report) = power_report && let Some(power) = power_report.nodes_delta.get(&node.skill) {
            if *power <= 1.0 {
                ([0.0, 0.0, 0.0, 1.0], false)
            } else {
                let factor = ((*power - 1.0) * 10.0).clamp(0.15, 1.0);
                ([factor, 0.0, 0.0, 1.0], true)
            }
        } else {
            ([1.0, 1.0, 1.0, 1.0], false)
        };
        if node.is_mastery {
            active = false;
        }
        node_gl(
            node,
            &mut dd_nodes,
            &mut dd_frames,
            &mut dd_masteries,
            &mut dd_asc_frames,
            &mut dd_tattoo_active_effects,
            &mut dd_mastery_active_effects,
            active,
            false,
            tint,
        );
    }
    [dd_nodes, dd_frames, dd_masteries, dd_asc_frames, dd_tattoo_active_effects]
}

/// Allocated & hovered Nodes, Frames and Masteries
pub fn nodes_gl_active(nodes_id: &[u32], nodes: &imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>, hovered: Option<&u32>) -> [DrawData; 6] {
    let mut dd_nodes = DrawData::default();
    let mut dd_frames = DrawData::default();
    let mut dd_masteries = DrawData::default();
    let mut dd_masteries_active = DrawData::default();
    let mut dd_asc_frames = DrawData::default();
    let mut dd_tattoo_active_effects = DrawData::default();
    let mut dd_mastery_active_effects = DrawData::default();
    let tint = [1.0, 1.0, 1.0, 1.0];

    for node in nodes_id
        .iter()
        .map(|id| &nodes[id])
        .filter(|n| n.class_start_index.is_none())
    {
        node_gl(
            node,
            &mut dd_nodes,
            &mut dd_frames,
            &mut dd_masteries_active,
            &mut dd_asc_frames,
            &mut dd_tattoo_active_effects,
            &mut dd_mastery_active_effects,
            true,
            false,
            tint,
        );
    }

    if let Some(id) = hovered {
        node_gl(
            &nodes[id],
            &mut dd_nodes,
            &mut dd_frames,
            &mut dd_masteries,
            &mut dd_asc_frames,
            &mut dd_tattoo_active_effects,
            &mut dd_mastery_active_effects,
            nodes_id.contains(id),
            true,
            tint,
        );
    }

    [dd_nodes, dd_frames, dd_masteries, dd_masteries_active, dd_asc_frames, dd_mastery_active_effects]
}

lazy_static! {
    static ref JEWELS_BASE_SPRITE: HashMap<&'static str, &'static str> = HashMap::from([
        ("Crimson Jewel", "JewelSocketActiveRed"),
        ("Viridian Jewel", "JewelSocketActiveGreen"),
        ("Cobalt Jewel", "JewelSocketActiveBlue"),
        ("Prismatic Jewel", "JewelSocketActivePrismatic"),
        ("Large Cluster Jewel", "JewelSocketActiveAltPurple"),
        ("Medium Cluster Jewel", "JewelSocketActiveAltBlue"),
        ("Small Cluster Jewel", "JewelSocketActiveAltRed"),
    ]);
}

pub fn jewels_gl(build: &Build, nodes: &imbl::GenericHashMap<u32, Node, rustc_hash::FxBuildHasher, archery::ArcK>) -> DrawData {
    let mut dd = DrawData::default();
    let sprite = &TREE.sprites["jewel"];

    for slot in build.equipment().keys() {
        if let Slot::TreeJewel(node) = slot {
            if let Some(sprite_name) = JEWELS_BASE_SPRITE.get(build.get_equipped(*slot).unwrap().base_item.as_str()) {
                let rect = &sprite.coords[*sprite_name];
                let (x, y) = node_pos(&nodes[node]);
                dd.append(x, y, rect, sprite, false, 1.6);
            }
        }
    }

    dd
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
    for group in TREE.groups.values().filter(|g| g.background.is_some() && !g.is_proxy) {
        let background = group.background.as_ref().unwrap();
        let rect = match sprite.coords.get(&background.image) {
            None => continue,
            Some(res) => res,
        };

        let x = group.x;
        let mut y = group.y.neg();
        if background.is_half_image.is_some() {
            // Need to draw upper half and then bottom half (vertically flipped)
            y += rect.h as f32;
            dd.append(x, y, rect, sprite, false, 2.0);
            y -= (rect.h as f32) * 2.0;
            dd.append(x, y, rect, sprite, true, 2.0);
        } else {
            dd.append(x, y, rect, sprite, false, 2.0);
        }
    }
    dd
}

pub fn bloodlines_background_inactive_gl(bloodline: Option<Ascendancy>) -> DrawData {
    let mut dd = DrawData::default();
    for node in TREE.nodes.values().filter(|n| n.is_ascendancy_start && n.is_bloodline && n.ascendancy != bloodline) {
        let key = &("Classes".to_string() + node.ascendancy.unwrap().into());
        // Find the sprite/coords by just iterating over all the sprites until we find a coords with the desired key.
        if let Some((sprite, rect)) = TREE.sprites.values().find_map(|sprite| {
            sprite.coords.get(key).map(|rect| (sprite, rect))
        }) {
            let (x, y) = node_pos(node);
            dd.append(x, y, rect, sprite, false, 2.5);
        }
    }
    dd
}

pub fn bloodlines_background_active_gl(bloodline: Option<Ascendancy>) -> DrawData {
    let mut dd = DrawData::default();
    for node in TREE.nodes.values().filter(|n| n.is_ascendancy_start && n.is_bloodline && n.ascendancy == bloodline) {
        let key = &("Classes".to_string() + node.ascendancy.unwrap().into());
        // Find the sprite/coords by just iterating over all the sprites until we find a coords with the desired key.
        if let Some((sprite, rect)) = TREE.sprites.values().find_map(|sprite| {
            sprite.coords.get(key).map(|rect| (sprite, rect))
        }) {
            let (x, y) = node_pos(node);
            dd.append(x, y, rect, sprite, false, 2.5);
        }
    }
    dd
}

pub fn ascendancies_background_inactive_gl(ascendancy: Option<Ascendancy>) -> DrawData {
    let mut dd = DrawData::default();
    for node in TREE.nodes.values().filter(|n| n.is_ascendancy_start && n.ascendancy != ascendancy) {
        let sprite = &TREE.sprites["ascendancy"];
        let key = &("Classes".to_string() + node.ascendancy.unwrap().into());
        if let Some(rect) = sprite.coords.get(key) {
            let (x, y) = node_pos(node);
            dd.append(x, y, rect, sprite, false, 2.5);
        }
    }
    dd
}

pub fn ascendancies_background_active_gl(ascendancy: Option<Ascendancy>) -> DrawData {
    let mut dd = DrawData::default();
    for node in TREE.nodes.values().filter(|n| n.is_ascendancy_start && n.ascendancy == ascendancy) {
        let sprite = &TREE.sprites["ascendancy"];
        let key = &("Classes".to_string() + node.ascendancy.unwrap().into());
        if let Some(rect) = sprite.coords.get(key) {
            let (x, y) = node_pos(node);
            dd.append(x, y, rect, sprite, false, 2.5);
        }
    }
    dd
}
