pub mod size_fixed;

pub struct DecoradedButton {
    pub border: f32,
    pub size: (f32, f32),
}

pub fn trim_text<const T: usize, const E: usize>(text: &str) -> String {
    if text.len() > T {
        format!("{}...{}", &text[..=T - E], &text[text.len() - E..])
    } else {
        format!("{}{}", text, " ".repeat(T - text.len()))
    }
}
