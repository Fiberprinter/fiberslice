use egui::{Context, SidePanel};
use egui::{ImageButton, Layout};

use crate::config;
use crate::ui::boundary::Boundary;
use crate::ui::tools::ToolState;
use crate::ui::UiComponent;
use crate::ui::UiComponentState;
use crate::ui::UiState;
use crate::GlobalState;
use crate::RootEvent;

#[derive(Debug)]
pub struct ToolBarState {
    enabled: bool,
    boundary: Boundary,
}

impl ToolBarState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            boundary: Boundary::zero(),
        }
    }
}

impl UiComponentState for ToolBarState {
    fn get_boundary(&self) -> Boundary {
        self.boundary
    }

    fn get_enabled(&mut self) -> &mut bool {
        &mut self.enabled
    }

    fn get_name(&self) -> &str {
        "Toolbar"
    }
}

pub struct Toolbar<'a, 'b> {
    state: &'a mut ToolBarState,
    top_tools: &'a mut [&'b mut dyn ToolState],
    bottom_tools: &'a mut [&'b mut dyn ToolState],
}

impl<'a, 'b> Toolbar<'a, 'b> {
    pub fn with_state(state: &'a mut ToolBarState) -> Self {
        Self {
            state,
            top_tools: &mut [],
            bottom_tools: &mut [],
        }
    }

    pub fn with_top_tools(mut self, tools: &'a mut [&'b mut dyn ToolState]) -> Self {
        self.top_tools = tools;
        self
    }

    pub fn with_bottom_tools(mut self, tools: &'a mut [&'b mut dyn ToolState]) -> Self {
        self.bottom_tools = tools;
        self
    }
}

impl<'a, 'b> UiComponent for Toolbar<'a, 'b> {
    fn show(&mut self, ctx: &Context, _shared_state: &(UiState, GlobalState<RootEvent>)) {
        if self.state.enabled {
            self.state.boundary = SidePanel::right("toolbar")
                .resizable(false)
                .default_width(config::gui::TOOLBAR_W)
                .show(ctx, |ui| {
                    ui.separator();

                    for tool in self.top_tools.iter_mut() {
                        show_tool(ui, tool);

                        ui.add_space(5.0);
                    }

                    ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.separator();

                        for tool in self.bottom_tools.iter_mut() {
                            show_tool(ui, tool);

                            ui.add_space(5.0);
                        }
                    });
                })
                .response
                .into();
        }
    }
}

fn show_tool(ui: &mut egui::Ui, tool: &mut &mut dyn ToolState) {
    let button = config::gui::TOOL_TOGGLE_BUTTON;

    // let icon = tool.get_icon();

    let image_button = ImageButton::new(tool.get_icon())
        .frame(true)
        .selected(*tool.get_enabled())
        .rounding(5.0);

    ui.allocate_ui(
        [button.size.0 + button.border, button.size.1 + button.border].into(),
        |ui| {
            let response = ui.add(image_button);

            if response.clicked() {
                *tool.get_enabled() = !*tool.get_enabled();
            } else if response.hovered() {
                egui::popup::show_tooltip(
                    ui.ctx(),
                    ui.layer_id(),
                    egui::Id::new(format!("popup-{}", tool.get_popup_string())),
                    |ui| {
                        ui.label(tool.get_popup_string());
                    },
                );
            }
        },
    );
}
