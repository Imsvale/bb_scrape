// src/main.rs

pub mod core;
pub mod specs;

pub mod cli;
pub mod csv;
pub mod file;
pub mod gui;
pub mod params;
pub mod runner;
pub mod store;
pub mod teams;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match cli::detect_mode()? {
        cli::Mode::Cli(params) => cli::run(params),
        cli::Mode::Gui(params) => gui::run(params),
    }
}