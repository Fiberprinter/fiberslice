use geo::prelude::*;
use geo::*;

use glam::vec2;
use itertools::Itertools;

use crate::settings::LayerSettings;
use crate::{Move, MoveChain, MoveType, TraceType};

use super::polygon_operations::PolygonOperations;

pub fn determine_move_type(
    settings: &LayerSettings,
    number_of_walls: usize,
    wall: usize,
    layer: usize,
    trace_type: TraceType,
    wall_ranges: &[u32],
) -> MoveType {
    if settings.fiber.wall_pattern.is_enabled() {
        let wall = if !wall_ranges.is_empty() {
            wall_ranges
                .iter()
                .find(|wall_index| ((number_of_walls + 1 - wall) as u32) == **wall_index)
                .map(|wall| number_of_walls + 1 - (*wall as usize))
        } else {
            Some(number_of_walls + 1 - wall)
        };

        if let Some(wall) = wall {
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
                        let wall_cycle_length =
                            settings.fiber.wall_pattern.alternating_wall_spacing
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
    } else {
        MoveType::WithoutFiber(trace_type)
    }
}

pub fn seam<'a>(points: &'a [Coord<f32>]) -> Vec<&'a Coord<f32>> {
    if points.len() < 3 {
        return points.iter().collect();
    }

    let mut last_direction = None;
    let (mut max_angle, mut index) = (0.0, None);

    let start = points.last().unwrap();

    for (i, end) in points.iter().enumerate() {
        let start = vec2(start.x, start.y);
        let end = vec2(end.x, end.y);
        let direction = end - start;

        if let Some(last_direction) = last_direction {
            let angle = direction.angle_to(last_direction).abs();
            if angle > max_angle {
                max_angle = angle;

                index = Some(i);
            }
        } else {
            index = Some(i)
        }

        last_direction = Some(direction);
    }

    let wanted_start_index = index.unwrap_or(0);

    // rotate the iterator to the wanted start index
    points
        .iter()
        .cycle()
        .skip(wanted_start_index)
        .take(points.len())
        .collect()
}

pub fn inset_polygon_recursive(
    poly: &MultiPolygon<f32>,
    settings: &LayerSettings,
    outer_perimeter: bool,
    number_of_walls: usize,
    walls_left: usize,
    layer: usize,
    wall_ranges: &[u32],
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

        let seamed_poly = seam(&polygon.exterior().0);
        let start_point = seamed_poly[0].clone();

        let moves: Vec<Move> = seamed_poly
            .into_iter()
            .circular_tuple_windows::<(_, _)>()
            .map(|(&_start, &end)| {
                let move_type = if outer_perimeter {
                    determine_move_type(
                        settings,
                        number_of_walls,
                        walls_left + 1,
                        layer,
                        TraceType::WallOuter,
                        wall_ranges,
                    )
                } else {
                    determine_move_type(
                        settings,
                        number_of_walls,
                        walls_left + 1,
                        layer,
                        TraceType::WallInner,
                        wall_ranges,
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
            start_point,
            moves,
            is_loop: true,
        });

        for interior in polygon.interiors() {
            let mut moves = vec![];
            let move_type = if outer_perimeter {
                determine_move_type(
                    settings,
                    number_of_walls,
                    walls_left + 1,
                    layer,
                    TraceType::InteriorWallOuter,
                    wall_ranges,
                )
            } else {
                determine_move_type(
                    settings,
                    number_of_walls,
                    walls_left + 1,
                    layer,
                    TraceType::InteriorWallInner,
                    wall_ranges,
                )
            };

            let seamed_poly = seam(&interior.0);
            let start_point = seamed_poly[0].clone();

            for (&_start, &end) in seamed_poly.into_iter().circular_tuple_windows::<(_, _)>() {
                moves.push(Move {
                    end,
                    move_type,
                    width: settings
                        .extrusion_width
                        .get_value_for_movement_type(&move_type),
                });
            }

            outer_chains.push(MoveChain {
                start_point,
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
                    number_of_walls,
                    walls_left - 1,
                    layer,
                    wall_ranges,
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

#[test]
fn test_seam() {
    let points = [
        Coord { x: -10.6, y: 1.7 },
        Coord { x: -4.1, y: 6.7 },
        Coord { x: -5.0, y: -1.7 },
    ];

    println!("{:?}", points);
    println!("{:?}", seam(&points));

    panic!("Test not implemented");
}
