use super::utils::{draw_item, mod_to_richtext};
use super::State;
use lightning_model::data::tree::NodeType;
use lightning_model::modifier::Source;
use lightning_model::build::Slot;

fn draw_hover_window(ctx: &egui::Context, state: &mut State) {
    let node = state.hovered_node.unwrap();
    let c = ctx.style().visuals.window_fill;
    let background_color = egui::Color32::from_rgba_premultiplied(c.r(), c.g(), c.b(), 210);
    egui::Window::new("Hover")
        .collapsible(false)
        .movable(false)
        .title_bar(false)
        .resizable(false)
        .fixed_pos([state.mouse_pos.0 + 15.0, state.mouse_pos.1 + 15.0])
        .frame(egui::Frame::window(&ctx.style()).fill(background_color))
        .show(ctx, |ui| {
            let mut item_spacing = ui.spacing().item_spacing;
            item_spacing.y += 5.0;
            match node.node_type() {
                NodeType::JewelSocket => {
                    if let Some(item) = state.build.get_equipped(Slot::TreeJewel(node.skill)) {
                        draw_item(ui, item, Source::Item(Slot::TreeJewel(node.skill)), state.config.show_debug);
                    } else {
                        ui.label(egui::RichText::new(&node.name).color(egui::Color32::WHITE).size(20.0));
                    }
                }
                NodeType::Mastery => {
                    ui.label(egui::RichText::new(&node.name).color(egui::Color32::WHITE).size(20.0));
                    ui.separator();
                    ui.spacing_mut().item_spacing = item_spacing;
                    // Try and find if we have a selected mastery effect for that mastery
                    if let Some(selected) = state.build.tree.masteries.get(&node.skill) {
                        if let Some(effect) = node.mastery_effects.iter().find(|m| m.effect == *selected) {
                            for stat in &effect.stats {
                                ui.label(mod_to_richtext(stat, Source::Mastery((node.skill, effect.effect)), state.config.show_debug));
                            }
                        }
                    } else {
                        for effect in &node.mastery_effects {
                            for stat in &effect.stats {
                                ui.label(mod_to_richtext(stat, Source::Mastery((node.skill, effect.effect)), state.config.show_debug));
                            }
                        }
                    }
                }
                _ => {
                    ui.label(egui::RichText::new(&node.name).color(egui::Color32::WHITE).size(20.0));
                    ui.separator();
                    ui.spacing_mut().item_spacing = item_spacing;
                    for stat in &node.stats {
                        for mod_str in stat.split('\n') {
                            ui.label(mod_to_richtext(mod_str, Source::Node(node.skill), state.config.show_debug));
                        }
                    }
                }
            }
            if !state.delta_compare.is_empty() {
                ui.separator();
                item_spacing.y -= 5.0;
                ui.spacing_mut().item_spacing = item_spacing;
                for (k, v) in &state.delta_compare {
                    ui.label(format!("{}: {:+}", k, v));
                }
            }
        });
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    if state.hovered_node.is_some() {
        draw_hover_window(ctx, state);
    }
}
