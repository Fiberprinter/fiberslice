use egui::Ui;
use egui_extras::Size;

pub struct ListBuilder {
    count: usize,
    fill: usize,
    cell_height: f32,
    builder: egui_grid::GridBuilder,
}

impl ListBuilder {
    pub fn new() -> Self {
        Self {
            count: 0,
            fill: 0,
            cell_height: 20.0,
            builder: egui_grid::GridBuilder::new(),
        }
    }

    pub fn with_cell_height(mut self, cell_height: f32) -> Self {
        self.cell_height = cell_height;
        self
    }

    pub fn entry(mut self) -> Self {
        self.count += 1;
        self.builder = self
            .builder
            .new_row_align(Size::exact(self.cell_height), egui::Align::Center)
            .cell(Size::remainder());
        self
    }

    pub fn entries(mut self, count: usize) -> Self {
        for _ in 0..count {
            self = self.entry();
        }

        self
    }

    #[allow(dead_code)]
    pub fn fill(mut self, fill: usize) -> Self {
        if self.count >= fill {
            self.fill = fill;
            return self;
        }

        let count = self.count;
        self = self.entries(fill - count);
        self.fill = fill;
        self
    }

    pub fn show(self, ui: &mut Ui, list: impl FnOnce(List)) {
        self.builder
            .show(ui, |grid| list(List::new(grid, self.fill)));
    }
}

pub struct List<'a, 'b> {
    count: usize,
    fill: usize,
    grid: egui_grid::Grid<'a, 'b>,
}

impl<'a, 'b> List<'a, 'b> {
    pub fn new(grid: egui_grid::Grid<'a, 'b>, fill: usize) -> Self {
        Self {
            count: 0,
            fill,
            grid,
        }
    }

    pub fn entry(&mut self, list: impl FnOnce(&mut Ui)) {
        self.count += 1;
        self.grid.cell(|ui| {
            let color = if self.count % 2 == 0 {
                ui.visuals().code_bg_color
            } else {
                ui.visuals().extreme_bg_color
            };

            ui.painter()
                .rect_filled(ui.available_rect_before_wrap(), 0.0, color);

            list(ui)
        });
    }

    pub fn fill(&mut self) {
        if self.count >= self.fill {
            return;
        }

        for _ in self.count..self.fill {
            self.entry(|_| {});
        }
    }
}

impl Drop for List<'_, '_> {
    fn drop(&mut self) {
        self.fill();
    }
}
