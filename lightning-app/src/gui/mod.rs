pub mod build_selection;
pub mod tree_view;
pub mod settings;
pub mod panel;
pub mod utils;

use crate::config::Config;
use crate::tree_gl::hover::QuadTreeHover;
use egui_glow::egui_winit::winit::event::Modifiers;
use lightning_model::build::Build;
use lightning_model::data::tree::Node;
use lightning_model::data::GEMS;
use lightning_model::gem::Gem;
use lightning_model::calc::{self, PowerReport};
use lightning_model::build::property;
use panel::items::ItemsPanelState;
use panel::skills::SkillsPanelState;
use panel::bottom::BottomPanelState;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;
use enumflags2::BitFlags;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MainState {
    Tree,
    Config,
    Skills,
    Items,
    Calc,
    ChooseMastery(u32),
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
    pub history_idx: usize,
    pub config: Config,
    pub import_account: String,
    pub import_character: String,
    pub request_recalc: bool,
    pub last_instant: Instant,
    pub show_settings: bool,
    pub modifiers: Modifiers,

    pub active_skill_calc: FxHashMap<&'static str, i64>,
    pub defence_calc: FxHashMap<&'static str, i64>,
    pub defence_stats: lightning_model::build::stat::Stats,
    pub delta_compare: FxHashMap<&'static str, i64>,
    pub delta_compare_single: FxHashMap<&'static str, i64>,
    pub power_report: Option<PowerReport>,
    pub passives_count: usize,
    pub passives_max: i64,
    pub abyssal_sockets: u16,
    pub hovered_node_id: Option<u32>,
    pub path_hovered: Option<Vec<u32>>,
    pub path_red: Option<Vec<u32>>,
    pub mouse_tree_drag: Option<(f32, f32)>,

    // Panels (TODO: separate other panels stuff into structs)
    panel_skills: SkillsPanelState,
    panel_items: ItemsPanelState,
    pub panel_bottom: BottomPanelState,

    // widget-specific values
    builds_list_cur: usize,
    pub gemlink_cur: usize,
    pub active_skill_cur: usize,
    builds_dir_settings: String,
    framerate_settings: u64,
    pub level: i64,
    can_save: bool,

    // OpenGL stuff
    pub dimensions: (u32, u32),
    pub tree_translate: (f32, f32),
    pub zoom: f32,
    pub zoom_tmp: f32,
    pub request_regen_gl: bool,
    pub request_regen_nodes_gl: bool,
    pub quadtree_hover: QuadTreeHover,

    // Controls
    pub mouse_pos: (f32, f32),

    // Debug
    pub redraw_counter: u64,
}

impl State {
    pub fn new(config: Config) -> Self {
        let build = Build::new_player();
        Self {
            ui_state: UiState::ChooseBuild,
            quadtree_hover: QuadTreeHover::build(&build.tree.nodes_data),
            build: build,
            build_compare: None,
            history: Default::default(),
            history_idx: 0,

            import_account: String::new(),
            import_character: String::new(),
            request_recalc: false,
            last_instant: Instant::now(),
            show_settings: false,
            modifiers: Default::default(),

            active_skill_calc: FxHashMap::default(),
            defence_calc: FxHashMap::default(),
            defence_stats: Default::default(),
            delta_compare: FxHashMap::default(),
            delta_compare_single: FxHashMap::default(),
            power_report: None,
            passives_count: 0,
            passives_max: 0,
            abyssal_sockets: 0,
            hovered_node_id: None,
            path_hovered: None,
            path_red: None,
            mouse_tree_drag: None,

            panel_skills: Default::default(),
            panel_items: Default::default(),
            panel_bottom: Default::default(),

            builds_list_cur: 0,
            gemlink_cur: 0,
            active_skill_cur: 0,
            builds_dir_settings: config.builds_dir.clone().into_os_string().into_string().unwrap(),
            framerate_settings: config.framerate,
            level: 1,
            can_save: true,
            config: config, // needs to be after fields that depend on config

            dimensions: (1280, 720),
            zoom: 1.0,
            zoom_tmp: 1.0,
            tree_translate: (0.0, 0.0),
            request_regen_gl: false,
            request_regen_nodes_gl: false,

            mouse_pos: (0.0, 0.0),

            redraw_counter: 0,
        }
    }

    pub fn reset(&mut self) {
        self.history.clear();
        self.history_idx = 0;
        self.snapshot();
        self.level = self.build.property_int(property::Int::Level);
        self.request_recalc = true;
        self.request_regen_gl = true;
        self.request_regen_nodes_gl = true;
        self.path_hovered = None;
        self.path_red = None;
        self.hovered_node_id = None;
        self.gemlink_cur = 0;
        self.active_skill_cur = 0;
        self.panel_items.editing_item_idx = None;
        self.panel_items.custom_text.clear();
        self.panel_items.editing_item = None;
        self.panel_skills.selected_gemlink = 0;
        self.panel_skills.selected_gem = None;
    }

    pub fn snapshot(&mut self) {
        if self.history_idx > 0 {
            self.history.drain(0..self.history_idx);
            self.history_idx = 0;
        }
        self.history.push_front(self.build.clone());
        if self.history.len() >= 40 {
            self.history.pop_back();
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev_build) = self.history.get(self.history_idx + 1) {
            self.build = prev_build.clone();
            self.history_idx += 1;
            self.request_recalc = true;
            self.request_regen_gl = true;
        }
    }

    pub fn redo(&mut self) {
        if self.history_idx == 0 {
            return;
        }
        if let Some(build) = self.history.get(self.history_idx - 1) {
            self.build = build.clone();
            self.request_recalc = true;
            self.request_regen_gl = true;
            self.history_idx -= 1;
        }
    }

    pub fn compare(&self, build_compare: &Build) -> FxHashMap<&'static str, i64> {
        let mut delta = FxHashMap::default();
        if let Some(gem_link_compare) = build_compare.gem_links.get(self.gemlink_cur) {
            if let Some(active_gem_compare) = gem_link_compare.active_gems().nth(self.active_skill_cur) {
                let supports: Vec<&Gem> = gem_link_compare.support_gems().filter(|g| g.enabled).collect();
                let active_gem_compare_calc = calc::calc_gem(build_compare, &supports, active_gem_compare);
                delta.extend(calc::compare(&self.active_skill_calc, &active_gem_compare_calc));
            }
        }
        let (defence_compare_calc, _) = calc::calc_defence(build_compare);
        delta.extend(calc::compare(&self.defence_calc, &defence_compare_calc));
        delta
    }

    pub fn recalc(&mut self) {
        self.can_save = true;
        self.build.update_item_allocations();
        let mods = self.build.calc_mods(true);
        let stats = self.build.calc_stats(&mods, BitFlags::EMPTY, BitFlags::EMPTY);
        self.passives_count = self.build.tree.passives_count();
        self.passives_max = stats.val(lightning_model::build::stat::StatId::PassiveSkillPoints);
        self.abyssal_sockets = stats.val(lightning_model::build::stat::StatId::AbyssalSockets) as u16;
        let (defence_calc, mut defence_stats) = calc::calc_defence(&self.build);
        for stat in defence_stats.stats.values_mut() {
            stat.mods.sort_unstable_by(|a, b| {
                let type_score = |t: lightning_model::modifier::Type| match t {
                    lightning_model::modifier::Type::Override => 3,
                    lightning_model::modifier::Type::Base => 2,
                    lightning_model::modifier::Type::Inc => 1,
                    lightning_model::modifier::Type::More => 0,
                };
                let a_score = type_score(a.typ);
                let b_score = type_score(b.typ);
                if a_score != b_score {
                    b_score.cmp(&a_score)
                } else {
                    b.final_amount().cmp(&a.final_amount())
                }
            });
        }
        self.defence_calc = defence_calc;
        self.defence_stats = defence_stats;
        self.active_skill_calc.clear();
        if let Some(gem_link) = self.build.gem_links.get(self.gemlink_cur) {
            if let Some(active_gem) = gem_link.active_gems().nth(self.active_skill_cur) {
                let supports: Vec<&Gem> = gem_link.support_gems().filter(|g| g.enabled).collect();
                self.active_skill_calc = calc::calc_gem(&self.build, &supports, active_gem);
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
                    let gem = Gem::new(id.clone(), true, 20, 20, 0);
                    if i > 0 {
                        build_compare.gem_links[self.panel_skills.selected_gemlink].gems.pop();
                    }
                    build_compare.gem_links[self.panel_skills.selected_gemlink].gems.push(gem);
                    let compare = self.compare(&build_compare);
                    let delta_dps = compare.get("DPS").unwrap_or(&0);
                    vec.push((*delta_dps, gem_data));
                }
                vec.sort_unstable_by(|a, b| if a.0 != b.0 { b.0.cmp(&a.0) } else { a.1.display_name().cmp(b.1.display_name()) });
                self.panel_skills.computed_gems = Some(vec);
            }
        } else {
            self.panel_skills.computed_gems = None;
        }
        if self.panel_bottom.power_report_checkbox {
            self.power_report = Some(PowerReport::new_defence(&self.build, "Maximum Life"));
        }
        self.request_recalc = false;
    }

    pub fn regen_quadtree_hover(&mut self) {
        self.quadtree_hover = QuadTreeHover::build(&self.build.tree.nodes_data);
    }

    pub fn save_build(&mut self) {
        if let Err(err) = self.build.save(&self.config.builds_dir) {
            eprintln!("Failed to save build: {err}");
        } else {
            self.can_save = false;
        }
    }
}

pub fn select_mastery_effect(ctx: &egui::Context, current_masteries: &FxHashMap<u32, u32>, mastery: &Node) -> Option<u32> {
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
                for effect in mastery.mastery_effects.iter().filter(|e| !current_masteries.iter().any(|(_, cur_effect)| *cur_effect == e.effect)) {
                    let mut string = String::new();
                    for (i, stat) in effect.stats.iter().enumerate() {
                        string.push_str(stat);
                        if i != effect.stats.len() - 1 {
                            string.push('\n');
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
