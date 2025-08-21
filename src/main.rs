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