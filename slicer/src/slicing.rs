use geo::Coord;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use shared::object::ObjectVertex;

use super::{
    error::SlicerErrors,
    settings::Settings,
    tower::{TriangleTower, TriangleTowerIterator},
    Object, Slice,
};

pub fn slice(
    towers: &[TriangleTower],
    max_height: f32,
    settings: &Settings,
) -> Result<Vec<Object>, SlicerErrors> {
    towers
        .iter()
        .map(|tower| {
            let mut tower_iter = TriangleTowerIterator::new(tower);

            let mut layer = 0.0;

            let mut first_layer = true;

            let res_points: Result<Vec<(f32, f32, Vec<Vec<ObjectVertex>>)>, SlicerErrors> =
                std::iter::repeat(())
                    .enumerate()
                    .map(|(layer_count, _)| {
                        //Advance to the correct height
                        let layer_height =
                            settings.get_layer_settings(layer_count, layer).layer_height;

                        let bottom_height = layer;
                        layer += layer_height / 2.0;
                        tower_iter.advance_to_height(layer)?;
                        layer += layer_height / 2.0;

                        let top_height = layer;

                        first_layer = false;

                        //Get the ordered lists of points
                        Ok((bottom_height, top_height, tower_iter.get_points()))
                    })
                    .take_while(|r| {
                        if let Ok((bottom, top, layer_loops)) = r {
                            !layer_loops.is_empty() || ((bottom + top) / 2.0 <= max_height)
                        } else {
                            true
                        }
                    })
                    .collect();

            let points = res_points?;

            let slices: Result<Vec<Slice>, SlicerErrors> = points
                .par_iter()
                .enumerate()
                .map(|(count, (bot, top, layer_loops))| {
                    //Add this slice to the
                    let slice = Slice::from_multiple_point_loop(
                        layer_loops
                            .iter()
                            .map(|verts| {
                                verts
                                    .iter()
                                    .map(|v| Coord { x: v.x, y: v.y })
                                    .collect::<Vec<Coord<f32>>>()
                            })
                            .collect(),
                        *bot,
                        *top,
                        count,
                        settings,
                    );
                    slice
                })
                .collect();

            Ok(Object { layers: slices? })
        })
        .collect()
}

pub fn slice_single(
    tower: &TriangleTower,
    max_height: f32,
    settings: &Settings,
) -> Result<Object, SlicerErrors> {
    let mut tower_iter = TriangleTowerIterator::new(tower);

    let mut layer = 0.0;

    let mut first_layer = true;

    let res_points: Result<Vec<(f32, f32, Vec<Vec<ObjectVertex>>)>, SlicerErrors> =
        std::iter::repeat(())
            .enumerate()
            .map(|(layer_count, _)| {
                //Advance to the correct height
                let layer_height = settings.get_layer_settings(layer_count, layer).layer_height;

                let bottom_height = layer;
                layer += layer_height / 2.0;
                tower_iter.advance_to_height(layer)?;
                layer += layer_height / 2.0;

                let top_height = layer;

                first_layer = false;

                //Get the ordered lists of points
                Ok((bottom_height, top_height, tower_iter.get_points()))
            })
            .take_while(|r| {
                if let Ok((bottom, top, layer_loops)) = r {
                    !layer_loops.is_empty() || ((bottom + top) / 2.0 <= max_height)
                } else {
                    true
                }
            })
            .collect();

    let points = res_points?;

    let slices: Result<Vec<Slice>, SlicerErrors> = points
        .par_iter()
        .enumerate()
        .map(|(count, (bot, top, layer_loops))| {
            //Add this slice to the
            let slice = Slice::from_multiple_point_loop(
                layer_loops
                    .iter()
                    .map(|verts| {
                        verts
                            .iter()
                            .map(|v| Coord { x: v.x, y: v.y })
                            .collect::<Vec<Coord<f32>>>()
                    })
                    .collect(),
                *bot,
                *top,
                count,
                settings,
            );
            slice
        })
        .collect();

    Ok(Object { layers: slices? })
}
