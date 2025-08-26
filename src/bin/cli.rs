// src/bin/cli.rs
use bb_scrape::cli;

fn main() {
    #[cfg(feature = "cli")]
    { let _ = color_eyre::install(); }

    if let Err(e) = cli::run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
