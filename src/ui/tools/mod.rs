use egui::{Color32, ImageSource};

use crate::{
    prelude::PrepareMode, viewer::camera_controller::CameraController, GlobalState, RootEvent,
};

use super::UiState;

mod debug;
mod explorer;
mod gcode;
mod visibility;

pub trait Tool {
    fn show(
        &mut self,
        ctx: &egui::Context,
        shared_state: &(UiState, GlobalState<RootEvent>),
    ) -> bool;
}

#[derive(Debug, Default)]
pub struct Tools {
    pub camera_tool: CameraToolState,
    pub gcode_tool: gcode::GCodeToolState,
    pub visibility_tool: visibility::VisibilityToolState,
    pub explorer_tool: explorer::ExplorerToolState,

    #[cfg(debug_assertions)]
    profile_tool: ProfilerState,

    #[cfg(debug_assertions)]
    debug_tool: debug::DebugToolState,
}

impl Tools {
    pub fn show(&mut self, ctx: &egui::Context, shared_state: &(UiState, GlobalState<RootEvent>)) {
        let mut pointer_over_tool = false;

        pointer_over_tool |= CameraTool::with_state(&mut self.camera_tool).show(ctx, shared_state);

        shared_state.0.mode.read_with_fn(|mode| match mode {
            crate::prelude::Mode::Preview => {
                pointer_over_tool |=
                    gcode::GCodeTool::with_state(&mut self.gcode_tool).show(ctx, shared_state);
                pointer_over_tool |=
                    visibility::VisibilityTool::with_state(&mut self.visibility_tool)
                        .show(ctx, shared_state);
            }
            crate::prelude::Mode::Prepare(PrepareMode::Objects) => {
                pointer_over_tool |= explorer::ExplorerTool::with_state(&mut self.explorer_tool)
                    .show(ctx, shared_state);
            }
            crate::prelude::Mode::Prepare(PrepareMode::Masks) => {
                pointer_over_tool |= explorer::ExplorerTool::with_state(&mut self.explorer_tool)
                    .show(ctx, shared_state);
            }
        });

        #[cfg(debug_assertions)]
        {
            pointer_over_tool |=
                Profiler::with_state(&mut self.profile_tool).show(ctx, shared_state);

            pointer_over_tool |=
                debug::DebugTool::with_state(&mut self.debug_tool).show(ctx, shared_state);
        }

        if pointer_over_tool {
            shared_state
                .0
                .pointer_in_use
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn states<'a>(
        &'a mut self,
        shared_state: &(UiState, GlobalState<RootEvent>),
        r#fn: impl FnOnce(&mut [&'a mut dyn ToolState], &mut [&'a mut dyn ToolState]),
    ) {
        let mode = shared_state.0.mode.read();

        match &*mode {
            crate::prelude::Mode::Preview => r#fn(
                &mut [&mut self.gcode_tool, &mut self.visibility_tool],
                &mut [
                    &mut self.camera_tool,
                    #[cfg(debug_assertions)]
                    &mut self.profile_tool,
                    #[cfg(debug_assertions)]
                    &mut self.debug_tool,
                ],
            ),
            crate::prelude::Mode::Prepare(PrepareMode::Objects) => r#fn(
                &mut [&mut self.explorer_tool],
                &mut [
                    &mut self.camera_tool,
                    #[cfg(debug_assertions)]
                    &mut self.profile_tool,
                    #[cfg(debug_assertions)]
                    &mut self.debug_tool,
                ],
            ),
            crate::prelude::Mode::Prepare(PrepareMode::Masks) => r#fn(
                &mut [&mut self.explorer_tool],
                &mut [
                    &mut self.camera_tool,
                    #[cfg(debug_assertions)]
                    &mut self.profile_tool,
                    #[cfg(debug_assertions)]
                    &mut self.debug_tool,
                ],
            ),
        }
    }
}

pub trait ToolState {
    fn get_enabled(&mut self) -> &mut bool;

    fn get_popup_string(&self) -> &str {
        ""
    }

    fn get_icon(&self) -> ImageSource<'static>;
}

macro_rules! impl_tool_state_trait {
    ($name:ident) => {
        impl crate::ui::tools::ToolState for $name {
            fn get_enabled(&mut self) -> &mut bool {
                &mut self.enabled
            }

            fn get_popup_string(&self) -> &str {
                stringify!($name)
            }

            fn get_icon(&self) -> ImageSource<'static> {
                egui::include_image!("assets/gcode_tool.svg")
            }
        }
    }; {
        $name:ident, $icon:expr
    } => {
        impl crate::ui::tools::ToolState for $name {
            fn get_enabled(&mut self) -> &mut bool {
                &mut self.enabled
            }

            fn get_popup_string(&self) -> &str {
                stringify!($name)
            }

            fn get_icon(&self) -> egui::ImageSource<'static> {
                egui::include_image!(concat!("assets/", $icon))
            }
        }
    }; {
        $name:ident, $popup:expr, $icon:expr
    } => {
        impl crate::ui::tools::ToolState for $name {
            fn get_enabled(&mut self) -> &mut bool {
                &mut self.enabled
            }

            fn get_popup_string(&self) -> &str {
                $popup
            }

            fn get_icon(&self) -> egui::ImageSource<'static> {
                egui::include_image!(concat!("../assets/", $icon))
            }
        }
    };
}

macro_rules! create_tool {
    ($name:ident, $state:ident) => {
        pub struct $name<'a> {
            state: &'a mut $state,
        }
    };
}

macro_rules! impl_with_state {
    ($name:ident, $state:ident) => {
        impl<'a> $name<'a> {
            #[allow(dead_code)]
            pub fn with_state(state: &'a mut $state) -> Self {
                Self { state }
            }
        }
    };
}

pub(crate) use create_tool;
pub(crate) use impl_tool_state_trait;
pub(crate) use impl_with_state;

#[derive(Debug, Default)]
pub struct CameraToolState {
    enabled: bool,
    anchored: bool,
}

impl_tool_state_trait!(CameraToolState, "Camera Settings", "camera_tool.svg");
create_tool!(CameraTool, CameraToolState);
impl_with_state!(CameraTool, CameraToolState);

impl Tool for CameraTool<'_> {
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
                230,
            );

            egui::Window::new("Camera Controls")
                .open(&mut self.state.enabled)
                .collapsible(false)
                .movable(!self.state.anchored)
                .frame(frame)
                .show(ctx, |ui| {
                    global_state.camera_controller.write_with_fn(|controller| {
                        ui.horizontal(|ui| {
                            ui.label(format!("{:20}", "Rotate Speed"));

                            let slider =
                                egui::Slider::new(&mut controller.rotate_speed, -0.1..=0.1);
                            ui.add(slider);

                            if ui.button("Reset").clicked() {
                                controller.rotate_speed = CameraController::default().rotate_speed;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(format!("{:20}", "Zoom Speed  "));

                            let slider = egui::Slider::new(&mut controller.zoom_speed, -5.0..=5.0);
                            ui.add(slider);

                            if ui.button("Reset").clicked() {
                                controller.zoom_speed = CameraController::default().zoom_speed;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label(format!("{:20}", "Move Speed  "));

                            let slider = egui::Slider::new(&mut controller.move_speed, -1.0..=1.0);
                            ui.add(slider);

                            if ui.button("Reset").clicked() {
                                controller.move_speed = CameraController::default().move_speed;
                            }
                        });
                    });

                    pointer_over_tool = ui.ui_contains_pointer();
                });
        }

        pointer_over_tool
    }
}

#[derive(Debug, Default)]
pub struct ProfilerState {
    enabled: bool,
    anchored: bool,
}

impl_tool_state_trait!(ProfilerState, "Profile", "profiler_tool.svg");
create_tool!(Profiler, ProfilerState);
impl_with_state!(Profiler, ProfilerState);

impl Tool for Profiler<'_> {
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

            egui::Window::new("Profile")
                .open(&mut self.state.enabled)
                .movable(!self.state.anchored)
                .collapsible(false)
                .frame(frame)
                .show(ctx, |ui| {
                    puffin_egui::profiler_ui(ui);

                    pointer_over_tool = ui.ui_contains_pointer();
                });
        }

        pointer_over_tool
    }
}
