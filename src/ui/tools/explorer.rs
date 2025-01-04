use egui::Color32;

use crate::{
    ui::{api::trim_text, UiState},
    GlobalState, RootEvent,
};

use super::{create_tool, impl_tool_state_trait, impl_with_state, Tool};

#[derive(Debug, Default)]
pub struct ExplorerToolState {
    enabled: bool,
    anchored: bool,
}

impl_tool_state_trait!(ExplorerToolState, "Explorer", "explorer_tool.svg");

create_tool!(ExplorerTool, ExplorerToolState);
impl_with_state!(ExplorerTool, ExplorerToolState);

impl Tool for ExplorerTool<'_> {
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

            egui::Window::new("Explorer")
                .open(&mut self.state.enabled)
                .movable(!self.state.anchored)
                .collapsible(false)
                .frame(frame)
                .show(ctx, |ui| {
                    let objects = global_state.viewer.objects();
                    let masks = global_state.viewer.masks();

                    ui.heading("Objects");
                    ui.add_space(5.0);

                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.vertical(|ui| {
                                    for (i, (name, object)) in objects.into_iter().enumerate() {
                                        if i > 0 {
                                            ui.separator();
                                        }

                                        ui.horizontal(|ui| {
                                            ui.label(trim_text::<15, 4>(&name));
                                            ui.add_space(10.0);

                                            ui.button("Select");
                                            ui.add_space(5.0);
                                            ui.button("Delete");
                                        });
                                    }
                                });
                            });
                        });

                    ui.add_space(10.0);

                    ui.heading("Masks");
                    ui.add_space(5.0);

                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.vertical(|ui| {
                                    for (i, (name, mask)) in masks.into_iter().enumerate() {
                                        if i > 0 {
                                            ui.separator();
                                        }

                                        ui.horizontal(|ui| {
                                            ui.label(trim_text::<15, 4>(&name));
                                            ui.add_space(10.0);

                                            ui.button("Select");
                                            ui.add_space(5.0);
                                            ui.button("Delete");
                                        });
                                    }
                                });
                            });
                        });

                    pointer_over_tool = ui.ui_contains_pointer();
                });
        }

        pointer_over_tool
    }
}
