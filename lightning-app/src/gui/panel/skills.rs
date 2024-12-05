use std::ops::RangeInclusive;

use egui_extras::{Column, TableBuilder};
use lightning_model::{data::GEMS, gem::Gem};
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

fn draw_skill_dropdown(ui: &mut egui::Ui, panel_skills: &mut SkillsPanelState, socketed_gem: Option<&mut Gem>, i: usize, request_recalc: &mut bool) -> Option<&'static str> {
    let mut ret = None;

    let is_currently_selected = {
        match panel_skills.selected_gem {
            Some(index) => index == i,
            None => false
        }
    };
    let name = {
        if is_currently_selected {
            &mut panel_skills.selected_gem_text
        } else if let Some(socketed_gem) = socketed_gem.as_ref() {
            &mut socketed_gem.data().display_name().to_owned()
        } else {
            &mut panel_skills.selected_gem_text
        }
    };
    let mut edit = egui::TextEdit::singleline(name);
    if let Some(socketed_gem) = socketed_gem {
        edit = edit.hint_text(socketed_gem.data().display_name());
    }
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
    if ui.memory(|m| m.is_popup_open(popup_id)) {
        if let Some(computed_gems) = panel_skills.computed_gems.as_ref() {
            egui::popup_below_widget(
                ui,
                popup_id,
                &r,
                egui::PopupCloseBehavior::CloseOnClick,
                |ui| {
                    // Disable label text selection, otherwise the cursor doesn't select the entire line
                    // when you hover a label.
                    ui.style_mut().interaction.selectable_labels = false;
                    let table = TableBuilder::new(ui)
                        .column(Column::remainder())
                        .column(Column::remainder())
                        .striped(true)
                        .sense(egui::Sense::click())
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::AlwaysVisible)
                        .max_scroll_height(400.0);
                    table.body(|body| {
                        let computed_gems_filtered: Vec<(i64, &'static str)> =
                            computed_gems.iter().filter(|v| name.is_empty() || v.1.to_lowercase().contains(&name.to_lowercase())).copied().collect();
                        body.rows(18.0, computed_gems_filtered.len(), |mut row| {
                            let (dps, gem_name) = computed_gems_filtered[row.index()];
                            row.col(|ui| {
                                ui.label(gem_name);
                            });
                            row.col(|ui| {
                                if dps != 0 {
                                    ui.label(format!("DPS: {}", dps.separate_with_commas()));
                                }
                            });
                            if row.response().clicked() {
                                ret = Some(gem_name);
                            }
                        });
                    });
                },
            );
        }
    } else if is_currently_selected {
        panel_skills.selected_gem = None;
    }

    ret
}

enum Action {
    AddGem(&'static str),
    SwapGem(&'static str),
    RemoveGem(usize),
    AddGemlink,
    RemoveSelectedGemlink,
}

fn gem_from_display_name(display_name: &str) -> Gem {
    let gem_id = GEMS.iter().find_map(|(id, gem_data)| if gem_data.display_name() == display_name { Some(id) } else { None }).unwrap();
    Gem {
        id: gem_id.clone(),
        enabled: true,
        level: 20,
        qual: 20,
        alt_qual: 0,
    }
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    let mut action: Option<Action> = None;
    egui::CentralPanel::default()
        .show(ctx, |ui| {
            ui.columns(2, |uis| {
                egui::Grid::new("grid_ui_gemlink").show(&mut uis[0], |ui| {
                    if ui.button("New").clicked() {
                        action = Some(Action::AddGemlink);
                    }
                    if ui.button("Delete").clicked() {
                        action = Some(Action::RemoveSelectedGemlink);
                    }
                    ui.end_row();
                });
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[0], |ui| {
                    for (i, gemlink) in state.build.gem_links.iter().enumerate() {
                        if ui.selectable_label(i == state.panel_skills.selected_gemlink, text_gemlink_cutoff(gemlink, 40)).clicked() {
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
                            .column(Column::auto())
                            .column(Column::remainder())
                            .column(Column::auto())
                            .column(Column::auto())
                            .column(Column::auto())
                            .vscroll(false)
                            .header(14.0, |mut header| {
                                header.col(|_| {
                                });
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
                                    row.col(|ui| {
                                        if ui.button("x").clicked() {
                                            action = Some(Action::RemoveGem(i));
                                        }
                                    });
                                    // Gem Name
                                    row.col(|ui| {
                                        if let Some(gem_name) = draw_skill_dropdown(ui, &mut state.panel_skills, Some(socketed_gem), i, &mut state.request_recalc) {
                                            action = Some(Action::SwapGem(gem_name));
                                        }
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
                            // Show empty gem slot
                            body.row(14.0, |mut row| {
                                row.col(|_| {
                                });
                                row.col(|ui| {
                                    if let Some(gem_name) = draw_skill_dropdown(ui, &mut state.panel_skills, None, gemlink.gems.len(), &mut state.request_recalc) {
                                        action = Some(Action::AddGem(gem_name));
                                    }
                                });
                                row.col(|_| {
                                });
                                row.col(|_| {
                                });
                                row.col(|_| {
                                });
                            });
                        });
                    }
                });
            });
        });
    if let Some(action) = action {
        match action {
            Action::RemoveGem(i) => {
                if let Some(gemlink) = state.build.gem_links.get_mut(state.panel_skills.selected_gemlink) {
                    gemlink.gems.remove(i);
                } else {
                    eprintln!("Trying to remove gem {i} but no selected gemlink");
                }
            }
            Action::SwapGem(gem_name) => {
                let gem = gem_from_display_name(gem_name);
                if let Some(gemlink) = state.build.gem_links.get_mut(state.panel_skills.selected_gemlink) {
                    gemlink.gems[state.panel_skills.selected_gem.unwrap()] = gem;
                } else {
                    eprintln!("Trying to swap gem \"{gem_name}\" but no selected gemlink");
                }
            }
            Action::AddGem(gem_name) => {
                let gem = gem_from_display_name(gem_name);
                if let Some(gemlink) = state.build.gem_links.get_mut(state.panel_skills.selected_gemlink) {
                    gemlink.gems.push(gem);
                } else {
                    eprintln!("Trying to push gem \"{gem_name}\" but no selected gemlink");
                }
            }
            Action::AddGemlink => {
                state.build.gem_links.push(Default::default());
            }
            Action::RemoveSelectedGemlink => {
                if state.build.gem_links.len() > state.panel_skills.selected_gemlink {
                    state.build.gem_links.remove(state.panel_skills.selected_gemlink);
                }
            }
        }
        state.request_recalc = true;
        state.panel_skills.selected_gem = None;
    }
}
