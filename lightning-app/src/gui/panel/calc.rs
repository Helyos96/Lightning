use crate::gui::State;
use lightning_model::data::base_item::Rarity;
use lightning_model::{build::stat::StatId, modifier::Mutation};
use lightning_model::modifier::{Source, Type};

use egui::Color32;
use egui_extras::{Column, TableBuilder};

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.heading(egui::RichText::new("Defence Calculations").size(24.0).color(Color32::WHITE));
            egui_flex::Flex::horizontal()
                .wrap(true)
                .align_items(egui_flex::FlexAlign::Start)
                .show(ui, |flex| {
                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_stat_breakdown(ui, state, StatId::MaximumLife, "Maximum Life", egui::Color32::LIGHT_RED);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_stat_breakdown(ui, state, StatId::MaximumMana, "Maximum Mana", Color32::LIGHT_BLUE);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_stat_breakdown(ui, state, StatId::Armour, "Armour", Color32::WHITE);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_stat_breakdown(ui, state, StatId::EvasionRating, "Evasion", Color32::GREEN);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_stat_breakdown(ui, state, StatId::MaximumEnergyShield, "Energy Shield", Color32::LIGHT_BLUE);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_stat_breakdown(ui, state, StatId::Strength, "Strength", Color32::LIGHT_RED);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_stat_breakdown(ui, state, StatId::Dexterity, "Dexterity", Color32::GREEN);
                        });
                    });
                });

                flex.add_ui(egui_flex::item(), |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_stat_breakdown(ui, state, StatId::Intelligence, "Intelligence", Color32::LIGHT_BLUE);
                        });
                    });
                });
            });
        });
    });
}

fn draw_stat_breakdown(ui: &mut egui::Ui, state: &State, stat_id: StatId, title: &str, color: egui::Color32) {
    let stat = state.defence_stats.stat(stat_id);

    ui.label(egui::RichText::new(format!("{}: {}", title, stat.val())).size(18.0).color(color));
    if stat.flat != 0 {
        ui.label(egui::RichText::new(format!("Flat: {}, Base: {}, Inc: {}%, More: {}%", stat.flat, stat.base, stat.inc, stat.more - 100)).italics());
    } else {
        ui.label(egui::RichText::new(format!("Base: {}, Inc: {}%, More: {}%", stat.base, stat.inc, stat.more - 100)).italics());
    }
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
                for (mstat, source) in stat.mods.iter().filter_map(|m| m.as_stat().map(|s| (s, m.source))) {
                    body.row(20.0, |mut row| {
                        row.col(|ui| {
                            match mstat.typ {
                                Type::Inc|Type::More => { ui.add(egui::Label::new(format!("{}%", mstat.final_amount())).wrap_mode(egui::TextWrapMode::Extend)); },
                                _ => { ui.add(egui::Label::new(mstat.final_amount().to_string()).wrap_mode(egui::TextWrapMode::Extend)); },
                            }
                        });
                        row.col(|ui| {
                            ui.add(egui::Label::new(format!("{:?}", mstat.typ)).wrap_mode(egui::TextWrapMode::Extend));
                        });
                        row.col(|ui| {
                            let source_text = match source {
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
                            for (i, f) in mstat.mutations.iter().enumerate() {
                                if i > 0 { mutations_str.push_str(", "); }
                                match f {
                                    Mutation::MultiplierStat((amt, stat)) => {
                                        if *amt == 1 {
                                            mutations_str.push_str(&format!("{} per {}", mstat.amount, stat));
                                        } else {
                                            mutations_str.push_str(&format!("{} per {} {}", mstat.amount, amt, stat));
                                        }
                                    },
                                    Mutation::MultiplierStatLowest((amt, stats)) => {
                                        let stats_str: Vec<String> = stats.iter().map(|s| s.to_string()).collect();
                                        if *amt == 1 {
                                            mutations_str.push_str(&format!("{} per lowest of {}", mstat.amount, stats_str.join(" and ")));
                                        } else {
                                            mutations_str.push_str(&format!("{} per {} lowest of {}", mstat.amount, amt, stats_str.join(" and ")));
                                        }
                                    },
                                    Mutation::MultiplierProperty((amt, prop)) => {
                                        if *amt == 1 {
                                            mutations_str.push_str(&format!("{} per {}", mstat.amount, prop));
                                        } else {
                                            mutations_str.push_str(&format!("{} per {} {}", mstat.amount, amt, prop));
                                        }
                                    },
                                    Mutation::StatPct((pct, stat_id)) => {
                                        mutations_str.push_str(&format!("{}% of {}", pct, stat_id.to_string()));
                                    },
                                    Mutation::UpTo(amt) => {
                                        mutations_str.push_str(&format!("up to {}", amt));
                                    },
                                    Mutation::IncreasedEffect(amt) => {
                                        mutations_str.push_str(&format!("{}% inc effect", amt));
                                    },
                                    Mutation::MultiplierQuality(per) => {
                                        mutations_str.push_str(&format!("{}% per {}% qual", mstat.amount, per));
                                    },
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
