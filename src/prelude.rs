use std::{fmt::Debug, sync::Arc};

use log::info;
use wgpu::InstanceDescriptor;
use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    window::Window,
};

pub use crate::error::Error;
use crate::{GlobalState, RootEvent};

pub mod shared;
pub mod wrap;

pub use shared::*;
pub use wrap::*;

pub type Viewport = (f32, f32, f32, f32);

#[derive(Debug, Clone)]
pub struct GlobalContext {
    pub frame_time: f32,
    pub mouse_position: Option<(f32, f32)>,
    last_frame_time: std::time::Instant,
}

impl Default for GlobalContext {
    fn default() -> Self {
        Self {
            frame_time: 0.0,
            last_frame_time: std::time::Instant::now(),
            mouse_position: None,
        }
    }
}

impl GlobalContext {
    pub fn begin_frame(&mut self) {
        self.last_frame_time = std::time::Instant::now();
    }

    pub fn end_frame(&mut self) {
        self.frame_time = self.last_frame_time.elapsed().as_secs_f32();

        println!("Frame time: {}", self.frame_time);
    }
}

pub struct WgpuContext {
    pub window: Arc<Window>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,

    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
}

impl WgpuContext {
    pub fn new(window: Arc<Window>) -> Result<Self, Error> {
        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();

        // WGPU 0.11+ support force fallback (if HW implementation not supported), set it to true or false (optional).
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        println!("Adapter: {:?}", adapter.get_info());

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits {
                    max_buffer_size: u32::MAX as u64,
                    ..Default::default()
                },
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();

        let size = window.inner_size();
        let surface_format = surface.get_capabilities(&adapter).formats[0];
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            desired_maximum_frame_latency: 1,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![surface_format],
        };
        surface.configure(&device, &surface_config);

        Ok(Self {
            window,
            device,
            queue,
            adapter,
            surface,
            surface_config,
            surface_format,
        })
    }
}

pub trait FrameHandle<'a, E, T, C> {
    fn update(&mut self, start_time: std::time::Instant) {}

    fn handle_frame(
        &'a mut self,
        wgpu_context: &WgpuContext,
        state: GlobalState<RootEvent>,
        ctx: C,
    ) -> Result<T, Error>;

    fn handle_window_event(
        &mut self,
        event: &WindowEvent,
        id: winit::window::WindowId,
        wgpu_context: &WgpuContext,
        state: GlobalState<RootEvent>,
    ) {
    }

    fn handle_device_event(
        &mut self,
        event: &DeviceEvent,
        id: winit::event::DeviceId,
        wgpu_context: &WgpuContext,
        state: GlobalState<RootEvent>,
    ) {
    }
}

pub type AdapterCreation<S, E, A> = (S, EventWriter<E>, A);

pub trait Adapter<'a, WinitE, S: Sized, T, C, E: Debug>: FrameHandle<'a, WinitE, T, C> {
    fn create(wgpu_context: &WgpuContext) -> AdapterCreation<S, E, Self>;

    #[allow(dead_code)]
    fn get_adapter_description(&self) -> String;

    fn get_reader(&self) -> EventReader<E>;

    fn handle_event(
        &mut self,
        wgpu_context: &WgpuContext,
        global_state: &GlobalState<RootEvent>,
        event: E,
    ) {
    }

    fn handle_events(&mut self, wgpu_context: &WgpuContext, global_state: &GlobalState<RootEvent>) {
        if self.get_reader().has_active_events() {
            self.get_reader().read(|events| {
                for event in events {
                    info!("Handling event: {:?}", event);
                    info!("Adapter: {:?}", self.get_adapter_description());

                    self.handle_event(wgpu_context, global_state, event);
                }
            });
        }
    }
}

use strum_macros::{EnumCount, EnumIter};

#[derive(Debug, Clone, Copy, EnumCount, EnumIter)]
pub enum TransformationMode {
    Translate,
    Rotate,
    Scale,
    PlaceOnFace,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Mode {
    Preview,
    Prepare,
    ForceAnalytics,
}

pub use event::{create_event_bundle, EventReader, EventWriter};

mod event {
    use std::fmt::Debug;

    use crate::prelude::SharedMut;

    pub struct EventReader<E: Debug> {
        events: SharedMut<Vec<E>>,
    }

    impl<E: Debug> Clone for EventReader<E> {
        fn clone(&self) -> Self {
            Self {
                events: self.events.clone(),
            }
        }
    }

    impl<E: Debug> EventReader<E> {
        pub fn read<F: FnMut(std::vec::Drain<'_, E>)>(&self, mut f: F) {
            f(self.events.write().drain(..))
        }

        pub fn has_active_events(&self) -> bool {
            !self.events.read().is_empty()
        }
    }

    #[derive(Debug)]
    pub struct EventWriter<E: Debug> {
        events: SharedMut<Vec<E>>,
    }

    impl<E: Debug> Clone for EventWriter<E> {
        fn clone(&self) -> Self {
            Self {
                events: self.events.clone(),
            }
        }
    }

    impl<E: Debug> EventWriter<E> {
        pub fn send(&self, event: E) {
            self.events.write().push(event);
        }
    }

    pub fn create_event_bundle<T: Debug>() -> (EventReader<T>, EventWriter<T>) {
        let events = SharedMut::from_inner(Vec::new());
        let reader = EventReader {
            events: events.clone(),
        };
        let writer = EventWriter { events };
        (reader, writer)
    }
}
