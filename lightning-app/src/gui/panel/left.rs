use crate::gui::{MainState, State, UiState};
use thousands::Separable;
use super::{text_gemlink, text_gemlink_cutoff};

pub const WIDTH: f32 = 240.0;

fn selected_text_gemlink(state: &State) -> String {
    if state.build.gem_links.is_empty() {
        return String::from("<No Gemlink>");
    }
    if let Some(selected) = state.build.gem_links.get(state.gemlink_cur) {
        return text_gemlink_cutoff(selected, 30);
    }
    return String::from("");
}

fn selected_text_active(state: &State) -> &str {
    if state.build.gem_links.iter().flat_map(|gl| gl.active_gems()).count() == 0 {
        return "<No Active Skill>";
    }
    if let Some(gemlink) = state.build.gem_links.get(state.gemlink_cur) {
        if let Some(active_skill) = gemlink.active_gems().nth(state.active_skill_cur) {
            return active_skill.data().display_name();
        }
    }
    return "";
}

fn calc_result_color(label: &str) -> egui::Color32 {
    match label {
        "Maximum Life" => egui::Color32::LIGHT_RED,
        "Life Regeneration" => egui::Color32::LIGHT_RED,
        "Strength" => egui::Color32::LIGHT_RED,

        "Fire Resistance" => egui::Color32::RED,

        "Dexterity" => egui::Color32::GREEN,
        "Chaos Resistance" => egui::Color32::DARK_GREEN,
        "Evasion" => egui::Color32::GREEN,

        "Intelligence" => egui::Color32::LIGHT_BLUE,
        "Cold Resistance" => egui::Color32::LIGHT_BLUE,
        "Energy Shield" => egui::Color32::LIGHT_BLUE,

        "Lightning Resistance" => egui::Color32::YELLOW,
        _ => egui::Color32::WHITE,
    }
}

enum Format {
    Flat,
    Percent,
    Percent100,
    PercentOtherStat(i64),
}

fn val_format(label: &str, val: i64, fmt: Format) -> String {
    match fmt {
        Format::Flat => {
            match label {
                "Speed" => format!("{:.2}", 1000.0 / val as f32),
                _ => val.separate_with_commas(),
            }
        }
        Format::Percent => {
            format!("{}%", val)
        }
        Format::Percent100 => {
            format!("{}%", (val as f32 / 100.0))
        }
        Format::PercentOtherStat(val_2) => {
            if val > val_2 {
                format!("{}% ({:+}%)", val_2, val - val_2)
            } else {
                format!("{}%", val)
            }
        }
    }
}

// TODO: cache these
fn draw_calc_result_row(ui: &mut egui::Ui, label: &str, val: Option<&i64>, fmt: Format) {
    if let Some(val) = val {
        if *val == 0 {
            return;
        }
        ui.label(egui::RichText::new(format!("{label}:")).color(calc_result_color(label)));
        ui.label(egui::RichText::new(val_format(label, *val, fmt)).color(calc_result_color(label)));
        ui.end_row();
    }
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
                if ui.button("Skills").clicked() {
                    state.ui_state = UiState::Main(MainState::Skills);
                }
                ui.end_row();
            });
            egui::ComboBox::from_id_salt("combo_gemlink")
                .selected_text(selected_text_gemlink(state))
                .show_ui(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    for gem_link in state.build.gem_links.iter().enumerate() {
                        if gem_link.1.active_gems().count() == 0 {
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
                        for active_gem in gemlink.active_gems().enumerate() {
                            if ui.selectable_value(&mut state.active_skill_cur, active_gem.0, active_gem.1.data().display_name()).clicked() {
                                state.request_recalc = true;
                            }
                        }
                    }
                }
            );
            egui::Grid::new("grid_active_skill_calc").show(ui, |ui| {
                draw_calc_result_row(ui, "DPS", state.active_skill_calc.get("DPS"), Format::Flat);
                draw_calc_result_row(ui, "Speed", state.active_skill_calc.get("Speed"), Format::Flat);
                draw_calc_result_row(ui, "Chance to Hit (MH)", state.active_skill_calc.get("Chance to Hit (MH)"), Format::Percent);
                draw_calc_result_row(ui, "Chance to Hit (OH)", state.active_skill_calc.get("Chance to Hit (OH)"), Format::Percent);
                draw_calc_result_row(ui, "Crit Chance (MH)", state.active_skill_calc.get("Crit Chance (MH)"), Format::Percent100);
                draw_calc_result_row(ui, "Crit Chance (OH)", state.active_skill_calc.get("Crit Chance (OH)"), Format::Percent100);
                draw_calc_result_row(ui, "Crit Multi", state.active_skill_calc.get("Crit Multi"), Format::Percent);
            });
            ui.separator();
            egui::Grid::new("grid_defence_calc").show(ui, |ui| {
                draw_calc_result_row(ui, "Maximum Life", state.defence_calc.get("Maximum Life"), Format::Flat);
                draw_calc_result_row(ui, "Life Regeneration", state.defence_calc.get("Life Regeneration"), Format::Flat);
                ui.separator(); ui.end_row();
                draw_calc_result_row(ui, "Fire Resistance", state.defence_calc.get("Fire Resistance"), Format::PercentOtherStat(*state.defence_calc.get("Maximum Fire Resistance").unwrap()));
                draw_calc_result_row(ui, "Cold Resistance", state.defence_calc.get("Cold Resistance"), Format::PercentOtherStat(*state.defence_calc.get("Maximum Cold Resistance").unwrap()));
                draw_calc_result_row(ui, "Lightning Resistance", state.defence_calc.get("Lightning Resistance"), Format::PercentOtherStat(*state.defence_calc.get("Maximum Lightning Resistance").unwrap()));
                draw_calc_result_row(ui, "Chaos Resistance", state.defence_calc.get("Chaos Resistance"), Format::PercentOtherStat(*state.defence_calc.get("Maximum Chaos Resistance").unwrap()));
                ui.separator(); ui.end_row();
                draw_calc_result_row(ui, "Armour", state.defence_calc.get("Armour"), Format::Flat);
                draw_calc_result_row(ui, "Evasion", state.defence_calc.get("Evasion"), Format::Flat);
                draw_calc_result_row(ui, "Energy Shield", state.defence_calc.get("Energy Shield"), Format::Flat);
                ui.separator(); ui.end_row();
                draw_calc_result_row(ui, "Strength", state.defence_calc.get("Strength"), Format::Flat);
                draw_calc_result_row(ui, "Dexterity", state.defence_calc.get("Dexterity"), Format::Flat);
                draw_calc_result_row(ui, "Intelligence", state.defence_calc.get("Intelligence"), Format::Flat);
            });
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                if ui.button("Settings").clicked {
                    state.show_settings = !state.show_settings;
                }
            });
            ui.allocate_space(ui.available_size());
        });
}
