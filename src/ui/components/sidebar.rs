/*
    Copyright (c) 2023 Elias Gottsbacher, Jan Traussnigg, Nico Huetter (HTBLA Kaindorf)
    All rights reserved.
    Note: The complete copyright description for this software thesis can be found at the beginning of each file.
    Please refer to the terms and conditions stated therein.
*/

use egui::*;
use settings::UiSetting;

use crate::config;
use crate::ui::boundary::Boundary;
use crate::ui::widgets::switch::{Switch, SwitchTab};
use crate::ui::widgets::tabbed::{Tab, Tabbed};
use crate::ui::*;

#[derive(Debug, Clone, PartialEq)]
pub enum SettingTab {
    Slicing,
    Printer,
    GCode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingMode {
    Global,
    Mask,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SettingSubTab {
    General,
    Fiber,
}

#[derive(Debug)]
pub struct SidebarState {
    enabled: bool,
    boundary: Boundary,

    mode: SettingMode,

    open_tab: SettingTab,
    open_sub_tab: SettingSubTab,
}

impl SidebarState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            boundary: Boundary::zero(),

            mode: SettingMode::Global,

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

    fn show_main(
        &mut self,
        shared_state: &(UiState, GlobalState<RootEvent>),
        ui: &mut Ui,
    ) -> InnerResponse<()> {
        ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
            ui.add_space(10.0);

            Tabbed::new(
                &mut self.state.open_tab,
                [
                    Tab::new(SettingTab::Slicing, "Slicing"),
                    Tab::new(SettingTab::Printer, "Printer"),
                    Tab::new(SettingTab::GCode, "GCode"),
                ],
            )
            .with_height(17.0)
            .with_clip(true)
            .show(ui);

            ui.add_space(10.0);

            match self.state.open_tab {
                SettingTab::Slicing => {
                    Switch::new(
                        &mut self.state.mode,
                        SwitchTab::new(SettingMode::Global, "Switch to Global Settings"),
                        SwitchTab::new(SettingMode::Mask, "Switch to Mask Settings"),
                    )
                    .with_width(40.0)
                    .show(ui);

                    match self.state.mode {
                        SettingMode::Global => {
                            ui.heading("Global Settings");

                            ui.separator();

                            self.show_global_settings(ui, shared_state);
                        }
                        SettingMode::Mask => {
                            ui.heading("Mask Settings");

                            ui.separator();

                            self.show_mask_settings(ui, shared_state);
                        }
                    }
                }
                SettingTab::Printer => {
                    egui::ScrollArea::both().show(ui, |ui| {
                        shared_state.1.slicer.write_with_fn(|slicer| {
                            slicer.settings.show_printer(ui);
                            slicer.settings.show_limits(ui);
                        });
                    });
                }
                SettingTab::GCode => {
                    egui::ScrollArea::both().show(ui, |ui| {
                        shared_state.1.slicer.write_with_fn(|slicer| {
                            slicer.settings.show_instructions(ui);
                        });
                    });
                }
            }

            ui.add_space(20.0);
        })
    }

    fn show_global_settings(
        &mut self,
        ui: &mut Ui,
        shared_state: &(UiState, GlobalState<RootEvent>),
    ) {
        Tabbed::new(
            &mut self.state.open_sub_tab,
            [
                Tab::new(SettingSubTab::General, "General"),
                Tab::new(SettingSubTab::Fiber, "Fiber"),
            ],
        )
        .with_height(17.0)
        .with_clip(true)
        .show(ui);

        ui.add_space(10.0);

        match self.state.open_sub_tab {
            SettingSubTab::General => {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                        shared_state.1.slicer.write_with_fn(|slicer| {
                            slicer.settings.show_general(ui);
                        });
                    });
                });
            }
            SettingSubTab::Fiber => {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                        shared_state.1.slicer.write_with_fn(|slicer| {
                            slicer.settings.show_fiber(ui);
                        });
                    });
                });
            }
        }
    }

    fn show_mask_settings(
        &mut self,
        ui: &mut Ui,
        shared_state: &(UiState, GlobalState<RootEvent>),
    ) {
        let masks = shared_state.1.viewer.masks();

        if masks.is_empty() {
            let text =
                RichText::new("No Masks").font(FontId::new(35.0, egui::FontFamily::Monospace));

            ui.centered_and_justified(|ui| {
                ui.label(text);
            });
        }
    }
}

impl<'a> UiComponent for Settingsbar<'a> {
    fn show(&mut self, ctx: &egui::Context, shared_state: &(UiState, GlobalState<RootEvent>)) {
        if self.state.enabled {
            self.state.boundary = Boundary::from(
                egui::SidePanel::left("settingsbar")
                    .resizable(true)
                    .default_width(config::gui::default::SETTINGSBAR_W)
                    .show(ctx, |ui| {
                        ui.with_layout(Layout::bottom_up(egui::Align::Center), |ui| {
                            ui.add_space(20.0);

                            show_buttons(shared_state, ui);

                            ui.add_space(20.0);

                            ui.separator();

                            self.show_main(shared_state, ui);
                        });
                    })
                    .response,
            );
        }
    }
}

fn show_buttons(shared_state: &(UiState, GlobalState<RootEvent>), ui: &mut Ui) {
    ui.allocate_ui(Vec2::new(ui.available_width(), 250.0), |ui| {
        let export_button =
            Button::new("Export GCode").min_size(Vec2::new(ui.available_width() * 0.5, 20.0));

        if ui
            .add_enabled(shared_state.1.viewer.already_sliced(), export_button)
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
}
