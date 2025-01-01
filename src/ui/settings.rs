use std::fmt::Debug;

use egui::{DragValue, InnerResponse, Response, Ui};
use egui_code_editor::{ColorTheme, Syntax};
use slicer::{
    fiber::{self, FiberSettings},
    FanSettings, FilamentSettings, MovementParameter, OptionalSetting, RetractionWipeSettings,
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
        show_f32(&mut self.layer_height, "Layer height", Some("mm"), 0.0, ui);

        egui::CollapsingHeader::new("Extrustion Width")
            .default_open(true)
            .show(ui, |ui| {
                ExtrusionMovementParameter(&mut self.extrusion_width).show(ui);
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

        show_optional_setting(
            &mut self.skirt,
            "Skirt Settings",
            |settings, ui| {
                settings.show(ui);
            },
            false,
            ui,
        );

        show_optional_setting(
            &mut self.support,
            "Support Settings",
            |settings, ui| {
                settings.show(ui);
            },
            true,
            ui,
        );

        show_f32(
            &mut self.nozzle_diameter,
            "Nozzle diameter",
            Some("mm"),
            0.0,
            ui,
        );
        show_f32(
            &mut self.retract_length,
            "Retract length",
            Some("mm"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.retract_lift_z,
            "Retract lift Z",
            Some("mm"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.retract_speed,
            "Retract speed",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_optional_setting(
            &mut self.retraction_wipe,
            "Retraction Wipe Settings",
            |settings, ui| {
                settings.show(ui);
            },
            false,
            ui,
        );

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
            0.0,
            ui,
        );

        show_bool(
            &mut self.inner_perimeters_first,
            "Inner perimeters first",
            None,
            true,
            ui,
        );

        show_usize(
            &mut self.number_of_perimeters,
            "Number of perimeters",
            None,
            2,
            ui,
        );

        show_usize(&mut self.top_layers, "Top layers", None, 4, ui);

        show_usize(&mut self.bottom_layers, "Bottom layers", None, 4, ui);

        show_optional_setting(
            &mut self.brim_width,
            "Brim",
            |setting, ui| {
                show_f32(setting, "Brim width", Some("mm"), 5.0, ui);
            },
            false,
            ui,
        );

        show_optional_setting(
            &mut self.layer_shrink_amount,
            "Layer shrink amount",
            |setting, ui| {
                show_f32(setting, "Layer shrink amount", Some("mm"), 0.0, ui);
            },
            false,
            ui,
        );

        show_f32(
            &mut self.minimum_retract_distance,
            "Minimum retract distance",
            Some("mm"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.infill_perimeter_overlap_percentage,
            "Infill perimeter overlap percentage",
            Some("%"),
            0.0,
            ui,
        );

        show_combo(&mut self.solid_infill_type, "Solid infill type", ui);
        show_combo(&mut self.partial_infill_type, "Partial infill type", ui);
    }

    fn show_printer(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Printer Dimension")
            .default_open(true)
            .show(ui, |ui| {
                show_f32(
                    &mut self.print_x,
                    "Printer Dimension X",
                    Some("mm"),
                    0.0,
                    ui,
                );
                show_f32(
                    &mut self.print_y,
                    "Printer Dimension Y",
                    Some("mm"),
                    0.0,
                    ui,
                );
                show_f32(
                    &mut self.print_z,
                    "Printer Dimension Z",
                    Some("mm"),
                    0.0,
                    ui,
                );
            });
    }

    fn show_layer_specific(&mut self, _ui: &mut egui::Ui) {
        todo!()
    }

    fn show_instructions(&mut self, ui: &mut egui::Ui) {
        ui.label("Starting instructions");

        egui::ScrollArea::both()
            .id_salt("start instruction editor scroll area")
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
            .id_salt("end instruction editor scroll area")
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
            .id_salt("before layer change instruction editor scroll area")
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
            .id_salt("after layer change instruction editor scroll area")
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
            .id_salt("object change instruction editor scroll area")
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
            0.0,
            ui,
        );

        show_f32(
            &mut self.max_acceleration_y,
            "Max acceleration Y",
            Some("mm/s²"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.max_acceleration_z,
            "Max acceleration Z",
            Some("mm/s²"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.max_acceleration_e,
            "Max travel acceleration E",
            Some("mm/s²"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.max_acceleration_extruding,
            "Max acceleration extruding",
            Some("mm/s²"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.max_acceleration_travel,
            "Max acceleration travel",
            Some("mm/s²"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.max_acceleration_retracting,
            "Max acceleration retracting",
            Some("mm/s²"),
            0.0,
            ui,
        );

        show_f32(&mut self.max_jerk_x, "Max jerk X", Some("mm/s"), 0.0, ui);

        show_f32(&mut self.max_jerk_y, "Max jerk Y", Some("mm/s"), 0.0, ui);

        show_f32(&mut self.max_jerk_z, "Max jerk Z", Some("mm/s"), 0.0, ui);

        show_f32(&mut self.max_jerk_e, "Max jerk E", Some("mm/s"), 0.0, ui);

        show_f32(
            &mut self.minimum_feedrate_print,
            "Minimum feedrate print",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.minimum_feedrate_travel,
            "Minimum feedrate travel",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.maximum_feedrate_x,
            "Maximum feedrate X",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.maximum_feedrate_y,
            "Maximum feedrate Y",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.maximum_feedrate_z,
            "Maximum feedrate Z",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.maximum_feedrate_e,
            "Maximum feedrate E",
            Some("mm/s"),
            0.0,
            ui,
        );
    }

    fn show_fiber(&mut self, ui: &mut egui::Ui) {
        self.fiber.show(ui);
    }
}

struct ExtrusionMovementParameter<'a>(&'a mut MovementParameter);

impl<'a> UiWidgetComponent for ExtrusionMovementParameter<'a> {
    fn show(&mut self, ui: &mut egui::Ui) {
        show_f32(
            &mut self.0.interior_inner_perimeter,
            "Interior inner perimeter",
            None,
            0.0,
            ui,
        );

        show_f32(
            &mut self.0.interior_surface_perimeter,
            "Interior surface perimeter",
            None,
            0.0,
            ui,
        );

        show_f32(
            &mut self.0.exterior_inner_perimeter,
            "Exterior inner perimeter",
            None,
            0.0,
            ui,
        );

        show_f32(
            &mut self.0.exterior_surface_perimeter,
            "Exterior surface perimeter",
            None,
            0.0,
            ui,
        );

        show_f32(
            &mut self.0.solid_top_infill,
            "Solid top infill",
            None,
            0.0,
            ui,
        );

        show_f32(&mut self.0.solid_infill, "Solid infill", None, 0.0, ui);

        show_f32(&mut self.0.infill, "Infill", None, 0.0, ui);

        show_f32(&mut self.0.travel, "Travel", None, 0.0, ui);

        show_f32(&mut self.0.bridge, "Bridge", None, 0.0, ui);

        show_f32(&mut self.0.support, "Support", None, 0.0, ui);
    }
}

impl UiWidgetComponent for MovementParameter {
    fn show(&mut self, ui: &mut egui::Ui) {
        show_f32(
            &mut self.interior_inner_perimeter,
            "Interior inner perimeter",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.interior_surface_perimeter,
            "Interior surface perimeter",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.exterior_inner_perimeter,
            "Exterior inner perimeter",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.exterior_surface_perimeter,
            "Exterior surface perimeter",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.solid_top_infill,
            "Solid top infill",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(
            &mut self.solid_infill,
            "Solid infill",
            Some("mm/s"),
            0.0,
            ui,
        );

        show_f32(&mut self.infill, "Infill", Some("mm/s"), 0.0, ui);

        show_f32(&mut self.travel, "Travel", Some("mm/s"), 0.0, ui);

        show_f32(&mut self.bridge, "Bridge", Some("mm/s"), 0.0, ui);

        show_f32(&mut self.support, "Support", Some("mm/s"), 0.0, ui);
    }
}

impl UiWidgetComponent for FilamentSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        let settings_default = FilamentSettings::default();

        show_f32(
            &mut self.diameter,
            "Diameter",
            Some("mm"),
            settings_default.diameter,
            ui,
        );
        show_f32(
            &mut self.density,
            "Density",
            Some("g/cm³"),
            settings_default.density,
            ui,
        );
        show_f32(
            &mut self.cost,
            "Cost",
            Some("€/kg"),
            settings_default.cost,
            ui,
        );
        show_f32(
            &mut self.extruder_temp,
            "Extruder temperature",
            Some("°C"),
            settings_default.extruder_temp,
            ui,
        );

        show_f32(
            &mut self.bed_temp,
            "Bed temperature",
            Some("°C"),
            settings_default.bed_temp,
            ui,
        );
    }
}

impl UiWidgetComponent for FanSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        let settings_default = FanSettings::default();

        show_f32(
            &mut self.fan_speed,
            "Fan speed",
            Some("%"),
            settings_default.fan_speed,
            ui,
        );
        show_usize(
            &mut self.disable_fan_for_layers,
            "Disable fan for layers",
            None,
            settings_default.disable_fan_for_layers,
            ui,
        );

        show_f32(
            &mut self.slow_down_threshold,
            "Slow down threshold",
            None,
            settings_default.slow_down_threshold,
            ui,
        );

        show_f32(
            &mut self.min_print_speed,
            "Min print speed",
            Some("mm/s"),
            settings_default.min_print_speed,
            ui,
        );
    }
}

impl UiWidgetComponent for SkirtSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        let settings_default = SkirtSettings::default();

        show_usize(
            &mut self.layers,
            "Layers",
            None,
            settings_default.layers,
            ui,
        );
        show_f32(
            &mut self.distance,
            "Distance",
            Some("mm"),
            settings_default.distance,
            ui,
        );
    }
}

impl UiWidgetComponent for SupportSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        let settings_default = SupportSettings::default();

        show_f32(
            &mut self.max_overhang_angle,
            "Max overhang angle",
            Some("°"),
            settings_default.max_overhang_angle,
            ui,
        );
        show_f32(
            &mut self.support_spacing,
            "Support spacing",
            Some("mm"),
            settings_default.support_spacing,
            ui,
        );
    }
}

impl UiWidgetComponent for RetractionWipeSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        let settings_default = RetractionWipeSettings::default();

        show_f32(
            &mut self.speed,
            "Speed",
            Some("mm/s"),
            settings_default.speed,
            ui,
        );
        show_f32(
            &mut self.acceleration,
            "Acceleration",
            Some("mm/s²"),
            settings_default.acceleration,
            ui,
        );
        show_f32(
            &mut self.distance,
            "Distance",
            Some("mm"),
            settings_default.distance,
            ui,
        );
    }
}

impl UiWidgetComponent for FiberSettings {
    fn show(&mut self, ui: &mut egui::Ui) {
        let settings_default = FiberSettings::default();

        show_f32(
            &mut self.diameter,
            "Diameter",
            Some("mm"),
            settings_default.diameter,
            ui,
        );
        show_f32(
            &mut self.cut_before,
            "Cut Before",
            Some("mm"),
            settings_default.cut_before,
            ui,
        );
        show_f32(
            &mut self.min_length,
            "Min Length",
            Some("mm"),
            settings_default.min_length,
            ui,
        );

        show_optional_setting(
            &mut self.wall_pattern,
            "Wall Fibers",
            |setting, ui| {
                show_combo(&mut setting.pattern, "Pattern", ui);

                match setting.pattern {
                    fiber::WallPatternType::Alternating => {
                        show_usize(
                            &mut setting.alternating_layer_spacing,
                            "Layer Pattern
                             Spacing",
                            None,
                            1,
                            ui,
                        );

                        show_usize(
                            &mut setting.alternating_layer_width,
                            "Layer Pattern Width",
                            None,
                            1,
                            ui,
                        );

                        show_usize(
                            &mut setting.alternating_wall_spacing,
                            "Wall Pattern Spacing",
                            None,
                            1,
                            ui,
                        );

                        show_usize(
                            &mut setting.alternating_wall_width,
                            "Wall Pattern Width",
                            None,
                            1,
                            ui,
                        );

                        show_usize(&mut setting.alternating_step, "Step", None, 1, ui);
                    }
                    fiber::WallPatternType::Random => {}
                    fiber::WallPatternType::Full => {}
                }
            },
            true,
            ui,
        );

        show_optional_setting(
            &mut self.infill,
            "Infill Fibers",
            |setting, ui| {
                show_combo(&mut setting.infill, "Infill Type", ui);
                show_f32(
                    &mut setting.infill_percentage,
                    "Infill Percentage",
                    None,
                    0.2,
                    ui,
                );

                show_usize(&mut setting.width, "Pattern Width", Some("Layers"), 1, ui);
                show_usize(
                    &mut setting.spacing,
                    "Pattern Spacing",
                    Some("Layers"),
                    1,
                    ui,
                );

                show_bool(
                    &mut setting.solid_infill,
                    "Fiber Solid Infill",
                    None,
                    false,
                    ui,
                );
                show_bool(&mut setting.air_spacing, "Air Spacing", None, false, ui);
            },
            true,
            ui,
        );

        show_f32(
            &mut self.speed_factor,
            "Speed Factor",
            None,
            settings_default.speed_factor,
            ui,
        );
        show_f32(
            &mut self.acceleration_factor,
            "Acceleration Factor",
            None,
            settings_default.acceleration_factor,
            ui,
        );
        show_f32(
            &mut self.jerk_factor,
            "Jerk Factor",
            None,
            settings_default.jerk_factor,
            ui,
        );
    }
}

fn show_f32(
    value: &mut f32,
    description: &str,
    unit: Option<&str>,
    default: f32,
    ui: &mut Ui,
) -> Response {
    ui.horizontal(|ui| {
        crate::config::gui::settings::SETTINGS_LABEL.label(ui, description);
        let response = ui.add(DragValue::new(value).max_decimals(3));
        if let Some(unit) = unit {
            ui.label(unit);
        }
        show_reset_button(value, default, ui);
        response
    })
    .inner
}

fn show_optional_setting<T>(
    setting: &mut OptionalSetting<T>,
    description: &str,
    r#fn: impl FnOnce(&mut T, &mut Ui),
    default: bool,
    ui: &mut Ui,
) {
    egui::CollapsingHeader::new(description)
        .default_open(default)
        .show(ui, |ui| {
            show_bool(setting.enabled_mut(), "Enabled", None, default, ui);

            if setting.is_enabled() {
                r#fn(setting, ui);
            }
        });
}

fn show_usize(
    value: &mut usize,
    description: &str,
    unit: Option<&str>,
    default: usize,
    ui: &mut Ui,
) -> Response {
    ui.horizontal(|ui| {
        crate::config::gui::settings::SETTINGS_LABEL.label(ui, description);
        let response = ui.add(DragValue::new(value).max_decimals(0));
        if let Some(unit) = unit {
            ui.label(unit);
        }
        show_reset_button(value, default, ui);
        response
    })
    .inner
}

fn show_bool(
    value: &mut bool,
    description: &str,
    unit: Option<&str>,
    default: bool,
    ui: &mut Ui,
) -> Response {
    ui.horizontal(|ui| {
        crate::config::gui::settings::SETTINGS_LABEL.label(ui, description);

        let response = ui.checkbox(value, "");
        if let Some(unit) = unit {
            ui.label(unit);
        }

        show_reset_button(value, default, ui);
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

fn show_reset_button<T: PartialEq>(value: &mut T, default: T, ui: &mut Ui) {
    let button = egui::Button::new("↺");

    if value != &default && ui.add(button).on_hover_text("Reset to default").clicked() {
        *value = default;
    }
}
