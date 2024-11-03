pub mod build_selection;
pub mod tree_view;
pub mod settings;

use crate::config::Config;
use lightning_model::build::{BanditChoice, Build, StatId, Stats};
use lightning_model::calc;
use lightning_model::data::TREE;
use lightning_model::modifier::{PropertyBool, PropertyInt};
use lightning_model::tree::Node;
use rustc_hash::FxHashMap;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::{io, fs};
use egui_glow::egui_winit::winit::event::ElementState;
use std::time::Instant;
use lazy_static::lazy_static;
use strum::IntoEnumIterator;

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
    // idx into gem_links; idx into gem_links.active_gems
    pub selected_gem: (usize, usize),
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
    active_skill_cur: usize,
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
            selected_gem: (0, 0),
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

pub fn select_mastery_effect(ctx: &egui::Context, mastery: &Node) -> Option<u16> {
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
                for effect in &mastery.mastery_effects {
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

const LEFT_PANEL_WIDTH: f32 = 240.0;
const TOP_PANEL_HEIGHT: f32 = 40.0;

fn get_selected_text(state: &State) -> &str {
    if state.build.gem_links.iter().flat_map(|gl| &gl.active_gems).count() == 0 {
        return "<No Active Skill>";
    }
    return &state.build.gem_links.iter().flat_map(|gl| &gl.active_gems).nth(state.active_skill_cur).unwrap().data().base_item.as_ref().unwrap().display_name;
}

pub fn draw_left_panel(ctx: &egui::Context, state: &mut State) {
    egui::SidePanel::left("LeftPanel")
        .resizable(false)
        .exact_width(LEFT_PANEL_WIDTH)
        .show(ctx, |ui| {
            egui::Grid::new("grid_ui_select").show(ui, |ui| {
                if ui.button("Tree").clicked() {
                    state.ui_state = UiState::Main(MainState::Tree);
                }
                if ui.button("Config").clicked() {
                    state.ui_state = UiState::Main(MainState::Config);
                }
                ui.end_row();
            });
            egui::ComboBox::from_id_salt("bandit_choice")
                .selected_text(state.build.bandit_choice.as_ref())
                .show_ui(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    for bandit_choice in BanditChoice::iter() {
                        if ui.selectable_label(bandit_choice == state.build.bandit_choice, bandit_choice.as_ref()).clicked() {
                            state.build.bandit_choice = bandit_choice;
                            state.request_recalc = true;
                        }
                    }
                }
            );
            egui::ComboBox::from_id_salt("combo_active_skill")
                .selected_text(get_selected_text(state))
                .show_ui(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    let mut index = 0;
                    for gem_link in state.build.gem_links.iter().enumerate() {
                        for active_gem in gem_link.1.active_gems.iter().enumerate() {
                            if ui.selectable_value(&mut state.active_skill_cur, index, &active_gem.1.data().base_item.as_ref().unwrap().display_name).clicked() {
                                state.selected_gem = (gem_link.0, active_gem.0);
                                state.active_skill_calc = calc::calc_gem(&state.build, &gem_link.1.support_gems, active_gem.1);
                            }
                            index += 1;
                        }
                    }
                }
            );
            egui::Grid::new("grid_active_skill_calc").show(ui, |ui| {
                for (k, v) in &state.active_skill_calc {
                    ui.label(k.to_string() + ":");
                    ui.label(v.to_string());
                    ui.end_row();
                }
            });
            ui.separator();
            egui::Grid::new("grid_defence_calc").show(ui, |ui| {
                for stat in &state.defence_calc {
                    ui.label(stat.0.to_string() + ":");
                    ui.label(stat.1.to_string());
                    ui.end_row();
                }
            });
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                if ui.button("Settings").clicked {
                    state.show_settings = !state.show_settings;
                }
            });
            ui.allocate_space(ui.available_size());
        });
}

fn save_build(build: &Build, dir: &Path) -> io::Result<()> {
    let mut file_path = dir.join(&build.name);
    file_path.set_extension("json");
    serde_json::to_writer(&fs::File::create(file_path)?, build)?;
    Ok(())
}

pub fn draw_top_panel(ctx: &egui::Context, state: &mut State) {
    egui::TopBottomPanel::top("TopPanel")
        .resizable(false)
        .exact_height(TOP_PANEL_HEIGHT)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.button("<< Builds").clicked() {
                    state.ui_state = UiState::ChooseBuild;
                }
                ui.text_edit_singleline(&mut state.build.name);
                if ui.button("Save").clicked() {
                    if let Err(err) = save_build(&state.build, &state.config.builds_dir) {
                        eprintln!("Failed to save build: {err}");
                    }
                }
                ui.label("Level");
                if ui.add(egui::DragValue::new(&mut state.level).range(RangeInclusive::new(1, 100))).changed() {
                    state.build.set_property_int(PropertyInt::Level, state.level);
                    state.request_recalc = true;
                }
                egui::ComboBox::from_id_salt("combo_class")
                    .selected_text(state.build.tree.class.as_ref())
                    .show_ui(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        for class in TREE.classes.keys() {
                            if ui.selectable_label(*class == state.build.tree.class, class.as_ref()).clicked() {
                                state.build.tree.set_class(*class);
                                state.request_regen = true;
                                state.request_recalc = true;
                            }
                        }
                    }
                );
                let selected_text = match state.build.tree.ascendancy {
                    Some(ascendancy) => ascendancy.into(),
                    None => "None",
                };
                egui::ComboBox::from_id_salt("combo_ascendancy")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        for ascendancy in state.build.tree.class.ascendancies() {
                            if ui.selectable_label(Some(ascendancy) == state.build.tree.ascendancy, Into::<&str>::into(ascendancy)).clicked() {
                                state.build.tree.set_ascendancy(Some(ascendancy));
                                state.request_regen = true;
                                state.request_recalc = true;
                            }
                        }
                    }
                );
                // Could optimize: don't recalc passives_count() every frame
                ui.label(format!("Passives: {}/{}", state.build.tree.passives_count(), state.stats.as_ref().unwrap().stat(StatId::PassiveSkillPoints).val()));
                ui.allocate_space(ui.available_size());
            });
        });
}

lazy_static! {
    static ref PROPERTIES_INT: Vec<(PropertyInt, &'static str)> = vec![
        (PropertyInt::FrenzyCharges, "Frenzy Charges"),
        (PropertyInt::PowerCharges, "Power Charges"),
        (PropertyInt::EnduranceCharges, "Endurance Charges"),
        (PropertyInt::Rage, "Rage"),
    ];
    static ref PROPERTIES_BOOL: Vec<(PropertyBool, &'static str)> = vec![
        (PropertyBool::Fortified, "Are you Fortified?"),
        (PropertyBool::Blinded, "Are you Blind?"),
        (PropertyBool::Onslaught, "Do you have Onslaught?"),
        (PropertyBool::DealtCritRecently, "Dealt a Crit Recently?"),
        (PropertyBool::Leeching, "Are you Leeching?"),
        (PropertyBool::OnFullLife, "Are you on Full Life?"),
        (PropertyBool::OnLowLife, "Are you on Low Life?"),
    ];
}

pub fn draw_config_panel(ctx: &egui::Context, state: &mut State) {
    egui::CentralPanel::default()
        .show(ctx, |ui| {
            ui.columns(2, |uis| {
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[0], |ui| {
                    egui::Grid::new("grid_ui_property_int").show(ui, |ui| {
                        for pint in PROPERTIES_INT.iter() {
                            let mut property = state.build.property_int(pint.0);
                            ui.label(pint.1);
                            if ui.add(egui::DragValue::new(&mut property)).changed() {
                                state.build.set_property_int(pint.0, property);
                                state.request_recalc = true;
                            }
                            ui.end_row();
                        }
                    });
                });
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[1], |ui| {
                    egui::Grid::new("grid_ui_property_bool").show(ui, |ui| {
                        for pbool in PROPERTIES_BOOL.iter() {
                            let mut property = state.build.property_bool(pbool.0);
                            ui.label(pbool.1);
                            if ui.checkbox(&mut property, "").clicked() {
                                state.build.set_property_bool(pbool.0, property);
                                state.request_recalc = true;
                            }
                            ui.end_row();
                        }
                    });
                });
            });
        });
}

pub fn is_over_tree(pos: &(f32, f32)) -> bool {
    pos.0 >= LEFT_PANEL_WIDTH && pos.1 >= TOP_PANEL_HEIGHT
}
