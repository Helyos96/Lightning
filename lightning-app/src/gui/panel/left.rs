use lightning_model::build::{BanditChoice, GemLink};
use strum::IntoEnumIterator;
use crate::gui::{MainState, State, UiState};

pub const WIDTH: f32 = 240.0;

fn text_gemlink(gemlink: &GemLink) -> String {
    let mut ret = String::new();
    for active_gem in &gemlink.active_gems {
        ret += &active_gem.data().base_item.as_ref().unwrap().display_name;
        ret += ", ";
    }
    return String::from(ret.trim_end_matches(", "));
}

fn selected_text_gemlink(state: &State) -> String {
    if state.build.gem_links.len() == 0 {
        return String::from("<No Gemlink>");
    }
    if let Some(selected) = state.build.gem_links.get(state.gemlink_cur) {
        let mut ret = text_gemlink(selected);
        if ret.len() > 30 {
            ret = String::from(&ret[0..27]) + "...";
        }
        return ret;
    }
    return String::from("");
}

fn selected_text_active(state: &State) -> &str {
    if state.build.gem_links.iter().flat_map(|gl| &gl.active_gems).count() == 0 {
        return "<No Active Skill>";
    }
    if let Some(gemlink) = state.build.gem_links.get(state.gemlink_cur) {
        if let Some(active_skill) = gemlink.active_gems.get(state.active_skill_cur) {
            return &active_skill.data().base_item.as_ref().unwrap().display_name;
        }
    }
    return "";
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::SidePanel::left("LeftPanel")
        .resizable(false)
        .exact_width(WIDTH)
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
            egui::ComboBox::from_id_salt("combo_gemlink")
                .selected_text(selected_text_gemlink(state))
                .show_ui(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    for gem_link in state.build.gem_links.iter().enumerate() {
                        if gem_link.1.active_gems.is_empty() {
                            continue;
                        }
                        if ui.selectable_value(&mut state.gemlink_cur, gem_link.0, &text_gemlink(gem_link.1)).clicked() {
                            state.active_skill_cur = 0;
                            state.request_recalc = true;
                        }
                    }
                }
            );
            egui::ComboBox::from_id_salt("combo_active_skill")
                .selected_text(selected_text_active(state))
                .show_ui(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    if let Some(gemlink) = state.build.gem_links.get(state.gemlink_cur) {
                        for active_gem in gemlink.active_gems.iter().enumerate() {
                            if ui.selectable_value(&mut state.active_skill_cur, active_gem.0, &active_gem.1.data().base_item.as_ref().unwrap().display_name).clicked() {
                                state.request_recalc = true;
                            }
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