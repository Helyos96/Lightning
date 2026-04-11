use crate::gui::State;
use lightning_model::build::stat::StatId;
use lightning_model::modifier::Source;

use egui::Color32;

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(egui::RichText::new("Defence Calculations").size(24.0).color(Color32::WHITE));
            egui_flex::Flex::horizontal()
                .wrap(true)
                .align_items(egui_flex::FlexAlign::Start)
                .show(ui, |flex| {
                flex.add_ui(egui_flex::item(), |ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Maximum Life").size(18.0).color(Color32::LIGHT_RED));
                        draw_stat_breakdown(ui, state, StatId::MaximumLife);
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Maximum Mana").size(18.0).color(Color32::LIGHT_BLUE));
                        draw_stat_breakdown(ui, state, StatId::MaximumMana);
                    });
                });
            });
        });
    });
}

fn draw_stat_breakdown(ui: &mut egui::Ui, state: &State, stat_id: StatId) {
    let stat = state.defence_stats.stat(stat_id);
    
    ui.label(egui::RichText::new(format!("Base: {}, Inc: {}%, More: {}%", stat.base, stat.inc, stat.more - 100)).italics());
    ui.add_space(5.0);

    egui::Grid::new(format!("calc_grid_{:?}", stat_id))
        .striped(true)
        .num_columns(4)
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Value").strong());
            ui.label(egui::RichText::new("Type").strong());
            ui.label(egui::RichText::new("Source").strong());
            ui.label(egui::RichText::new("Mutations").strong());
            ui.end_row();

            for m in &stat.mods {
                ui.label(m.final_amount().to_string());
                ui.label(format!("{:?}", m.typ));
                
                let source_text = match m.source {
                    Source::Innate => egui::RichText::new("Innate"),
                    Source::Node(id) => {
                        let name = state.build.tree.nodes_data.get(&id).map(|n| n.name.clone()).unwrap_or_else(|| format!("Node {:?}", id));
                        egui::RichText::new(name).color(Color32::LIGHT_GREEN)
                    },
                    Source::Mastery(id) => {
                        let name = state.build.tree.nodes_data.get(&id.1).map(|n| n.name.clone()).unwrap_or_else(|| format!("Mastery {:?}", id));
                        egui::RichText::new(name).color(Color32::LIGHT_GREEN)
                    },
                    Source::Item(slot) => {
                        if let Some(item) = state.build.get_equipped(slot) {
                            egui::RichText::new(item.name().to_string()).color(crate::gui::utils::rarity_to_color(item.rarity))
                        } else {
                            egui::RichText::new(format!("{:?}", slot))
                        }
                    },
                    Source::Gem => egui::RichText::new("Gem"),
                };
                ui.label(source_text);
                
                let mut mutations_str = String::new();
                for (i, f) in m.mutations.iter().enumerate() {
                    if i > 0 { mutations_str.push_str(", "); }
                    match f {
                        lightning_model::modifier::Mutation::MultiplierStat((amt, stat)) => {
                            if *amt == 1 {
                                mutations_str.push_str(&format!("{} per {}", m.amount, stat));
                            } else {
                                mutations_str.push_str(&format!("{} per {} {}", m.amount, amt, stat));
                            }
                        },
                        lightning_model::modifier::Mutation::MultiplierStatLowest((amt, stats)) => {
                            let stats_str: Vec<String> = stats.iter().map(|s| s.to_string()).collect();
                            if *amt == 1 {
                                mutations_str.push_str(&format!("{} per lowest of {}", m.amount, stats_str.join(" and ")));
                            } else {
                                mutations_str.push_str(&format!("{} per {} lowest of {}", m.amount, amt, stats_str.join(" and ")));
                            }
                        },
                        lightning_model::modifier::Mutation::MultiplierProperty((amt, prop)) => {
                            if *amt == 1 {
                                mutations_str.push_str(&format!("{} per {}", m.amount, prop));
                            } else {
                                mutations_str.push_str(&format!("{} per {} {}", m.amount, amt, prop));
                            }
                        },
                        lightning_model::modifier::Mutation::UpTo(amt) => {
                            mutations_str.push_str(&format!("up to {}", amt));
                        }
                    }
                }
                ui.label(mutations_str);
                ui.end_row();
            }
        });
}
