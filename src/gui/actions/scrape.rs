// src/gui/actions/scrape.rs
use std::thread::{self};

use crate::{
    config::options::PageKind::{self},
    data,
    gui::{self, app::App, progress::GuiProgress, pages::Page},
    progress::Progress,
    store,
};

pub enum ScrapeOutcome {
    Ok { kind: PageKind, ds: store::DataSet },
    Err { kind: PageKind, msg: String },
}

/// Ensure headers exist in a freshly scraped dataset by using the page's defaults when missing.
pub fn inject_headers_if_missing(page: &dyn Page, ds: &mut store::DataSet) {
    if ds.headers.is_none() {
        if let Some(hs) = page.default_headers() {
            ds.headers = Some(hs.iter().map(|s| s.to_string()).collect());
        }
    }
}

// Call this from the button click
pub fn scrape(app: &mut App) {
    let page   = app.current_page();
    let kind   = page.kind();
    let status = app.status.clone();

    // Paranoia
    debug_assert_eq!(
        app.state.options.scrape.page, kind,
        "scrape.page drifted from current tab"
    );

    // Align scrape options
    app.sync_gui_selection_into_scrape();

    // Snapshot just what we need (avoid borrowing App across threads)
    let state = app.state.clone();                  // If AppState: Clone
    let teams = app.teams.clone();                  // If needed by validation

    app.running = true;                    // ← enable spinner
    app.status("Waiting for server response…");
    logf!("Scrape: Begin page={:?} teams={:?}", kind, app.state.options.scrape.teams);

    let handle = thread::spawn(move || {
        let page = gui::router::page_for(&kind);
        // Progress into the same status line
        let mut gp = GuiProgress::new(status);
        // let prog: Option<&mut dyn Progress> = Some(&mut gp);

        // 1) → This is where the scrape happens ←
        let mut ds = match page.scrape(&state, Some(&mut gp)) {
            Ok(ds) => ds,
            Err(e) => return ScrapeOutcome::Err { kind, msg: e.to_string() },
        };

        // If the scraper didn't provide headers, inject page defaults so downstream
        // code (cache, export, UI) can rely on headers being present.
        inject_headers_if_missing(page, &mut ds);

        // 1a) Ensure non-empty
        if ds.row_count() == 0 {
            return ScrapeOutcome::Err { 
                kind, 
                msg: "Scrape returned no rows".into() };
            };
        

        // Page-level validation (uses teams if your impl needs it)
        if let Err(msg) = page.validate_scrape(&state, &teams, &ds) {
            return ScrapeOutcome::Err { kind, msg: format!("Validation failed: {msg}") };
        }

        let page_text = match kind {
            PageKind::Players       => "players",
            PageKind::GameResults   => "games",
            PageKind::Teams         => "teams",
            PageKind::SeasonStats   => "season stats",
            PageKind::CareerStats   => "career stats",
            PageKind::Injuries      => "injury events",
        };

        gp.log(&format!("Found {} {}", ds.row_count(), page_text));

        ScrapeOutcome::Ok { kind, ds }


    });

    app.scrape_handle = Some(handle);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::state::AppState, store::DataSet};
    use std::error::Error;

    struct DummyPage;
    impl Page for DummyPage {
        fn title(&self) -> &'static str { "Dummy" }
        fn kind(&self) -> PageKind { PageKind::GameResults }
        fn default_headers(&self) -> Option<&'static [&'static str]> {
            Some(&["A","B","C"]) }
        fn scrape(&self, _s: &AppState, _p: Option<&mut dyn Progress>) -> Result<DataSet, Box<dyn Error>> {
            Ok(DataSet { headers: None, rows: vec![vec!["1".into(),"2".into(),"3".into()]] })
        }
    }

    #[test]
    fn injects_headers_when_missing() {
        let page = DummyPage;
        let mut ds = DataSet { headers: None, rows: vec![vec!["x".into()]] };
        inject_headers_if_missing(&page, &mut ds);
        let expected: Vec<String> = ["A","B","C"].iter().map(|s| s.to_string()).collect();
        assert_eq!(ds.headers.as_ref().unwrap(), &expected);
    }
}

// Call this once per frame (early in your update)
pub fn poll(app: &mut App) {
    let Some(handle) = app.scrape_handle.as_ref() else { return; };

    if !handle.is_finished() {
        // still working; keep the spinner alive
        return;
    }

    // finished: join and consume the handle
    let outcome = app.scrape_handle.take().unwrap().join();
    app.running = false;

    match outcome {
        Ok(ScrapeOutcome::Ok { kind, ds: new_ds }) => {
            // accept into cache
            let page = app.current_page(); // router page for `kind`
            let entry = app.raw_data.entry(kind)
                .or_insert_with(|| data::RawData::new(kind, store::DataSet { headers: None, rows: Vec::new() }));
            entry.merge_from_scrape(page, new_ds);

            // persist
            if let Some(entry2) = app.raw_data.get_mut(&kind) {
                let save_ref = entry2.dataset_mut_for_io();
                match store::save_dataset(&kind, save_ref) {
                    Ok(p) => logf!("Cache: Saved {:?} → {}", kind, p.display()),
                    Err(e) => loge!("Cache: Save failed {:?}: {}", kind, e),
                }
            }

            // invalidate row-index cache for this page + rebuild view
            app.row_ix_cache.retain(|(k, _), _| *k != kind);
            app.rebuild_view();
            // app.status("Ready");
        }
        Ok(ScrapeOutcome::Err { msg, .. }) => {
            app.status(msg);
        }
        Err(e) => {
            app.status(format!("Worker panicked: {e:?}"));
        }
    }
}
