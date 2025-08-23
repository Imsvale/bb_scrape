// src/progress.rs
/// Lightweight progress reporting used by long-running operations (scrape/export).
/// Frontends (GUI/CLI) implement this to surface status to users.
pub trait Progress {
    /// Called at the start with the total number of items (if known).
    fn begin(&mut self, _total: usize) {}

    /// Free-form status line for human eyes.
    fn log(&mut self, _msg: &str) {}

    /// Called when one logical unit completes (e.g., a team ID was scraped).
    fn item_done(&mut self, _id: u32) {}

    /// Called at the end, successful or not.
    fn finish(&mut self) {}
}

/// A no-op progress sink.
pub struct NullProgress;
impl Progress for NullProgress {}
