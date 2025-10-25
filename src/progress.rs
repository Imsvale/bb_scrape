// src/progress.rs

/// Lightweight progress reporting used by long-running operations (scrape/export).
/// Frontends (GUI/CLI) implement this to surface status to users.
pub trait Progress {
    /// Called at the start with the total number of items (if known).
    fn begin(&mut self, _total: usize) {}

    /// Free-form status line for human eyes.
    fn log(&mut self, _msg: &str) {}

    /// Called when one logical unit completes successfully (e.g., a team ID was scraped).
    fn item_done(&mut self, _id: u32, _team_name: &str) {}

    /// Called when one logical unit fails (e.g., a team scrape returned no data or errored).
    fn item_failed(&mut self, _id: u32, _team_name: &str) {}

    /// Called at the end, successful or not.
    fn finish(&mut self) {}
}

/// A no-op progress sink.
pub struct NullProgress;
impl Progress for NullProgress {}