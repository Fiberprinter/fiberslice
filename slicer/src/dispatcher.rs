use geo::EuclideanDistance;

use crate::{MoveChain, Settings};

pub fn dispatch_moves(chains: &mut Vec<MoveChain>, settings: &Settings) {
    let mut traces = 0.0;

    for chain in chains.iter_mut() {
        let start = chain.start_point;

        for mov in chain.moves.iter_mut() {
            let distance = mov.end.euclidean_distance(&start);

            match mov.move_type {
                crate::MoveType::WithFiber(move_print_type) => {}
                crate::MoveType::WithoutFiber(move_print_type) => traces += distance,
                crate::MoveType::Travel => todo!(),
            }
        }
    }
}
