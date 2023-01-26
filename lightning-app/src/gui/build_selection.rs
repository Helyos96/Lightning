use imgui::{ListBox, MouseButton, Ui};
use std::fs;
use std::path::PathBuf;
use super::{UiState, State};

pub fn get_build_files(build_dir: &PathBuf) -> Vec<PathBuf> {
    let mut build_files = vec![];

    let files = match fs::read_dir(build_dir) {
        Ok(files) => files,
        Err(_) => return build_files,
    };

    for file in files {
        build_files.push(file.unwrap().path());
    }

    build_files
}

pub fn draw(ui: &mut Ui, state: &mut State) {
    ui.window("Build Selection")
        .size([500.0, 500.0], imgui::Condition::FirstUseEver)
        .build(|| {
            if ui.button("New Build") {
                state.ui_state = UiState::NewBuild;
            }
            ui.separator();
            let build_files = get_build_files(&state.config.builds_dir);
            ListBox::new("Local saves").build(ui, || {
                for (index, item) in build_files.iter().enumerate() {
                    if ui
                        .selectable_config(item.clone().with_extension("").file_name().unwrap().to_str().unwrap())
                        .allow_double_click(true)
                        .build()
                    {
                        state.builds_list_cur = index;
                        if ui.is_mouse_double_clicked(MouseButton::Left) {
                            state.ui_state = UiState::LoadBuild(item.clone());
                        }
                    }
                }
            });
            ui.separator();
            ui.text("From pathofexile.com");
            ui.input_text("Account", &mut state.import_account).build();
            ui.input_text("Character", &mut state.import_character).build();
            if ui.button("Import") {
                state.ui_state = UiState::ImportBuild;
            }
            if state.ui_state == UiState::ImportBuild {
                ui.text("Importing..");
            }
        });
}
