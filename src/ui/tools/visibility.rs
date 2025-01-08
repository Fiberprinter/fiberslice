use egui::{CollapsingResponse, Color32, FontId, InnerResponse, RichText};
use slicer::TraceType;
use strum::EnumCount;
use wgpu::Color;

use crate::{ui::UiState, GlobalState, RootEvent};

use super::{create_tool, impl_tool_state_trait, impl_with_state, Tool};

#[derive(Debug)]
pub struct VisibilityToolState {
    enabled: bool,
    anchored: bool,
    transparent_vision: bool,
    trace_types: [bool; TraceType::COUNT],
    travel: bool,
    fiber: bool,
}

impl Default for VisibilityToolState {
    fn default() -> Self {
        Self {
            enabled: Default::default(),
            anchored: Default::default(),
            transparent_vision: false,
            trace_types: [true; TraceType::COUNT],
            travel: false,
            fiber: true,
        }
    }
}

impl_tool_state_trait!(VisibilityToolState, "Visibility", "visibility_tool.svg");

create_tool!(VisibilityTool, VisibilityToolState);
impl_with_state!(VisibilityTool, VisibilityToolState);

impl Tool for VisibilityTool<'_> {
    fn show(
        &mut self,
        ctx: &egui::Context,
        (_ui_state, global_state): &(UiState, GlobalState<RootEvent>),
    ) -> bool {
        let mut pointer_over_tool = false;

        if self.state.enabled {
            let mut frame = egui::Frame::window(&ctx.style());
            frame.fill = Color32::from_rgba_premultiplied(
                frame.fill.r(),
                frame.fill.g(),
                frame.fill.b(),
                220,
            );

            if let Some(count_map) = global_state.viewer.sliced_count_map() {
                egui::Window::new("Visibility")
                    .open(&mut self.state.enabled)
                    .movable(!self.state.anchored)
                    .collapsible(false)
                    .resizable(false)
                    .frame(frame)
                    .show(ctx, |ui| {
                        ui.separator();

                        if Self::show_transparent_vision_checkbox(
                            &mut self.state.transparent_vision,
                            ui,
                        )
                        .inner
                        {
                            global_state
                                .viewer
                                .enable_transparent_vision(self.state.transparent_vision);
                        }

                        if Self::show_trace_visibility_checkboxes(
                            &mut self.state.trace_types,
                            ui,
                            count_map,
                        )
                        .body_returned
                        .unwrap_or(false)
                        {
                            let mut visibility = 0;

                            for (index, visible) in self.state.trace_types.iter().enumerate() {
                                if *visible {
                                    visibility |= 1 << index;
                                }
                            }

                            global_state.viewer.update_gpu_visibility(visibility);
                        }

                        if Self::show_travel_checkbox(&mut self.state.travel, ui).inner {
                            global_state.viewer.enable_travel(self.state.travel);
                        }

                        if Self::show_fiber_checkbox(&mut self.state.fiber, ui).inner {
                            global_state.viewer.enable_fiber(self.state.fiber);
                        }

                        ui.separator();

                        pointer_over_tool = ui.ui_contains_pointer();
                    });
            }
        }

        pointer_over_tool
    }
}

impl<'a> VisibilityTool<'a> {
    fn show_fiber_checkbox(fiber: &mut bool, ui: &mut egui::Ui) -> InnerResponse<bool> {
        ui.horizontal(|ui| {
            ui.checkbox(
                fiber,
                RichText::new("Fiber")
                    .font(FontId::monospace(15.0))
                    .strong()
                    .color(Color32::BLACK),
            )
            .changed()
        })
    }

    fn show_travel_checkbox(travel: &mut bool, ui: &mut egui::Ui) -> InnerResponse<bool> {
        ui.horizontal(|ui| {
            ui.checkbox(
                travel,
                RichText::new("Travel")
                    .font(FontId::monospace(15.0))
                    .strong()
                    .color(Color32::BLACK),
            )
            .changed()
        })
    }

    fn show_trace_visibility_checkboxes(
        trace_types: &mut [bool; TraceType::COUNT],
        ui: &mut egui::Ui,
        count_map: std::collections::HashMap<TraceType, usize, egui::ahash::RandomState>,
    ) -> CollapsingResponse<bool> {
        egui::CollapsingHeader::new(
            RichText::new("Trace Types")
                .font(FontId::monospace(15.0))
                .strong()
                .color(Color32::BLACK),
        )
        .default_open(true)
        .show(ui, |ui| {
            let mut changed = false;

            for (trace_type, count) in count_map.iter() {
                let str_type: String = format!("{}", trace_type);
                let color_vec = trace_type.into_color_vec4();

                let color: wgpu::Color = Color {
                    r: color_vec.x as f64,
                    g: color_vec.y as f64,
                    b: color_vec.z as f64,
                    a: color_vec.w as f64,
                };

                let egui_color = Color32::from_rgba_premultiplied(
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    (color.a * 255.0) as u8,
                );

                ui.horizontal(|ui| {
                    changed |= ui
                        .checkbox(
                            &mut trace_types[*trace_type as usize],
                            RichText::new(str_type)
                                .font(FontId::monospace(15.0))
                                .strong()
                                .color(egui_color),
                        )
                        .changed();

                    ui.add_space(25.0);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{:7}", count))
                                .font(FontId::monospace(15.0))
                                .strong(),
                        );
                    });
                });
            }

            ui.separator();

            changed
        })
    }

    fn show_transparent_vision_checkbox(
        transparent_vision: &mut bool,
        ui: &mut egui::Ui,
    ) -> InnerResponse<bool> {
        ui.horizontal(|ui| {
            ui.checkbox(
                transparent_vision,
                RichText::new("Trace Transparent Mode")
                    .font(FontId::monospace(15.0))
                    .strong()
                    .color(Color32::BLACK),
            )
            .changed()
        })
    }
}
