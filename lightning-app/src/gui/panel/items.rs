use lightning_model::{build::Slot, item::Item};
use crate::gui::{utils::{draw_item_window, rarity_to_color}, State};

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

fn item_to_richtext(item: &Item) -> egui::RichText {
    egui::RichText::new(&item.name).color(rarity_to_color(item.rarity))
}

fn draw_item_combo(ui: &mut egui::Ui, state: &mut State, slot: Slot) -> Option<Option<usize>> {
    let mut ret = None;
    let idx = state.build.equipment.get(&slot);
    let selected_text = match state.build.get_equipped(slot) {
        Some(item) => item_to_richtext(item),
        None => egui::RichText::new("<No Item>"),
    };
    let response = egui::ComboBox::from_id_salt(format!("item_choice_{:?}", slot))
        // TODO: DPI Aware
        .width(200.0)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            if ui.selectable_label(false, "<No Item>").clicked() && idx.is_some() {
                ret = Some(None);
            }
            for (i, item) in state.build.inventory.iter().enumerate().filter(|(_, it)| it.data().item_class.allowed_slots().contains(&slot)) {
                let response = ui.selectable_label(idx.is_some() && *idx.unwrap() == i, item_to_richtext(item));
                if response.clicked() {
                    ret = Some(Some(i));
                } else if response.hovered() {
                    // TODO: gets hidden by combobox popup.
                    draw_item_window(ui, item, [state.mouse_pos.0 + 15.0, state.mouse_pos.1 + 15.0], state.config.show_debug);
                }
            }
        }).response;
    if response.hovered() && idx.is_some() {
        draw_item_window(ui, state.build.get_equipped(slot).unwrap(), [state.mouse_pos.0 + 15.0, state.mouse_pos.1 + 15.0], state.config.show_debug);
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
           ui.columns(2, |uis| {
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
                        for item in &state.build.inventory {
                            if ui.selectable_label(false, item_to_richtext(item)).hovered() {
                                draw_item_window(ui, item, [state.mouse_pos.0 + 15.0, state.mouse_pos.1 + 15.0], state.config.show_debug);
                            }
                        }
                    });
                });
            });
        });
}