use egui::ImageSource;

use crate::viewer::Orientation;

use super::components::addons::{cad_tools::CADTool, gizmo::GizmoTool};

pub fn get_orientation_asset(orientation: Orientation) -> ImageSource<'static> {
    match orientation {
        Orientation::Default => egui::include_image!("assets/orientation_default_30x30.png"),
        Orientation::Diagonal => egui::include_image!("assets/orientation_default_30x30.png"),
        Orientation::Top => egui::include_image!("assets/orientation_top_30x30.png"),
        Orientation::Left => egui::include_image!("assets/orientation_left_30x30.png"),
        Orientation::Right => egui::include_image!("assets/orientation_right_30x30.png"),
        Orientation::Front => egui::include_image!("assets/orientation_front_30x30.png"),
    }
}

pub fn get_gizmo_tool_icon(tool: GizmoTool) -> ImageSource<'static> {
    match tool {
        GizmoTool::Translate => egui::include_image!("assets/gizmo_translate.svg"),
        GizmoTool::Rotate => egui::include_image!("assets/gizmo_rotate.svg"),
        GizmoTool::Scale => egui::include_image!("assets/gizmo_scale.svg"),
        GizmoTool::Flatten => egui::include_image!("assets/gizmo_flatten.svg"),
    }
}

pub fn get_cad_tool_icon(tool: CADTool) -> ImageSource<'static> {
    match tool {
        CADTool::Import => {
            egui::include_image!("assets/cad_import.svg")
        }
        CADTool::ObjectMode => {
            egui::include_image!("assets/cad_object_mode.png")
        }
        CADTool::MaskMode => {
            egui::include_image!("assets/cad_mask_mode.svg")
        }
        CADTool::AddCone => {
            egui::include_image!("assets/cad_obj_cone.svg")
        }
        CADTool::AddCube => {
            egui::include_image!("assets/cad_obj_cube.svg")
        }
        CADTool::AddCylinder => {
            egui::include_image!("assets/cad_obj_cylinder.svg")
        }
    }
}
