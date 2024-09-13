use super::State;
//use imgui::Ui;
use std::{ops::RangeInclusive, path::PathBuf};

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::Window::new("Settings")
        .default_size(egui::Vec2::new(500.0, 500.0))
        .collapsible(false)
        .show(ctx, |ui| {
            egui::Grid::new("grid_settings").show(ui, |ui| {
                ui.label("Builds directory");
                let mut size = ui.spacing().interact_size;
                size.x = 250.0;
                if ui.add_sized(size, egui::TextEdit::singleline(&mut state.builds_dir_settings)).changed() {
                    if let Ok(path) = PathBuf::try_from(&state.builds_dir_settings) {
                        state.config.builds_dir = path;
                        let _ = state.config.save();
                    }
                }
                ui.end_row();
                ui.label("Framerate");
                if ui.add_enabled(!state.config.vsync, egui::DragValue::new(&mut state.framerate_settings).range(RangeInclusive::new(20, 240))).changed() {
                    state.config.framerate = state.framerate_settings;
                    let _ = state.config.save();
                }
                ui.end_row();
                ui.label("VSync");
                if ui.checkbox(&mut state.config.vsync, "").clicked() {
                    let _ = state.config.save();
                }
                ui.end_row();
            });
            if ui.button("Close").clicked() {
                state.show_settings = false;
            }
            ui.allocate_space(ui.available_size());
        });
}
