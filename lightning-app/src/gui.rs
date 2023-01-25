use crate::config::Config;
use crate::tree_gl::TreeGl;
use glutin::event::ElementState;
use imgui::{ListBox, MouseButton, Ui};
use lightning_model::build::Build;
use lightning_model::calc;
use rustc_hash::FxHashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UiState {
    ChooseBuild,
    LoadBuild(PathBuf),
    ImportBuild,
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
    pub tree_gl: TreeGl,
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

            tree_gl: Default::default(),
            zoom: 1.0,
            tree_translate: (0, 0),
            key_left: ElementState::Released,
            key_right: ElementState::Released,
            key_up: ElementState::Released,
            key_down: ElementState::Released,
        }
    }
}

pub fn get_build_files(build_dir: &PathBuf) -> Vec<PathBuf> {
    let mut build_files = vec![];

    let files = match fs::read_dir(build_dir) {
        Ok(files) => files,
        Err(_) => return build_files,
    };

    for file in files {
        build_files.push(file.unwrap().path());
    }

    build_files
}

pub fn draw_builds(ui: &mut Ui, state: &mut State) {
    ui.window("Build Selection")
        .size([500.0, 500.0], imgui::Condition::FirstUseEver)
        .build(|| {
            let build_files = get_build_files(&state.config.builds_dir);
            ListBox::new("Local saves").build(ui, || {
                for (index, item) in build_files.iter().enumerate() {
                    //let selected = matches!(state.builds_list_cur, i if i == index);
                    //let selected = index == state.builds_list_cur;
                    if ui
                        .selectable_config(item.clone().with_extension("").file_name().unwrap().to_str().unwrap())
                        //.selected(selected)
                        .allow_double_click(true)
                        .build()
                    {
                        state.builds_list_cur = index;
                        if ui.is_mouse_double_clicked(MouseButton::Left) {
                            state.ui_state = UiState::LoadBuild(item.clone());
                        }
                    }
                }
            });
            ui.separator();
            ui.text("From pathofexile.com");
            ui.input_text("Account", &mut state.import_account).build();
            ui.input_text("Character", &mut state.import_character).build();
            if ui.button("Import") {
                state.ui_state = UiState::ImportBuild;
            }
            if state.ui_state == UiState::ImportBuild {
                ui.text("Importing..");
            }
        });
}
pub fn draw_main(ui: &mut Ui, state: &mut State) {
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
                        state.active_skill_calc_res = calc::calc_gem(&state.build, &vec![], gem);
                    }
                }
                combo.end();
            }
            for (k, v) in &state.active_skill_calc_res {
                ui.text(k.to_string() + ": " + &v.to_string());
            }
        });
}
