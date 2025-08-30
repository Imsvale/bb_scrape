// src/gui/actions/copy.rs
use eframe::egui;
use crate::{gui::app::App, file};

pub fn copy(app: &mut App, ui_ctx: &egui::Context) {
    
    if app.row_ix.is_empty() {
        app.status("Nothing to copy");
        logd!("Copy: Clicked, but there's nothing to copy");
        return;
    }

    let page = app.current_page();

    let txt = {
        let Some(raw_ds) = super::current_raw(app) else {
            app.status("Nothing to copy (no cached data)");
            logd!("Copy: Clicked, but there's no cached dataset");
            return;
        };

        // Clipboard path: small clone of just the selected rows.
        let selected_rows: Vec<Vec<String>> = app
            .row_ix
            .iter()
            .filter_map(|&ix| raw_ds.rows.get(ix).cloned())
            .collect();

        let (h, r) = page.view_for_export(&app.state, &raw_ds.headers, &selected_rows);
        logf!(
            "Copy: page={:?}, rows={}, headers={}",
            page.kind(),
            r.len(),
            h.as_ref().map(|x| x.len()).unwrap_or(0)
        );

        file::to_export_string(&app.state.options, &h, &r)
    };

    ui_ctx.copy_text(txt);
    app.status("Copied to clipboard");
}