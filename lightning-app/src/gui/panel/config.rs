use lazy_static::lazy_static;
use lightning_model::build::property;
use crate::gui::State;

lazy_static! {
    static ref PROPERTIES_INT: Vec<(property::Int, &'static str)> = vec![
        (property::Int::FrenzyCharges, "Frenzy Charges"),
        (property::Int::PowerCharges, "Power Charges"),
        (property::Int::EnduranceCharges, "Endurance Charges"),
        (property::Int::Rage, "Rage"),
    ];
    static ref PROPERTIES_BOOL: Vec<(property::Bool, &'static str)> = vec![
        (property::Bool::Fortified, "Are you Fortified?"),
        (property::Bool::Blinded, "Are you Blind?"),
        (property::Bool::Onslaught, "Do you have Onslaught?"),
        (property::Bool::DealtCritRecently, "Dealt a Crit Recently?"),
        (property::Bool::Leeching, "Are you Leeching?"),
        (property::Bool::OnFullLife, "Are you on Full Life?"),
        (property::Bool::OnLowLife, "Are you on Low Life?"),
    ];
}

pub fn draw(ctx: &egui::Context, state: &mut State) {
    egui::CentralPanel::default()
        .show(ctx, |ui| {
            ui.columns(2, |uis| {
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[0], |ui| {
                    egui::Grid::new("grid_ui_property_int").show(ui, |ui| {
                        for pint in PROPERTIES_INT.iter() {
                            let mut property = state.build.property_int(pint.0);
                            ui.label(pint.1);
                            if ui.add(egui::DragValue::new(&mut property)).changed() {
                                state.build.set_property_int(pint.0, property);
                                state.request_recalc = true;
                            }
                            ui.end_row();
                        }
                    });
                });
                egui::Frame::default().inner_margin(4.0).fill(egui::Color32::BLACK).show(&mut uis[1], |ui| {
                    egui::Grid::new("grid_ui_property_bool").show(ui, |ui| {
                        for pbool in PROPERTIES_BOOL.iter() {
                            let mut property = state.build.property_bool(pbool.0);
                            ui.label(pbool.1);
                            if ui.checkbox(&mut property, "").clicked() {
                                state.build.set_property_bool(pbool.0, property);
                                state.request_recalc = true;
                            }
                            ui.end_row();
                        }
                    });
                });
            });
        });
}
