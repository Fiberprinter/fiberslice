/*
    Copyright (c) 2023 Elias Gottsbacher, Jan Traussnigg, Nico Huetter (HTBLA Kaindorf)
    All rights reserved.
    Note: The complete copyright description for this software thesis can be found at the beginning of each file.
    Please refer to the terms and conditions stated therein.
*/

use egui::*;
use egui_xml::load_layout;
use gizmo::GizmoTools;
use orientation::OrientationAddon;

use crate::config::gui::shaded_color;
use crate::prelude::{Mode, PrepareMode};
use crate::ui::boundary::Boundary;
use crate::ui::{ui_temp_mut, AllocateInnerUiRect, UiState};
use crate::ui::{UiComponentState, UiInnerComponent};
use crate::{GlobalState, RootEvent};

pub mod cad_tools;
pub mod gizmo;

pub mod orientation {
    use egui::{Color32, ImageButton, Visuals, Widget};
    use egui_extras::Size;
    use egui_grid::GridBuilder;
    use strum::{EnumCount, IntoEnumIterator};

    use crate::{
        config::{self, gui::shaded_color},
        ui::{icon::get_orientation_asset, visual::customize_look_and_feel, UiState},
        viewer::CameraEvent,
        GlobalState, RootEvent,
    };

    use crate::viewer::Orientation;

    pub struct OrientationAddon<'a> {
        shared_state: &'a (UiState, GlobalState<RootEvent>),
    }

    impl Widget for OrientationAddon<'_> {
        fn ui(self, ui: &mut egui::Ui) -> egui::Response {
            let (_ui_state, global_state) = self.shared_state;

            let layout = egui::Layout {
                main_dir: egui::Direction::RightToLeft,
                main_wrap: true,
                main_align: egui::Align::Center,
                main_justify: false,
                cross_align: egui::Align::Center,
                cross_justify: true,
            };

            let shaded_color = shaded_color(ui.visuals().dark_mode);

            ui.painter()
                .rect_filled(ui.available_rect_before_wrap(), 5.0, shaded_color);

            //skip first because first is Orientation::Default we don't want that
            let builder = (1..Orientation::COUNT).fold(
                GridBuilder::new()
                    .new_row_align(Size::remainder(), egui::Align::Center)
                    .layout_standard(layout)
                    .clip(true)
                    .cell(Size::remainder()),
                |builder, _| builder.cell(Size::initial(40.0)),
            );

            *ui.visuals_mut() = Visuals::light();
            customize_look_and_feel(ui.visuals_mut());
            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

            let response = builder.cell(Size::remainder()).show(ui, |mut grid| {
                grid.empty();

                //skip first because first is Orientation::Default we don't want that
                Orientation::iter().skip(1).for_each(|orientation| {
                    grid.cell(|ui| {
                        let button = config::gui::ORIENATION_BUTTON;

                        let icon = get_orientation_asset(orientation);

                        let image_button = ImageButton::new(icon).rounding(5.0).frame(true);

                        ui.allocate_ui(
                            [button.size.0 + button.border, button.size.1 + button.border].into(),
                            |ui| {
                                let response =
                                    ui.add_sized([button.size.0, button.size.1], image_button);

                                if response.clicked() {
                                    global_state
                                        .camera_event_writer
                                        .send(CameraEvent::CameraOrientationChanged(orientation));
                                }
                            },
                        );
                    });
                });
                grid.empty();
            });

            response
        }
    }

    impl<'a> OrientationAddon<'a> {
        pub fn new(shared_state: &'a (UiState, GlobalState<RootEvent>)) -> Self {
            Self { shared_state }
        }
    }
}

pub struct AddonsState {
    gizmo_tools: gizmo::GizmoTools,
    cad_tools: cad_tools::CADTools,
    enabled: bool,
}

impl AddonsState {
    pub fn new() -> Self {
        Self {
            gizmo_tools: GizmoTools::default(),
            cad_tools: cad_tools::CADTools,
            enabled: true,
        }
    }
}

impl UiComponentState for AddonsState {
    fn get_boundary(&self) -> Boundary {
        Boundary::zero()
    }

    fn get_enabled(&mut self) -> &mut bool {
        &mut self.enabled
    }

    fn get_name(&self) -> &str {
        "Addons"
    }
}

pub struct Addons<'a> {
    state: &'a mut AddonsState,
}

impl<'a> Addons<'a> {
    pub fn with_state(state: &'a mut AddonsState) -> Self {
        Self { state }
    }

    fn show_orientation(&mut self, ui: &mut Ui, shared_state: &(UiState, GlobalState<RootEvent>)) {
        ui.add(OrientationAddon::new(shared_state));
    }

    fn show_bottom_addon(&mut self, ui: &mut Ui, shared_state: &(UiState, GlobalState<RootEvent>)) {
        let shaded_color = shaded_color(ui.visuals().dark_mode);

        let mode = *shared_state.0.mode.read();

        match mode {
            Mode::Preview => {}
            Mode::Prepare(_) => {
                load_layout!(
                    <Strip direction="west">
                        <Panel size="remainder"></Panel>
                        <Panel size="exact" value="70">
                            ui.painter()
                                .rect_filled(ui.available_rect_before_wrap(), 5.0, shaded_color);

                            self.state.cad_tools.show_left(ui, shared_state);
                        </Panel>
                        <Panel size="exact" value="20"></Panel>
                        <Panel size="exact" value="210">
                            ui.painter()
                                .rect_filled(ui.available_rect_before_wrap(), 5.0, shaded_color);

                            self.state.cad_tools.show_objects(ui, shared_state);
                        </Panel>
                        <Panel size="exact" value="20"></Panel>
                        <Panel size="exact" value="140">
                            ui.painter()
                                .rect_filled(ui.available_rect_before_wrap(), 5.0, shaded_color);

                            self.state.cad_tools.show_right(ui, shared_state);
                        </Panel>
                        <Panel size="remainder"></Panel>
                    </Strip>
                );
            }
        }
    }

    fn show_right_addon(
        &mut self,
        ui: &mut Ui,
        (ui_state, global_state): &(UiState, GlobalState<RootEvent>),
    ) {
        ui_state.mode.read_with_fn(|mode| match mode {
            Mode::Preview => {
                ui.allocate_ui_in_rect(
                    Rect::from_two_pos(
                        Pos2::new(0.0, ui.available_height() * 0.25),
                        Pos2::new(ui.available_width(), ui.available_height() * 0.75),
                    ),
                    |ui| {
                        ui_state.layer_max.write_with_fn(|layer_max| {
                            ui_temp_mut(
                                ui,
                                ui.available_height(),
                                |ui| &mut ui.spacing_mut().slider_width,
                                |ui| {
                                    if let Some(max) = global_state.viewer.sliced_max_layer() {
                                        let slider = egui::Slider::new(layer_max, 0..=max)
                                            .orientation(egui::SliderOrientation::Vertical);

                                        let response = ui.add_sized(ui.available_size(), slider);

                                        if response.changed() {
                                            global_state.viewer.update_gpu_max_layer(*layer_max);

                                            global_state.viewer.sliced_gcode(|sliced_gcode| {
                                                if let Some(index) = sliced_gcode
                                                    .navigator
                                                    .get_layer_change_index(*layer_max as usize)
                                                {
                                                    global_state.ui_event_writer.send(
                                                        crate::ui::UiEvent::GCodeReaderLookAt(
                                                            index,
                                                        ),
                                                    );
                                                }
                                            });
                                        }
                                    }
                                },
                            );
                        });
                    },
                );
            }
            Mode::Prepare(_) => {}
        });
    }

    fn show_left_addon(&mut self, ui: &mut Ui, shared_state: &(UiState, GlobalState<RootEvent>)) {
        let shaded_color = shaded_color(ui.visuals().dark_mode);

        shared_state.0.mode.read_with_fn(|mode| match mode {
            Mode::Preview => {}
            Mode::Prepare(PrepareMode::Objects) => {
                ui.allocate_ui_in_rect(
                    Rect::from_two_pos(
                        Pos2::new(0.0, ui.available_height() * 0.25),
                        Pos2::new(ui.available_width(), ui.available_height() * 0.75),
                    ),
                    |ui| {
                        ui.painter().rect_filled(
                            ui.available_rect_before_wrap(),
                            5.0,
                            shaded_color,
                        );

                        self.state.gizmo_tools.show_icons(ui, shared_state);
                    },
                );
            }
            Mode::Prepare(PrepareMode::Masks) => {
                ui.allocate_ui_in_rect(
                    Rect::from_two_pos(
                        Pos2::new(0.0, ui.available_height() * 0.25),
                        Pos2::new(ui.available_width(), ui.available_height() * 0.75),
                    ),
                    |ui| {
                        ui.painter().rect_filled(
                            ui.available_rect_before_wrap(),
                            5.0,
                            shaded_color,
                        );

                        self.state.gizmo_tools.show_icons(ui, shared_state);
                    },
                );
            }
        });
    }
}

impl<'a> UiInnerComponent for Addons<'a> {
    fn show(&mut self, ui: &mut Ui, shared_state: &(UiState, GlobalState<RootEvent>)) {
        if shared_state.1.viewer.gizmo_enabled() {
            self.state.gizmo_tools.show_tool_wíndow(ui, shared_state);
        }

        if self.state.enabled {
            let available_size = ui.available_size();

            load_layout!(
                <Strip direction="north">
                    <Panel size="exact" value="50">
                        <Strip direction="west">
                            <Panel size="remainder"></Panel>
                            <Panel size="exact" value="240">
                                if available_size.x >= 240.0 {
                                    self.show_orientation(ui, shared_state);
                                }
                            </Panel>
                        </Strip>
                    </Panel>
                    <Panel size="remainder">
                        <Strip direction="west">
                            <Panel size="exact" value="60">
                                <Strip direction="north">
                                    <Panel size="remainder"></Panel>
                                    <Panel size="exact" value="500">
                                        if available_size.y >= 500.0 && available_size.x >= 60.0 {
                                            self.show_left_addon(ui, shared_state);
                                        }
                                    </Panel>
                                    <Panel size="remainder"></Panel>
                                </Strip>
                            </Panel>
                            <Panel size="remainder"></Panel>
                            <Panel size="exact" value="50">
                                if available_size.x >= 50.0 {
                                    self.show_right_addon(ui, shared_state);
                                }
                            </Panel>
                        </Strip>
                    </Panel>
                    <Panel size="exact" value="60">
                        if available_size.y >= 60.0 {
                            self.show_bottom_addon(ui, shared_state);
                        }
                    </Panel>
                </Strip>
            );
        }
    }
}
