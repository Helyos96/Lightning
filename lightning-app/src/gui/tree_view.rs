use super::State;
use lightning_model::tree::NodeType;
use lightning_model::build::Slot;

fn draw_hover_window(ctx: &egui::Context, state: &mut State) {
    let node = state.hovered_node.unwrap();
    egui::Window::new("Hover")
        .collapsible(false)
        .movable(false)
        .title_bar(false)
        .resizable(false)
        .fixed_pos([state.mouse_pos.0 + 15.0, state.mouse_pos.1 + 15.0])
        .show(ctx, |ui| {
            let mut item_spacing = ui.spacing().item_spacing;
            item_spacing.y += 5.0;
            match node.node_type() {
                NodeType::JewelSocket => {
                    if let Some(item) = state.build.equipment.get(&Slot::TreeJewel(node.skill)) {
                        ui.label(egui::RichText::new(&item.name).color(egui::Color32::WHITE).size(20.0));
                        ui.separator();

                        ui.spacing_mut().item_spacing = item_spacing;
                        for stat in &item.mods_impl {
                            ui.label(stat);
                        }
                        if !item.mods_impl.is_empty() {
                            ui.separator();
                        }
                        for stat in &item.mods_expl {
                            ui.label(stat);
                        }
                    } else {
                        ui.label(egui::RichText::new(&node.name).color(egui::Color32::WHITE).size(20.0));
                    }
                }
                NodeType::Mastery => {
                    ui.label(egui::RichText::new(&node.name).color(egui::Color32::WHITE).size(20.0));
                    ui.separator();
                    ui.spacing_mut().item_spacing = item_spacing;
                    for effect in &node.mastery_effects {
                        for stat in &effect.stats {
                            ui.label(stat);
                        }
                    }
                }
                _ => {
                    ui.label(egui::RichText::new(&node.name).color(egui::Color32::WHITE).size(20.0));
                    ui.separator();
                    ui.spacing_mut().item_spacing = item_spacing;
                    for stat in &node.stats {
                        ui.label(stat);
                    }
                }
            }
        });
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    if state.hovered_node.is_some() {
        draw_hover_window(ctx, state);
    }
}
