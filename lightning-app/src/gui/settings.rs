use super::State;
use imgui::Ui;
use std::path::PathBuf;

pub fn draw(ui: &mut Ui, state: &mut State) {
    ui.window("Settings")
        .size([500.0, 500.0], imgui::Condition::FirstUseEver)
        .position([500.0, 60.0], imgui::Condition::FirstUseEver)
        .build(|| {
            if ui.input_text("Builds path", &mut state.builds_dir_settings).build() {
                if let Ok(path) = PathBuf::try_from(&state.builds_dir_settings) {
                    state.config.builds_dir = path;
                    let _ = state.config.save();
                }
            }
            if ui.input_scalar("Framerate", &mut state.framerate_settings).build() {
                if state.framerate_settings >= 20 {
                    state.config.framerate = state.framerate_settings;
                } else {
                    state.framerate_settings = 20;
                }
                let _ = state.config.save();
            }
            if ui.checkbox("VSync", &mut state.config.vsync) {
                let _ = state.config.save();
            }
            if ui.button("Close") {
                state.show_settings = false;
            }
        });
}
