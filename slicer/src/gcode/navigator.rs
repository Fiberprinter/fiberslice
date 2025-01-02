use std::collections::HashMap;

use crate::MoveId;

#[derive(Debug)]
struct MoveEntry {
    line: usize,
    layer: u32,
}

#[derive(Debug)]
pub struct Navigator {
    layer_indices: Vec<usize>,
    move_mapping: HashMap<MoveId, MoveEntry>,
}

impl Navigator {
    pub fn new() -> Self {
        Self {
            layer_indices: Vec::new(),
            move_mapping: HashMap::new(),
        }
    }

    pub fn get_layer_change_index(&self, layer: usize) -> Option<usize> {
        self.layer_indices.get(layer).copied()
    }

    pub fn get_trace_index(&self, id: &MoveId) -> Option<usize> {
        self.move_mapping.get(id).map(|o| o.line)
    }

    pub fn get_trace_layer(&self, id: &MoveId) -> Option<u32> {
        self.move_mapping.get(id).map(|o| o.layer)
    }

    pub(crate) fn record_layer_change(&mut self, line: usize) {
        self.layer_indices.push(line);
    }

    pub(crate) fn record_trace(&mut self, id: MoveId, line: usize) {
        self.move_mapping.insert(
            id,
            MoveEntry {
                line,
                layer: self.layer_indices.len() as u32,
            },
        );
    }
}
