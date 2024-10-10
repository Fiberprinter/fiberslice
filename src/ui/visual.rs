use egui::Rounding;

pub fn customize_look_and_feel(visuals: &mut egui::Visuals) {
    visuals.selection.bg_fill = egui::Color32::from_rgb(76, 255, 0);
    visuals.selection.stroke.color = egui::Color32::from_rgb(0, 0, 0);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(76, 255, 0);

    disable_rounding(&mut visuals.window_rounding);
    disable_rounding(&mut visuals.menu_rounding);

    disable_rounding(&mut visuals.widgets.noninteractive.rounding);
    disable_rounding(&mut visuals.widgets.active.rounding);
    disable_rounding(&mut visuals.widgets.hovered.rounding);
    disable_rounding(&mut visuals.widgets.inactive.rounding);
    disable_rounding(&mut visuals.widgets.open.rounding);
}

pub fn disable_rounding(rounding: &mut Rounding) {
    rounding.ne = 1.0;
    rounding.nw = 1.0;
    rounding.se = 1.0;
    rounding.sw = 1.0;
}
