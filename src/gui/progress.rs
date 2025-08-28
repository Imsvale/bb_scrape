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
        *self.status.lock().unwrap() = msg.into();
    }
}

impl Progress for GuiProgress {
    fn begin(&mut self, total: usize) {
        self.total = total;
        self.set_status(format!("Starting… {} team(s)", total));
    }
    fn log(&mut self, msg: &str) {
        self.set_status(msg.to_string());
    }
    fn item_done(&mut self, team_id: u32) {
        self.done += 1;
        self.set_status(format!("Fetched team {} ({}/{})", team_id, self.done, self.total));
    }
    fn finish(&mut self) {
        self.set_status("Fetch complete".to_string());
    }
}