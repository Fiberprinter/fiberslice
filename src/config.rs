//S = Size (Width and Height)
//H = height
//W = width

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<&Color> for egui::Color32 {
    fn from(color: &Color) -> Self {
        egui::Color32::from_rgba_premultiplied(color.r, color.g, color.b, color.a)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub settings_path: String,
    pub theme_color: Color,
}

pub mod default {

    //pub const WINDOW_S: Vec2 = Vec2::new(0., 0.);
    pub const WINDOW_S: (u32, u32) = (1200, 900);
}

pub mod gui {
    use crate::ui::api::DecoradedButton;

    pub const fn shaded_color(darkmode: bool) -> egui::Color32 {
        match darkmode {
            true => egui::Color32::from_rgba_premultiplied(200, 200, 200, 50),
            false => egui::Color32::from_rgba_premultiplied(200, 200, 200, 50),
        }
    }

    pub const MENUBAR_H: f32 = 17.0;
    pub const MODEBAR_H: f32 = 17.0;
    pub const TASKBAR_H: f32 = 20.0;
    pub const TOOLBAR_W: f32 = 40.0;

    pub const TOOL_TOGGLE_BUTTON: DecoradedButton = DecoradedButton {
        border: 15.,
        size: (35., 35.),
    };

    pub const GIZMO_TOGGLE_BUTTON: DecoradedButton = DecoradedButton {
        border: 15.,
        size: (45., 45.),
    };

    pub const CAD_TOOL_BUTTON: DecoradedButton = DecoradedButton {
        border: 15.,
        size: (45., 45.),
    };

    pub const ORIENATION_BUTTON: DecoradedButton = DecoradedButton {
        border: 5.,
        size: (35., 35.),
    };

    pub mod default {
        pub const SETTINGSBAR_W: f32 = 400.0;
    }

    pub mod settings {
        use crate::ui::api::size_fixed::StaticSizedLabel;

        pub const SETTINGS_LABEL: StaticSizedLabel = StaticSizedLabel::new(200.0);
    }
}
