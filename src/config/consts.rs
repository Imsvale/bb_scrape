// src/config/consts.rs

// Net config
pub const HOST: &str = "dozerverse.com";
pub const PREFIX: &str = "/brutalball/";

// Local cache
pub const STORE_DIR: &str = ".store";
pub const STORE_SEP: char = ',';

// Export
pub const DEFAULT_OUT_DIR: &str ="out";
pub const DEFAULT_PLAYERS_SUBDIR: &str = "players";
pub const DEFAULT_FILE: &str = "all";

// Concurrency
pub const WORKERS: usize = 4;
pub const REQUEST_PAUSE_MS: u64 = 75; // be polite