use egui::{text::LayoutJob, RichText};
use egui_code_editor::{ColorTheme, Editor, Syntax, Token, TokenType};

use egui::widgets::text_edit::TextEditOutput;
use egui_code_editor::highlighting::highlight;
use std::hash::{Hash, Hasher};

#[derive(Debug, PartialEq)]
/// CodeEditor struct which stores settings for highlighting.
pub struct EfficientReader<'a> {
    id: String,
    theme: ColorTheme,
    syntax: Syntax,
    numlines: bool,
    fontsize: f32,
    shrink: bool,
    editable: bool,

    view: &'a mut ReadSection,
}

impl Hash for EfficientReader<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.theme.hash(state);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        (self.fontsize as u32).hash(state);
        self.syntax.hash(state);
    }
}

impl<'a> EfficientReader<'a> {
    pub fn new(view: &'a mut ReadSection) -> Self {
        EfficientReader {
            id: "code_editor".to_string(),
            theme: ColorTheme::GRUVBOX,
            syntax: Syntax::default(),
            numlines: true,
            fontsize: 10.0,
            shrink: false,
            editable: false,
            view,
        }
    }

    pub fn id_source(self, id_source: impl Into<String>) -> Self {
        EfficientReader {
            id: id_source.into(),
            ..self
        }
    }

    /// Use custom Color Theme
    ///
    /// **Default: Gruvbox**
    pub fn with_theme(self, theme: ColorTheme) -> Self {
        EfficientReader { theme, ..self }
    }

    /// Use custom font size
    ///
    /// **Default: 10.0**
    pub fn with_fontsize(self, fontsize: f32) -> Self {
        EfficientReader { fontsize, ..self }
    }

    #[allow(dead_code)]
    /// Use UI font size
    pub fn with_ui_fontsize(self, ui: &mut egui::Ui) -> Self {
        EfficientReader {
            fontsize: egui::TextStyle::Monospace.resolve(ui.style()).size,
            ..self
        }
    }

    /// Show or hide lines numbering
    ///
    /// **Default: true**
    pub fn with_numlines(self, numlines: bool) -> Self {
        EfficientReader { numlines, ..self }
    }

    /// Use custom syntax for highlighting
    ///
    /// **Default: Rust**
    pub fn with_syntax(self, syntax: Syntax) -> Self {
        EfficientReader { syntax, ..self }
    }

    pub fn format(&self, ty: TokenType) -> egui::text::TextFormat {
        let font_id = egui::FontId::monospace(self.fontsize);
        let color = self.theme.type_color(ty);
        egui::text::TextFormat::simple(font_id, color)
    }

    fn numlines_show(&mut self, ui: &mut egui::Ui) {
        let ReadSection { offset, size } = *self.view;

        let counter = ((offset + 1)..=(size + offset))
            .map(|i| i.to_string())
            .collect::<Vec<String>>()
            .join("\n");

        ui.add(egui::Label::new(RichText::new(counter)));
    }

    pub fn show(&mut self, ui: &mut egui::Ui, text: &str, line_breaks: &[usize]) -> TextEditOutput {
        let mut text_edit_output: Option<TextEditOutput> = None;
        ui.horizontal_top(|h| {
            self.theme.modify_style(h, self.fontsize);
            if self.numlines {
                self.numlines_show(h);
            }
            egui::ScrollArea::horizontal()
                .id_salt(format!("{}_inner_scroll", self.id))
                .show(h, |ui| {
                    let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                        let layout_job = highlight(ui.ctx(), self, string);
                        ui.fonts(|f| f.layout_job(layout_job))
                    };

                    let ReadSection { offset, size } = *self.view;

                    let char_start = if offset == 0 {
                        0
                    } else {
                        line_breaks.get(offset - 1).copied().unwrap_or(0) + 1
                    };

                    let char_end = line_breaks
                        .get(offset + size - 1)
                        .copied()
                        .unwrap_or(text.len());

                    let mut text = text[char_start..char_end].to_string();

                    let output = egui::TextEdit::multiline(&mut text)
                        .id_source(&self.id)
                        .lock_focus(true)
                        .desired_rows(size)
                        .interactive(false)
                        .frame(true)
                        .desired_width(if self.shrink { 0.0 } else { f32::MAX })
                        .layouter(&mut layouter)
                        .show(ui);
                    text_edit_output = Some(output);
                });
        });

        let scroll_delta = ui.ctx().input(|input| input.smooth_scroll_delta.y);

        if ui.ui_contains_pointer() && self.view.size < line_breaks.len() {
            self.view.offset = ((self.view.offset as f32 - scroll_delta * 0.1).max(0.0) as usize)
                .min(line_breaks.len() - 1);
        }

        text_edit_output.expect("TextEditOutput should exist at this point")
    }
}

impl Editor for EfficientReader<'_> {
    fn append(&self, job: &mut LayoutJob, token: &Token) {
        job.append(token.buffer(), 0.0, self.format(token.ty()));
    }

    fn syntax(&self) -> &Syntax {
        &self.syntax
    }
}

// be right back

#[derive(Debug, Clone, Default, Copy, PartialEq)]
pub struct ReadSection {
    offset: usize,
    size: usize,
}

impl ReadSection {
    pub fn new(offset: usize, size: usize) -> Self {
        ReadSection { offset, size }
    }

    #[allow(dead_code)]
    pub fn with_offset(self, offset: usize) -> Self {
        ReadSection { offset, ..self }
    }

    #[allow(dead_code)]
    pub fn with_size(self, size: usize) -> Self {
        ReadSection { size, ..self }
    }
}
