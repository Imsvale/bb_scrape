// src/lib.rs
// #![allow(dead_code)]
// #![allow(unused)]

#[macro_use] pub mod macros;
#[macro_use] pub mod log;

pub mod cli;
pub mod gui;

pub mod config;

pub mod core;
pub mod data;
pub mod file;
pub mod progress;
pub mod scrape;
pub mod specs;
pub mod store;
pub mod teams;