// src/gui/actions/scrape.rs
use crate::{
    store,
    data::RawData,
    gui::app::App, 
    gui::progress::GuiProgress, 
};

pub fn scrape(app: &mut App) {
    let page = app.current_page();
    let kind = page.kind();

    // Paranoia
    debug_assert_eq!(
        app.state.options.scrape.page, kind,
        "scrape.page drifted from current tab"
    );

    // Align scrape options
    app.sync_gui_selection_into_scrape();

    logf!("Scrape: Begin page={:?} teams={:?}", kind, app.state.options.scrape.teams);

    let mut prog = GuiProgress::new(app.status.clone());

    // → This is where the scrape happens ←
    let ds_res = page.scrape(&app.state, Some(&mut prog));

    match ds_res {
        Ok(new_ds) => {
            logf!(
                "Scrape: OK page={:?}, rows={} headers={}",
                kind,
                new_ds.row_count(),
                new_ds.header_count()
            );

            // Update: Scraped data → memory
            let entry = app.raw_data.entry(kind)
                .or_insert_with(|| RawData::new(kind, store::DataSet { headers: None, rows: Vec::new() }));
            entry.merge_from_scrape(page, new_ds);

            // Save scraped data to disk
            if let Some(entry2) = app.raw_data.get_mut(&kind) {
                let save_ref = entry2.dataset_mut_for_io();
                match store::save_dataset(&kind, save_ref) {
                    Ok(p) => logf!("Cache: Saved {:?} → {}", kind, p.display()),
                    Err(e) => loge!("Cache: Save failed {:?}: {}", kind, e),
                }
            }

            // data changed → invalidate row index cache for this page
            app.row_ix_cache.retain(|(k, _), _| *k != kind);

            // rebuild table view
            app.rebuild_view();
            app.status("Ready");
        }
        Err(e) => {
            loge!("Scrape: Error page={:?}: {}", kind, e);
            app.status(format!("Error: {e}"));
        }
    }
}
