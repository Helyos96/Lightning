use super::State;
use lightning_model::item::Rarity;
use lightning_model::modifier::Source;
use lightning_model::tree::NodeType;
use lightning_model::build::Slot;

fn rarity_to_color(rarity: Rarity) -> egui::Color32 {
    match rarity {
        Rarity::Normal => egui::Color32::WHITE,
        Rarity::Magic => egui::Color32::LIGHT_BLUE,
        Rarity::Rare => egui::Color32::YELLOW,
        Rarity::Unique => egui::Color32::from_rgb(252, 132, 3),
    }
}

fn mod_to_richtext(mod_str: &str, source: Source, show_debug: bool) -> egui::text::LayoutJob {
    let mut ret = egui::text::LayoutJob::default();
    let modifier = lightning_model::modifier::parse_mod(mod_str, source);

    match modifier {
        Some(modifier) => {
            ret.append(
                mod_str,
                0.0,
                egui::text::TextFormat {
                    color: egui::Color32::LIGHT_BLUE,
                    ..Default::default()
                },
            );
            if show_debug {
                ret.append(
                    format!("\n{:?}", modifier).as_str(),
                    0.0,
                    egui::text::TextFormat {
                        font_id: egui::FontId::new(10.0, egui::FontFamily::Monospace),
                        color: egui::Color32::LIGHT_GRAY,
                        ..Default::default()
                    },
                );
            }
        }
        None => {
            ret.append(
                mod_str,
                0.0,
                egui::text::TextFormat {
                    color: egui::Color32::LIGHT_RED,
                    ..Default::default()
                },
            );
        }
    }
    ret
}

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
                    if let Some(item) = state.build.equipment.get(&Slot::TreeJewel(node.skill)) {
                        ui.label(egui::RichText::new(&item.name).color(rarity_to_color(item.rarity)).size(20.0));
                        ui.separator();

                        ui.spacing_mut().item_spacing = item_spacing;
                        for stat in &item.mods_impl {
                            ui.label(mod_to_richtext(stat, Source::Item(Slot::TreeJewel(node.skill)), state.config.show_debug));
                        }
                        if !item.mods_impl.is_empty() {
                            ui.separator();
                        }
                        for stat in &item.mods_expl {
                            ui.label(mod_to_richtext(stat, Source::Item(Slot::TreeJewel(node.skill)), state.config.show_debug));
                        }
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
                        ui.label(mod_to_richtext(stat, Source::Node(node.skill), state.config.show_debug));
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
