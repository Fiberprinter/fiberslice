use egui::Response;

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy)]
pub struct Boundary {
    location: egui::Pos2,
    size: egui::Vec2,
}

impl Boundary {
    pub fn zero() -> Self {
        Self {
            location: egui::Pos2::ZERO,
            size: egui::Vec2::ZERO,
        }
    }

    pub fn get_width(&self) -> f32 {
        self.size.x
    }

    pub fn get_height(&self) -> f32 {
        self.size.y
    }
}

impl From<Response> for Boundary {
    fn from(response: Response) -> Self {
        Self {
            location: response.rect.min,
            size: response.rect.size(),
        }
    }
}
