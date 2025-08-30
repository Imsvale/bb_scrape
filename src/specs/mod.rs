// src/specs/mod.rs
//! # Scraping “specs” module
//!
//! This module hosts the **page-specific scraping specifications** for the site.
//! Each spec focuses on a single page/endpoint and encodes *where the ground truth
//! lives in the HTML* and *how to extract it robustly*.
//!
//! ## What lives here
//! - **Pure HTML parsing** for remote pages (e.g., `/index.php`, `/players.php`, …).
//! - **Selector choice & precedence** (e.g., prefer league-table full names over
//!   mega-menu short names for teams).
//! - **Tolerant extraction** using `core::html` helpers (case-insensitive tag blocks,
//!   tag stripping, whitespace/entity normalization) and minimal hand-rolled scanning
//!   where it improves resilience.
//! - **Light shaping** of results into small “bundle” structs (headers + rows) or
//!   directly into `store::DataSet`-compatible shapes.
//!
//! ## What does **not** live here
//! - **Caching/persistence** (`store::load_dataset` / `store::save_dataset`) – that’s
//!   handled by higher layers (`src/teams.rs` facade or `scrape::collect_*`).
//! - **GUI concerns, filtering, or export formatting** – the GUI reads canonical data
//!   and applies view/export transforms elsewhere.
//! - **Cross-page merging/business logic** – specs only extract; merging lives with the
//!   page owner (e.g., `Page::merge` implementations).
//!
//! ## Typical call chain
//! ```text
//! GUI / runner → scrape::collect_* → specs::<page>::fetch()
//!                                ↘  returns headers+rows bundle
//!                    store::save_dataset (outside of specs)
//! ```
//!
//! ## Conventions & invariants
//! - **Case-insensitive** tag detection; avoid brittle full-document regexes.
//! - Prefer **local scanning within known blocks** (`<table>…</table>`, `<td class="namecheck">…`).
//! - Return **stable column shapes** per page (documented in each spec) so the rest of
//!   the pipeline can rely on them (e.g., Teams = `[Id, Team]`, Id is `u32`).
//! - **No logging spam**; keep logs informative when selection precedence matters
//!   (e.g., “League table missing – falling back to mega-menu”).
//!
//! ## Current specs (non-exhaustive)
//! - `teams` – Canonical team list from the *league table* on `index.php`, with a
//!   fallback to the *mega-menu* when needed (full vs short names).
//! - (future) `players`, `game_results`, etc., each constrained to one page’s HTML.
//!
//! ## Testing notes
//! - Specs should be testable **offline** against captured fixtures (saved HTML).
//! - Keep selectors resilient to whitespace, attribute order, and harmless markup noise.
//!
//! In short: **`specs` knows how to read the pages.** Other layers decide when to
//! scrape, how to cache, and how to present/export.
pub mod teams;
pub mod players;
pub mod game_results;
// pub mod career_stats; 
// pub mod season_stats; 
// pub mod injuries;
