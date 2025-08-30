// src/gui/actions/mod.rs
//
// Folder module facade: re-export public entrypoints.
// Submodules stay private; consumers only see actions::{copy,export,scrape}.

mod copy;    // src/gui/actions/copy.rs
mod export;  // src/gui/actions/export.rs
mod scrape;  // src/gui/actions/scrape.rs

pub use copy::copy;
pub use export::export;
pub use scrape::scrape;

use crate::{gui::app::App, store::DataSet};

#[inline]
pub(super) fn current_raw(app: &App) -> Option<&DataSet> {
    let kind = app.current_page_kind();
    app.raw_data.get(&kind).map(|rd| rd.dataset())
}