/*
    Copyright (c) 2023 Elias Gottsbacher, Jan Traussnigg, Nico Huetter (HTBLA Kaindorf)
    All rights reserved.
    Note: The complete copyright description for this software thesis can be found at the beginning of each file.
    Please refer to the terms and conditions stated therein.
*/

use egui::*;
use egui_extras::Size;
use egui_grid::GridBuilder;
use settings::UiSetting;

use crate::config;
use crate::ui::boundary::Boundary;
use crate::ui::*;

#[derive(Debug, Clone, PartialEq)]
pub enum SettingTab {
    Slicing,
    GCode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingSubTab {
    General,
    PrinterAndLimits,
    Instructions,
}

struct TabbedSettings;

impl TabbedSettings {
    pub fn new() -> Self {
        Self {}
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        shared_state: &(UiState, GlobalState<RootEvent>),
        side_view: &mut Settingsbar,
    ) {
        ui.horizontal(|ui| {
            let layout = egui::Layout {
                main_dir: egui::Direction::TopDown,
                main_wrap: false,
                main_align: egui::Align::Center,
                main_justify: false,
                cross_align: egui::Align::Center,
                cross_justify: true,
            };

            GridBuilder::new()
                // Allocate a new row
                .new_row_align(Size::initial(17.0), egui::Align::Center)
                // Give this row a couple cells
                .layout_standard(layout)
                .clip(true)
                .cell(Size::remainder())
                .cell(Size::initial(5.0))
                .cell(Size::remainder())
                .cell(Size::initial(5.0))
                .cell(Size::remainder())
                .new_row_align(Size::initial(5.0), egui::Align::Center)
                .cell(Size::remainder())
                .cell(Size::remainder())
                .cell(Size::remainder())
                .show(ui, |mut grid| {
                    // Cells are represented as they were allocated
                    grid.cell(|ui| {
                        ui.selectable_value(
                            &mut side_view.state.open_sub_tab,
                            SettingSubTab::General,
                            "General",
                        );
                    });
                    grid.cell(|ui| {
                        ui.horizontal(|ui| {
                            ui.separator();
                        });
                    });
                    grid.cell(|ui| {
                        ui.selectable_value(
                            &mut side_view.state.open_sub_tab,
                            SettingSubTab::PrinterAndLimits,
                            "Printer and Limits",
                        );
                    });
                    grid.cell(|ui| {
                        ui.horizontal(|ui| {
                            ui.separator();
                        });
                    });
                    grid.cell(|ui| {
                        ui.selectable_value(
                            &mut side_view.state.open_sub_tab,
                            SettingSubTab::Instructions,
                            "Fiber",
                        );
                    });
                    grid.cell(|ui| {
                        ui.vertical(|ui| {
                            if side_view.state.open_sub_tab != SettingSubTab::General {
                                ui.separator();
                            }
                        });
                    });
                    grid.cell(|ui| {
                        ui.vertical(|ui| {
                            if side_view.state.open_sub_tab != SettingSubTab::PrinterAndLimits {
                                ui.separator();
                            }
                        });
                    });
                    grid.cell(|ui| {
                        ui.vertical(|ui| {
                            if side_view.state.open_sub_tab != SettingSubTab::Instructions {
                                ui.separator();
                            }
                        });
                    });
                });
        });

        //ui.add_space(20.0);

        match side_view.state.open_sub_tab {
            SettingSubTab::General => {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.with_layout(Layout::top_down(egui::Align::Max), |ui| {
                        egui::ScrollArea::both().show(ui, |ui| {
                            // TODO data.borrow_shared_state().settings.main.show(ui);
                            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                                shared_state.1.slicer.write_with_fn(|slicer| {
                                    slicer.settings.show_general(ui);
                                });
                            });
                        });
                    });
                });
            }
            SettingSubTab::PrinterAndLimits => {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.with_layout(Layout::top_down(egui::Align::Max), |ui| {
                        egui::ScrollArea::both().show(ui, |ui| {
                            // TODO data.borrow_shared_state().settings.main.show(ui);
                            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                                shared_state.1.slicer.write_with_fn(|slicer| {
                                    slicer.settings.show_printer(ui);
                                    slicer.settings.show_limits(ui);
                                });
                            });
                        });
                    });
                });
            }
            SettingSubTab::Instructions => {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.with_layout(Layout::top_down(egui::Align::Max), |ui| {
                        egui::ScrollArea::both().show(ui, |ui| {
                            // let now = Instant::now();
                            // TODO data.borrow_shared_state().settings.main.show(ui);
                            // println!("Tree Time: {:?}", now.elapsed());
                            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                                shared_state.1.slicer.write_with_fn(|slicer| {
                                    slicer.settings.show_fiber(ui);
                                });
                            });
                        });
                    });
                });
            }
        }

        ui.add_space(20.0);
    }
}

#[derive(Debug)]
pub struct SidebarState {
    enabled: bool,
    boundary: Boundary,

    open_tab: SettingTab,
    open_sub_tab: SettingSubTab,
}

impl SidebarState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            boundary: Boundary::zero(),

            open_tab: SettingTab::Slicing,
            open_sub_tab: SettingSubTab::General,
        }
    }
}

impl UiComponentState for SidebarState {
    fn get_boundary(&self) -> Boundary {
        self.boundary
    }

    fn get_enabled(&mut self) -> &mut bool {
        &mut self.enabled
    }

    fn get_name(&self) -> &str {
        "Settingsbar"
    }
}

#[derive(Debug)]
pub struct Settingsbar<'a> {
    state: &'a mut SidebarState,
}

impl<'a> Settingsbar<'a> {
    pub fn with_state(state: &'a mut SidebarState) -> Self {
        Self { state }
    }
}

impl<'a> UiComponent for Settingsbar<'a> {
    fn show(&mut self, ctx: &egui::Context, shared_state: &(UiState, GlobalState<RootEvent>)) {
        let mut tabbed_view = TabbedSettings::new();

        if self.state.enabled {
            self.state.boundary = Boundary::from(
                egui::SidePanel::left("settingsbar")
                    .resizable(true)
                    .default_width(config::gui::default::SETTINGSBAR_W)
                    .show(ctx, |ui| {
                        ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                            ui.add_space(20.0);

                            ui.allocate_ui(Vec2::new(ui.available_width(), 250.0), |ui| {
                                let export_button = Button::new("Export GCode")
                                    .min_size(Vec2::new(ui.available_width() * 0.5, 20.0));

                                if ui
                                    .add_enabled(
                                        shared_state.1.viewer.already_sliced(),
                                        export_button,
                                    )
                                    .clicked()
                                {
                                    shared_state.1.viewer.export_gcode();
                                }

                                let rich_text = RichText::new("Slice")
                                    .color(Color32::BLACK)
                                    .font(FontId::new(18.0, egui::FontFamily::Monospace));
                                let widget_text = widget_text::WidgetText::RichText(rich_text);

                                let slice_button = Button::new(widget_text)
                                    .fill(ui.style().visuals.selection.bg_fill)
                                    .min_size(Vec2::new(ui.available_width() * 0.8, 50.0));

                                if ui.add(slice_button).clicked() {
                                    shared_state.1.slicer.write_with_fn(|slicer| {
                                        slicer.slice(&shared_state.1);
                                    });
                                };
                            });

                            ui.add_space(20.0);

                            ui.separator();

                            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                                ui.horizontal(|ui| {
                                    let layout = egui::Layout {
                                        main_dir: egui::Direction::TopDown,
                                        main_wrap: false,
                                        main_align: egui::Align::Center,
                                        main_justify: false,
                                        cross_align: egui::Align::Center,
                                        cross_justify: true,
                                    };

                                    GridBuilder::new()
                                        // Allocate a new row
                                        .new_row_align(Size::initial(27.5), egui::Align::Center)
                                        // Give this row a couple cells
                                        .layout_standard(layout)
                                        .clip(true)
                                        .cell(Size::initial(5.0))
                                        .cell(Size::remainder())
                                        .cell(Size::initial(-13.0))
                                        .cell(Size::remainder())
                                        .cell(Size::initial(5.0))
                                        .show(ui, |mut grid| {
                                            grid.empty();
                                            // Cells are represented as they were allocated
                                            grid.cell(|ui| {
                                                ui.selectable_value(
                                                    &mut self.state.open_tab,
                                                    SettingTab::Slicing,
                                                    "Slicing",
                                                );
                                            });
                                            grid.empty();
                                            grid.cell(|ui| {
                                                ui.selectable_value(
                                                    &mut self.state.open_tab,
                                                    SettingTab::GCode,
                                                    "GCode",
                                                );
                                            });
                                            grid.empty();
                                        });
                                });

                                ui.add_space(5.0);

                                match self.state.open_tab {
                                    SettingTab::Slicing => {
                                        tabbed_view.show(ui, shared_state, self);
                                    }
                                    SettingTab::GCode => {
                                        shared_state
                                            .1
                                            .slicer
                                            .write()
                                            .settings
                                            .show_instructions(ui);
                                    }
                                }
                            });
                        });
                    })
                    .response,
            );
        }
    }
}
