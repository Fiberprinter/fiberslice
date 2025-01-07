/*
    Copyright (c) 2023 Elias Gottsbacher, Jan Traussnigg, Nico Huetter (HTBLA Kaindorf)
    All rights reserved.
    Note: The complete copyright description for this software thesis can be found at the beginning of each file.
    Please refer to the terms and conditions stated therein.
*/

use glam::vec3;
use input::InputEvent;
use log::{info, LevelFilter};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::{sync::Arc, time::Instant};
use ui::UiEvent;
use viewer::{tracker::ProcessTracker, CameraEvent};

use prelude::{
    Adapter, EventWriter, FrameHandle, GlobalContext, Shared, SharedMut, Viewport, WgpuContext,
};

mod api;
mod config;
mod error;
mod geometry;
mod input;
mod prelude;
mod render;
mod slicer;
mod tools;
mod ui;
mod viewer;
mod window;

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event_loop::{EventLoop, EventLoopProxy},
};

pub static DEVICE: RwLock<Option<Arc<wgpu::Device>>> = RwLock::new(None);
pub static QUEUE: RwLock<Option<Arc<wgpu::Queue>>> = RwLock::new(None);
pub static CONFIG: OnceCell<config::Config> = OnceCell::new();

fn load_config() {
    let content = include_str!("../config.toml");
    match toml::from_str(content) {
        Ok(config) => {
            CONFIG.set(config).unwrap();
        }
        Err(e) => {
            panic!("Failed to load config: {}", e);
        }
    }
}

// HACK with this using Model is way easier than before you don't have to worry about the device and queue
fn set_device(device: Arc<wgpu::Device>) {
    *DEVICE.write() = Some(device);
}
// HACK with this using Model is way easier than before you don't have to worry about the device and queue
fn set_queue(queue: Arc<wgpu::Queue>) {
    *QUEUE.write() = Some(queue);
}

#[derive(Debug, Clone)]
pub enum RootEvent {
    SetMode(prelude::Mode),
    Exit,
}

pub static GLOBAL_STATE: RwLock<Option<GlobalState<RootEvent>>> = RwLock::new(None);

#[derive(Debug, Clone)]
pub struct GlobalState<T: 'static> {
    pub proxy: EventLoopProxy<T>,

    pub window: Arc<winit::window::Window>,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,

    pub picking_state: input::InputState,
    pub picking_event_writer: EventWriter<InputEvent>,

    pub ui_state: ui::UiState,
    pub ui_event_writer: EventWriter<UiEvent>,

    pub camera_event_writer: EventWriter<CameraEvent>,

    pub viewer: Shared<viewer::Viewer>,
    pub slicer: SharedMut<slicer::Slicer>,

    pub camera_controller: SharedMut<viewer::camera_controller::CameraController>,
    pub viewport: SharedMut<Viewport>,

    pub progress_tracker: SharedMut<ProcessTracker>,

    pub ctx: GlobalContext,
}

#[tokio::main]
async fn main() -> Result<(), EventLoopError> {
    load_config();

    let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
    let _puffin_server = puffin_http::Server::new(&server_addr).unwrap();
    info!("Run this to view profiling data:  puffin_viewer {server_addr}");

    #[cfg(debug_assertions)]
    puffin::set_scopes_on(true);

    #[cfg(debug_assertions)]
    simple_logging::log_to_file("app.log", LevelFilter::Info).unwrap();

    const VERSION: &str = env!("CARGO_PKG_VERSION");
    info!("Starting up version {}", VERSION);

    let event_loop: EventLoop<RootEvent> = EventLoop::with_user_event().build().unwrap();

    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let mut application = Application {
        proxy: event_loop.create_proxy(),
        state: None,
    };

    event_loop.run_app(&mut application)
}

struct ApplicationState {
    window: Arc<winit::window::Window>,
    wgpu_context: WgpuContext,

    global_state: GlobalState<RootEvent>,

    ui_adapter: ui::UiAdapter,
    camera_adapter: viewer::CameraAdapter,
    render_adapter: render::RenderAdapter,
    picking_adapter: input::InputAdapter,

    start_time: Instant,
}

impl ApplicationState {
    fn update(&mut self) {
        self.global_state.viewer.update(&self.global_state);
        self.global_state.progress_tracker.write().update();

        self.ui_adapter.update(self.start_time);

        self.camera_adapter.update(self.start_time);

        self.render_adapter.update(self.start_time);

        self.picking_adapter.update(self.start_time);

        self.ui_adapter
            .handle_events(&self.wgpu_context, &self.global_state);

        self.camera_adapter
            .handle_events(&self.wgpu_context, &self.global_state);

        self.render_adapter
            .handle_events(&self.wgpu_context, &self.global_state);

        self.picking_adapter
            .handle_events(&self.wgpu_context, &self.global_state);
    }

    fn handle_frame(&mut self) {
        let (ui_output, viewport) = self
            .ui_adapter
            .handle_frame(&self.wgpu_context, self.global_state.clone(), ())
            .expect("Failed to handle frame");

        let camera_result = self
            .camera_adapter
            .handle_frame(&self.wgpu_context, self.global_state.clone(), viewport)
            .expect("Failed to handle frame");

        self.render_adapter
            .handle_frame(
                &self.wgpu_context,
                self.global_state.clone(),
                (Some(ui_output), &camera_result),
            )
            .expect("Failed to handle frame");

        self.picking_adapter
            .handle_frame(
                &self.wgpu_context,
                self.global_state.clone(),
                &camera_result,
            )
            .expect("Failed to handle frame");
    }

    fn handle_window_event(
        &mut self,
        event: winit::event::WindowEvent,
        window_id: winit::window::WindowId,
    ) {
        self.ui_adapter.handle_window_event(
            &event,
            window_id,
            &self.wgpu_context,
            self.global_state.clone(),
        );

        self.camera_adapter.handle_window_event(
            &event,
            window_id,
            &self.wgpu_context,
            self.global_state.clone(),
        );

        self.render_adapter.handle_window_event(
            &event,
            window_id,
            &self.wgpu_context,
            self.global_state.clone(),
        );

        self.picking_adapter.handle_window_event(
            &event,
            window_id,
            &self.wgpu_context,
            self.global_state.clone(),
        );
    }

    fn handle_device_event(
        &mut self,
        event: winit::event::DeviceEvent,
        device_id: winit::event::DeviceId,
    ) {
        self.ui_adapter.handle_device_event(
            &event,
            device_id,
            &self.wgpu_context,
            self.global_state.clone(),
        );

        self.camera_adapter.handle_device_event(
            &event,
            device_id,
            &self.wgpu_context,
            self.global_state.clone(),
        );

        self.render_adapter.handle_device_event(
            &event,
            device_id,
            &self.wgpu_context,
            self.global_state.clone(),
        );

        self.picking_adapter.handle_device_event(
            &event,
            device_id,
            &self.wgpu_context,
            self.global_state.clone(),
        );
    }
}

struct Application {
    proxy: EventLoopProxy<RootEvent>,
    state: Option<ApplicationState>,
}

impl ApplicationHandler<RootEvent> for Application {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(window::create_window(event_loop).expect("Failed to create window"));

        let wgpu_context = WgpuContext::new(window.clone()).unwrap();

        set_device(wgpu_context.device.clone());
        set_queue(wgpu_context.queue.clone());

        let (_, _, render_adapter) = render::RenderAdapter::create(&wgpu_context);

        let (_, camera_event_writer, mut camera_adapter) =
            viewer::CameraAdapter::create(&wgpu_context);
        camera_event_writer.send(CameraEvent::CameraOrientationChanged(
            viewer::Orientation::Default,
        ));

        let (picking_state, picking_event_writer, picking_adapter) =
            input::InputAdapter::create(&wgpu_context);

        let (ui_state, ui_event_writer, ui_adapter) = ui::UiAdapter::create(&wgpu_context);

        let global_state = GlobalState {
            proxy: self.proxy.clone(),

            window: window.clone(),
            device: wgpu_context.device.clone(),
            queue: wgpu_context.queue.clone(),

            picking_state,
            picking_event_writer,

            ui_state,
            ui_event_writer,

            camera_event_writer,

            viewer: Shared::new(viewer::Viewer::instance(&wgpu_context)),

            slicer: SharedMut::from_inner(slicer::Slicer::default()),

            camera_controller: SharedMut::from_inner(
                viewer::camera_controller::CameraController::default(),
            ),
            viewport: SharedMut::from_inner(Viewport::default()),

            progress_tracker: SharedMut::from_inner(ProcessTracker::new()),

            ctx: GlobalContext::default(),
        };

        *GLOBAL_STATE.write() = Some(global_state.clone());

        window.set_visible(true);

        {
            global_state.viewer.set_mode(prelude::Mode::default());
            let slicer_read = global_state.slicer.read();
            let settings = &slicer_read.settings;

            camera_adapter.init_target(vec3(
                settings.print_x / 2.0,
                settings.print_z * 0.25,
                settings.print_y / 2.0,
            ));

            global_state.viewer.update_printer_dimension(
                settings.print_x,
                settings.print_y,
                settings.print_z,
            );
        }

        self.state = Some(ApplicationState {
            window,
            wgpu_context,
            global_state,

            ui_adapter,
            camera_adapter,
            render_adapter,
            picking_adapter,

            start_time: Instant::now(),
        });
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(state) = self.state.as_mut() {
            match event {
                winit::event::WindowEvent::RedrawRequested => {
                    state.global_state.ctx.begin_frame();
                    puffin::GlobalProfiler::lock().new_frame();

                    state.handle_frame();

                    state.global_state.ctx.end_frame();
                }
                winit::event::WindowEvent::Resized(size) => {
                    resize_surface(&mut state.wgpu_context, size);

                    state.window.request_redraw();
                }
                winit::event::WindowEvent::ScaleFactorChanged { .. } => {
                    let size = state.wgpu_context.window.inner_size();

                    resize_surface(&mut state.wgpu_context, size);

                    state.window.request_redraw();
                }
                winit::event::WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                winit::event::WindowEvent::CursorMoved { position, .. } => {
                    state.global_state.ctx.mouse_position =
                        Some((position.x as f32, position.y as f32));
                    state.window.request_redraw();
                }
                _ => {
                    state.window.request_redraw();
                }
            }

            state.handle_window_event(event.clone(), window_id);
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let Some(state) = self.state.as_mut() {
            state.handle_device_event(event, device_id);
        }
    }

    fn user_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, event: RootEvent) {
        match event {
            RootEvent::SetMode(mode) => {
                if let Some(state) = self.state.as_mut() {
                    state.global_state.viewer.set_mode(mode);
                }
            }
            RootEvent::Exit => {
                event_loop.exit();
            }
        }
    }

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _cause: winit::event::StartCause,
    ) {
        if let Some(state) = self.state.as_mut() {
            state.update();
        }
    }

    fn exiting(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(state) = self.state.as_mut() {
            state.global_state.slicer.write().exit();
        }

        println!("Exiting");
    }
}

fn resize_surface(wgpu_context: &mut WgpuContext, size: winit::dpi::PhysicalSize<u32>) {
    if size.width > 0 && size.height > 0 {
        wgpu_context.surface_config.width = size.width;
        wgpu_context.surface_config.height = size.height;
        wgpu_context
            .surface
            .configure(&wgpu_context.device, &wgpu_context.surface_config);
    }
}
