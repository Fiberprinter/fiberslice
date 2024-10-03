use egui::{Color32, FontId, RichText};
use strum::{EnumCount, IntoEnumIterator};

use crate::{
    slicer::print_type::PrintType,
    ui::{api::trim_text, UiState},
    GlobalState, RootEvent,
};

use super::{Tool, ToolState};

#[derive(Debug)]
pub struct VisibilityToolState {
    enabled: bool,
    anchored: bool,
    print_types: [bool; PrintType::COUNT],
}

impl Default for VisibilityToolState {
    fn default() -> Self {
        Self {
            enabled: Default::default(),
            anchored: Default::default(),
            print_types: [true; PrintType::COUNT],
        }
    }
}

impl ToolState for VisibilityToolState {
    fn get_enabled(&mut self) -> &mut bool {
        &mut self.enabled
    }

    fn get_popup_string(&self) -> &str {
        "Visibility"
    }

    fn get_icon(&self) -> &str {
        "👓"
    }
}

pub struct VisibilityTool<'a> {
    state: &'a mut VisibilityToolState,
}

impl<'a> VisibilityTool<'a> {
    pub fn with_state(state: &'a mut VisibilityToolState) -> Self {
        Self { state }
    }
}

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

            egui::Window::new("Visibility")
                .open(&mut self.state.enabled)
                .movable(!self.state.anchored)
                .collapsible(false)
                .resizable(false)
                .frame(frame)
                .show(ctx, |ui| {
                    ui.separator();

                    egui::CollapsingHeader::new("Print Types")
                        .default_open(true)
                        .show(ui, |ui| {
                            let old = self.state.print_types;

                            for print_type in PrintType::iter() {
                                let str_type: &'static str = (&print_type).into();
                                let color: wgpu::Color = (&print_type).into();

                                let egui_color = Color32::from_rgba_premultiplied(
                                    (color.r * 255.0) as u8,
                                    (color.g * 255.0) as u8,
                                    (color.b * 255.0) as u8,
                                    (color.a * 255.0) as u8,
                                );

                                ui.horizontal(|ui| {
                                    ui.checkbox(
                                        &mut self.state.print_types[print_type as usize],
                                        RichText::new(str_type)
                                            .font(FontId::monospace(15.0))
                                            .strong()
                                            .color(egui_color),
                                    );

                                    ui.add_space(25.0);

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(format!("{:7}", 0))
                                                    .font(FontId::monospace(15.0))
                                                    .strong(),
                                            );
                                        },
                                    );
                                });
                            }

                            if old != self.state.print_types {
                                let mut visibility = 0;

                                for (index, visible) in self.state.print_types.iter().enumerate() {
                                    if *visible {
                                        visibility |= 1 << index;
                                    }
                                }

                                global_state
                                    .viewer
                                    .toolpath_server
                                    .write()
                                    .set_visibility(visibility);
                            }
                        });

                    ui.separator();

                    if let Some(toolpath) =
                        global_state.viewer.toolpath_server.read().get_toolpath()
                    {
                        egui::CollapsingHeader::new("Toolpath Parts")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let file_name = toolpath.origin_path.clone();

                                    let text = trim_text::<20, 4>(&file_name);

                                    ui.checkbox(
                                        &mut true,
                                        RichText::new(text).font(FontId::monospace(15.0)).strong(),
                                    );

                                    ui.add_space(15.0);

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{:7}",
                                                    toolpath.wire_model.len()
                                                ))
                                                .font(FontId::monospace(15.0))
                                                .strong(),
                                            );
                                        },
                                    );
                                });
                            });

                        ui.separator();
                    }

                    pointer_over_tool = ui.ui_contains_pointer();
                });
        }

        pointer_over_tool
    }
}
