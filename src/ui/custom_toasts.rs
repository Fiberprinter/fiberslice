use std::sync::Arc;

use egui_toast::Toast;

use log::info;
use shared::process::Process;

const PROGRESS_BAR_WIDTH: f32 = 250.0;

const STAY_DURATION_MS: u64 = 1000;

pub const OBJECT_LOAD_PROGRESS: u32 = 0;

pub fn object_load_progress(ui: &mut egui::Ui, toast: &mut Toast) -> egui::Response {
    let global_state = crate::GLOBAL_STATE.read();
    let global_state = global_state.as_ref().unwrap();

    global_state.progress_tracker.read_with_fn(|tracker| {
        match tracker.get(OBJECT_LOAD_PROGRESS, toast.get_name()) {
            Some(process) => show_progress(ui, toast, process),
            None => show_finished(ui, toast),
        }
    })
}

pub const SLICING_PROGRESS: u32 = 1;

pub fn slicing_progress(ui: &mut egui::Ui, toast: &mut Toast) -> egui::Response {
    let global_state = crate::GLOBAL_STATE.read();
    let global_state = global_state.as_ref().unwrap();

    global_state.progress_tracker.read_with_fn(|tracker| {
        match tracker.get(SLICING_PROGRESS, toast.get_name()) {
            Some(process) => show_progress(ui, toast, process),
            None => show_finished(ui, toast),
        }
    })
}

fn show_progress(ui: &mut egui::Ui, toast: &mut Toast, process: &Arc<Process>) -> egui::Response {
    egui::Frame::window(ui.style())
        .show(ui, |ui| {
            if process.is_finished() && !process.is_closed() {
                toast.options = toast.options.duration_in_millis(STAY_DURATION_MS);
                toast.options.show_progress = false;

                info!("Closing toast: {}", toast.get_name());
                process.close();
            }

            let progress = process.get();

            ui.label(toast.get_name());

            ui.separator();

            ui.label(process.task());

            ui.add(
                egui::widgets::ProgressBar::new(progress)
                    .show_percentage()
                    .animate(true)
                    .desired_width(PROGRESS_BAR_WIDTH),
            );
        })
        .response
}

fn show_finished(ui: &mut egui::Ui, toast: &mut Toast) -> egui::Response {
    egui::Frame::window(ui.style())
        .show(ui, |ui| {
            ui.label(toast.get_name());

            ui.separator();

            ui.label("Finished");

            ui.add(
                egui::widgets::ProgressBar::new(1.0)
                    .show_percentage()
                    .animate(true)
                    .desired_width(PROGRESS_BAR_WIDTH),
            );
        })
        .response
}
