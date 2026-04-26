use lightning_model::calc::PowerReport;

use crate::gui::State;

pub const HEIGHT: f32 = 40.0;

pub struct BottomPanelState {
    pub search: String,
    pub search_nodes: Vec<u32>,
    pub power_report_checkbox: bool,
    pub power_report_selected: (&'static str, PowerReportType),
}

impl Default for BottomPanelState {
    fn default() -> Self {
        BottomPanelState {
            search: Default::default(),
            search_nodes: Default::default(),
            power_report_checkbox: false,
            power_report_selected: ("DPS", PowerReportType::Gem),
        }
    }
}

#[derive(Clone, Copy)]
pub enum PowerReportType {
    Defence,
    Gem,
}

const POWER_REPORT_OPTIONS: &[(&'static str, PowerReportType)] = &[
    ("DPS", PowerReportType::Gem),
    ("Crit Chance (MH)", PowerReportType::Gem),
    ("Crit Chance (OH)", PowerReportType::Gem),
    ("Crit Multi", PowerReportType::Gem),
    ("Bleed DPS", PowerReportType::Gem),
    ("Maximum Life", PowerReportType::Defence),
    ("Maximum Mana", PowerReportType::Defence),
    ("Fire Resistance", PowerReportType::Defence),
    ("Cold Resistance", PowerReportType::Defence),
    ("Lightning Resistance", PowerReportType::Defence),
    ("Chaos Resistance", PowerReportType::Defence),
    ("Life Regeneration", PowerReportType::Defence),
    ("Mana Regeneration", PowerReportType::Defence),
    ("Strength", PowerReportType::Defence),
    ("Dexterity", PowerReportType::Defence),
    ("Intelligence", PowerReportType::Defence),
    ("Armour", PowerReportType::Defence),
    ("Evasion", PowerReportType::Defence),
    ("Energy Shield", PowerReportType::Defence),
    ("Spell Suppression", PowerReportType::Defence),
    ("Block", PowerReportType::Defence),
    ("Spell Block", PowerReportType::Defence),
];

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::TopBottomPanel::bottom("BottomPanel")
        .resizable(false)
        .exact_height(HEIGHT)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let search = egui::TextEdit::singleline(&mut state.panel_bottom.search).desired_width(120.0).hint_text("Search");
                let response = search.show(ui).response;
                if response.changed() {
                    state.panel_bottom.search_nodes.clear();
                    let search_str = state.panel_bottom.search.to_lowercase();
                    if !state.panel_bottom.search.is_empty() {
                        'outer: for node in state.build.tree.nodes_data.values().filter(|n| n.group.is_some()) {
                            if node.name.to_lowercase().contains(&search_str) {
                                state.panel_bottom.search_nodes.push(node.skill);
                                continue;
                            }
                            for stat in &node.stats {
                                if stat.to_lowercase().contains(&search_str) {
                                    state.panel_bottom.search_nodes.push(node.skill);
                                    continue 'outer;
                                }
                            }
                            for mastery_effect in &node.mastery_effects {
                                for stat in &mastery_effect.stats {
                                    if stat.to_lowercase().contains(&search_str) {
                                        state.panel_bottom.search_nodes.push(node.skill);
                                        continue 'outer;
                                    }
                                }
                            }
                        }
                    }
                    state.request_regen_gl = true;
                }
                ui.label("Power Report:");
                if ui.checkbox(&mut state.panel_bottom.power_report_checkbox, "").changed() {
                    if state.panel_bottom.power_report_checkbox {
                        state.request_recalc = true;
                    } else {
                        state.request_regen_nodes_gl = true;
                        state.power_report = None;
                    }
                }
                egui::ComboBox::from_id_salt("combo_power_report")
                    .selected_text(state.panel_bottom.power_report_selected.0)
                    .show_ui(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        for (string, typ) in POWER_REPORT_OPTIONS {
                            if ui.selectable_label(*string == state.panel_bottom.power_report_selected.0, *string).clicked() {
                                state.panel_bottom.power_report_checkbox = true;
                                state.panel_bottom.power_report_selected = (string, *typ);
                                state.request_recalc = true;
                            }
                        }
                    }
                );
            });
        });
}