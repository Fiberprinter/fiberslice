use egui::{Color32, RichText, Ui};

pub struct SwitchTab<T> {
    tab: T,
    hint: &'static str,
}

impl<T> SwitchTab<T> {
    pub fn new(tab: T, hint: &'static str) -> Self {
        Self { tab, hint }
    }
}

pub struct Switch<'a, T> {
    tab: &'a mut T,
    left: SwitchTab<T>,
    right: SwitchTab<T>,

    width: f32,
}

impl<'a, T: PartialEq + Clone> Switch<'a, T> {
    pub fn new(tab: &'a mut T, left: SwitchTab<T>, right: SwitchTab<T>) -> Self {
        Self {
            tab,
            left,
            right,
            width: 40.0,
        }
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let SwitchTab {
            tab: left,
            hint: l_hint,
        } = self.left;

        let SwitchTab {
            tab: right,
            hint: r_hint,
        } = self.right;

        if *self.tab == left {
            egui::SidePanel::right("switch mask")
                .resizable(false)
                .default_width(40.0)
                .show_separator_line(true)
                .show_inside(ui, |ui| {
                    ui.centered_and_justified(|ui| {
                        let button =
                            egui::Button::new(RichText::new(">>").color(Color32::BLACK).strong())
                                .selected(true);

                        let response = ui.add(button);

                        if response.clicked() {
                            *self.tab = right.clone();
                        }

                        response.on_hover_text_at_pointer(r_hint);
                    });
                });
        } else if *self.tab == right {
            egui::SidePanel::right("switch global")
                .resizable(false)
                .default_width(self.width)
                .show_separator_line(true)
                .show_inside(ui, |ui| {
                    ui.centered_and_justified(|ui| {
                        let button =
                            egui::Button::new(RichText::new("<<").color(Color32::BLACK).strong())
                                .selected(true);

                        let response = ui.add(button);

                        if response.clicked() {
                            *self.tab = left.clone();
                        }

                        response.on_hover_text_at_pointer(l_hint);
                    });
                });
        } else {
            unreachable!();
        }
    }
}
