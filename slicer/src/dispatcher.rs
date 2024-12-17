use crate::{MoveChain, Settings};

pub fn dispatch_moves(chains: &mut Vec<MoveChain>, settings: &Settings) {
    for chain in chains.iter_mut() {
        for mov in chain.moves.iter_mut() {
            match mov.move_type {
                crate::MoveType::WithFiber(move_print_type) => todo!(),
                crate::MoveType::WithoutFiber(move_print_type) => todo!(),
                crate::MoveType::Travel => todo!(),
            }
        }
    }
}
