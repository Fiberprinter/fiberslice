use glam::Vec3;

#[derive(Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn from_view(
        viewport: (f32, f32, f32, f32),
        position: (f32, f32),
        view: glam::Mat4,
        proj: glam::Mat4,
        eye: glam::Vec3,
    ) -> Self {
        // Unpack the viewport parameters
        let (x, y, width, height) = viewport;

        // Convert screen space position to normalized device coordinates (NDC)
        let ndc_x = 1.0 - (2.0 * (position.0 - x)) / width;
        let ndc_y = (2.0 * (position.1 - y)) / height - 1.0; // flip y-axis

        // Clip space coordinates
        let clip_coords = glam::Vec4::new(ndc_x, ndc_y, -1.0, 1.0);

        // Convert clip space coordinates to eye space using the inverse projection matrix
        let inv_proj = proj.inverse();
        let eye_coords = inv_proj * clip_coords;

        // Homogeneous divide (perspective divide) to get eye space coordinates
        let eye_coords = glam::Vec4::new(eye_coords.x, eye_coords.y, -1.0, 0.0);

        // Convert eye space coordinates to world space using the inverse view matrix
        let inv_view = view.inverse();
        let world_coords = inv_view * eye_coords;

        // Normalize the direction
        let direction = (Vec3::new(world_coords.x, world_coords.y, world_coords.z)).normalize();

        Ray {
            origin: eye,
            direction,
        }
    }

    pub fn intersection_plane(&self, plane: Vec3, point: Vec3) -> Vec3 {
        let d = plane.dot(self.direction);
        if d.abs() > f32::EPSILON {
            let t = (point - self.origin).dot(plane) / d;
            self.origin + self.direction * t
        } else {
            Vec3::new(0.0, 0.0, 0.0)
        }
    }
}
