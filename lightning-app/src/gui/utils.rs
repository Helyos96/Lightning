use lightning_model::{data::{DAMAGE_GROUPS, base_item::Rarity, gem::{GemData, GemTag}}, item::Item, modifier::Source};

pub fn rarity_to_color(rarity: Rarity) -> egui::Color32 {
    match rarity {
        Rarity::Normal => egui::Color32::WHITE,
        Rarity::Magic => egui::Color32::LIGHT_BLUE,
        Rarity::Rare => egui::Color32::YELLOW,
        Rarity::Unique => egui::Color32::from_rgb(252, 132, 3),
    }
}

pub fn gem_colour(gem: &'static GemData) -> egui::Color32 {
    match gem.color.as_str() {
        "r" => egui::Color32::LIGHT_RED,
        "g" => egui::Color32::LIGHT_GREEN,
        "b" => egui::Color32::LIGHT_BLUE,
        _ => egui::Color32::WHITE,
    }
}

pub fn gem_name_richtext(gem: &'static GemData) -> egui::RichText {
    egui::RichText::new(gem.display_name()).color(gem_colour(gem))
}

pub fn mod_to_richtext(mod_str: &str, source: Source, show_debug: bool) -> egui::text::LayoutJob {
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

pub fn draw_item(ui: &mut egui::Ui, item: &Item, source: Source, show_debug: bool) {
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        if !item.name.is_empty() {
            ui.label(egui::RichText::new(&item.name).color(rarity_to_color(item.rarity)).size(20.0));
        }
        ui.label(egui::RichText::new(&item.base_item).color(rarity_to_color(item.rarity)).size(20.0));
        ui.separator();

        let defences = item.calc_defence();
        let armour = defences.armour.val();
        let evasion = defences.evasion.val();
        let energy_shield = defences.energy_shield.val();
        let block_chance = defences.block_chance.val();
        if armour > 0 || evasion > 0 || energy_shield > 0 || block_chance > 0 || item.quality > 0 || item.attack_speed().is_some() {
            if item.quality > 0 {
                ui.label(format!("Quality: {}%", item.quality));
            }
            if let Some(attack_speed) = item.attack_speed() {
                for dg in DAMAGE_GROUPS {
                    if let Some((min, max)) = item.calc_dmg(dg.damage_type) {
                        ui.label(format!("{} Damage: {}-{}", Into::<&str>::into(dg.damage_type), min, max));
                    }
                }
                ui.label(format!("Attack Speed: {:.2}", 1000.0 / attack_speed as f32));
            }
            if armour > 0 {
                ui.label(format!("Armour: {armour}"));
            }
            if evasion > 0 {
                ui.label(format!("Evasion: {evasion}"));
            }
            if energy_shield > 0 {
                ui.label(format!("Energy Shield: {energy_shield}"));
            }
            if block_chance > 0 {
                ui.label(format!("Block Chance: {block_chance}%"));
            }
            ui.separator();
        }
        //ui.spacing_mut().item_spacing = item_spacing;
        if !item.mods_enchant.is_empty() {
            for stat in &item.mods_enchant {
                ui.label(mod_to_richtext(stat, source, show_debug));
            }
            ui.separator();
        }
        if !item.mods_impl.is_empty() {
            for stat in &item.mods_impl {
                ui.label(mod_to_richtext(stat, source, show_debug));
            }
            ui.separator();
        }
        for stat in &item.mods_expl {
            ui.label(mod_to_richtext(stat, source, show_debug));
        }
    });
}

pub fn draw_item_window(ui: &mut egui::Ui, item: &Item, pos: impl Into<egui::Pos2>, show_debug: bool, deltas: Option<&[(String, rustc_hash::FxHashMap<&'static str, i64>)]>) {
    let window_id = egui::Id::new("Hover Item")
        .with(&item.name)
        .with(&item.base_item)
        .with(deltas.map_or(0, |d| d.len()));

    egui::Window::new("Hover Item")
        .id(window_id)
        .collapsible(false)
        .movable(false)
        .title_bar(false)
        .resizable(false)
        .fixed_pos(pos)
        .frame(egui::Frame::window(&ui.ctx().style()))
        .show(ui.ctx(), |ui| {
            draw_item(ui, item, Source::Innate, show_debug);
            if let Some(deltas) = deltas {
                draw_item_deltas(ui, deltas);
            }
        });
}

pub fn draw_item_deltas(ui: &mut egui::Ui, deltas: &[(String, rustc_hash::FxHashMap<&'static str, i64>)]) {
    if !deltas.is_empty() {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.separator();
            let mut item_spacing = ui.spacing().item_spacing;
            item_spacing.y -= 5.0;
            ui.spacing_mut().item_spacing = item_spacing;

            let col_count = deltas.len() + 1;
            let total_spacing = 20.0 * (col_count - 1) as f32;
            let col_width = ((ui.available_width() - total_spacing) / col_count as f32).max(50.0);

            egui::Grid::new("delta_grid")
                .num_columns(col_count)
                .min_col_width(col_width)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Stat").strong());
                    for (name, _) in deltas {
                        ui.label(egui::RichText::new(name).strong());
                    }
                    ui.end_row();

                    let mut keys: Vec<&'static str> = deltas.iter().flat_map(|(_, m)| m.keys()).copied().collect();
                    keys.sort_unstable();
                    keys.dedup();

                    for k in keys {
                        ui.label(k);
                        for (_, delta_map) in deltas {
                            let val = delta_map.get(k).unwrap_or(&0);
                            if *val != 0 {
                                ui.label(format!("{val:+}"));
                            } else {
                                ui.label("-");
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }
}
