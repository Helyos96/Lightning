use crate::gui::State;
use lightning_model::data::base_item::Rarity;
use lightning_model::{build::stat::StatId, modifier::Mutation};
use lightning_model::modifier::Source;

use egui::Color32;
use egui_extras::{Column, TableBuilder};

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(egui::RichText::new("Defence Calculations").size(24.0).color(Color32::WHITE));
            egui_flex::Flex::horizontal()
                .wrap(true)
                .align_items(egui_flex::FlexAlign::Start)
                .show(ui, |flex| {
                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Maximum Life").size(18.0).color(Color32::LIGHT_RED));
                            draw_stat_breakdown(ui, state, StatId::MaximumLife);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Maximum Mana").size(18.0).color(Color32::LIGHT_BLUE));
                            draw_stat_breakdown(ui, state, StatId::MaximumMana);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Armour").size(18.0).color(Color32::WHITE));
                            draw_stat_breakdown(ui, state, StatId::Armour);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Evasion").size(18.0).color(Color32::GREEN));
                            draw_stat_breakdown(ui, state, StatId::EvasionRating);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Energy Shield").size(18.0).color(Color32::LIGHT_BLUE));
                            draw_stat_breakdown(ui, state, StatId::MaximumEnergyShield);
                        });
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

    ui.push_id(format!("calc_grid_{:?}", stat_id), |ui| {
        ui.spacing_mut().scroll.floating = false;
        ui.spacing_mut().scroll.bar_width = 4.0;
        TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::auto())
            .max_scroll_height(500.0)
            .header(20.0, |mut header| {
                header.col(|ui| { ui.label(egui::RichText::new("Value").strong()); });
                header.col(|ui| { ui.label(egui::RichText::new("Type").strong()); });
                header.col(|ui| { ui.label(egui::RichText::new("Source").strong()); });
                header.col(|ui| { ui.label(egui::RichText::new("Mutations").strong()); });
            })
            .body(|mut body| {
                for m in &stat.mods {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            ui.add(egui::Label::new(m.final_amount().to_string()).wrap_mode(egui::TextWrapMode::Extend));
                        });
                        row.col(|ui| {
                            ui.add(egui::Label::new(format!("{:?}", m.typ)).wrap_mode(egui::TextWrapMode::Extend));
                        });
                        row.col(|ui| {
                            let source_text = match m.source {
                                Source::Innate => egui::RichText::new("Innate"),
                                Source::Node(id) => {
                                    let name = state.build.tree.nodes_data.get(&id).map(|n| n.name.clone()).unwrap_or_else(|| format!("Node {:?}", id));
                                    egui::RichText::new(name).color(Color32::LIGHT_GREEN)
                                },
                                Source::Mastery(id) => {
                                    let name = state.build.tree.nodes_data.get(&id.0).map(|n| n.name.clone()).unwrap_or_else(|| format!("Mastery {:?}", id));
                                    egui::RichText::new(name).color(Color32::LIGHT_GREEN)
                                },
                                Source::Item(slot) => {
                                    if let Some(item) = state.build.get_equipped(slot) {
                                        if item.rarity == Rarity::Unique {
                                            egui::RichText::new(format!("{}", item.name)).color(crate::gui::utils::rarity_to_color(item.rarity))
                                        } else {
                                            egui::RichText::new(format!("{slot}")).color(crate::gui::utils::rarity_to_color(item.rarity))
                                        }
                                    } else {
                                        egui::RichText::new(format!("{:?}", slot))
                                    }
                                },
                                Source::Gem(gem_name) => egui::RichText::new(gem_name),
                                Source::Custom(custom) => egui::RichText::new(custom),
                            };
                            ui.add(egui::Label::new(source_text).wrap_mode(egui::TextWrapMode::Extend));
                        });
                        row.col(|ui| {
                            let mut mutations_str = String::new();
                            for (i, f) in m.mutations.iter().enumerate() {
                                if i > 0 { mutations_str.push_str(", "); }
                                match f {
                                    Mutation::MultiplierStat((amt, stat)) => {
                                        if *amt == 1 {
                                            mutations_str.push_str(&format!("{} per {}", m.amount, stat));
                                        } else {
                                            mutations_str.push_str(&format!("{} per {} {}", m.amount, amt, stat));
                                        }
                                    },
                                    Mutation::MultiplierStatLowest((amt, stats)) => {
                                        let stats_str: Vec<String> = stats.iter().map(|s| s.to_string()).collect();
                                        if *amt == 1 {
                                            mutations_str.push_str(&format!("{} per lowest of {}", m.amount, stats_str.join(" and ")));
                                        } else {
                                            mutations_str.push_str(&format!("{} per {} lowest of {}", m.amount, amt, stats_str.join(" and ")));
                                        }
                                    },
                                    Mutation::MultiplierProperty((amt, prop)) => {
                                        if *amt == 1 {
                                            mutations_str.push_str(&format!("{} per {}", m.amount, prop));
                                        } else {
                                            mutations_str.push_str(&format!("{} per {} {}", m.amount, amt, prop));
                                        }
                                    },
                                    Mutation::StatPct((pct, stat_id)) => {
                                        mutations_str.push_str(&format!("{}% of {}", pct, stat_id.to_string()));
                                    }
                                    Mutation::UpTo(amt) => {
                                        mutations_str.push_str(&format!("up to {}", amt));
                                    }
                                    _ => {}
                                }
                            }
                            ui.add(egui::Label::new(mutations_str).wrap_mode(egui::TextWrapMode::Extend));
                        });
                    });
                }
            });
    });
}
