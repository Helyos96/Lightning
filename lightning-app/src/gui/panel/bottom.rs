use lightning_model::calc::PowerReport;

use crate::gui::State;

pub const HEIGHT: f32 = 40.0;

#[derive(Default)]
pub struct BottomPanelState {
    pub search: String,
    pub search_nodes: Vec<u32>,
    pub power_report_checkbox: bool,
}

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
                        state.power_report = Some(PowerReport::new_defence(&state.build, "Maximum Life"));
                    } else {
                        state.power_report = None;
                    }
                    state.request_regen_nodes_gl = true;
                }
            });
        });
}