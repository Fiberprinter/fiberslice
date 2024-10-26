use egui::RichText;

use crate::ui::boundary::Boundary;
use crate::ui::{Theme, UiComponent, UiComponentState, UiState};
use crate::{config, GlobalState, RootEvent};

pub struct TaskbarState {
    enabled: bool,
    boundary: Boundary,
}

impl TaskbarState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            boundary: Boundary::zero(),
        }
    }
}

impl UiComponentState for TaskbarState {
    fn get_boundary(&self) -> Boundary {
        self.boundary
    }

    fn get_enabled(&mut self) -> &mut bool {
        &mut self.enabled
    }

    fn get_name(&self) -> &str {
        "Taskbar"
    }
}

pub struct Taskbar<'a> {
    state: &'a mut TaskbarState,
}

impl<'a> Taskbar<'a> {
    pub fn with_state(state: &'a mut TaskbarState) -> Self {
        Self { state }
    }
}

impl<'a> UiComponent for Taskbar<'a> {
    fn show(
        &mut self,
        ctx: &egui::Context,
        (ui_state, global_state): &(UiState, GlobalState<RootEvent>),
    ) {
        if self.state.enabled {
            self.state.boundary = egui::TopBottomPanel::bottom("taskbar")
                .default_height(config::gui::TASKBAR_H)
                .show(ctx, |ui: &mut egui::Ui| {
                    egui::menu::bar(ui, |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add_space(10.0);
                            ui.label(format!("{:.3} ms", global_state.ctx.frame_time * 1000.0));
                        });

                        const VERSION: &str = env!("CARGO_PKG_VERSION");

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            theme_button(ui, ui_state);
                            ui.label(RichText::new(format!("Version: {}", VERSION)));
                        });
                    });
                })
                .response
                .into();
        }
    }
}

fn theme_button(ui: &mut egui::Ui, ui_state: &UiState) {
    let clicked = ui_state.theme.read_with_fn(|theme| match theme {
        Theme::Dark => ui.button("💡").clicked(),
        Theme::Light => ui.button("🌙").clicked(),
    });

    if clicked {
        ui_state.toggle_theme();
    }
}
