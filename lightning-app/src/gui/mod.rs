pub mod build_selection;
pub mod tree_view;
pub mod settings;
pub mod panel;

use crate::config::Config;
use egui_glow::egui_winit::winit::event::Modifiers;
use lightning_model::build::Build;
use lightning_model::data::tree::Node;
use lightning_model::data::GEMS;
use lightning_model::gem::Gem;
use lightning_model::{calc, hset};
use panel::skills::SkillsPanelState;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MainState {
    Tree,
    Config,
    Skills,
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
    // Used for stat comparison on hover
    pub build_compare: Option<Build>,
    pub history: VecDeque<Build>,
    pub config: Config,
    pub import_account: String,
    pub import_character: String,
    pub request_recalc: bool,
    pub last_instant: Instant,
    pub show_settings: bool,
    pub modifiers: Modifiers,

    pub active_skill_calc: FxHashMap<&'static str, i64>,
    pub defence_calc: FxHashMap<&'static str, i64>,
    pub delta_compare: FxHashMap<&'static str, i64>,
    pub passives_count: usize,
    pub passives_max: i64,
    pub hovered_node: Option<&'static Node>,
    pub path_hovered: Option<Vec<u16>>,
    pub path_red: Option<Vec<u16>>,
    pub mouse_tree_drag: Option<(f32, f32)>,

    // Panels (TODO: separate other panels stuff into structs)
    panel_skills: SkillsPanelState,

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

    // Debug
    pub redraw_counter: u64,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            ui_state: UiState::ChooseBuild,
            build: Build::new_player(),
            build_compare: None,
            history: Default::default(),

            import_account: String::new(),
            import_character: String::new(),
            request_recalc: false,
            last_instant: Instant::now(),
            show_settings: false,
            modifiers: Default::default(),

            active_skill_calc: FxHashMap::default(),
            defence_calc: FxHashMap::default(),
            delta_compare: FxHashMap::default(),
            passives_count: 0,
            passives_max: 0,
            hovered_node: None,
            path_hovered: None,
            path_red: None,
            mouse_tree_drag: None,

            panel_skills: Default::default(),

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

            redraw_counter: 0,
        }
    }

    pub fn snapshot(&mut self) {
        if self.history.len() >= 100 {
            self.history.pop_front();
        }
        self.history.push_back(self.build.clone());
    }

    pub fn undo(&mut self) {
        if let Some(build) = self.history.pop_front() {
            self.build = build;
            self.request_recalc = true;
            self.request_regen = true;
        }
    }

    pub fn compare(&self, build_compare: &Build) -> FxHashMap<&'static str, i64> {
        let mut delta = FxHashMap::default();
        if let Some(gem_link_compare) = build_compare.gem_links.get(self.gemlink_cur) {
            if let Some(active_gem_compare) = gem_link_compare.active_gems().nth(self.active_skill_cur) {
                let active_gem_compare_calc = calc::calc_gem(build_compare, gem_link_compare.support_gems(), active_gem_compare);
                delta.extend(calc::compare(&self.active_skill_calc, &active_gem_compare_calc));
            }
        }
        let defence_compare_calc = calc::calc_defence(build_compare);
        delta.extend(calc::compare(&self.defence_calc, &defence_compare_calc));
        delta
    }

    pub fn recalc(&mut self) {
        let mods = self.build.calc_mods(true);
        let stats = self.build.calc_stats(&mods, &hset![]);
        self.passives_count = self.build.tree.passives_count();
        self.passives_max = stats.val(lightning_model::build::stat::StatId::PassiveSkillPoints);
        self.defence_calc = calc::calc_defence(&self.build);
        self.active_skill_calc.clear();
        if let Some(gem_link) = self.build.gem_links.get(self.gemlink_cur) {
            if let Some(active_gem) = gem_link.active_gems().nth(self.active_skill_cur) {
                self.active_skill_calc = calc::calc_gem(&self.build, gem_link.support_gems().filter(|g| g.enabled), active_gem);
            }
        }
        if let Some(build_compare) = self.build_compare.as_ref() {
            self.delta_compare = self.compare(build_compare);
        }
        if self.panel_skills.selected_gem.is_some() {
            if self.build.gem_links.len() > self.panel_skills.selected_gemlink {
                let mut vec = vec![];
                let mut build_compare = self.build.clone();
                for (i, (id, gem_data)) in GEMS.iter().enumerate() {
                    let gem = Gem {
                        id: id.clone(),
                        enabled: true,
                        level: 20,
                        qual: 20,
                        alt_qual: 0,
                    };
                    if i > 0 {
                        build_compare.gem_links[self.panel_skills.selected_gemlink].gems.pop();
                    }
                    build_compare.gem_links[self.panel_skills.selected_gemlink].gems.push(gem);
                    let compare = self.compare(&build_compare);
                    let delta_dps = compare.get("DPS").unwrap_or(&0);
                    vec.push((*delta_dps, gem_data.display_name()));
                }
                vec.sort_by(|a, b| b.0.cmp(&a.0));
                self.panel_skills.computed_gems = Some(vec);
            }
        } else {
            self.panel_skills.computed_gems = None;
        }
        self.request_recalc = false;
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
                    let mut string = String::new();
                    for (i, stat) in effect.stats.iter().enumerate() {
                        string.push_str(&stat);
                        if i != effect.stats.len() - 1 {
                            string.push_str("\n");
                        }
                    }
                    if ui.selectable_label(false, egui::RichText::new(string).color(egui::Color32::WHITE)).clicked() {
                        found = Some(effect.effect);
                        return;
                    }
                }
            });
        });
    found
}

pub fn is_over_tree(pos: &(f32, f32)) -> bool {
    pos.0 >= panel::left::WIDTH && pos.1 >= panel::top::HEIGHT
}
