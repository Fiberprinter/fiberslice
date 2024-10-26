use egui::TextWrapMode;
use egui::Ui;
use native_dialog::FileDialog;

use crate::config;
use crate::ui::boundary::Boundary;
use crate::ui::UiComponent;
use crate::ui::UiComponentState;
use crate::ui::UiState;
use crate::GlobalState;
use crate::RootEvent;

pub struct MenubarState {
    enabled: bool,
    boundary: Boundary,
}

impl MenubarState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            boundary: Boundary::zero(),
        }
    }
}

impl UiComponentState for MenubarState {
    fn get_boundary(&self) -> Boundary {
        self.boundary
    }

    fn get_enabled(&mut self) -> &mut bool {
        &mut self.enabled
    }
}

pub struct Menubar<'a> {
    state: &'a mut MenubarState,

    component_states: &'a mut [&'a mut dyn UiComponentState],
}

impl<'a> Menubar<'a> {
    pub fn with_state(state: &'a mut MenubarState) -> Self {
        Self {
            state,
            component_states: &mut [],
        }
    }

    pub fn with_component_states(
        mut self,
        component_states: &'a mut [&'a mut dyn UiComponentState],
    ) -> Self {
        self.component_states = component_states;
        self
    }
}

impl<'a> UiComponent for Menubar<'a> {
    fn show(&mut self, ctx: &egui::Context, shared_state: &(UiState, GlobalState<RootEvent>)) {
        if self.state.enabled {
            self.state.boundary = egui::TopBottomPanel::top("menubar")
                .default_height(config::gui::MENUBAR_H)
                .show(ctx, |ui: &mut Ui| {
                    egui::menu::bar(ui, |ui| {
                        file_button(ui, shared_state);
                        self.window_button(ui, shared_state);
                        // self.setting_button(ui, shared_state);
                        help_button(ui, shared_state);
                    });
                })
                .response
                .into();
        }
    }
}

impl<'a> Menubar<'a> {
    fn window_button(&mut self, ui: &mut Ui, _shared_state: &(UiState, GlobalState<RootEvent>)) {
        ui.menu_button("Window", |ui| {
            ui.set_min_width(220.0);
            ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

            /*
                        ui.checkbox(
                &mut data.borrow_mut_ui_state().components.addons.enabled,
                "Addons",
            );
            ui.separator();
            */

            for component_state in self.component_states.iter_mut() {
                let name = component_state.get_name().to_string();

                ui.checkbox(component_state.get_enabled(), name);
            }
        });
    }

    #[allow(dead_code)]
    fn setting_button(&mut self, ui: &mut Ui, _shared_state: &(UiState, GlobalState<RootEvent>)) {
        ui.menu_button("Settings", |ui| {
            ui.set_min_width(220.0);
            ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

            // let dialog = SlicerSettingInstructionDialog::new(ui.ctx(), shared_state);

            build_sub_menu(ui, "GCode Instructions", |_ui| {
                // dialog.show();
            });
        });
    }
}

fn file_button(ui: &mut Ui, (_ui_state, global_state): &(UiState, GlobalState<RootEvent>)) {
    ui.menu_button("File", |ui| {
        ui.set_min_width(220.0);
        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

        build_sub_menu(ui, "Import Object", |_ui| {
            let path = FileDialog::new()
                .set_location("~")
                .add_filter("STL Files", &["stl"])
                .show_open_single_file()
                .unwrap();

            match path {
                Some(path) => {
                    global_state.viewer.model_server.write().load(path);
                }
                None => {
                    println!("No file selected")
                }
            }
        });

        build_sub_menu(ui, "Import Intersection Object", |_ui| {});

        build_sub_menu(ui, "Save As", |_ui| {});

        build_sub_menu(ui, "Save", |_ui| {});

        build_sub_menu(ui, "Exit", |_ui| {
            global_state.proxy.send_event(RootEvent::Exit).unwrap();
        });
    });
}

fn help_button(ui: &mut Ui, _shared_state: &(UiState, GlobalState<RootEvent>)) {
    ui.menu_button("Help", |ui| {
        ui.set_min_width(220.0);
        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

        if ui.button("About").hovered() {
            egui::popup::show_tooltip(
                ui.ctx(),
                ui.layer_id(),
                egui::Id::new("menubar-about-popup"),
                |ui| {
                    ui.label("This is a special slicer for placing fibers in gcode.");
                },
            );
        }
    });
}

fn build_sub_menu(ui: &mut Ui, title: &str, action: impl FnOnce(&mut Ui)) {
    if ui.button(title).clicked() {
        action(ui);
    }
}
