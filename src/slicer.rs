use shared::{object::ObjectMesh, SliceInput};
use slicer::{Mask, MaskSettings, Settings};
use tokio::task::JoinHandle;

use crate::{
    ui::{api::trim_text, custom_toasts::SLICING_PROGRESS},
    GlobalState, RootEvent,
};

#[derive(Debug, Default)]
pub struct Slicer {
    pub settings: Settings,
    handle: Option<JoinHandle<()>>,
}

impl Slicer {
    pub fn slice(&mut self, global_state: &GlobalState<RootEvent>) {
        if let Some(handle) = self.handle.take() {
            if !handle.is_finished() {
                return;
            }
        }

        let settings = self.settings.clone();
        let objects: Vec<ObjectMesh> = global_state.viewer.prepare_objects(&settings);
        let masks: Vec<Mask> = global_state.viewer.prepare_masks(&settings);

        let global_state = global_state.clone();

        let handle = tokio::spawn(async move {
            let process = global_state
                .progress_tracker
                .write()
                .add(SLICING_PROGRESS, trim_text::<20, 4>("Slicing model"));

            let result = slicer::slice(SliceInput { objects, masks }, &settings, &process)
                .expect("Failed to slice model");

            global_state.viewer.load_sliced(result, process);

            global_state
                .ui_event_writer
                .send(crate::ui::UiEvent::ShowSuccess(
                    "Slicing finished".to_string(),
                ));
        });

        self.handle = Some(handle);
    }
}
