use imgui::Ui;
use super::State;
use lightning_model::calc;

fn draw_left_panel(ui: &mut Ui, state: &mut State) {
    ui.window("##LeftPanel")
        .position([0.0, 0.0], imgui::Condition::FirstUseEver)
        .size([200.0, 1024.0], imgui::Condition::FirstUseEver)
        .movable(false)
        .resizable(false)
        .title_bar(false)
        .build(|| {
            let preview = match state
                .build
                .gem_links
                .iter()
                .flat_map(|gl| &gl.active_gems)
                .nth(state.active_skill_cur)
            {
                Some(gem) => &gem.data().base_item.as_ref().unwrap().display_name,
                None => "",
            };
            if let Some(combo) = ui.begin_combo("##ActiveSkills", preview) {
                for (index, gem) in state.build.gem_links.iter().flat_map(|gl| &gl.active_gems).enumerate() {
                    let selected = index == state.active_skill_cur;
                    if ui
                        .selectable_config(&gem.data().base_item.as_ref().unwrap().display_name)
                        .selected(selected)
                        .build()
                    {
                        state.active_skill_cur = index;
                        state.active_skill_calc_res = calc::calc_gem(&state.build, &vec![], gem);
                    }
                }
                combo.end();
            }
            for (k, v) in &state.active_skill_calc_res {
                ui.text(k.to_string() + ": " + &v.to_string());
            }
        }
    );
}

fn draw_hover_window(ui: &mut Ui, state: &mut State) {
    let node = state.hovered_node.unwrap();

    ui.window("##NodeHover")
        .position([state.mouse_pos.0 + 15.0, state.mouse_pos.1 + 15.0], imgui::Condition::Always)
        .always_auto_resize(true)
        .focus_on_appearing(false)
        .movable(false)
        .resizable(false)
        .title_bar(false)
        .build(|| {
            ui.text(&node.name);
            ui.separator();
            for stat in &node.stats {
                ui.text(stat);
            }
        });
        
}

pub fn draw(ui: &mut Ui, state: &mut State) {
    draw_left_panel(ui, state);
    if state.hovered_node.is_some() {
        draw_hover_window(ui, state);
    }
}