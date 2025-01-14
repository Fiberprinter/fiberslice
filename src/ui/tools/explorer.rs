use egui::{vec2, Color32, FontId, Id, RichText};

use crate::{
    prelude::Destroyable,
    ui::{
        api::{size_fixed::StaticSizedLabel, trim_text},
        widgets::list::ListBuilder,
        UiState,
    },
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

                    ui.allocate_ui(vec2(ui.available_width(), 100.0), |ui| {
                        if objects.is_empty() {
                            let text = RichText::new("No Objects")
                                .font(FontId::new(35.0, egui::FontFamily::Monospace));

                            ui.centered_and_justified(|ui| {
                                ui.label(text);
                            });
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt(Id::new("objects explorer"))
                                .show(ui, |ui| {
                                    ListBuilder::new()
                                        .with_cell_height(25.0)
                                        .entries(objects.len())
                                        .show(ui, |mut list| {
                                            for (name, object) in objects.into_iter() {
                                                list.entry(|ui| {
                                                    ui.horizontal_centered(|ui| {
                                                        StaticSizedLabel::new(50.0)
                                                            .label(ui, trim_text::<15, 4>(&name));

                                                        ui.add_space(10.0);

                                                        if ui.button("Select").clicked() {
                                                            global_state
                                                                .viewer
                                                                .select_object(&object);
                                                        }
                                                        ui.add_space(5.0);

                                                        if ui.button("Delete").clicked() {
                                                            object.destroy();
                                                        }
                                                    });
                                                });
                                            }
                                        });
                                });
                        }
                    });

                    ui.add_space(10.0);

                    ui.heading("Masks");
                    ui.add_space(5.0);

                    ui.allocate_ui(vec2(ui.available_width(), 100.0), |ui| {
                        if masks.is_empty() {
                            let text = RichText::new("No Masks")
                                .font(FontId::new(35.0, egui::FontFamily::Monospace));

                            ui.centered_and_justified(|ui| {
                                ui.label(text);
                            });
                        } else {
                            egui::ScrollArea::vertical()
                                .id_salt(Id::new("masks explorer"))
                                .show(ui, |ui| {
                                    ListBuilder::new()
                                        .with_cell_height(25.0)
                                        .entries(masks.len())
                                        .show(ui, |mut list| {
                                            for (name, mask) in masks.into_iter() {
                                                list.entry(|ui| {
                                                    ui.horizontal_centered(|ui| {
                                                        StaticSizedLabel::new(50.0)
                                                            .label(ui, trim_text::<15, 4>(&name));

                                                        ui.add_space(10.0);

                                                        if ui.button("Select").clicked() {
                                                            global_state.viewer.select_mask(&mask);
                                                        }
                                                        ui.add_space(5.0);

                                                        if ui.button("Delete").clicked() {
                                                            mask.destroy();
                                                        }
                                                    });
                                                });
                                            }
                                        });
                                });
                        }
                    });

                    pointer_over_tool = ui.ui_contains_pointer();
                });
        }

        pointer_over_tool
    }
}
