// src/gui/progress.rs
use std::sync::{ Arc, Mutex };
use crate::progress::Progress;

pub struct GuiProgress {
    status: Arc<Mutex<String>>,
    done: usize,
    total: usize,
}

impl GuiProgress {
    pub fn new(status: Arc<Mutex<String>>) -> Self {
        Self { status, done: 0, total: 0 }
    }
    fn set_status(&self, msg: impl Into<String>) {
        let text = msg.into();
        *self.status.lock().unwrap() = text;
    }
}

impl Progress for GuiProgress {
    fn begin(&mut self, total: usize) {
        self.total = total;
    }
    fn log(&mut self, msg: &str) {
        self.set_status(s!(msg));
    }
    fn item_done(&mut self, team_id: u32) {
        self.done += 1;
        self.set_status(format!("Fetched team {} ({}/{})", team_id, self.done, self.total));
    }
    fn finish(&mut self) {
        if self.total == 0 {
            self.set_status(s!("Fetch complete")); // no counts if we never began
        } else {
            self.set_status(format!("Fetch complete ({}/{})", self.done, self.total));
        }
    }
}