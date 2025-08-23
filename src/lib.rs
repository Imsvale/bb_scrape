// src/lib.rs
// #![allow(dead_code)]
// #![allow(unused)]

#[macro_use]
pub mod macros;

pub mod cli;
pub mod config;
pub mod core;
pub mod specs;

pub mod file;
pub mod gui;
pub mod progress;
pub mod scrape;
pub mod store;
pub mod teams;