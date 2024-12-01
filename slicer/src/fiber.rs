use std::collections::HashMap;

use geo::{EuclideanDistance, EuclideanLength, Line, ToDegrees, ToRadians};

use crate::{mask::ObjectMask, FiberSettings, MovePrintType, MoveType, Settings};

pub trait ApplyFibers {
    fn apply_fibers(&mut self, settings_map: HashMap<MovePrintType, FiberSettings>);
}

impl ApplyFibers for ObjectMask {
    fn apply_fibers(&mut self, settings_map: HashMap<MovePrintType, FiberSettings>) {
        for (print_type, fiber_settings) in settings_map.iter() {
            for layer in self.layers.iter_mut() {
                let mut already_traced = 0.0;
                let mut already_traced_with_fibers = 0.0;

                for chain in layer.chains.iter_mut() {
                    let mut start = chain.start_point;

                    for index in 0..chain.moves.len() {
                        if MoveType::WithoutFiber(*print_type) == chain.moves[index].move_type {
                            let distance = chain.moves[index].end.euclidean_distance(&start);

                            if distance >= fiber_settings.min_length {
                                if already_traced * fiber_settings.percentage
                                    >= already_traced_with_fibers
                                {
                                    chain.moves[index].move_type = MoveType::WithFiber(
                                        chain.moves[index].move_type.print_type().unwrap(),
                                    )
                                }
                            } else {
                                let mut last_line = Line::new(start, chain.moves[index].end);
                                let mut distance = 0.0;
                                let mut last_index = None;

                                for index in (index + 1)..chain.moves.len() {
                                    if MoveType::WithoutFiber(*print_type)
                                        == chain.moves[index].move_type
                                    {
                                        let line = Line::new(last_line.end, chain.moves[index].end);

                                        let angle = angle_between_lines(last_line, line);

                                        if angle >= fiber_settings.max_angle.to_radians() {
                                            last_index = Some(index);
                                            break;
                                        } else {
                                            distance += line.euclidean_length();
                                        }

                                        last_line = line;
                                    } else {
                                        last_index = Some(index);
                                        break;
                                    }
                                }

                                if distance >= fiber_settings.min_length {
                                    if already_traced * fiber_settings.percentage
                                        >= already_traced_with_fibers
                                    {
                                        chain.moves[index].move_type = MoveType::WithFiber(
                                            chain.moves[index].move_type.print_type().unwrap(),
                                        );

                                        if let Some(last_index) = last_index {
                                            for index in (index + 1)..last_index {
                                                chain.moves[index].move_type = MoveType::WithFiber(
                                                    chain.moves[index]
                                                        .move_type
                                                        .print_type()
                                                        .unwrap(),
                                                );
                                            }
                                        }
                                    }
                                }
                            }

                            already_traced += distance;
                        } else if MoveType::WithFiber(*print_type) == chain.moves[index].move_type {
                            already_traced_with_fibers +=
                                chain.moves[index].end.euclidean_distance(&start);
                        }

                        start = chain.moves[index].end;
                    }
                }
            }
        }
    }
}

fn angle_between_lines(line1: Line<f32>, line2: Line<f32>) -> f32 {
    // Get direction vectors for the lines
    let vec1 = (line1.end.x - line1.start.x, line1.end.y - line1.start.y);
    let vec2 = (line2.end.x - line2.start.x, line2.end.y - line2.start.y);

    // Dot product and magnitudes
    let dot_product = vec1.0 * vec2.0 + vec1.1 * vec2.1;
    let magnitude1 = (vec1.0.powi(2) + vec1.1.powi(2)).sqrt();
    let magnitude2 = (vec2.0.powi(2) + vec2.1.powi(2)).sqrt();

    // Calculate the cosine of the angle
    let cos_theta = dot_product / (magnitude1 * magnitude2);

    // Clamp cos_theta to avoid numerical issues
    let cos_theta = cos_theta.clamp(-1.0, 1.0);

    cos_theta.acos().abs()
}
