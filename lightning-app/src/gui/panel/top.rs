use std::{ops::RangeInclusive};
use lightning_model::{build::{property, BanditChoice, CampaignChoice}, data::TREE, data::tree::Ascendancy};
use strum::IntoEnumIterator;
use crate::gui::{State, UiState};

pub const HEIGHT: f32 = 40.0;

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::TopBottomPanel::top("TopPanel")
        .resizable(false)
        .exact_height(HEIGHT)
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.button("<< Builds").clicked() {
                    state.ui_state = UiState::ChooseBuild;
                }
                ui.add(egui::TextEdit::singleline(&mut state.build.name).desired_width(100.0));
                if ui.add_enabled(state.can_save, egui::Button::new("Save")).clicked() {
                    state.save_build();
                }
                ui.label("Level");
                if ui.add(egui::DragValue::new(&mut state.level).range(RangeInclusive::new(1, 100))).changed() {
                    state.build.set_property_int(property::Int::Level, state.level);
                    state.request_recalc = true;
                }
                egui::ComboBox::from_id_salt("combo_class")
                    .selected_text(state.build.tree.class.as_ref())
                    .show_ui(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        for class in TREE.classes.keys() {
                            if ui.selectable_label(*class == state.build.tree.class, class.as_ref()).clicked() {
                                state.build.tree.set_class(*class);
                                state.request_regen_gl = true;
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
                                state.request_regen_gl = true;
                                state.request_recalc = true;
                            }
                        }
                    }
                );

                let bloodlines: Vec<Ascendancy> = Ascendancy::iter()
                    .skip_while(|&asc| asc != Ascendancy::Aul)
                    .collect();

                let bloodline_selected_text = match state.build.tree.bloodline {
                    Some(bloodline) => bloodline.display_name(),
                    None => "None",
                };

                egui::ComboBox::from_id_salt("combo_bloodline")
                    .selected_text(bloodline_selected_text)
                    .show_ui(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

                        if ui.selectable_label(state.build.tree.bloodline.is_none(), "None").clicked() {
                            state.build.tree.set_bloodline(None);
                            state.request_regen_gl = true;
                            state.request_recalc = true;
                        }

                        for asc in &bloodlines {
                            if ui.selectable_label(Some(*asc) == state.build.tree.bloodline, asc.display_name()).clicked() {
                                state.build.tree.set_bloodline(Some(*asc));
                                state.request_regen_gl = true;
                                state.request_recalc = true;
                            }
                        }
                    }
                );

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
                egui::ComboBox::from_id_salt("campaign_choice")
                    .selected_text(state.build.campaign_choice.as_ref())
                    .show_ui(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                        for campaign_choice in CampaignChoice::iter() {
                            if ui.selectable_label(campaign_choice == state.build.campaign_choice, campaign_choice.as_ref()).clicked() {
                                state.build.campaign_choice = campaign_choice;
                                state.request_recalc = true;
                            }
                        }
                    }
                );

                // Could optimize: don't recalc passives_count() every frame
                ui.label(format!("Passives: {}/{}", state.passives_count, state.passives_max));

                if state.config.show_debug {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("Redraws: {}", state.redraw_counter));
                    });
                }
                ui.allocate_space(ui.available_size());
            });
        });
}
