pub mod build_selection;
pub mod tree_view;
pub mod settings;
pub mod panel;

use crate::config::Config;
use lightning_model::build::{Build, Stats};
use lightning_model::tree::Node;
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use egui_glow::egui_winit::winit::event::ElementState;
use std::time::Instant;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MainState {
    Tree,
    Config,
    ChooseMastery(u16),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UiState {
    ChooseBuild,
    LoadBuild(PathBuf),
    ImportBuild,
    NewBuild,
    Main(MainState),
}

/// Global state, contains everything
pub struct State {
    pub ui_state: UiState,
    pub build: Build,
    pub config: Config,
    pub import_account: String,
    pub import_character: String,
    pub request_recalc: bool,
    pub last_instant: Instant,
    pub show_settings: bool,

    pub active_skill_calc: FxHashMap<&'static str, i64>,
    pub defence_calc: Vec<(String, i64)>,
    pub stats: Option<Stats>,
    pub hovered_node: Option<&'static Node>,
    pub path_hovered: Option<Vec<u16>>,
    pub path_red: Option<Vec<u16>>,
    pub mouse_tree_drag: Option<(f32, f32)>,

    // widget-specific values
    builds_list_cur: usize,
    pub gemlink_cur: usize,
    pub active_skill_cur: usize,
    builds_dir_settings: String,
    framerate_settings: u64,
    pub level: i64,

    // OpenGL stuff
    pub dimensions: (u32, u32),
    pub tree_translate: (f32, f32),
    pub zoom: f32,
    pub zoom_tmp: f32,
    pub request_regen: bool,

    // Controls
    pub mouse_pos: (f32, f32),
    pub key_left: ElementState,
    pub key_right: ElementState,
    pub key_up: ElementState,
    pub key_down: ElementState,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            ui_state: UiState::ChooseBuild,
            build: Build::new_player(),

            import_account: String::new(),
            import_character: String::new(),
            request_recalc: false,
            last_instant: Instant::now(),
            show_settings: false,

            active_skill_calc: FxHashMap::default(),
            defence_calc: vec![],
            stats: None,
            hovered_node: None,
            path_hovered: None,
            path_red: None,
            mouse_tree_drag: None,

            builds_list_cur: 0,
            gemlink_cur: 0,
            active_skill_cur: 0,
            builds_dir_settings: config.builds_dir.clone().into_os_string().into_string().unwrap(),
            framerate_settings: config.framerate,
            level: 1,
            config: config, // needs to be after fields that depend on config

            dimensions: (1280, 720),
            zoom: 1.0,
            zoom_tmp: 1.0,
            tree_translate: (0.0, 0.0),
            request_regen: false,

            mouse_pos: (0.0, 0.0),
            key_left: ElementState::Released,
            key_right: ElementState::Released,
            key_up: ElementState::Released,
            key_down: ElementState::Released,
        }
    }
}

pub fn select_mastery_effect(ctx: &egui::Context, current_masteries: &FxHashMap<u16, u16>, mastery: &Node) -> Option<u16> {
    let mut found = None;
    egui::Window::new("ChooseMastery")
        .collapsible(false)
        .movable(false)
        .title_bar(false)
        .resizable(false)
        .fixed_pos(egui::Pos2::new(700.0, 350.0))
        .show(ctx, |ui| {
            ui.label(egui::RichText::new(&mastery.name).color(egui::Color32::WHITE).size(20.0));
            egui::Frame::default().inner_margin(4.0).fill(egui::Color32::DARK_GRAY).show(ui, |ui| {
                // Show mastery choices that haven't been selected yet in other parts of the tree
                for effect in mastery.mastery_effects.iter().filter(|e| current_masteries.iter().find(|(_, cur_effect)| **cur_effect == e.effect).is_none()) {
                    for stat in &effect.stats {
                        if ui.selectable_label(false, egui::RichText::new(stat).color(egui::Color32::WHITE)).clicked() {
                            found = Some(effect.effect);
                            return;
                        }
                    }
                }
            });
        });
    found
}

pub fn is_over_tree(pos: &(f32, f32)) -> bool {
    pos.0 >= panel::left::WIDTH && pos.1 >= panel::top::HEIGHT
}
