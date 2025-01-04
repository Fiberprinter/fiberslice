use egui::{Color32, ImageButton, Visuals};
use egui_extras::Size;
use egui_grid::GridBuilder;
use native_dialog::FileDialog;
use strum::EnumCount;
use strum_macros::{EnumCount, EnumIter};

use crate::{
    config,
    ui::{icon::get_cad_tool_icon, visual::customize_look_and_feel},
    RootEvent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, EnumCount)]
pub enum CADTool {
    Import,
    AddCone,
    AddCube,
    AddCylinder,
    ObjectMode,
    MaskMode,
}

const CAD_TOOL_LABELS: [(&str, CADTool); CADTool::COUNT] = [
    ("Import CAD Model", CADTool::Import),
    ("Add Cone", CADTool::AddCone),
    ("Add Cube", CADTool::AddCube),
    ("Add Cylinder", CADTool::AddCylinder),
    ("Object Mode", CADTool::ObjectMode),
    ("Mask Mode", CADTool::MaskMode),
];

#[derive(Debug, Default)]
pub struct CADTools;

impl CADTools {
    pub fn show_left(
        &mut self,
        ui: &mut egui::Ui,
        shared_state: &(crate::ui::UiState, crate::GlobalState<crate::RootEvent>),
    ) {
        self.show(ui, shared_state, &[CADTool::Import]);
    }

    pub fn show_objects(
        &mut self,
        ui: &mut egui::Ui,
        shared_state: &(crate::ui::UiState, crate::GlobalState<crate::RootEvent>),
    ) {
        self.show(
            ui,
            shared_state,
            &[CADTool::AddCube, CADTool::AddCylinder, CADTool::AddCone],
        );
    }

    pub fn show_right(
        &mut self,
        ui: &mut egui::Ui,
        shared_state: &(crate::ui::UiState, crate::GlobalState<crate::RootEvent>),
    ) {
        self.show(ui, shared_state, &[CADTool::ObjectMode, CADTool::MaskMode]);
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        shared_state: &(crate::ui::UiState, crate::GlobalState<crate::RootEvent>),
        tools: &[CADTool],
    ) {
        let button = config::gui::CAD_TOOL_BUTTON;

        let mut builder = GridBuilder::new()
            .new_row(Size::remainder())
            .new_row_align(Size::exact(button.size.0), egui::Align::Center);

        builder = builder.cell(Size::remainder());

        for _ in 0..tools.len() {
            builder = builder.cell(Size::exact(button.size.0));
            builder = builder.cell(Size::remainder());
        }

        builder = builder.new_row(Size::remainder());

        *ui.visuals_mut() = Visuals::light();
        customize_look_and_feel(ui.visuals_mut());
        ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

        builder.show(ui, |mut grid| {
            grid.empty();
            for tool in tools {
                let (name, _) = CAD_TOOL_LABELS[*tool as usize];

                grid.cell(|ui| {
                    // let is_selected = self.selected == Some(tool);

                    let mut image_button = ImageButton::new(get_cad_tool_icon(*tool))
                        .rounding(5.0)
                        .frame(true);

                    let mode = *shared_state.0.mode.read();

                    match mode {
                        crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Objects) => {
                            if *tool == CADTool::ObjectMode {
                                image_button = image_button.selected(true);
                            }
                        }
                        crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Masks) => {
                            if *tool == CADTool::MaskMode {
                                image_button = image_button.selected(true);
                            }
                        }
                        _ => {}
                    }

                    let response = ui.add(image_button);

                    if response.clicked() {
                        match *tool {
                            CADTool::Import => match mode {
                                crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Objects) => {
                                    let path = FileDialog::new()
                                        .set_location("~")
                                        .add_filter("STL Files", &["stl"])
                                        .show_open_single_file()
                                        .unwrap();

                                    match path {
                                        Some(path) => {
                                            shared_state.1.viewer.load_object_from_file(path);
                                        }
                                        None => {
                                            println!("No file selected")
                                        }
                                    }
                                }
                                crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Masks) => {
                                    let path = FileDialog::new()
                                        .set_location("~")
                                        .add_filter("STL Files", &["stl"])
                                        .show_open_single_file()
                                        .unwrap();

                                    match path {
                                        Some(path) => {
                                            shared_state.1.viewer.load_mask_from_file(path);
                                        }
                                        None => {
                                            println!("No file selected")
                                        }
                                    }
                                }
                                _ => {}
                            },
                            CADTool::AddCone => match mode {
                                crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Objects) => {
                                    shared_state.1.viewer.load_object_from_bytes(
                                        "Cone",
                                        include_bytes!("../../../assets/unitCube.binary.stl"),
                                    );
                                }
                                crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Masks) => {
                                    shared_state.1.viewer.load_mask_from_bytes(
                                        "Cone",
                                        include_bytes!("../../../assets/unitCube.binary.stl"),
                                    );
                                }
                                _ => {}
                            },
                            CADTool::AddCube => match mode {
                                crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Objects) => {
                                    shared_state.1.viewer.load_object_from_bytes(
                                        "Cone",
                                        include_bytes!("../../../assets/unitCube.binary.stl"),
                                    );
                                }
                                crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Masks) => {
                                    shared_state.1.viewer.load_mask_from_bytes(
                                        "Cone",
                                        include_bytes!("../../../assets/unitCube.binary.stl"),
                                    );
                                }
                                _ => {}
                            },
                            CADTool::AddCylinder => match mode {
                                crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Objects) => {
                                    shared_state.1.viewer.load_object_from_bytes(
                                        "Cone",
                                        include_bytes!("../../../assets/unitCube.binary.stl"),
                                    );
                                }
                                crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Masks) => {
                                    shared_state.1.viewer.load_mask_from_bytes(
                                        "Cone",
                                        include_bytes!("../../../assets/unitCube.binary.stl"),
                                    );
                                }
                                _ => {}
                            },
                            CADTool::ObjectMode => {
                                *shared_state.0.mode.write() =
                                    crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Objects);

                                shared_state
                                    .1
                                    .proxy
                                    .send_event(RootEvent::SetMode(crate::ui::Mode::Prepare(
                                        crate::prelude::PrepareMode::Objects,
                                    )))
                                    .expect("Failed to send event");
                            }
                            CADTool::MaskMode => {
                                *shared_state.0.mode.write() =
                                    crate::ui::Mode::Prepare(crate::prelude::PrepareMode::Masks);

                                shared_state
                                    .1
                                    .proxy
                                    .send_event(RootEvent::SetMode(crate::ui::Mode::Prepare(
                                        crate::prelude::PrepareMode::Masks,
                                    )))
                                    .expect("Failed to send event");
                            }
                        }
                    } else if response.hovered() {
                        egui::popup::show_tooltip(
                            ui.ctx(),
                            ui.layer_id(),
                            egui::Id::new(format!("popup-{}", name)),
                            |ui| {
                                ui.label(name.to_string());
                            },
                        );
                    }
                });
                grid.empty();
            }
        });
    }
}
