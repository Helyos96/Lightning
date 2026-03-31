use lightning_model::{build::Slot, item::Item, modifier::Source};
use crate::gui::{State, utils::{draw_item, draw_item_window, rarity_to_color}};

const SLOTS: [Slot; 15] = [
    Slot::Weapon,
    Slot::Offhand,
    Slot::Helm,
    Slot::Amulet,
    Slot::BodyArmour,
    Slot::Gloves,
    Slot::Boots,
    Slot::Belt,
    Slot::Ring,
    Slot::Ring2,
    Slot::Flask(0),
    Slot::Flask(1),
    Slot::Flask(2),
    Slot::Flask(3),
    Slot::Flask(4),
];

#[derive(Default)]
pub struct ItemsPanelState {
    pub custom_text: String,
    pub editing_item_idx: Option<usize>,
    pub editing_item: Option<Item>,
    pub can_save: bool,
    pub flask_enabled: [bool; 5],
    pub hovered_item_idx: Option<usize>,
    pub hovered_item_deltas: Vec<(String, rustc_hash::FxHashMap<&'static str, i64>)>,
}

fn format_slot(slot: Slot) -> String {
    match slot {
        Slot::Weapon => "Weapon".to_string(),
        Slot::Offhand => "Offhand".to_string(),
        Slot::Helm => "Helmet".to_string(),
        Slot::Amulet => "Amulet".to_string(),
        Slot::BodyArmour => "Body Armour".to_string(),
        Slot::Gloves => "Gloves".to_string(),
        Slot::Boots => "Boots".to_string(),
        Slot::Belt => "Belt".to_string(),
        Slot::Ring => "Ring 1".to_string(),
        Slot::Ring2 => "Ring 2".to_string(),
        Slot::Flask(i) => format!("Flask {}", i + 1),
        Slot::TreeJewel(i) => format!("Jewel {}", i),
    }
}

fn item_to_richtext(item: &Item) -> egui::RichText {
    let text = if !item.name.is_empty() {
        format!("{}, {}", item.name, item.data().name)
    } else {
        item.data().name.to_owned()
    };
    egui::RichText::new(text).color(rarity_to_color(item.rarity))
}

const COMBO_WIDTH: f32 = 300.0;

fn draw_item_combo(ui: &mut egui::Ui, state: &mut State, slot: Slot) -> Option<usize> {
    let mut ret = None;
    let mut hovered_idx = None;
    let idx = state.build.equipment.get(&slot);
    let selected_text = match state.build.get_equipped(slot) {
        Some(item) => item_to_richtext(item),
        None => egui::RichText::new("<No Item>"),
    };
    let mut item_hover = None;

    let label_text = format_slot(slot);

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if let Slot::Flask(flask_idx) = slot {
            ui.checkbox(&mut state.panel_items.flask_enabled[flask_idx as usize], "");
        }
        ui.label(egui::RichText::new(label_text).strong());
    });

    let response = egui::ComboBox::from_id_salt(format!("item_choice_{:?}", slot))
        .width(COMBO_WIDTH)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            ui.spacing_mut().item_spacing = [ui.spacing().item_spacing.x, ui.spacing().item_spacing.y - 2.0].into();
            if ui.selectable_label(false, "<No Item>").clicked() && idx.is_some() {
                ret = Some(None);
            }
            for (i, item) in state.build.inventory.iter().enumerate().filter(|(_, it)| it.data().item_class.allowed_slots().iter().any(|&s| s.compatible(slot))) {
                let response = ui.selectable_label(idx.is_some() && *idx.unwrap() == i, item_to_richtext(item));
                if response.clicked() {
                    ret = Some(Some(i));
                } else if response.hovered() {
                    item_hover = Some(item);
                    hovered_idx = Some(i);
                }
            }
        }).response;

    if let Some(item) = item_hover {
        draw_item_window(ui, item, [response.rect.max.x + 10.0, response.rect.min.y], state.config.show_debug, Some(&state.panel_items.hovered_item_deltas));
    } else if response.hovered() && idx.is_some() {
        hovered_idx = Some(*idx.unwrap());
        draw_item_window(ui, state.build.get_equipped(slot).unwrap(), [response.rect.max.x + 10.0, response.rect.min.y], state.config.show_debug, Some(&state.panel_items.hovered_item_deltas));
    }

    match ret {
        Some(Some(i)) => {
            state.build.equipment.insert(slot, i);
            state.request_recalc = true;
        },
        Some(None) => {
            state.build.equipment.remove(&slot);
            state.request_recalc = true;
        },
        None => {},
    }

    ui.end_row();

    hovered_idx
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    let mut newly_hovered_idx = None;

    egui::CentralPanel::default()
        .show(ctx, |ui| {
           ui.columns(3, |uis| {
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[0] /*ui*/, |ui| {
                    egui::Grid::new("slots_grid")
                        .num_columns(2)
                        .spacing([10.0, 4.0])
                        .show(ui, |ui| {
                            for slot in SLOTS {
                                if let Some(hov) = draw_item_combo(ui, state, slot) {
                                    newly_hovered_idx = Some(hov);
                                }
                            }
                            let jewel_slots = state.build.tree.jewel_slots();
                            for jewel_node in jewel_slots {
                                if let Some(hov) = draw_item_combo(ui, state, Slot::TreeJewel(jewel_node)) {
                                    newly_hovered_idx = Some(hov);
                                }
                            }
                        });
                });
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[1] /*ui*/, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, item) in state.build.inventory.iter().enumerate() {
                            let response = ui.selectable_label(state.panel_items.editing_item_idx == Some(i), item_to_richtext(item));
                            if response.hovered() {
                                newly_hovered_idx = Some(i);
                                draw_item_window(ui, item, [state.mouse_pos.0 + 15.0, state.mouse_pos.1 + 15.0], state.config.show_debug, Some(&state.panel_items.hovered_item_deltas));
                            }
                            if response.clicked() {
                                state.panel_items.editing_item_idx = Some(i);
                                state.panel_items.custom_text = item.to_str();
                                state.panel_items.editing_item = Some(item.clone());
                            }
                        }
                    });
                });
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[2] /*ui*/, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Clear").clicked() {
                            state.panel_items.editing_item_idx = None;
                            state.panel_items.editing_item = None;
                            state.panel_items.can_save = false;
                            state.panel_items.custom_text.clear();
                        }
                        if ui.add_enabled(state.panel_items.can_save, egui::Button::new("Save")).clicked() {
                            state.panel_items.can_save = false;
                            state.request_recalc = true;
                            state.build.inventory[state.panel_items.editing_item_idx.unwrap()] = state.panel_items.editing_item.as_ref().unwrap().to_owned();
                        }
                        if ui.add_enabled(state.panel_items.editing_item_idx.is_some(), egui::Button::new("Delete")).clicked() {
                            state.build.remove_inventory(state.panel_items.editing_item_idx.unwrap());
                            state.panel_items.can_save = false;
                            state.panel_items.custom_text.clear();
                            state.panel_items.editing_item_idx = None;
                            state.panel_items.editing_item = None;
                            state.request_recalc = true;
                        }
                        if ui.add_enabled(state.panel_items.editing_item.is_some() && state.panel_items.editing_item_idx.is_none(), egui::Button::new("Add to Build")).clicked() {
                            state.build.inventory.push(state.panel_items.editing_item.as_ref().unwrap().to_owned());
                            state.panel_items.editing_item_idx = Some(state.build.inventory.len() - 1);
                        }
                    });
                    egui::ScrollArea::vertical().id_salt("custom_item").max_height(400.0).show(ui, |ui| {
                        let response = egui::TextEdit::multiline(&mut state.panel_items.custom_text).desired_width(f32::INFINITY).show(ui).response;
                        if response.changed() {
                            state.panel_items.editing_item = Item::from_str(&state.panel_items.custom_text);
                            if state.panel_items.editing_item.is_some() && state.panel_items.editing_item_idx.is_some() {
                                state.panel_items.can_save = true;
                            } else {
                                state.panel_items.can_save = false;
                            }
                        }
                    });
                    if let Some(item) = state.panel_items.editing_item.as_ref() {
                        ui.separator();
                        draw_item(ui, item, Source::Innate, state.config.show_debug);
                    }
                });
            });
        });

    if state.panel_items.hovered_item_idx != newly_hovered_idx {
        state.panel_items.hovered_item_idx = newly_hovered_idx;
        state.panel_items.hovered_item_deltas.clear();

        if let Some(idx) = newly_hovered_idx {
            let item = state.build.inventory.get(idx).cloned();
            if let Some(item) = item {
                // 1. Calculate removal deltas
                let equipped_in: Vec<Slot> = state.build.equipment.iter()
                    .filter_map(|(s, i)| if *i == idx { Some(*s) } else { None })
                    .collect();

                for slot in equipped_in {
                    let mut build_compare = state.build.clone();
                    build_compare.equipment.remove(&slot);
                    let delta = state.compare(&build_compare);
                    if !delta.is_empty() {
                        let name = format!("Remove from {}", format_slot(slot));
                        state.panel_items.hovered_item_deltas.push((name, delta));
                    }
                }

                // 2. Calculate equip deltas
                let mut potential_slots = vec![];
                potential_slots.extend_from_slice(&SLOTS);
                for jewel_node in state.build.tree.jewel_slots() {
                    potential_slots.push(Slot::TreeJewel(jewel_node));
                }

                for slot in potential_slots {
                    if item.data().item_class.allowed_slots().iter().any(|&s| s.compatible(slot)) {
                        // Skip if already equipped in this exact slot
                        if state.build.equipment.get(&slot) == Some(&idx) {
                            continue;
                        }
                        let mut build_compare = state.build.clone();
                        build_compare.equipment.insert(slot, idx);
                        let delta = state.compare(&build_compare);
                        if !delta.is_empty() {
                            let action = if state.build.equipment.contains_key(&slot) {
                                "Replace"
                            } else {
                                "Equip in"
                            };
                            let name = format!("{} {}", action, format_slot(slot));
                            state.panel_items.hovered_item_deltas.push((name, delta));
                        }
                    }
                }
            }
        }
    }
}
