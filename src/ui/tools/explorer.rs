use egui::Color32;

use crate::{ui::UiState, GlobalState, RootEvent};

use super::{create_tool, impl_tool_state_trait, impl_with_state, Tool};

#[derive(Debug, Default)]
pub struct ExplorerToolState {
    enabled: bool,
    anchored: bool,
}

impl_tool_state_trait!(ExplorerToolState, "Explorer", "⥈"); // ⤡ ⟳ ⥈

create_tool!(ExplorerTool, ExplorerToolState);
impl_with_state!(ExplorerTool, ExplorerToolState);

impl Tool for ExplorerTool<'_> {
    fn show(
        &mut self,
        ctx: &egui::Context,
        (_ui_state, _global_state): &(UiState, GlobalState<RootEvent>),
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

            egui::Window::new("Debug")
                .open(&mut self.state.enabled)
                .movable(!self.state.anchored)
                .collapsible(false)
                .frame(frame)
                .show(ctx, |ui| {
                    if ui.button("🔍").clicked() {
                        //
                    }

                    pointer_over_tool = ui.ui_contains_pointer();
                });
        }

        pointer_over_tool
    }
}
