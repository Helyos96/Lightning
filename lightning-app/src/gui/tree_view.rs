use super::State;
use super::utils::{draw_item, mod_to_richtext};
use lightning_model::build::Slot;
use lightning_model::data::tree::NodeType;
use lightning_model::modifier::Source;

fn is_mouse_left_area(state: &State) -> bool {
    let tree_three_quarters = ((state.dimensions.0 as f32 - super::panel::left::WIDTH) * 0.75)
        + super::panel::left::WIDTH;
    if state.mouse_pos.0 <= tree_three_quarters {
        return true;
    }
    false
}

fn is_mouse_top_area(state: &State) -> bool {
    let tree_center = (state.dimensions.1 as f32 / 2.0) + (super::panel::top::HEIGHT / 2.0);
    if state.mouse_pos.1 <= tree_center {
        return true;
    }
    false
}

// Used to adjust where the hover window will pop depending on where the mouse is
fn get_align(state: &State) -> (egui::Align2, (f32, f32)) {
    let (h_align, h_margin) = if is_mouse_left_area(state) {
        (egui::Align::Min, 15.0)
    } else {
        (egui::Align::Max, -15.0)
    };

    let (v_align, v_margin) = if is_mouse_top_area(state) {
        (egui::Align::Min, 15.0)
    } else {
        (egui::Align::Max, -15.0)
    };

    (egui::Align2([h_align, v_align]), (h_margin, v_margin))
}

fn draw_hover_window(ctx: &egui::Context, state: &mut State) {
    let node = state.build.tree.nodes_data.get(&state.hovered_node_id.unwrap()).unwrap();
    let c = ctx.style().visuals.window_fill;
    let background_color = egui::Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), 210);
    let (align, margin) = get_align(state);
    egui::Window::new("Hover")
        .collapsible(false)
        .movable(false)
        .title_bar(false)
        .resizable(false)
        .pivot(align)
        .fixed_pos([state.mouse_pos.0 + margin.0, state.mouse_pos.1 + margin.1])
        .frame(egui::Frame::window(&ctx.style()).fill(background_color))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                let mut item_spacing = ui.spacing().item_spacing;
                item_spacing.y += 5.0;
                match node.node_type() {
                    NodeType::JewelSocket => {
                        if let Some(item) = state.build.get_equipped(Slot::TreeJewel(node.skill)) {
                            draw_item(
                                ui,
                                item,
                                Source::Item(Slot::TreeJewel(node.skill)),
                                state.config.show_debug,
                            );
                        } else {
                            ui.label(
                                egui::RichText::new(&node.name)
                                    .color(egui::Color32::WHITE)
                                    .size(20.0),
                            );
                        }
                    }
                    NodeType::Mastery => {
                        ui.label(
                            egui::RichText::new(&node.name)
                                .color(egui::Color32::WHITE)
                                .size(20.0),
                        );
                        ui.separator();
                        ui.spacing_mut().item_spacing = item_spacing;
                        // Try and find if we have a selected mastery effect for that mastery
                        if let Some(selected) = state.build.tree.masteries.get(&node.skill) {
                            if let Some(effect) =
                                node.mastery_effects.iter().find(|m| m.effect == *selected)
                            {
                                for stat in &effect.stats {
                                    ui.label(mod_to_richtext(
                                        stat,
                                        Source::Mastery((node.skill, effect.effect)),
                                        state.config.show_debug,
                                    ));
                                }
                            }
                        } else {
                            for effect in &node.mastery_effects {
                                for stat in &effect.stats {
                                    ui.label(mod_to_richtext(
                                        stat,
                                        Source::Mastery((node.skill, effect.effect)),
                                        state.config.show_debug,
                                    ));
                                }
                            }
                        }
                    }
                    _ => {
                        ui.label(
                            egui::RichText::new(&node.name)
                                .color(egui::Color32::WHITE)
                                .size(20.0),
                        );
                        ui.separator();
                        ui.spacing_mut().item_spacing = item_spacing;
                        for stat in &node.stats {
                            for mod_str in stat.split('\n') {
                                ui.label(mod_to_richtext(
                                    mod_str,
                                    Source::Node(node.skill),
                                    state.config.show_debug,
                                ));
                            }
                        }
                    }
                }
                if !state.delta_compare.is_empty() || !state.delta_compare_single.is_empty() {
                    ui.separator();
                    item_spacing.y -= 5.0;
                    ui.spacing_mut().item_spacing = item_spacing;
                    let nb_nodes = if let Some(path_hovered) = &state.path_hovered {
                        path_hovered.len() - 1
                    } else if let Some(path_red) = &state.path_red {
                        path_red.len()
                    } else {
                        0
                    };

                    egui::Grid::new("delta_grid")
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Stat").strong());
                            ui.label(egui::RichText::new("This node").strong());
                            if nb_nodes > 1 {
                                ui.label(egui::RichText::new("All nodes").strong());
                            }
                            ui.end_row();

                            let mut keys: Vec<&'static str> = state
                                .delta_compare
                                .keys()
                                .chain(state.delta_compare_single.keys())
                                .copied()
                                .collect();
                            keys.sort_unstable();
                            keys.dedup();

                            for k in keys {
                                ui.label(k);

                                let single = state.delta_compare_single.get(k).unwrap_or(&0);
                                if *single != 0 {
                                    ui.label(format!("{single:+}"));
                                } else {
                                    ui.label("-");
                                }

                                if nb_nodes > 1 {
                                    let all = state.delta_compare.get(k).unwrap_or(&0);
                                    if *all != 0 {
                                        ui.label(format!("{all:+}"));
                                    } else {
                                        ui.label("-");
                                    }
                                }

                                ui.end_row();
                            }
                        });
                }
                if state.config.show_debug {
                    ui.separator();
                    ui.label(format!("node id: {} / orbit_index: {}", node.skill, node.orbit_index.unwrap_or(0)));
                }
            });
        });
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    if state.hovered_node_id.is_some() {
        draw_hover_window(ctx, state);
    }
}
