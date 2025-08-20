// src/main.rs

pub mod cli;
pub mod gui;
pub mod core;
pub mod file;
pub mod specs;
pub mod csv;
pub mod runner;
pub mod params;
pub mod teams;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match cli::detect_mode()? {
        cli::Mode::Cli(params) => cli::run(params),
        cli::Mode::Gui => gui::run(),
    }
}