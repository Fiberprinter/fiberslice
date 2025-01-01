use geo::prelude::*;
use geo::*;

use itertools::Itertools;

use crate::settings::LayerSettings;
use crate::{Move, MoveChain, MoveType, TraceType};

use super::polygon_operations::PolygonOperations;

/*
let is_wall_space = |layer_num: usize, wall_num: usize| {
    // Calculate the layer's pattern
    let layer_cycle_length = pattern_layer_space + pattern_layer_width;
    let layer_position = layer_num % layer_cycle_length;
    if layer_position < pattern_layer_space {
        // Entire layer is space
        true
    } else {
        // Calculate the wall's pattern
        let wall_cycle_length = pattern_wall_space + pattern_wall_width;
        let wall_position = wall_num % wall_cycle_length;
        wall_position < pattern_wall_space
    }
};
*/

pub fn determine_move_type(
    settings: &LayerSettings,
    wall: usize,
    layer: usize,
    trace_type: TraceType,
) -> MoveType {
    if settings.fiber.wall_pattern.is_enabled() {
        match settings.fiber.wall_pattern.pattern {
            crate::fiber::WallPatternType::Alternating => {
                let layer_cycle_length = settings.fiber.wall_pattern.alternating_layer_spacing
                    + settings.fiber.wall_pattern.alternating_layer_width;
                let layer_position = layer % layer_cycle_length;

                if layer_position < settings.fiber.wall_pattern.alternating_layer_spacing {
                    // Entire layer is space
                    return MoveType::WithoutFiber(trace_type);
                } else {
                    // Calculate the wall's pattern
                    let wall_cycle_length = settings.fiber.wall_pattern.alternating_wall_spacing
                        + settings.fiber.wall_pattern.alternating_wall_width;
                    let wall_position = (wall
                        + (layer * settings.fiber.wall_pattern.alternating_step))
                        % wall_cycle_length;

                    if wall_position < settings.fiber.wall_pattern.alternating_wall_spacing {
                        return MoveType::WithoutFiber(trace_type);
                    } else {
                        return MoveType::WithFiber(trace_type);
                    }
                }
            }
            crate::fiber::WallPatternType::Full => MoveType::WithFiber(trace_type),
        }
    } else {
        MoveType::WithoutFiber(trace_type)
    }
}

pub fn inset_polygon_recursive(
    poly: &MultiPolygon<f32>,
    settings: &LayerSettings,
    outer_perimeter: bool,
    walls_left: usize,
    layer: usize,
) -> Option<MoveChain> {
    let mut move_chains = vec![];
    let inset_poly = poly.offset_from(
        if outer_perimeter {
            settings.extrusion_width.interior_surface_perimeter
        } else {
            settings.extrusion_width.interior_inner_perimeter
        } / -2.0,
    );

    for raw_polygon in inset_poly.0.iter() {
        let polygon = raw_polygon.simplify(&0.01);
        let mut outer_chains = vec![];
        let moves: Vec<Move> = polygon
            .exterior()
            .0
            .iter()
            .circular_tuple_windows::<(_, _)>()
            .map(|(&_start, &end)| {
                let move_type = if outer_perimeter {
                    determine_move_type(settings, walls_left + 1, layer, TraceType::WallOuter)
                    // MoveType::WithoutFiber(TraceType::WallOuter)
                } else {
                    determine_move_type(
                        settings,
                        walls_left + 1,
                        layer,
                        TraceType::InteriorWallOuter,
                    )
                    // MoveType::WithoutFiber(TraceType::InteriorWallOuter)
                };

                Move {
                    end,
                    move_type,
                    width: settings
                        .extrusion_width
                        .get_value_for_movement_type(&move_type),
                }
            })
            .collect();

        outer_chains.push(MoveChain {
            start_point: polygon.exterior()[0],
            moves,
            is_loop: true,
        });

        for interior in polygon.interiors() {
            let mut moves = vec![];
            let move_type = if outer_perimeter {
                determine_move_type(settings, walls_left + 1, layer, TraceType::WallInner)
                // MoveType::WithoutFiber(TraceType::WallInner)
            } else {
                determine_move_type(
                    settings,
                    walls_left + 1,
                    layer,
                    TraceType::InteriorWallInner,
                )
                // MoveType::WithoutFiber(TraceType::InteriorWallInner)
            };

            for (&_start, &end) in interior.0.iter().circular_tuple_windows::<(_, _)>() {
                moves.push(Move {
                    end,
                    move_type,
                    width: settings
                        .extrusion_width
                        .get_value_for_movement_type(&move_type),
                });
            }

            outer_chains.push(MoveChain {
                start_point: interior.0[0],
                moves,
                is_loop: true,
            });
        }

        let mut inner_chains = vec![];
        if walls_left != 0 {
            let rec_inset_poly = polygon.offset_from(
                if outer_perimeter {
                    settings.extrusion_width.interior_surface_perimeter
                } else {
                    settings.extrusion_width.interior_inner_perimeter
                } / -2.0,
            );

            for polygon_rec in rec_inset_poly {
                if let Some(mc) = inset_polygon_recursive(
                    &MultiPolygon::from(polygon_rec),
                    settings,
                    false,
                    walls_left - 1,
                    layer,
                ) {
                    inner_chains.push(mc);
                }
            }
        }

        if settings.inner_perimeters_first {
            move_chains.append(&mut inner_chains);
            move_chains.append(&mut outer_chains);
        } else {
            move_chains.append(&mut inner_chains);
            move_chains.append(&mut outer_chains);
        }
    }

    let mut full_moves = vec![];
    move_chains
        .first()
        .map(|mc| mc.start_point)
        .map(|starting_point| {
            for mut chain in move_chains {
                full_moves.push(Move {
                    end: chain.start_point,
                    move_type: MoveType::Travel,
                    width: 0.0,
                });
                full_moves.append(&mut chain.moves)
            }

            MoveChain {
                moves: full_moves,
                start_point: starting_point,
                is_loop: true,
            }
        })
}
