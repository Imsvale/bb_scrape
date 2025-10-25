// src/gui/progress.rs
use std::sync::{ Arc, Mutex };
use crate::progress::Progress;

pub struct GuiProgress {
    status: Arc<Mutex<String>>,
    done: usize,
    failed: usize,
    total: usize,
}

impl GuiProgress {
    pub fn new(status: Arc<Mutex<String>>) -> Self {
        Self { status, done: 0, failed: 0, total: 0 }
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
    fn item_done(&mut self, _team_id: u32, team_name: &str) {
        self.done += 1;
        let completed = self.done + self.failed;
        let failure_suffix = if self.failed > 0 {
            format!(" ({} failed)", self.failed)
        } else {
            String::new()
        };
        self.set_status(format!("[{}/{}] Fetched: {}{}", completed, self.total, team_name, failure_suffix));
    }
    fn item_failed(&mut self, _team_id: u32, team_name: &str) {
        self.failed += 1;
        let completed = self.done + self.failed;
        self.set_status(format!("[{}/{}] Failed: {} ({} failed)", completed, self.total, team_name, self.failed));
    }
    fn finish(&mut self) {
        if self.total == 0 {
            self.set_status(s!("Fetch complete")); // no counts if we never began
        } else {
            let failure_suffix = if self.failed > 0 {
                format!(" ({} failed)", self.failed)
            } else {
                String::new()
            };
            self.set_status(format!("Fetch complete ({}/{}){}", self.done, self.total, failure_suffix));
        }
    }
}