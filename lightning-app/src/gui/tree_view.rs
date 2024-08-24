use super::State;
use imgui::Ui;
use lightning_model::tree::NodeType;

fn draw_hover_window(ui: &mut Ui, state: &mut State) {
    let node = state.hovered_node.unwrap();

    ui.window("##NodeHover")
        .position(
            [state.mouse_pos.0 + 15.0, state.mouse_pos.1 + 15.0],
            imgui::Condition::Always,
        )
        .always_auto_resize(true)
        .focus_on_appearing(false)
        .movable(false)
        .resizable(false)
        .title_bar(false)
        .build(|| {
            ui.text(&node.name);
            ui.separator();

            match node.node_type() {
                NodeType::Mastery => {
                    for effect in &node.mastery_effects {
                        for stat in &effect.stats {
                            ui.text(stat);
                        }
                    }
                }
                _ => {
                    for stat in &node.stats {
                        ui.text(stat);
                    }
                }
            }
        });
}

pub fn draw(ui: &mut Ui, state: &mut State) {
    if state.hovered_node.is_some() {
        draw_hover_window(ui, state);
    }
}
