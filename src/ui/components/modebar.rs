use egui_extras::Size;
use egui_grid::GridBuilder;

use crate::prelude::Mode;
use crate::ui::boundary::Boundary;
use crate::ui::{UiComponent, UiComponentState, UiState};
use crate::{config, GlobalState, RootEvent};

pub struct ModebarState {
    enabled: bool,
    boundary: Boundary,
}

impl ModebarState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            boundary: Boundary::zero(),
        }
    }
}

impl UiComponentState for ModebarState {
    fn get_boundary(&self) -> Boundary {
        self.boundary
    }

    fn get_enabled(&mut self) -> &mut bool {
        &mut self.enabled
    }

    fn get_name(&self) -> &str {
        "Modebar"
    }
}

pub struct Modebar<'a> {
    state: &'a mut ModebarState,
}

impl<'a> Modebar<'a> {
    pub fn with_state(state: &'a mut ModebarState) -> Self {
        Self { state }
    }
}

impl<'a> UiComponent for Modebar<'a> {
    fn show(
        &mut self,
        ctx: &egui::Context,
        (ui_state, global_state): &(UiState, GlobalState<RootEvent>),
    ) {
        if self.state.enabled {
            self.state.boundary = egui::TopBottomPanel::bottom("modebar")
                .default_height(config::gui::MODEBAR_H)
                .show(ctx, |ui: &mut egui::Ui| {
                    egui::menu::bar(ui, |ui| {
                        let layout = egui::Layout {
                            main_dir: egui::Direction::TopDown,
                            main_wrap: false,
                            main_align: egui::Align::Center,
                            main_justify: false,
                            cross_align: egui::Align::Center,
                            cross_justify: true,
                        };

                        let last_mode = *ui_state.mode.read();

                        GridBuilder::new()
                            // Allocate a new row
                            .new_row_align(Size::initial(17.0), egui::Align::Center)
                            // Give this row a couple cells
                            .layout_standard(layout)
                            .clip(true)
                            .cell(Size::remainder())
                            .cell(Size::initial(-13.0))
                            .cell(Size::remainder())
                            .cell(Size::initial(-13.0))
                            .cell(Size::remainder())
                            .show(ui, |mut grid| {
                                // Cells are represented as they were allocated
                                grid.cell(|ui| {
                                    ui_state.mode.write_with_fn(|mode| {
                                        ui.selectable_value(mode, Mode::Prepare, "Prepare");
                                    });
                                });
                                grid.empty();
                                grid.cell(|ui| {
                                    ui_state.mode.write_with_fn(|mode| {
                                        ui.selectable_value(mode, Mode::Masks, "Force - Analytics");
                                    });
                                });
                                grid.empty();
                                grid.cell(|ui| {
                                    ui_state.mode.write_with_fn(|mode| {
                                        ui.selectable_value(mode, Mode::Preview, "Preview");
                                    });
                                });
                            });

                        if last_mode != *ui_state.mode.read() {
                            global_state.viewer.mode_changed(*ui_state.mode.read());
                        }
                    });
                })
                .response
                .into();
        }
    }
}
