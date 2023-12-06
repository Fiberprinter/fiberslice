//S = Size (Width and Height)
//H = height
//W = width

pub mod default {

    //pub const WINDOW_S: Vec2 = Vec2::new(0., 0.);
    pub const WINDOW_S: (u32, u32) = (1200, 900);
}

pub mod gui {
    use three_d::egui;

    use crate::ui::api::buttons::DecoradedButton;

    pub const fn shaded_color(darkmode: bool) -> egui::Color32 {
        match darkmode {
            true => egui::Color32::from_rgba_premultiplied(25, 25, 25, 100),
            false => egui::Color32::from_rgba_premultiplied(145, 145, 145, 50),
        }
    }

    pub const MENUBAR_H: f32 = 17.0;
    pub const MODEBAR_H: f32 = 17.0;
    pub const TASKBAR_H: f32 = 20.0;
    pub const TOOLBAR_W: f32 = 35.0;

    pub const TOOL_TOGGLE_BUTTON: DecoradedButton = DecoradedButton {
        border: 5.,
        size: (30., 30.),
        hover_color: egui::Color32::from_rgba_premultiplied(75, 255, 0, 100),
    };

    pub const ORIENATION_BUTTON: DecoradedButton = DecoradedButton {
        border: 5.,
        size: (30., 30.),
        hover_color: egui::Color32::from_rgba_premultiplied(75, 255, 0, 100),
    };

    pub mod default {
        pub const SETTINGSBAR_W: f32 = 400.0;
    }

    pub mod settings {
        use crate::ui::api::size_fixed::StaticSizedLabel;

        pub const SETTINGS_LABEL: StaticSizedLabel = StaticSizedLabel::new(200.0);
    }
}
