use egui::Ui;
use egui_extras::Size;
use egui_grid::GridBuilder;

pub struct Tab<T> {
    tab: T,
    title: &'static str,
}

impl<T> Tab<T> {
    pub fn new(tab: T, title: &'static str) -> Self {
        Self { tab, title }
    }
}

pub struct Tabbed<'a, T, const N: usize> {
    tab: &'a mut T,
    tabs: [Tab<T>; N],
    clip: bool,
    height: f32,
}

impl<'a, T: PartialEq + Clone, const N: usize> Tabbed<'a, T, N> {
    pub fn new(tab: &'a mut T, tabs: [Tab<T>; N]) -> Self {
        Self {
            tab,
            tabs,
            clip: true,
            height: 20.0,
        }
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn with_clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let layout = egui::Layout {
            main_dir: egui::Direction::TopDown,
            main_wrap: false,
            main_align: egui::Align::Center,
            main_justify: false,
            cross_align: egui::Align::Center,
            cross_justify: true,
        };

        let mut builder = GridBuilder::new()
            // Allocate a new row
            .new_row_align(Size::initial(self.height), egui::Align::Center)
            // Give this row a couple cells
            .layout_standard(layout)
            .clip(self.clip);

        for i in 0..N {
            builder = builder.cell(Size::remainder());

            if i < (N - 1) {
                builder = builder.cell(Size::initial(5.0));
            }
        }

        builder = builder.new_row_align(Size::initial(5.0), egui::Align::Center);

        builder = builder.cells(Size::remainder(), N as i32);

        builder.show(ui, |mut grid| {
            for (index, tab) in self.tabs.iter().enumerate() {
                grid.cell(|ui| {
                    ui.selectable_value(self.tab, tab.tab.clone(), tab.title);
                });

                if index < (N - 1) {
                    grid.cell(|ui| {
                        ui.horizontal(|ui| {
                            ui.separator();
                        });
                    });
                }
            }

            for tab in self.tabs.iter() {
                grid.cell(|ui| {
                    ui.vertical(|ui| {
                        if *self.tab != tab.tab {
                            ui.separator();
                        }
                    });
                });
            }
        });
    }
}
