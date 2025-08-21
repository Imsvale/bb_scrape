// src/main.rs

mod core;
mod specs;

mod cli;
mod csv;
mod file;
mod gui;
mod params;
mod runner;
mod store;
mod teams;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match cli::detect_mode()? {
        cli::Mode::Cli(params) => cli::run(params),
        cli::Mode::Gui(params) => gui::run(params),
    }
}