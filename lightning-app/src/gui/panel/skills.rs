use std::ops::RangeInclusive;

use egui_extras::{Column, TableBuilder};
use lightning_model::gem::Gem;
use thousands::Separable;
use crate::gui::State;
use super::text_gemlink_cutoff;

#[derive(Default)]
pub struct SkillsPanelState {
    pub selected_gemlink: usize,
    pub selected_gem: Option<usize>,
    selected_gem_text: String,
    pub computed_gems: Option<Vec<(i64, &'static str)>>,
}

fn draw_skill_dropdown(ui: &mut egui::Ui, panel_skills: &mut SkillsPanelState, socketed_gem: &mut Gem, i: usize, request_recalc: &mut bool) {
    let is_currently_selected = {
        match panel_skills.selected_gem {
            Some(index) => if index == i {
                true
            } else {
                false
            },
            None => false
        }
    };
    let name = {
        if is_currently_selected {
            &mut panel_skills.selected_gem_text
        } else {
            &mut socketed_gem.data().base_item.display_name.clone()
        }
    };
    let edit = egui::TextEdit::singleline(name).hint_text(&socketed_gem.data().base_item.display_name);
    let edit_output = edit.show(ui);
    let r = edit_output.response;
    let popup_id = egui::Id::new(format!("popup {}", i));
    if r.gained_focus() {
        ui.memory_mut(|m| m.open_popup(popup_id));
        panel_skills.selected_gem = Some(i);
        name.clear();
        if panel_skills.computed_gems.is_none() {
            *request_recalc = true;
        }
    }
    if r.changed() {

    }
    if ui.memory(|m| m.is_popup_open(popup_id)) {
        if let Some(computed_gems) = panel_skills.computed_gems.as_ref() {
            egui::popup_below_widget(
                ui,
                popup_id,
                &r,
                egui::PopupCloseBehavior::CloseOnClick,
                |ui| {
                    ui.set_max_height(400.0);
                    let table = TableBuilder::new(ui)
                        .column(Column::remainder())
                        .column(Column::remainder())
                        .max_scroll_height(400.0);
                    table.body(|body| {
                        let computed_gems_filtered: Vec<(i64, &'static str)> =
                            computed_gems.iter().filter(|v| name.is_empty() || v.1.to_lowercase().contains(&name.to_lowercase())).copied().collect();
                        body.rows(10.0, computed_gems_filtered.len(), |mut row| {
                            let (dps, gem_name) = computed_gems_filtered[row.index()];
                            row.col(|ui| {
                                ui.label(gem_name);
                            });
                            row.col(|ui| {
                                if dps != 0 {
                                    ui.label(format!("DPS: {}", dps.separate_with_commas()));
                                }
                            });
                        });
                    });
                },
            );
        }
    } else if is_currently_selected {
        panel_skills.selected_gem = None;
    }
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::CentralPanel::default()
        .show(ctx, |ui| {
            ui.columns(2, |uis| {
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[0], |ui| {
                    for (i, gemlink) in state.build.gem_links.iter().enumerate() {
                        if ui.selectable_label(false, &text_gemlink_cutoff(gemlink, 40)).clicked() {
                            if state.panel_skills.selected_gemlink != i {
                                state.panel_skills.selected_gemlink = i;
                                state.panel_skills.computed_gems = None;
                            }
                            state.panel_skills.selected_gem = None;
                        }
                    }
                });
                // Frame showing active/support gems in a gemlink
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[1], |ui| {
                    if let Some(gemlink) = state.build.gem_links.get_mut(state.panel_skills.selected_gemlink) {
                        let table = TableBuilder::new(ui)
                            .column(Column::remainder())
                            .column(Column::auto())
                            .column(Column::auto())
                            .column(Column::auto())
                            .vscroll(false)
                            .header(14.0, |mut header| {
                                header.col(|ui| {
                                    ui.strong("Gem Name");
                                });
                                header.col(|ui| {
                                    ui.strong("Level");
                                });
                                header.col(|ui| {
                                    ui.strong("Quality");
                                });
                                header.col(|ui| {
                                    ui.strong("Enabled");
                                });
                            });

                        table.body(|mut body| {
                            for (i, socketed_gem) in gemlink.gems.iter_mut().enumerate() {
                                body.row(14.0, |mut row| {
                                    // Gem Name
                                    row.col(|ui| {
                                        draw_skill_dropdown(ui, &mut state.panel_skills, socketed_gem, i, &mut state.request_recalc);
                                    });
                                    // Level
                                    row.col(|ui| {
                                        if ui.add(egui::DragValue::new(&mut socketed_gem.level).range(RangeInclusive::new(1, 40))).changed() {
                                            state.request_recalc = true;
                                        }
                                    });
                                    // Quality
                                    row.col(|ui| {
                                        if ui.add(egui::DragValue::new(&mut socketed_gem.qual).range(RangeInclusive::new(1, 100))).changed() {
                                            state.request_recalc = true;
                                        }
                                    });
                                    // Enabled
                                    row.col(|ui| {
                                        if ui.checkbox(&mut socketed_gem.enabled, "").clicked() {
                                            state.request_recalc = true;
                                        }
                                    });
                                });
                            }
                        });
                    }
                });
            });
        });
}
