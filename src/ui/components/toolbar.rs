use egui::{Button, Layout, RichText};
use egui::{Context, SidePanel};

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

pub struct Toolbar<'a> {
    state: &'a mut ToolBarState,
    tools: &'a mut [&'a mut dyn ToolState],
}

impl<'a> Toolbar<'a> {
    pub fn with_state(state: &'a mut ToolBarState) -> Self {
        Self {
            state,
            tools: &mut [],
        }
    }

    pub fn with_tools(mut self, tools: &'a mut [&'a mut dyn ToolState]) -> Self {
        self.tools = tools;
        self
    }
}

impl<'a> UiComponent for Toolbar<'a> {
    fn show(&mut self, ctx: &Context, _shared_state: &(UiState, GlobalState<RootEvent>)) {
        if self.state.enabled {
            self.state.boundary = SidePanel::right("toolbar")
                .resizable(false)
                .default_width(config::gui::TOOLBAR_W)
                .show(ctx, |ui| {
                    ui.separator();

                    for tool in self.tools.iter_mut() {
                        let button = config::gui::TOOL_TOGGLE_BUTTON;

                        // let icon = tool.get_icon();

                        let image_button = Button::new(RichText::new(tool.get_icon()).size(35.0))
                            .frame(true)
                            .selected(*tool.get_enabled())
                            .rounding(5.0);

                        ui.allocate_ui(
                            [button.size.0 + button.border, button.size.1 + button.border].into(),
                            |ui| {
                                let response =
                                    ui.add_sized([button.size.0, button.size.1], image_button);

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

                        ui.add_space(5.0);
                    }

                    ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| ui.separator());
                })
                .response
                .into();
        }
    }
}
