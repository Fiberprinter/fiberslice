/*
    Copyright (c) 2023 Elias Gottsbacher, Jan Traussnigg, Nico Huetter (HTBLA Kaindorf)
    All rights reserved.
    Note: The complete copyright description for this software thesis can be found at the beginning of each file.
    Please refer to the terms and conditions stated therein.
*/

pub mod api;
pub mod boundary;
pub mod components;
pub mod custom_toasts;
pub mod screen;
pub mod settings;
pub mod tools;
pub mod widgets;

mod icon;
pub mod visual;

use std::sync::atomic::AtomicBool;

use egui_toast::ToastOptions;
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::{Platform, PlatformDescriptor};
use screen::Screen;

use egui::{FontDefinitions, InnerResponse, Pos2, Rect, Ui, Visuals};
use visual::customize_look_and_feel;
use winit::event::WindowEvent;

use crate::{
    prelude::{
        create_event_bundle, Adapter, AdapterCreation, Error, EventReader, FrameHandle, Mode,
        Shared, SharedMut, Viewport, WgpuContext,
    },
    GlobalState, RootEvent,
};

use boundary::Boundary;

#[derive(Debug, Clone)]
pub enum UiEvent {
    ShowInfo(String),
    ShowSuccess(String),
    ShowError(String),
    ShowProgressBar(u32, String),
    GCodeReaderLookAt(usize),
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub pointer_in_use: Shared<AtomicBool>,
    pub theme: SharedMut<Theme>,
    pub mode: SharedMut<Mode>,

    pub layer_max: SharedMut<u32>,
    pub time_stamp: SharedMut<u32>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            pointer_in_use: Shared::new(AtomicBool::new(false)),
            theme: SharedMut::from_inner(Theme::Dark),
            mode: SharedMut::from_inner(Mode::default()),

            layer_max: SharedMut::from_inner(u32::MAX),
            time_stamp: SharedMut::from_inner(u32::MAX),
        }
    }
}

impl UiState {
    pub fn toggle_theme(&self) {
        self.theme.write_with_fn(|theme| {
            let current_theme = theme.clone();

            *theme = match current_theme {
                Theme::Light => Theme::Dark,
                Theme::Dark => Theme::Light,
            };
        });
    }
}

pub fn ui_temp_mut<T>(
    ui: &mut Ui,
    temp_value: T,
    get_dest: impl Fn(&mut Ui) -> &mut T,
    add_contents: impl FnOnce(&mut Ui),
) {
    let old_value = std::mem::replace(get_dest(ui), temp_value);
    add_contents(ui);
    let _ = std::mem::replace(get_dest(ui), old_value);
}

pub struct UiUpdateOutput {
    pub paint_jobs: Vec<egui::ClippedPrimitive>,
    pub tdelta: egui::TexturesDelta,
    pub screen_descriptor: ScreenDescriptor,
}

pub struct UiAdapter {
    state: UiState,
    screen: Screen,
    platform: Platform,

    event_reader: EventReader<UiEvent>,
}

impl<'a> FrameHandle<'a, RootEvent, (UiUpdateOutput, (f32, f32, f32, f32)), ()> for UiAdapter {
    fn update(&mut self, start_time: std::time::Instant) {
        self.platform
            .update_time(start_time.elapsed().as_secs_f64());
    }

    fn handle_frame(
        &'a mut self,
        wgpu_context: &WgpuContext,
        global_state: GlobalState<RootEvent>,
        _ctx: (),
    ) -> Result<(UiUpdateOutput, Viewport), Error> {
        puffin::profile_function!();

        let (x, y, width, height) = self.screen.construct_viewport(wgpu_context);

        let is_pointer_over_viewport = {
            if let Some(pos) = self.platform.context().pointer_latest_pos() {
                pos.x >= x && pos.x <= x + width && pos.y >= y && pos.y <= y + height
            } else {
                false
            }
        };

        self.state.pointer_in_use.store(
            !is_pointer_over_viewport || self.platform.context().is_using_pointer(),
            std::sync::atomic::Ordering::Relaxed,
        );

        self.platform.begin_frame();

        self.platform.context().style_mut(|style| {
            // catppuccin_egui::set_style_theme(style, catppuccin_egui::MOCHA);
            // style.visuals = Visuals::light();

            match &*self.state.theme.read() {
                Theme::Light => {
                    style.visuals = Visuals::light();
                }
                Theme::Dark => {
                    style.visuals = Visuals::dark();
                }
            }

            style.visuals.popup_shadow = egui::epaint::Shadow::NONE;
            style.visuals.window_shadow = egui::epaint::Shadow::NONE;

            customize_look_and_feel(&mut style.visuals);
        });

        self.screen.show(
            &self.platform.context(),
            &(self.state.clone(), global_state),
        );

        let full_output = self.platform.end_frame(Some(&wgpu_context.window));

        let viewport = self.screen.construct_viewport(wgpu_context);

        let paint_jobs = self
            .platform
            .context()
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let tdelta: egui::TexturesDelta = full_output.textures_delta;

        let screen_descriptor = ScreenDescriptor {
            physical_width: wgpu_context.surface_config.width,
            physical_height: wgpu_context.surface_config.height,
            scale_factor: wgpu_context.window.scale_factor() as f32,
        };

        if self.platform.context().has_requested_repaint() {
            wgpu_context.window.request_redraw();
        }

        Ok((
            UiUpdateOutput {
                paint_jobs,
                tdelta,
                screen_descriptor,
            },
            viewport,
        ))
    }

    fn handle_window_event(
        &mut self,
        event: &WindowEvent,
        _id: winit::window::WindowId,
        _wgpu_context: &WgpuContext,
        _state: GlobalState<RootEvent>,
    ) {
        self.platform.handle_event(event);
    }
}

impl<'a> Adapter<'a, RootEvent, UiState, (UiUpdateOutput, (f32, f32, f32, f32)), (), UiEvent>
    for UiAdapter
{
    fn create(context: &WgpuContext) -> AdapterCreation<UiState, UiEvent, Self> {
        let platform = Platform::new(PlatformDescriptor {
            physical_width: context.window.inner_size().width,
            physical_height: context.window.inner_size().height,
            scale_factor: context.window.scale_factor(),
            font_definitions: FontDefinitions::default(),
            style: Default::default(),
        });

        egui_extras::install_image_loaders(&platform.context());
        // crate::ui::font::setup_symbols(&platform.context());

        let state = UiState::default();
        let screen = Screen::new();

        // We use the egui_wgpu_backend crate as the render backend.
        let (reader, writer) = create_event_bundle::<UiEvent>();

        (
            state.clone(),
            writer,
            Self {
                state,
                screen,
                platform,
                event_reader: reader,
            },
        )
    }

    fn get_adapter_description(&self) -> String {
        "UiAdapter".to_string()
    }

    fn get_reader(&self) -> crate::prelude::EventReader<UiEvent> {
        self.event_reader.clone()
    }

    fn handle_event(
        &mut self,
        wgpu_context: &WgpuContext,
        _global_state: &GlobalState<RootEvent>,
        event: UiEvent,
    ) {
        match event {
            UiEvent::ShowInfo(message) => {
                self.screen.add_toast(
                    egui_toast::Toast::with_name("Info".into())
                        .kind(egui_toast::ToastKind::Info)
                        .text(message)
                        .options(
                            ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_progress(true),
                        ),
                );

                wgpu_context.window.request_redraw();
            }
            UiEvent::ShowSuccess(message) => {
                self.screen.add_toast(
                    egui_toast::Toast::with_name("Success".into())
                        .kind(egui_toast::ToastKind::Success)
                        .text(message)
                        .options(
                            ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_progress(true),
                        ),
                );

                wgpu_context.window.request_redraw();
            }
            UiEvent::ShowError(message) => {
                self.screen.add_toast(
                    egui_toast::Toast::with_name("Error".into())
                        .kind(egui_toast::ToastKind::Error)
                        .text(message)
                        .options(
                            ToastOptions::default()
                                .duration_in_seconds(5.0)
                                .show_progress(true),
                        ),
                );

                wgpu_context.window.request_redraw();
            }
            UiEvent::ShowProgressBar(id, name) => {
                self.screen.add_progress_bar_toast(
                    egui_toast::Toast::with_name(name.clone())
                        .kind(egui_toast::ToastKind::Custom(id))
                        .text(name)
                        .options(ToastOptions::default().show_progress(true)),
                );

                wgpu_context.window.request_redraw();
            }
            UiEvent::GCodeReaderLookAt(line) => {
                self.screen.tools_mut().gcode_tool.look_at(line);
            }
        }
    }
}

impl From<&Theme> for Visuals {
    fn from(theme: &Theme) -> Self {
        match *theme {
            Theme::Light => Visuals::light(),
            Theme::Dark => Visuals::dark(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Theme {
    Light,
    Dark,
}

pub trait UiComponentState {
    fn get_boundary(&self) -> Boundary;

    fn get_enabled(&mut self) -> &mut bool;

    fn get_name(&self) -> &str {
        "Component"
    }
}

pub trait UiComponent {
    fn show(&mut self, ctx: &egui::Context, shared_state: &(UiState, GlobalState<RootEvent>));
}

pub trait UiInnerComponent {
    fn show(&mut self, ui: &mut egui::Ui, shared_state: &(UiState, GlobalState<RootEvent>));
}

pub trait UiWidgetComponent {
    fn show(&mut self, ui: &mut egui::Ui);
}

pub trait AllocateInnerUiRect {
    fn allocate_ui_in_rect<R>(
        &mut self,
        inner: Rect,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> InnerResponse<R>;
}

impl AllocateInnerUiRect for egui::Ui {
    fn allocate_ui_in_rect<R>(
        &mut self,
        inner: Rect,
        add_contents: impl FnOnce(&mut Self) -> R,
    ) -> InnerResponse<R> {
        let rect = self.available_rect_before_wrap();

        self.allocate_ui_at_rect(
            Rect::from_two_pos(
                Pos2::new(inner.left() + rect.left(), inner.top() + rect.top()),
                Pos2::new(
                    inner.left() + rect.left() + inner.width(),
                    inner.top() + rect.top() + inner.height(),
                ),
            ),
            add_contents,
        )
    }
}
