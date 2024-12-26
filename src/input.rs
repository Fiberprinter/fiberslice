use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};

use crate::{
    prelude::{
        create_event_bundle, Adapter, AdapterCreation, Error, EventReader, FrameHandle, WgpuContext,
    },
    viewer::CameraResult,
    GlobalState, RootEvent,
};

pub mod hitbox;
pub mod interact;
mod queue;
mod ray;

pub use ray::Ray;

#[derive(Debug)]
pub enum InputEvent {}

#[derive(Debug)]
pub struct MouseClickEvent {
    pub ray: Ray,
    pub button: MouseButton,
    pub state: ElementState,
}

pub struct MouseMotionEvent {
    pub ray: Ray,
    pub delta: (f64, f64),
}

#[derive(Debug, Clone)]
pub struct InputState {
    is_drag_left: bool,
    is_drag_right: bool,
}

pub struct InputAdapter {
    state: InputState,

    camera_result: Option<CameraResult>,
    event_reader: EventReader<InputEvent>,
}

impl FrameHandle<'_, RootEvent, (), &CameraResult> for InputAdapter {
    fn handle_frame(
        &mut self,
        _wgpu_context: &WgpuContext,
        _global_state: GlobalState<RootEvent>,
        camera_result: &CameraResult,
    ) -> Result<(), Error> {
        self.camera_result = Some(camera_result.clone());

        Ok(())
    }

    fn handle_window_event(
        &mut self,
        event: &WindowEvent,
        _id: winit::window::WindowId,
        _wgpu_context: &WgpuContext,
        global_state: GlobalState<RootEvent>,
    ) {
        let pointer_in_use = global_state
            .ui_state
            .pointer_in_use
            .load(std::sync::atomic::Ordering::Relaxed);

        if !pointer_in_use {
            if let Some(CameraResult {
                view,
                proj,
                viewport,
                eye,
            }) = self.camera_result.clone()
            {
                match event {
                    WindowEvent::MouseInput { button, state, .. } => {
                        let (x, y) = global_state.ctx.mouse_position.unwrap_or((0.0, 0.0));

                        let ray = Ray::from_view(viewport, (x, y), view, proj, eye);

                        global_state.viewer.mouse_input(MouseClickEvent {
                            ray,
                            button: *button,
                            state: *state,
                        });

                        match button {
                            winit::event::MouseButton::Left => {
                                self.state.is_drag_left = *state == ElementState::Pressed;
                            }
                            winit::event::MouseButton::Right => {
                                self.state.is_drag_right = *state == ElementState::Pressed;
                            }
                            _ => (),
                        }
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        global_state.viewer.keyboard_input(event.clone());
                    }
                    _ => (),
                }
            }
        } else if let WindowEvent::MouseInput { button, state, .. } = event {
            match button {
                winit::event::MouseButton::Left => {
                    self.state.is_drag_left = *state == ElementState::Pressed;
                }
                winit::event::MouseButton::Right => {
                    self.state.is_drag_right = *state == ElementState::Pressed;
                }
                _ => (),
            }
        }
    }

    fn handle_device_event(
        &mut self,
        event: &DeviceEvent,
        _id: winit::event::DeviceId,
        _wgpu_context: &WgpuContext,
        global_state: GlobalState<RootEvent>,
    ) {
        let pointer_in_use = global_state
            .ui_state
            .pointer_in_use
            .load(std::sync::atomic::Ordering::Relaxed);

        if !pointer_in_use {
            if let Some(CameraResult {
                view,
                proj,
                viewport,
                eye,
            }) = self.camera_result.clone()
            {
                if let DeviceEvent::MouseMotion { delta } = event {
                    let (x, y) = global_state.ctx.mouse_position.unwrap_or((0.0, 0.0));

                    let ray = Ray::from_view(viewport, (x, y), view, proj, eye);

                    global_state.viewer.mouse_delta(MouseMotionEvent {
                        ray,
                        delta: (delta.0, delta.1),
                    });

                    if self.state.is_drag_left {
                        println!("PickingAdapter: Dragging Left Click");
                    }

                    if self.state.is_drag_right {
                        println!("PickingAdapter: Dragging Right Click");
                    }
                }
            }
        }
    }
}

impl<'a> Adapter<'a, RootEvent, InputState, (), &CameraResult, InputEvent> for InputAdapter {
    fn create(_wgpu_context: &WgpuContext) -> AdapterCreation<InputState, InputEvent, Self> {
        let state = InputState {
            is_drag_left: false,
            is_drag_right: false,
        };

        let (reader, writer) = create_event_bundle::<InputEvent>();

        (
            state.clone(),
            writer,
            InputAdapter {
                camera_result: None,
                state,
                event_reader: reader,
            },
        )
    }

    fn get_adapter_description(&self) -> String {
        "PickingAdapter".to_string()
    }

    fn get_reader(&self) -> crate::prelude::EventReader<InputEvent> {
        self.event_reader.clone()
    }

    fn handle_event(
        &mut self,
        _wgpu_context: &WgpuContext,
        _global_state: &GlobalState<RootEvent>,
        event: InputEvent,
    ) {
        match event {}
    }
}
