pub mod build_selection;
pub mod tree_view;

use crate::config::Config;
use glutin::event::ElementState;
use imgui::Ui;
use lightning_model::build::{Build, Stat};
use lightning_model::calc;
use lightning_model::data::TREE;
use lightning_model::tree::Node;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UiState {
    ChooseBuild,
    LoadBuild(PathBuf),
    ImportBuild,
    NewBuild,
    Main,
}

/// Global state, contains everything
pub struct State {
    pub ui_state: UiState,
    pub build: Build,
    pub config: Config,
    pub import_account: String,
    pub import_character: String,

    active_skill_calc: FxHashMap<&'static str, i64>,
    pub defence_calc: Vec<(String, Stat)>,
    pub hovered_node: Option<&'static Node>,
    pub path_hovered: Option<Vec<u16>>,
    pub mouse_tree_drag: Option<(f32, f32)>,

    // widget-specific values
    builds_list_cur: usize,
    active_skill_cur: usize,

    // OpenGL stuff
    pub dimensions: (u32, u32),
    pub tree_translate: (i32, i32),
    pub zoom: f32,
    pub request_regen: bool,

    // Controls
    pub mouse_pos: (f32, f32),
    pub key_left: ElementState,
    pub key_right: ElementState,
    pub key_up: ElementState,
    pub key_down: ElementState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            ui_state: UiState::ChooseBuild,
            build: Build::new_player(),
            config: Config::default(),
            import_account: String::new(),
            import_character: String::new(),

            active_skill_calc: FxHashMap::default(),
            defence_calc: vec![],
            hovered_node: None,
            path_hovered: None,
            mouse_tree_drag: None,

            builds_list_cur: 0,
            active_skill_cur: 0,

            dimensions: (1280, 720),
            zoom: 1.0,
            tree_translate: (0, 0),
            request_regen: false,

            mouse_pos: (0.0, 0.0),
            key_left: ElementState::Released,
            key_right: ElementState::Released,
            key_up: ElementState::Released,
            key_down: ElementState::Released,
        }
    }
}

const LEFT_PANEL_WIDTH: f32 = 200.0;
const TOP_PANEL_HEIGHT: f32 = 40.0;

pub fn draw_left_panel(ui: &mut Ui, state: &mut State) {
    ui.window("##LeftPanel")
        .position([0.0, TOP_PANEL_HEIGHT], imgui::Condition::FirstUseEver)
        .size(
            [LEFT_PANEL_WIDTH, state.dimensions.1 as f32 - TOP_PANEL_HEIGHT],
            imgui::Condition::Always,
        )
        .movable(false)
        .resizable(false)
        .title_bar(false)
        .build(|| {
            let preview = match state
                .build
                .gem_links
                .iter()
                .flat_map(|gl| &gl.active_gems)
                .nth(state.active_skill_cur)
            {
                Some(gem) => &gem.data().base_item.as_ref().unwrap().display_name,
                None => "",
            };
            if let Some(combo) = ui.begin_combo("##ActiveSkills", preview) {
                for (index, gem) in state.build.gem_links.iter().flat_map(|gl| &gl.active_gems).enumerate() {
                    let selected = index == state.active_skill_cur;
                    if ui
                        .selectable_config(&gem.data().base_item.as_ref().unwrap().display_name)
                        .selected(selected)
                        .build()
                    {
                        state.active_skill_cur = index;
                        state.active_skill_calc = calc::calc_gem(&state.build, &vec![], gem);
                    }
                }
                combo.end();
            }
            for (k, v) in &state.active_skill_calc {
                ui.text(k.to_string() + ": " + &v.to_string());
            }
            ui.separator();
            for stat in &state.defence_calc {
                ui.text(stat.0.to_string() + ": " + &stat.1.val().to_string());
            }
        });
}

pub fn draw_top_panel(ui: &mut Ui, state: &mut State) {
    ui.window("##TopPanel")
        .position([0.0, 0.0], imgui::Condition::FirstUseEver)
        .size([state.dimensions.0 as f32, TOP_PANEL_HEIGHT], imgui::Condition::Always)
        .movable(false)
        .resizable(false)
        .title_bar(false)
        .build(|| {
            if ui.button("<< Builds") {
                state.ui_state = UiState::ChooseBuild;
            }
            ui.same_line();
            if ui.button("Save") {}
            ui.same_line();
            let preview = state.build.tree.class.as_str();
            let _w = ui.push_item_width(150.0);
            if let Some(combo) = ui.begin_combo("##ClassCombo", preview) {
                for class in TREE.classes.keys() {
                    let selected = *class == state.build.tree.class;
                    if ui.selectable_config(class.as_str()).selected(selected).build() {
                        state.build.tree.set_class(*class);
                        state.request_regen = true;
                    }
                }
                combo.end();
            }
        });
}

pub fn is_over_tree(pos: &(f32, f32)) -> bool {
    pos.0 >= LEFT_PANEL_WIDTH && pos.1 >= TOP_PANEL_HEIGHT
}
