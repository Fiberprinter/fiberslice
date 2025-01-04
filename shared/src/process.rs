use std::sync::atomic::AtomicBool;

use atomic_float::AtomicF32;
use parking_lot::RwLock;

#[derive(Debug)]
pub struct Process {
    task: RwLock<String>,
    progress: AtomicF32,
    finished: AtomicBool,
    closed: AtomicBool,
    closed_safely: AtomicBool,
}

impl Process {
    pub fn new() -> Self {
        Self {
            task: RwLock::new(String::new()),
            progress: AtomicF32::new(0.0),
            finished: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            closed_safely: AtomicBool::new(false),
        }
    }

    pub fn set_progress(&self, progress: f32) {
        self.progress
            .store(progress, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_task(&self, task: String) {
        *self.task.write() = task;
    }

    pub fn get(&self) -> f32 {
        self.progress.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn finish(&self) {
        self.finished
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn close(&self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn close_safely(&self) {
        self.closed_safely
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn task(&self) -> String {
        self.task.read().clone()
    }

    pub fn is_finished(&self) -> bool {
        self.finished.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_closed_safely(&self) -> bool {
        self.closed_safely
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
