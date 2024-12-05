use super::{State, UiState};
use std::{io, fs};
use std::path::PathBuf;

fn get_build_files(build_dir: &PathBuf) -> io::Result<Vec<PathBuf>> {
    let files = fs::read_dir(build_dir)?;
    Ok(files.map(|f| f.unwrap().path()).collect())
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::Window::new("Build Selection")
        .default_size(egui::Vec2::new(500.0, 500.0))
        .collapsible(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New Build").clicked() {
                    state.ui_state = UiState::NewBuild;
                }
                if ui.button("Settings").clicked {
                    state.show_settings = !state.show_settings;
                }
            });
            ui.separator();
            let build_files = get_build_files(&state.config.builds_dir).unwrap_or_default();
            ui.columns(1, |uis| {
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::DARK_GRAY).show(&mut uis[0], |ui| {
                    for item in build_files {
                        if ui.selectable_label(false, egui::RichText::new(item.clone().with_extension("").file_name().unwrap().to_str().unwrap()).color(egui::Color32::WHITE)).clicked() {
                            state.ui_state = UiState::LoadBuild(item.clone());
                        }
                    }
                });
            });
            #[cfg(feature = "import")]
            {
                ui.separator();
                ui.label("From pathofexile.com");
                ui.add(egui::TextEdit::singleline(&mut state.import_account).hint_text("Account"));
                ui.add(egui::TextEdit::singleline(&mut state.import_character).hint_text("Character"));

                if ui.button("Import").clicked() {
                    state.ui_state = UiState::ImportBuild;
                }
                if state.ui_state == UiState::ImportBuild {
                    ui.label("Importing..");
                }
            }

            ui.allocate_space(ui.available_size());
        });
}
