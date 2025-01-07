use std::f32::consts::PI;

use glam::{Mat4, Vec3};
use strum_macros::{EnumCount, EnumIter};
use winit::event::WindowEvent;

use crate::{
    geometry::BoundingBox,
    prelude::{
        create_event_bundle, Adapter, AdapterCreation, EventReader, FrameHandle, Viewport,
        WgpuContext,
    },
    GlobalState, RootEvent,
};

pub mod camera_controller;
pub mod orbit_camera;

pub use self::orbit_camera::OrbitCamera;
// pub use self::orbit_camera::OrbitCameraBounds;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, EnumCount, EnumIter)] //maybe performance bit worse
pub enum Orientation {
    Default,
    Diagonal,
    Top,
    Left,
    Right,
    Front,
}

#[derive(Debug)]
pub enum CameraEvent {
    CameraOrientationChanged(Orientation),
    UpdatePreferredDistance(BoundingBox),
}

#[derive(Debug, Clone)]
pub struct CameraResult {
    pub view: Mat4,
    pub proj: Mat4,
    pub eye: Vec3,
    pub viewport: Viewport,
}

pub struct CameraAdapter {
    camera: OrbitCamera,
    viewport: Option<Viewport>,
    view: Mat4,
    proj: Mat4,

    event_reader: EventReader<CameraEvent>,
}

impl CameraAdapter {
    pub fn init_target(&mut self, target: Vec3) {
        self.camera.target = target;
    }
}

impl FrameHandle<'_, RootEvent, CameraResult, Viewport> for CameraAdapter {
    fn handle_frame(
        &'_ mut self,
        _wgpu_context: &crate::prelude::WgpuContext,
        _global_state: GlobalState<RootEvent>,
        viewport: Viewport,
    ) -> Result<CameraResult, crate::prelude::Error> {
        if viewport != self.viewport.unwrap_or_default() {
            self.viewport = Some(viewport);
            self.camera.aspect = viewport.2 / viewport.3;
        }

        let (view, proj) = self.camera.build_view_proj_matrix();
        self.view = view;
        self.proj = proj;

        Ok(CameraResult {
            view: self.view,
            proj: self.proj,
            eye: self.camera.eye,
            viewport,
        })
    }

    fn handle_window_event(
        &mut self,
        event: &WindowEvent,
        _id: winit::window::WindowId,
        wgpu_context: &WgpuContext,
        state: GlobalState<RootEvent>,
    ) {
        state.camera_controller.write_with_fn(|controller| {
            controller.handle_window_events(
                event,
                &wgpu_context.window,
                &mut self.camera,
                state
                    .ui_state
                    .pointer_in_use
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
        });
    }

    fn handle_device_event(
        &mut self,
        event: &winit::event::DeviceEvent,
        _id: winit::event::DeviceId,
        wgpu_context: &WgpuContext,
        state: GlobalState<RootEvent>,
    ) {
        state.camera_controller.write_with_fn(|controller| {
            controller.handle_device_events(
                event,
                &wgpu_context.window,
                &mut self.camera,
                state
                    .ui_state
                    .pointer_in_use
                    .load(std::sync::atomic::Ordering::Relaxed),
            )
        });
    }
}

impl Adapter<'_, RootEvent, (), CameraResult, Viewport, CameraEvent> for CameraAdapter {
    fn create(
        wgpu_context: &crate::prelude::WgpuContext,
    ) -> AdapterCreation<(), CameraEvent, Self> {
        let mut camera = OrbitCamera::new(
            2.0,
            1.5,
            1.25,
            Vec3::new(0.0, 100.0, 0.0),
            wgpu_context.window.inner_size().width as f32
                / wgpu_context.window.inner_size().height as f32,
        );
        camera.bounds.min_distance = Some(0.1);
        camera.bounds.min_pitch = -std::f32::consts::FRAC_PI_2 + 0.1;
        camera.bounds.max_pitch = std::f32::consts::FRAC_PI_2 - 0.1;
        camera.handle_orientation(Orientation::Default);

        let (reader, writer) = create_event_bundle::<CameraEvent>();

        (
            (),
            writer,
            Self {
                camera,
                viewport: None,
                view: Mat4::IDENTITY,
                proj: Mat4::IDENTITY,
                event_reader: reader,
            },
        )
    }

    fn get_adapter_description(&self) -> String {
        "CameraAdapter".to_string()
    }

    fn get_reader(&self) -> EventReader<CameraEvent> {
        self.event_reader.clone()
    }

    fn handle_event(
        &mut self,
        wgpu_context: &WgpuContext,
        _global_state: &GlobalState<RootEvent>,
        event: CameraEvent,
    ) {
        match event {
            CameraEvent::CameraOrientationChanged(orientation) => {
                self.camera.handle_orientation(orientation);
            }
            CameraEvent::UpdatePreferredDistance(distance) => {
                self.camera.set_preferred_distance(&distance);
            }
        }

        wgpu_context.window.request_redraw();
    }
}

/// A camera is used for rendering specific parts of the scene.
pub trait Camera: Sized {
    fn build_view_proj_matrix(&self) -> (Mat4, Mat4);
}

/// The camera uniform contains the data linked to the camera that is passed to the shader.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// The eye position of the camera in homogenous coordinates.
    ///
    /// Homogenous coordinates are used to fullfill the 16 byte alignment requirement.
    pub view_position: [f32; 4],

    /// Contains the view projection matrix.
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    /// Updates the view projection matrix of this [CameraUniform].
    ///
    /// Arguments:
    /// * `camera`: The [OrbitCamera] from which the matrix will be computed.
    pub fn update_view_proj(&mut self, view_proj: Mat4, eye: Vec3) {
        self.view_position = [eye.x, eye.y, eye.z, 1.0];
        self.view_proj = view_proj.to_cols_array_2d();
    }
}

impl Default for CameraUniform {
    /// Creates a default [CameraUniform].
    fn default() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

pub trait HandleOrientation {
    fn handle_orientation(&mut self, orientation: Orientation);
}

impl HandleOrientation for OrbitCamera {
    fn handle_orientation(&mut self, orientation: Orientation) {
        let (yaw, pitch) = match orientation {
            Orientation::Default => (PI + (PI / 8.0), PI / 4.0),
            Orientation::Diagonal => (PI + (PI / 8.0), PI / 4.0),
            Orientation::Top => (PI, PI / 2.0),
            Orientation::Left => (-PI / 2.0, 0.0),
            Orientation::Right => (PI / 2.0, 0.0),
            Orientation::Front => (PI, 0.0),
        };

        self.set_yaw(yaw);
        self.set_pitch(pitch);
    }
}
