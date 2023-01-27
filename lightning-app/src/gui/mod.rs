pub mod build_selection;
pub mod tree_view;

use crate::config::Config;
use glutin::event::ElementState;
use lightning_model::build::{Build, Stat};
use rustc_hash::FxHashMap;
use std::path::PathBuf;
use lightning_model::tree::Node;
use lightning_model::calc;
use imgui::Ui;

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

    // widget-specific values
    builds_list_cur: usize,
    active_skill_cur: usize,

    // OpenGL stuff
    pub dimensions: (u32, u32),
    pub tree_translate: (i32, i32),
    pub zoom: f32,

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

            builds_list_cur: 0,
            active_skill_cur: 0,

            dimensions: (1280, 720),
            zoom: 1.0,
            tree_translate: (0, 0),

            mouse_pos: (0.0, 0.0),
            key_left: ElementState::Released,
            key_right: ElementState::Released,
            key_up: ElementState::Released,
            key_down: ElementState::Released,
        }
    }
}

pub fn draw_left_panel(ui: &mut Ui, state: &mut State) {
    ui.window("##LeftPanel")
        .position([0.0, 0.0], imgui::Condition::FirstUseEver)
        .size([200.0, 1024.0], imgui::Condition::FirstUseEver)
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
        }
    );
}
