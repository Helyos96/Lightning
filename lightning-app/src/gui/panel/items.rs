use lightning_model::{build::Slot, item::Item, modifier::Source};
use crate::gui::{State, utils::{draw_item, draw_item_window, rarity_to_color}};

const SLOTS: [Slot; 10] = [
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
];

#[derive(Default)]
pub struct ItemsPanelState {
    pub custom_text: String,
    pub editing_item_idx: Option<usize>,
    pub editing_item: Option<Item>,
    pub can_save: bool,
}

fn item_to_richtext(item: &Item) -> egui::RichText {
    let text = if !item.name.is_empty() {
        format!("{}, {}", item.name, item.data().name)
    } else {
        item.data().name.to_owned()
    };
    egui::RichText::new(text).color(rarity_to_color(item.rarity))
}

// TODO: DPI Aware
const COMBO_WIDTH: f32 = 300.0;

fn draw_item_combo(ui: &mut egui::Ui, state: &mut State, slot: Slot) -> Option<Option<usize>> {
    let mut ret = None;
    let idx = state.build.equipment.get(&slot);
    let selected_text = match state.build.get_equipped(slot) {
        Some(item) => item_to_richtext(item),
        None => egui::RichText::new("<No Item>"),
    };
    let mut item_hover = None;
    let response = egui::ComboBox::from_id_salt(format!("item_choice_{:?}", slot))
        .width(COMBO_WIDTH)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            ui.spacing_mut().item_spacing = [ui.spacing().item_spacing.x, ui.spacing().item_spacing.y - 2.0].into();
            if ui.selectable_label(false, "<No Item>").clicked() && idx.is_some() {
                ret = Some(None);
            }
            for (i, item) in state.build.inventory.iter().enumerate().filter(|(_, it)| it.data().item_class.allowed_slots().contains(&slot)) {
                let response = ui.selectable_label(idx.is_some() && *idx.unwrap() == i, item_to_richtext(item));
                if response.clicked() {
                    ret = Some(Some(i));
                } else if response.hovered() {
                    item_hover = Some(item);
                }
            }
        }).response;

    if let Some(item) = item_hover {
        draw_item_window(ui, item, [response.rect.max.x + 10.0, response.rect.min.y], state.config.show_debug);
    } else if response.hovered() && idx.is_some() {
        draw_item_window(ui, state.build.get_equipped(slot).unwrap(), [response.rect.max.x + 10.0, response.rect.min.y], state.config.show_debug);
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

    None
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::CentralPanel::default()
        .show(ctx, |ui| {
           ui.columns(3, |uis| {
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[0] /*ui*/, |ui| {
                    for slot in SLOTS {
                        draw_item_combo(ui, state, slot);
                    }
                    for jewel_node in state.build.tree.jewel_slots() {
                        draw_item_combo(ui, state, Slot::TreeJewel(jewel_node));
                    }
                });
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[1] /*ui*/, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, item) in state.build.inventory.iter().enumerate() {
                            let response = ui.selectable_label(state.panel_items.editing_item_idx == Some(i), item_to_richtext(item));
                            if response.hovered() {
                                draw_item_window(ui, item, [state.mouse_pos.0 + 15.0, state.mouse_pos.1 + 15.0], state.config.show_debug);
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
}
