use egui::Color32;
use egui_code_editor::{ColorTheme, Syntax};

use crate::{
    ui::{
        widgets::reader::{EfficientReader, ReadSection},
        UiState,
    },
    viewer::GCodeSyntax,
    GlobalState, RootEvent,
};

use super::{create_tool, impl_tool_state_trait, impl_with_state, Tool};

#[derive(Debug)]
pub struct GCodeToolState {
    enabled: bool,
    anchored: bool,
    view: ReadSection,
}

impl Default for GCodeToolState {
    fn default() -> Self {
        Self {
            enabled: false,
            anchored: false,
            view: ReadSection::new(0, 20),
        }
    }
}

impl_tool_state_trait!(GCodeToolState, "GCode", "📄");

create_tool!(GCodeTool, GCodeToolState);
impl_with_state!(GCodeTool, GCodeToolState);

impl Tool for GCodeTool<'_> {
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

            egui::Window::new("GCode")
                .open(&mut self.state.enabled)
                .movable(!self.state.anchored)
                .collapsible(false)
                .frame(frame)
                .show(ctx, |ui| {
                    if let Some(gcode) = global_state.viewer.sliced_gcode() {
                        // let line_breaks = &toolpath.line_breaks;

                        EfficientReader::new(&mut self.state.view)
                            .id_source("code editor")
                            .with_fontsize(14.0)
                            .with_theme(ColorTheme::GRUVBOX)
                            .with_syntax(Syntax::gcode())
                            .with_numlines(true)
                            // .with_focus(Some(ReadSection::new(0, 20)))
                            .show(ui, gcode, &[]);
                    }

                    pointer_over_tool = ui.ui_contains_pointer();
                });
        }

        pointer_over_tool
    }
}
