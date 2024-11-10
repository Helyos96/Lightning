use std::{fs, io, ops::RangeInclusive, path::Path};
use lightning_model::{build::{property, Build, StatId}, data::TREE};
use crate::gui::{State, UiState};

pub const HEIGHT: f32 = 40.0;

fn save_build(build: &Build, dir: &Path) -> io::Result<()> {
    let mut file_path = dir.join(&build.name);
    file_path.set_extension("json");
    serde_json::to_writer(&fs::File::create(file_path)?, build)?;
    Ok(())
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::TopBottomPanel::top("TopPanel")
        .resizable(false)
        .exact_height(HEIGHT)
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
                if state.config.show_debug {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("Redraws: {}", state.redraw_counter));
                    });
                }
                ui.allocate_space(ui.available_size());
            });
        });
}
