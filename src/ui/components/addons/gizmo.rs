use egui::{Color32, DragValue, ImageButton, Visuals};
use egui_extras::Size;
use egui_grid::GridBuilder;
use glam::{Mat4, Quat};
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount, EnumIter};

use crate::{
    config,
    ui::{icon::get_gizmo_tool_icon, visual::customize_look_and_feel},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumCount)]
pub enum GizmoTool {
    Translate,
    Rotate,
    Scale,
    Flatten,
}

const GIZMO_TOOL_LABELS: [(&str, GizmoTool); GizmoTool::COUNT] = [
    ("Translate", GizmoTool::Translate),
    ("Rotate", GizmoTool::Rotate),
    ("Scale", GizmoTool::Scale),
    ("Flatten", GizmoTool::Flatten),
];

#[derive(Debug, Default)]
pub struct GizmoTools {
    selected: Option<GizmoTool>,
}

impl GizmoTools {
    pub fn show_icons(
        &mut self,
        ui: &mut egui::Ui,
        _shared_state: &(crate::ui::UiState, crate::GlobalState<crate::RootEvent>),
    ) {
        ui.vertical(|ui| {
            let mut builder = GridBuilder::new();

            let button = config::gui::GIZMO_TOGGLE_BUTTON;

            for _ in 0..GizmoTool::COUNT {
                builder = builder.new_row(Size::remainder());
                builder = builder.new_row_align(Size::exact(button.size.0), egui::Align::Center);
                builder = builder.cell(Size::exact(button.size.0));
            }

            builder = builder.new_row(Size::remainder());

            *ui.visuals_mut() = Visuals::light();
            customize_look_and_feel(ui.visuals_mut());
            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

            builder.show(ui, |mut grid| {
                for (tool, (name, _)) in GizmoTool::iter().zip(GIZMO_TOOL_LABELS.iter()) {
                    grid.cell(|ui| {
                        // let is_selected = self.selected == Some(tool);

                        let image_button = ImageButton::new(get_gizmo_tool_icon(tool))
                            .rounding(5.0)
                            .selected(self.selected == Some(tool))
                            .frame(true);

                        let response = ui.add(image_button);

                        if response.clicked() {
                            self.selected = Some(tool);
                        } else if response.hovered() {
                            egui::popup::show_tooltip(
                                ui.ctx(),
                                ui.layer_id(),
                                egui::Id::new(format!("popup-{}", name)),
                                |ui| {
                                    ui.label(name.to_string());
                                },
                            );
                        }
                    });
                }
            });
        });
    }

    pub fn show_tool_wíndow(
        &mut self,
        ui: &mut egui::Ui,
        (_ui_state, global_state): &(crate::ui::UiState, crate::GlobalState<crate::RootEvent>),
    ) {
        let index = self.selected.as_ref().map(|tool| *tool as usize);

        if let Some(index) = index {
            let (name, tool) = GIZMO_TOOL_LABELS[index];

            let mut frame = egui::Frame::window(ui.style());
            frame.fill = Color32::from_rgba_premultiplied(
                frame.fill.r(),
                frame.fill.g(),
                frame.fill.b(),
                220,
            );

            let mut open = true;

            egui::Window::new(name)
                .open(&mut open)
                .movable(true)
                .collapsible(false)
                .resizable(false)
                .frame(frame)
                .show(ui.ctx(), |ui| {
                    global_state.viewer.transform_selected(|transform| {
                        let (mut scale, rotation, mut translation) =
                            transform.to_scale_rotation_translation();
                        match tool {
                            GizmoTool::Translate => {
                                let mut changed = false;

                                ui.horizontal(|ui| {
                                    fn drag_value(ui: &mut egui::Ui, value: &mut f32) -> bool {
                                        let response =
                                            ui.add(DragValue::new(value).max_decimals(3));

                                        response.changed()
                                    }

                                    changed |= drag_value(ui, &mut translation.x);
                                    changed |= drag_value(ui, &mut translation.z);
                                    changed |= drag_value(ui, &mut translation.y);
                                });

                                *transform = Mat4::from_scale_rotation_translation(
                                    scale,
                                    rotation,
                                    translation,
                                );

                                changed
                            }
                            GizmoTool::Rotate => {
                                let (mut x, mut y, mut z) = rotation.to_euler(glam::EulerRot::XZY);

                                let mut changed = false;

                                ui.horizontal(|ui| {
                                    fn drag_angle(ui: &mut egui::Ui, value: &mut f32) -> bool {
                                        let response = ui.drag_angle(value);

                                        response.changed()
                                    }

                                    changed |= drag_angle(ui, &mut x);
                                    changed |= drag_angle(ui, &mut y);
                                    changed |= drag_angle(ui, &mut z);
                                });

                                *transform = Mat4::from_scale_rotation_translation(
                                    scale,
                                    Quat::from_euler(glam::EulerRot::XZY, x, y, z),
                                    translation,
                                );

                                changed
                            }
                            GizmoTool::Scale => {
                                let mut changed = false;
                                ui.horizontal(|ui| {
                                    fn drag_value(ui: &mut egui::Ui, value: &mut f32) -> bool {
                                        let response = ui.add(
                                            DragValue::new(value)
                                                .speed(0.025)
                                                .range(0.1..=100.0)
                                                .max_decimals(3),
                                        );

                                        response.changed()
                                    }

                                    changed |= drag_value(ui, &mut scale.x);
                                    changed |= drag_value(ui, &mut scale.z);
                                    changed |= drag_value(ui, &mut scale.y);
                                });

                                *transform = Mat4::from_scale_rotation_translation(
                                    scale,
                                    rotation,
                                    translation,
                                );

                                changed
                            }
                            GizmoTool::Flatten => {
                                ui.label("Flatten");

                                false
                            }
                        }
                    });
                });

            if !open {
                self.selected = None;
            }
        }
    }
}
