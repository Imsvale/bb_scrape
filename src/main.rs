// src/main.rs
#![allow(dead_code)]
#![allow(unused)]

#[macro_use]
mod macros;

mod config;
mod core;
mod specs;

mod cli;
mod csv;
mod file;
mod gui;

mod runner;
mod store;
mod teams;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match cli::detect_mode()? {
        cli::Mode::Cli(app_state) => cli::run(app_state),
        cli::Mode::Gui(app_state) => gui::run(app_state),
    }
}