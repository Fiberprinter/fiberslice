use geo::*;

//todo remove dependency on geo clipper and by extension bindgen

pub trait PolygonOperations {
    fn offset_from(&self, delta: f32) -> MultiPolygon<f32>;

    fn difference_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32>;

    fn intersection_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32>;

    fn union_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32>;

    #[allow(dead_code)]
    fn xor_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32>;
}

impl PolygonOperations for MultiPolygon<f32> {
    fn offset_from(&self, delta: f32) -> MultiPolygon<f32> {
        geo_clipper::Clipper::offset(
            self,
            delta,
            geo_clipper::JoinType::Square,
            geo_clipper::EndType::ClosedPolygon,
            1000000.0,
        )
    }

    fn difference_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32> {
        geo_clipper::Clipper::difference(self, other, 1000000.0)
    }

    fn intersection_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32> {
        geo_clipper::Clipper::intersection(self, other, 1000000.0)
    }

    fn union_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32> {
        geo_clipper::Clipper::union(self, other, 1000000.0)
    }

    fn xor_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32> {
        geo_clipper::Clipper::xor(self, other, 1000000.0)
    }
}

impl PolygonOperations for Polygon<f32> {
    fn offset_from(&self, delta: f32) -> MultiPolygon<f32> {
        geo_clipper::Clipper::offset(
            self,
            delta,
            geo_clipper::JoinType::Square,
            geo_clipper::EndType::ClosedPolygon,
            1000000.0,
        )
    }

    fn difference_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32> {
        geo_clipper::Clipper::difference(self, other, 1000000.0)
    }

    fn intersection_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32> {
        geo_clipper::Clipper::intersection(self, other, 1000000.0)
    }

    fn union_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32> {
        geo_clipper::Clipper::union(self, other, 1000000.0)
    }

    fn xor_with(&self, other: &MultiPolygon<f32>) -> MultiPolygon<f32> {
        geo_clipper::Clipper::xor(self, other, 1000000.0)
    }
}
