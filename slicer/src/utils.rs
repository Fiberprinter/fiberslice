use geo::Coord;
use glam::Vec2;

#[inline]
pub fn point_y_lerp(a: &Coord<f32>, b: &Coord<f32>, y: f32) -> Coord<f32> {
    Coord {
        x: lerp(a.x, b.x, (y - a.y) / (b.y - a.y)),
        y,
    }
}

#[inline]
pub fn point_lerp(a: &Coord<f32>, b: &Coord<f32>, f: f32) -> Coord<f32> {
    Coord {
        x: lerp(a.x, b.x, f),
        y: lerp(a.y, b.y, f),
    }
}

#[inline]
pub fn lerp(a: f32, b: f32, f: f32) -> f32 {
    a + f * (b - a)
}

#[allow(dead_code)]
///Function to generate a unit bisector of the angle p0,p1,p2 that will always be inside the angle to the left
pub fn directional_unit_bisector_left(p0: &Coord<f32>, p1: &Coord<f32>, p2: &Coord<f32>) -> Vec2 {
    let v1 = Vec2::new(p0.x - p1.x, p0.y - p1.y);
    let v2 = Vec2::new(p2.x - p1.x, p2.y - p1.y);

    let v1_scale = v1 * v2.normalize();
    let v2_scale = v2 * v1.normalize();

    let direction = v1_scale + v2_scale;

    match orientation(p0, p1, p2) {
        Orientation::Linear => {
            let perp = Vec2::new(-v1.y, v1.x).normalize();
            match orientation(p0, p1, &Coord::from((p1.x + perp.x, p1.y + perp.y))) {
                Orientation::Linear => {
                    unreachable!()
                }
                Orientation::Left => perp.normalize(),
                Orientation::Right => perp.normalize() * -1.0,
            }
        }
        Orientation::Left => direction.normalize(),
        Orientation::Right => direction.normalize() * -1.0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Orientation {
    Linear,
    Left,
    Right,
}

pub fn orientation(p: &Coord<f32>, q: &Coord<f32>, r: &Coord<f32>) -> Orientation {
    let left_val = (q.x - p.x) * (r.y - p.y);
    let right_val = (q.y - p.y) * (r.x - p.x);

    if left_val == right_val {
        Orientation::Linear
    } else if left_val > right_val {
        Orientation::Left
    } else {
        Orientation::Right
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_directional_unit_bisector() {
        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((0.0, 0.0)),
                &Coord::from((1.0, 0.0)),
                &Coord::from((1.0, 1.0))
            ),
            Vec2::new(-1.0, 1.0).normalize()
        );
        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((1.0, 1.0)),
                &Coord::from((1.0, 0.0)),
                &Coord::from((0.0, 0.0))
            ),
            Vec2::new(1.0, -1.0).normalize()
        );

        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((0.0, 0.0)),
                &Coord::from((1.0, 0.0)),
                &Coord::from((2.0, 0.0))
            ),
            Vec2::new(0.0, 1.0)
        );
        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((2.0, 0.0)),
                &Coord::from((1.0, 0.0)),
                &Coord::from((0.0, 0.0))
            ),
            Vec2::new(0.0, -1.0)
        );

        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((0.0, 0.0)),
                &Coord::from((0.0, 1.0)),
                &Coord::from((0.0, 1.0))
            ),
            Vec2::new(-1.0, 0.0)
        );
        assert_eq!(
            directional_unit_bisector_left(
                &Coord::from((0.0, 2.0)),
                &Coord::from((0.0, 1.0)),
                &Coord::from((0.0, 0.0))
            ),
            Vec2::new(1.0, 0.0)
        );
    }
}
