use std::{collections::HashMap, sync::Arc};

use shared::process::Process;

use crate::{
    prelude::Shared,
    ui::custom_toasts::{OBJECT_LOAD_PROGRESS, SLICING_PROGRESS},
    GLOBAL_STATE,
};

#[derive(Debug, Default)]
pub struct ProcessTracker {
    map: HashMap<u32, HashMap<String, Arc<Process>>>,
}

impl ProcessTracker {
    pub fn new() -> Self {
        let mut map = HashMap::new();

        map.insert(OBJECT_LOAD_PROGRESS, HashMap::new());
        map.insert(SLICING_PROGRESS, HashMap::new());

        Self { map }
    }

    pub fn update(&mut self) {
        self.map.values_mut().for_each(|processes| {
            processes.retain(|_, process| !process.is_closed());
        });
    }

    pub fn add(&mut self, id: u32, mut name: String) -> Shared<Process> {
        let process = Shared::new(Process::new());

        {
            let inner_map = self.map.get(&id).unwrap();

            if inner_map.contains_key(&name) {
                name = self.find_unused_name(id, name);

                log::info!("Process with name already exists, using {}", name);
            }
        }

        self.map
            .entry(id)
            .or_default()
            .insert(name.clone(), process.clone());

        let global_state = GLOBAL_STATE.read();
        let global_state = global_state.as_ref().unwrap();

        global_state
            .ui_event_writer
            .send(crate::ui::UiEvent::ShowProgressBar(id, name));

        global_state.window.request_redraw();

        process
    }

    fn find_unused_name(&self, id: u32, old_name: String) -> String {
        let mut name = old_name.clone();

        let mut i = 1;

        while self.map[&id].contains_key(&name) {
            name = format!("{} ({})", old_name, i);

            i += 1;
        }

        name
    }

    pub fn get(&self, id: u32, name: &str) -> Option<&Arc<Process>> {
        self.map.get(&id)?.get(name)
    }
}
