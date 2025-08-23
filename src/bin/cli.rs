// src/bin/cli.rs
use bb_scrape::config::state::AppState;
use bb_scrape::cli::run;


fn main() {
    color_eyre::install().ok();
    if let Err(e) = run(AppState::default()) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
