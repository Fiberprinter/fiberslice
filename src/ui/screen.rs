use super::*;
use components::{
    addons,
    menubar::{self, MenubarState},
    modebar::{self, ModebarState},
    sidebar, taskbar,
    toolbar::{self, ToolBarState},
    topbar::{self, TopBarState},
};

use egui::{Align2, Direction, Id, Margin};
use egui_toast::Toasts;

#[derive(Debug, Default)]
pub struct ViewerTooltip {
    server_type: String,
    model: String,
}

impl ViewerTooltip {
    pub fn new(server_type: String, model: String) -> Self {
        Self { server_type, model }
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new(&self.server_type).strong());
        ui.label(&self.model);
    }
}

pub struct Screen {
    toasts: Toasts,
    toasts_progress_bar: Toasts,

    tools: tools::Tools,
    addons_state: addons::AddonsState,

    settings_state: sidebar::SidebarState,
    menubar_state: MenubarState,
    taskbar_state: taskbar::TaskbarState,
    modebar_state: ModebarState,
    toolbar_state: ToolBarState,
    topbar_state: TopBarState,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            tools: tools::Tools::default(),
            toasts: Toasts::with_id(Id::new("__toasts"))
                .anchor(Align2::CENTER_TOP, (0.0, 10.0))
                .direction(Direction::TopDown),
            toasts_progress_bar: Toasts::with_id(Id::new("__toasts_progress_bar"))
                .anchor(Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(Direction::TopDown)
                .custom_contents(
                    crate::ui::custom_toasts::OBJECT_LOAD_PROGRESS,
                    crate::ui::custom_toasts::object_load_progress,
                )
                .custom_contents(
                    crate::ui::custom_toasts::SLICING_PROGRESS,
                    crate::ui::custom_toasts::slicing_progress,
                ),
            addons_state: addons::AddonsState::new(),
            settings_state: sidebar::SidebarState::new(),
            menubar_state: MenubarState::new(),
            taskbar_state: taskbar::TaskbarState::new(),
            modebar_state: ModebarState::new(),
            toolbar_state: ToolBarState::new(),
            topbar_state: TopBarState::new(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, shared_state: &(UiState, GlobalState<RootEvent>)) {
        let frame = egui::containers::Frame {
            fill: egui::Color32::TRANSPARENT,
            outer_margin: Margin::symmetric(10.0, 10.0),
            ..Default::default()
        };

        menubar::Menubar::with_state(&mut self.menubar_state)
            .with_component_states(&mut [
                &mut self.addons_state,
                &mut self.settings_state,
                &mut self.taskbar_state,
                &mut self.modebar_state,
                &mut self.toolbar_state,
                &mut self.topbar_state,
            ])
            .show(ctx, shared_state);

        topbar::Topbar::with_state(&mut self.topbar_state).show(ctx, shared_state);

        taskbar::Taskbar::with_state(&mut self.taskbar_state).show(ctx, shared_state);

        sidebar::Settingsbar::with_state(&mut self.settings_state).show(ctx, shared_state);

        modebar::Modebar::with_state(&mut self.modebar_state).show(ctx, shared_state);

        self.tools
            .states(shared_state, |top_states, bottom_states| {
                toolbar::Toolbar::with_state(&mut self.toolbar_state)
                    .with_top_tools(top_states)
                    .with_bottom_tools(bottom_states)
                    .show(ctx, shared_state);
            });

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            self.toasts.show_inside(ui);
            self.toasts_progress_bar.show_inside(ui);

            shared_state.1.viewer.read_tooltip_with_fn(|tooltip| {
                if ui.ui_contains_pointer() {
                    egui::show_tooltip_at_pointer(
                        ui.ctx(),
                        ui.layer_id(),
                        egui::Id::new("viewer_tooltip"),
                        |ui| {
                            tooltip.show(ui);
                        },
                    );
                }
            });

            addons::Addons::with_state(&mut self.addons_state).show(ui, shared_state);
            self.tools.show(ctx, shared_state);
        });
    }

    pub fn add_toast(&mut self, toast: egui_toast::Toast) {
        self.toasts.add(toast);
    }

    pub fn add_process_as_toast(&mut self, toast: egui_toast::Toast) {
        self.toasts_progress_bar.add(toast);
    }

    pub fn tools_mut(&mut self) -> &mut tools::Tools {
        &mut self.tools
    }

    pub fn construct_viewport(&self, wgpu_context: &WgpuContext) -> (f32, f32, f32, f32) {
        let height = wgpu_context.surface_config.height as f32
            - self.taskbar_state.get_boundary().get_height()
            - self.modebar_state.get_boundary().get_height()
            - self.menubar_state.get_boundary().get_height();

        (
            self.settings_state.get_boundary().get_width(),
            self.taskbar_state.get_boundary().get_height()
                + self.modebar_state.get_boundary().get_height(),
            wgpu_context.surface_config.width as f32
                - self.toolbar_state.get_boundary().get_width()
                - self.settings_state.get_boundary().get_width(),
            height,
        )
    }
}
