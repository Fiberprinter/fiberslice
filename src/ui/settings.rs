use std::fmt::Debug;

use egui::{DragValue, InnerResponse, Response, TextEdit, Ui};
use egui_code_editor::{ColorTheme, Syntax};
use slicer::{
    FanSettings, FiberSettings, FilamentSettings, MovementParameter, RetractionWipeSettings,
    SkirtSettings, SupportSettings,
};
use strum::IntoEnumIterator;

use crate::{ui::UiWidgetComponent, viewer::GCodeSyntax};

pub trait UiSetting {
    fn show_general(&mut self, ui: &mut egui::Ui);

    fn show_printer(&mut self, ui: &mut egui::Ui);

    fn show_layer_specific(&mut self, ui: &mut egui::Ui);

    fn show_instructions(&mut self, ui: &mut egui::Ui);

    fn show_limits(&mut self, ui: &mut egui::Ui);

    fn show_fiber(&mut self, ui: &mut egui::Ui);
}

impl UiSetting for slicer::Settings {
    fn show_general(&mut self, ui: &mut egui::Ui) {
        show_f32(&mut self.layer_height, "Layer height", Some("mm"), ui);

        egui::CollapsingHeader::new("Extrustion width")
            .default_open(true)
            .show(ui, |ui| {
                self.extrusion_width.show(ui);
            });

        egui::CollapsingHeader::new("Filament")
            .default_open(true)
            .show(ui, |ui| {
                self.filament.show(ui);
            });

        egui::CollapsingHeader::new("Fan Settings")
            .default_open(true)
            .show(ui, |ui| {
                self.fan.show(ui);
            });

        let mut skirt_enabled = self.skirt.is_some();

        show_bool(
            &mut skirt_enabled,
            "Skirt",
            Some("Enable/Disable the skirt"),
            ui,
        );

        if skirt_enabled {
            if self.skirt.is_none() {
                self.skirt = Default::default();
            }

            if let Some(skirt) = &mut self.skirt {
                egui::CollapsingHeader::new("Skirt Settings")
                    .default_open(true)
                    .show(ui, |ui| {
                        skirt.show(ui);
                    });
            }
        } else {
            self.skirt = None;
        }

        let mut support_enabled = self.support.is_some();

        show_bool(
            &mut support_enabled,
            "Support",
            Some("Enable/Disable the support"),
            ui,
        );

        if support_enabled {
            if self.support.is_none() {
                self.support = Default::default();
            }

            if let Some(support) = &mut self.support {
                egui::CollapsingHeader::new("Support Settings")
                    .default_open(true)
                    .show(ui, |ui| {
                        support.show(ui);
                    });
            }
        } else {
            self.support = None;
        }

        show_f32(&mut self.nozzle_diameter, "Nozzle diameter", Some("mm"), ui);
        show_f32(&mut self.retract_length, "Retract length", Some("mm"), ui);

        show_f32(&mut self.retract_lift_z, "Retract lift Z", Some("mm"), ui);

        show_f32(&mut self.retract_speed, "Retract speed", Some("mm/s"), ui);

        let mut retraction_wipe_enabled = self.retraction_wipe.is_some();

        show_bool(
            &mut retraction_wipe_enabled,
            "Retraction wipe",
            Some("Enable/Disable the retraction wipe"),
            ui,
        );

        if retraction_wipe_enabled {
            if self.retraction_wipe.is_none() {
                self.retraction_wipe = Default::default();
            }

            if let Some(retraction_wipe) = &mut self.retraction_wipe {
                egui::CollapsingHeader::new("Retraction Wipe Settings")
                    .default_open(true)
                    .show(ui, |ui| {
                        retraction_wipe.show(ui);
                    });
            }
        } else {
            self.retraction_wipe = None;
        }

        egui::CollapsingHeader::new("Movement Speed")
            .default_open(true)
            .show(ui, |ui| {
                self.speed.show(ui);
            });

        egui::CollapsingHeader::new("Acceleration")
            .default_open(true)
            .show(ui, |ui| {
                self.acceleration.show(ui);
            });

        show_f32(
            &mut self.infill_percentage,
            "Infill percentage",
            Some("%"),
            ui,
        );

        show_bool(
            &mut self.inner_perimeters_first,
            "Inner perimeters first",
            None,
            ui,
        );

        show_usize(
            &mut self.number_of_perimeters,
            "Number of perimeters",
            None,
            ui,
        );

        show_usize(&mut self.top_layers, "Top layers", None, ui);

        show_usize(&mut self.bottom_layers, "Bottom layers", None, ui);

        let mut brim_width = self.brim_width.is_some();

        show_bool(&mut brim_width, "Brim width", None, ui);

        if brim_width {
            if self.brim_width.is_none() {
                self.brim_width = Default::default();
            }

            if let Some(brim_width) = &mut self.brim_width {
                show_f32(brim_width, "Brim width", Some("mm"), ui);
            }
        } else {
            self.brim_width = None;
        }

        let mut layer_shrink = self.layer_shrink_amount.is_some();

        show_bool(&mut layer_shrink, "Layer shrink", None, ui);

        if layer_shrink {
            if self.layer_shrink_amount.is_none() {
                self.layer_shrink_amount = Default::default();
            }

            if let Some(layer_shrink) = &mut self.layer_shrink_amount {
                show_f32(layer_shrink, "Layer shrink amount", Some("mm"), ui);
            }
        } else {
            self.layer_shrink_amount = None;
        }

        show_f32(
            &mut self.minimum_retract_distance,
            "Minimum retract distance",
            Some("mm"),
            ui,
        );

        show_f32(
            &mut self.infill_perimeter_overlap_percentage,
            "Infill perimeter overlap percentage",
            Some("%"),
            ui,
        );

        show_combo(&mut self.solid_infill_type, "Solid infill type", ui);
        show_combo(&mut self.partial_infill_type, "Partial infill type", ui);
    }

    fn show_printer(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Printer Dimension")
            .default_open(true)
            .show(ui, |ui| {
                show_f32(&mut self.print_x, "Printer Dimension X", Some("mm"), ui);
                show_f32(&mut self.print_y, "Printer Dimension Y", Some("mm"), ui);
                show_f32(&mut self.print_z, "Printer Dimension Z", Some("mm"), ui);
            });
    }

    fn show_layer_specific(&mut self, _ui: &mut egui::Ui) {
        todo!()
    }

    fn show_instructions(&mut self, ui: &mut egui::Ui) {
        ui.label("Starting instructions");

        egui::ScrollArea::both()
            .id_source("start instruction editor scroll area")
            .max_height(150.0)
            .show(ui, |ui| {
                egui_code_editor::CodeEditor::default()
                    .id_source("start instruction editor")
                    .with_fontsize(14.0)
                    .with_rows(10)
                    .with_theme(ColorTheme::GRUVBOX)
                    .with_numlines(false)
                    .with_syntax(Syntax::gcode())
                    .show(ui, &mut self.starting_instructions);
            });

        ui.separator();

        ui.add_space(10.0);

        ui.label("Ending instructions");

        egui::ScrollArea::both()
            .id_source("end instruction editor scroll area")
            .max_height(150.0)
            .show(ui, |ui| {
                egui_code_editor::CodeEditor::default()
                    .id_source("end instruction editor")
                    .with_fontsize(14.0)
                    .with_rows(10)
                    .with_theme(ColorTheme::GRUVBOX)
                    .with_numlines(false)
                    .with_syntax(Syntax::gcode())
                    .show(ui, &mut self.ending_instructions);
            });

        ui.separator();

        ui.add_space(10.0);

        ui.label("Before layer change instructions");

        egui::ScrollArea::both()
            .id_source("before layer change instruction editor scroll area")
            .max_height(75.0)
            .show(ui, |ui| {
                egui_code_editor::CodeEditor::default()
                    .id_source("Before layer change instruction editor")
                    .with_fontsize(14.0)
                    .with_rows(5)
                    .with_theme(ColorTheme::GRUVBOX)
                    .with_numlines(false)
                    .with_syntax(Syntax::gcode())
                    .show(ui, &mut self.before_layer_change_instructions);
            });

        ui.label("After layer change instructions");

        egui::ScrollArea::both()
            .id_source("after layer change instruction editor scroll area")
            .max_height(75.0)
            .show(ui, |ui| {
                egui_code_editor::CodeEditor::default()
                    .id_source("After layer change instruction editor")
                    .with_fontsize(14.0)
                    .with_rows(5)
                    .with_theme(ColorTheme::GRUVBOX)
                    .with_numlines(false)
                    .with_syntax(Syntax::gcode())
                    .show(ui, &mut self.after_layer_change_instructions);
            });

        ui.separator();

        ui.add_space(10.0);

        ui.label("Object change instructions");

        egui::ScrollArea::both()
            .id_source("object change instruction editor scroll area")
            .max_height(75.0)
            .show(ui, |ui| {
                egui_code_editor::CodeEditor::default()
                    .id_source("object change instruction editor")
                    .with_fontsize(14.0)
                    .with_rows(5)
                    .with_theme(ColorTheme::GRUVBOX)
                    .with_numlines(false)
                    .with_syntax(Syntax::gcode())
                    .show(ui, &mut self.object_change_instructions);
            });
    }

    fn show_limits(&mut self, ui: &mut egui::Ui) {
        show_f32(
            &mut self.max_acceleration_x,
            "Max acceleration X",
            Some("mm/s²"),
            ui,
        );

        show_f32(
            &mut self.max_acceleration_y,
            "Max acceleration Y",
            Some("mm/s²"),
            ui,
        );

        show_f32(
            &mut self.max_acceleration_z,
            "Max acceleration Z",
            Some("mm/s²"),
            ui,
        );

        show_f32(
            &mut self.max_acceleration_e,
            "Max travel acceleration E",
            Some("mm/s²"),
            ui,
        );

        show_f32(
            &mut self.max_acceleration_extruding,
            "Max acceleration extruding",
            Some("mm/s²"),
            ui,
        );

        show_f32(
            &mut self.max_acceleration_travel,
            "Max acceleration travel",
            Some("mm/s²"),
            ui,
        );

        show_f32(
            &mut self.max_acceleration_retracting,
            "Max acceleration retracting",
            Some("mm/s²"),
            ui,
        );

        show_f32(&mut self.max_jerk_x, "Max jerk X", Some("mm/s"), ui);

        show_f32(&mut self.max_jerk_y, "Max jerk Y", Some("mm/s"), ui);

        show_f32(&mut self.max_jerk_z, "Max jerk Z", Some("mm/s"), ui);

        show_f32(&mut self.max_jerk_e, "Max jerk E", Some("mm/s"), ui);

        show_f32(
            &mut self.minimum_feedrate_print,
            "Minimum feedrate print",
            Some("mm/s"),
            ui,
        );

        show_f32(
            &mut self.minimum_feedrate_travel,
            "Minimum feedrate travel",
            Some("mm/s"),
            ui,
        );

        show_f32(
            &mut self.maximum_feedrate_x,
            "Maximum feedrate X",
            Some("mm/s"),
            ui,
        );

        show_f32(
            &mut self.maximum_feedrate_y,
            "Maximum feedrate Y",
            Some("mm/s"),
            ui,
        );

        show_f32(
            &mut self.maximum_feedrate_z,
            "Maximum feedrate Z",
            Some("mm/s"),
            ui,
        );

        show_f32(
            &mut self.maximum_feedrate_e,
            "Maximum feedrate E",
            Some("mm/s"),
            ui,
        );
    }

    fn show_fiber(&mut self, ui: &mut egui::Ui) {
        if let Some(fiber) = &mut self.fiber {
            egui::CollapsingHeader::new("Fiber Settings")
                .default_open(true)
                .show(ui, |ui| {
                    fiber.show(ui);
                });
        }
    }
}

impl UiWidgetComponent for MovementParameter {
    fn show(&mut self, ui: &mut egui::Ui) {
        show_f32(
            &mut self.interior_inner_perimeter,
            "Interior inner perimeter",
            Some("mm/s"),
            ui,
        );

        show_f32(
            &mut self.interior_surface_perimeter,
            "Interior surface perimeter",
            Some("mm/s"),
            ui,
        );

        show_f32(
            &mut self.exterior_inner_perimeter,
            "Exterior inner perimeter",
            Some("mm/s"),
            ui,
        );

        show_f32(
            &mut self.exterior_surface_perimeter,
            "Exterior surface perimeter",
            Some("mm/s"),
            ui,
        );

        show_f32(
            &mut self.solid_top_infill,
            "Solid top infill",
            Some("mm/s"),
            ui,
        );

        show_f32(&mut self.solid_infill, "Solid infill", Some("mm/s"), ui);

        show_f32(&mut self.infill, "Infill", Some("mm/s"), ui);

        show_f32(&mut self.travel, "Travel", Some("mm/s"), ui);

        show_f32(&mut self.bridge, "Bridge", Some("mm/s"), ui);

        show_f32(&mut self.support, "Support", Some("mm/s"), ui);
    }
}

impl UiWidgetComponent for FilamentSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        show_f32(&mut self.diameter, "Diameter", Some("mm"), ui);
        show_f32(&mut self.density, "Density", Some("g/cm³"), ui);
        show_f32(&mut self.cost, "Cost", Some("€/kg"), ui);
        show_f32(
            &mut self.extruder_temp,
            "Extruder temperature",
            Some("°C"),
            ui,
        );

        show_f32(&mut self.bed_temp, "Bed temperature", Some("°C"), ui);
    }
}

impl UiWidgetComponent for FanSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        show_f32(&mut self.fan_speed, "Fan speed", Some("%"), ui);
        show_usize(
            &mut self.disable_fan_for_layers,
            "Disable fan for layers",
            None,
            ui,
        );

        show_f32(
            &mut self.slow_down_threshold,
            "Slow down threshold",
            None,
            ui,
        );

        show_f32(
            &mut self.min_print_speed,
            "Min print speed",
            Some("mm/s"),
            ui,
        );
    }
}

impl UiWidgetComponent for SkirtSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        show_usize(&mut self.layers, "Layers", None, ui);
        show_f32(&mut self.distance, "Distance", Some("mm"), ui);
    }
}

impl UiWidgetComponent for SupportSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        show_f32(
            &mut self.max_overhang_angle,
            "Max overhang angle",
            Some("°"),
            ui,
        );
        show_f32(&mut self.support_spacing, "Support spacing", Some("mm"), ui);
    }
}

impl UiWidgetComponent for RetractionWipeSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        show_f32(&mut self.speed, "Speed", Some("mm/s"), ui);
        show_f32(&mut self.acceleration, "Acceleration", Some("mm/s²"), ui);
        show_f32(&mut self.distance, "Distance", Some("mm"), ui);
    }
}

impl UiWidgetComponent for FiberSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        show_f32(&mut self.diameter, "Diameter", Some("mm"), ui);
        show_f32(&mut self.cut_before, "Cut Before", Some("mm"), ui);
        show_f32(&mut self.min_length, "Min Length", Some("mm"), ui);
        show_f32(&mut self.speed_factor, "Speed Factor", None, ui);
        show_f32(
            &mut self.acceleration_factor,
            "Acceleration Factor",
            None,
            ui,
        );
        show_f32(&mut self.jerk_factor, "Jerk Factor", None, ui);
    }
}

fn show_str(text: &mut String, description: &str, unit: Option<&str>, ui: &mut Ui) -> Response {
    ui.horizontal(|ui| {
        crate::config::gui::settings::SETTINGS_LABEL.label(ui, description);
        let response = ui.add(TextEdit::singleline(text));
        if let Some(unit) = unit {
            ui.label(unit);
        }
        response
    })
    .inner
}

fn show_f32(value: &mut f32, description: &str, unit: Option<&str>, ui: &mut Ui) -> Response {
    ui.horizontal(|ui| {
        crate::config::gui::settings::SETTINGS_LABEL.label(ui, description);
        let response = ui.add(DragValue::new(value).max_decimals(3));
        if let Some(unit) = unit {
            ui.label(unit);
        }
        response
    })
    .inner
}

fn show_optional_f32(
    value: &mut Option<f32>,
    description: &str,
    unit: Option<&str>,
    ui: &mut Ui,
) -> Option<Response> {
    let mut enabled = value.is_some();

    show_bool(&mut enabled, description, None, ui);

    if enabled {
        if value.is_none() {
            *value = Default::default();
        }

        Some(show_f32(value.as_mut().unwrap(), description, unit, ui))
    } else {
        *value = None;

        None
    }
}

fn show_usize(value: &mut usize, description: &str, unit: Option<&str>, ui: &mut Ui) -> Response {
    ui.horizontal(|ui| {
        crate::config::gui::settings::SETTINGS_LABEL.label(ui, description);
        let response = ui.add(DragValue::new(value).max_decimals(0));
        if let Some(unit) = unit {
            ui.label(unit);
        }
        response
    })
    .inner
}

fn show_bool(value: &mut bool, description: &str, unit: Option<&str>, ui: &mut Ui) -> Response {
    ui.horizontal(|ui| {
        crate::config::gui::settings::SETTINGS_LABEL.label(ui, description);
        let response = ui.checkbox(value, "");
        if let Some(unit) = unit {
            ui.label(unit);
        }
        response
    })
    .inner
}

fn show_combo<T: Debug + PartialEq + IntoEnumIterator>(
    value: &mut T,
    description: &str,
    ui: &mut Ui,
) -> InnerResponse<Option<()>> {
    ui.horizontal(|ui| {
        crate::config::gui::settings::SETTINGS_LABEL.label(ui, description);
        egui::ComboBox::from_label(description)
            .selected_text(format!("{:?}", value))
            .show_ui(ui, |ui| {
                for variant in T::iter() {
                    let label = format!("{:?}", variant);
                    ui.selectable_value(value, variant, label);
                }
            })
    })
    .inner
}
