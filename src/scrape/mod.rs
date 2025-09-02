// src/scrape/mod.rs
mod scrape;
mod teams;
mod players;
mod game_results;
// pub mod career_stats; 
// pub mod season_stats; 
// pub mod injuries;
pub use scrape::list_teams;
pub use scrape::collect_teams;
pub use scrape::collect_players;
pub use scrape::collect_game_results;
