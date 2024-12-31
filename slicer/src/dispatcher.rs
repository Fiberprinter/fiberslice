use geo::{EuclideanDistance, EuclideanLength, Line};
use log::info;

use crate::{MoveChain, MoveType, Settings};

/*
pub fn dispatch_fiber_moves(chains: &mut Vec<MoveChain>, settings: &Settings) {
    let mut trace_distance = 0.0;
    let mut fiber_distance = 0.0;

    info!("Dispatching fiber moves");

    let fiber_settings = settings.fiber.clone();

    info!("Fiber settings: {:?}", fiber_settings);
    info!("Chains: {:?}", chains.len());

    for chain in chains.iter_mut() {
        let start = chain.start_point;

        for index in 0..chain.moves.len() {
            let distance = chain.moves[index].end.euclidean_distance(&start);

            match chain.moves[index].move_type {
                MoveType::WithFiber(_) => fiber_distance += distance,
                MoveType::WithoutFiber(_) => {
                    if distance >= fiber_settings.min_length {
                        if (trace_distance + fiber_distance) * fiber_settings.percentage
                            >= fiber_distance
                        {
                            chain.moves[index].move_type = MoveType::WithFiber(
                                chain.moves[index].move_type.print_type().unwrap(),
                            );

                            info!("Fiber move: {:?}", chain.moves[index].move_type);

                            fiber_distance += distance;
                        } else {
                            trace_distance += distance;
                        }
                    } else {
                        let mut last_line = Line::new(start, chain.moves[index].end);
                        let mut distance = 0.0;
                        let mut last_index = None;

                        for index in (index + 1)..chain.moves.len() {
                            let line = Line::new(last_line.end, chain.moves[index].end);

                            let angle = angle_between_lines(last_line, line);

                            if angle >= fiber_settings.max_angle.to_radians() {
                                last_index = Some(index);
                                break;
                            } else {
                                distance += line.euclidean_length();
                            }

                            last_line = line;
                        }

                        if distance >= fiber_settings.min_length {
                            if (trace_distance + fiber_distance) * fiber_settings.percentage
                                >= fiber_distance
                            {
                                chain.moves[index].move_type = MoveType::WithFiber(
                                    chain.moves[index].move_type.print_type().unwrap(),
                                );

                                if let Some(last_index) = last_index {
                                    for index in (index + 1)..last_index {
                                        chain.moves[index].move_type = MoveType::WithFiber(
                                            chain.moves[index].move_type.print_type().unwrap(),
                                        );
                                    }
                                }
                            }

                            fiber_distance += distance;
                        } else {
                            trace_distance += distance;
                        }
                    }
                }
                _ => {}
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
*/
