pub mod build_selection;
pub mod left_panel;

use crate::config::Config;
use glutin::event::ElementState;
use lightning_model::build::Build;
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

    active_skill_calc_res: FxHashMap<&'static str, i64>,
    defence_calc_res: FxHashMap<&'static str, i64>,

    // widget-specific values
    builds_list_cur: usize,
    active_skill_cur: usize,

    // OpenGL stuff
    pub tree_translate: (i32, i32),
    pub zoom: f32,

    // Controls
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
            active_skill_calc_res: FxHashMap::default(),
            defence_calc_res: FxHashMap::default(),
            import_account: String::new(),
            import_character: String::new(),

            builds_list_cur: 0,
            active_skill_cur: 0,

            zoom: 1.0,
            tree_translate: (0, 0),
            key_left: ElementState::Released,
            key_right: ElementState::Released,
            key_up: ElementState::Released,
            key_down: ElementState::Released,
        }
    }
}

