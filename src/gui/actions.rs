// src/gui/actions.rs
//
// Button "executive" actions for the export bar.
// Keeps UI code (layout) in export_bar.rs and the operational logic here.
//
// Design notes:
// - These functions update status/logs and mutate `app` as needed.
// - `do_copy` and `do_export` use the current page and the canonical RAW dataset.
// - Selection is respected using the same SelectionView indices as the table.
// - Per-team export groups by "what the page would show if only this team were selected".
//   For Game Results that means home OR away, which matches your filter.
//
// Dependencies you already have:
// - file::write_export_single (SingleFile path)
// - file::stream_write_table_to_path (the streaming helper we added earlier)
//   If you haven't added stream_write_table_to_path yet, drop me in and I'll inline a
//   minimal version, or temporarily fall back to materializing Vec<Vec<String>>.

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;

use crate::core::sanitize::sanitize_team_filename;
use crate::data::{Selection, SelectionView};
use crate::file::{self, ColumnProjection};
use crate::gui::app::App;
use crate::gui::progress::GuiProgress;
use crate::config::options::{ExportOptions, ExportType, PageKind};
use crate::store;

#[inline]
fn current_raw(app: &App) -> Option<&crate::store::DataSet> {
    let kind = app.current_page_kind();
    app.raw_data.get(&kind).map(|rd| rd.dataset())
}

pub fn copy(app: &mut App, ui_ctx: &egui::Context) {
    let kind = app.current_page_kind();
    let page = app.current_page();

    let Some(raw_ds) = current_raw(app) else {
        app.status("Nothing to copy (no cached data)");
        logd!("Copy: Clicked, but there's no cached dataset");
        return;
    };

    if app.row_ix.is_empty() {
        app.status("Nothing to copy");
        logd!("Copy: Clicked, but there's nothing to copy");
        return;
    }

    // Materialize only the selected rows (small, one-shot clone is fine for clipboard)
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

    let txt = file::to_export_string(&app.state.options, &h, &r);
    ui_ctx.copy_text(txt);
    app.status("Copied to clipboard");
}

pub fn export(app: &mut App) {
    let kind = app.current_page_kind();
    let page = app.current_page();
    let Some(raw_ds) = current_raw(app) else {
        app.status("Nothing to export (no cached data)");
        logd!("Export: Clicked, but there's no cached dataset");
        return;
    };

    if app.out_path_dirty {
        app.state.options.export.set_path(&app.out_path_text);
        logf!(
            "Export: Out path set → {}",
            app.state.options.export.out_path().display()
        );
        app.out_path_dirty = false;
    }

    let options = &app.state.options;
    let export  = &options.export;

    match export.export_type {
        ExportType::SingleFile => {
            if app.row_ix.is_empty() {
                app.status("Nothing to export");
                logd!("Export: Clicked, but there's nothing to export");
                return;
            }

            // Selection-aware SingleFile
            let selected_rows: Vec<Vec<String>> = app
                .row_ix
                .iter()
                .filter_map(|&ix| raw_ds.rows.get(ix).cloned())
                .collect();

            let (h, r) = page.view_for_export(&app.state, &raw_ds.headers, &selected_rows);
            logf!(
                "Export: Begin page={:?}, rows={}, headers={}, type=SingleFile",
                kind,
                r.len(),
                h.as_ref().map(|hh| hh.len()).unwrap_or(0)
            );

            match file::write_export_single(options, &h, &r) {
                Ok(path) => {
                    logf!("Export: OK count=1 last={}", path.display());
                    app.status(format!("Exported 1 file. Last: {}", path.display()));
                }
                Err(e) => {
                    loge!("Export: Error: {}", e);
                    app.status(format!("Export error: {e}"));
                }
            }
        }

        ExportType::PerTeam => {
            // Determine target teams: if "All" selected → all teams; else the subset.
            let selected_ids = &app.state.gui.selected_team_ids;
            let all_team_ids: Vec<u32> = app.teams.iter().map(|(id, _)| *id).collect();
            let ids_to_export: &[u32] = if selected_ids.len() == app.teams.len() {
                &all_team_ids
            } else {
                selected_ids
            };

            // Ensure the target directory exists
            let dir = export.out_path();
            if let Err(e) = fs::create_dir_all(&dir) {
                loge!("Export: create_dir_all failed: {}", e);
                app.status(format!("Export error: {}", e));
                return;
            }

            // Match the table/export rules (e.g., hide match id for Game Results)
            let proj = if matches!(kind, PageKind::GameResults)
                && !app.state.gui.game_results_show_match_id
            {
                ColumnProjection::DropLast
            } else {
                ColumnProjection::KeepAll
            };

            let mut written = 0usize;
            let mut last_path: Option<PathBuf> = None;

            for &team_id in ids_to_export {
                // Lookup name (skip unknown id)
                let team_name = match app.teams.iter().find(|(id, _)| *id == team_id) {
                    Some((_, name)) => name.as_str(),
                    None => continue,
                };

                // One-team Selection → SelectionView (home OR away for results, etc.)
                let one = [team_id];
                let sel  = Selection { ids: &one, teams: &app.teams };
                let view = SelectionView::from_raw(page, &app.raw_data[&kind], sel);

                if view.row_ix.is_empty() {
                    continue;
                }

                // Build file path: sanitized team name + selected format ext
                let stem = sanitize_team_filename(team_name, team_id);
                let ext  = export.format.ext();
                let file_name = if ext.is_empty() { stem.clone() } else { format!("{stem}.{ext}") };
                let path = ExportOptions::join_dir_and_filename(&dir, &file_name);

                // Stream headers + selected rows with projection, no row cloning
                match file::stream_write_table_to_path(
                    &path,
                    &raw_ds.headers,
                    &raw_ds.rows,
                    &view.row_ix,
                    export.delimiter(),
                    proj,
                ) {
                    Ok(_) => {
                        written += 1;
                        last_path = Some(path.clone());
                        logd!("Export: per-team OK → {}", path.display());
                    }
                    Err(e) => {
                        loge!("Export: per-team write failed {}: {}", path.display(), e);
                    }
                }
            }

            if written > 0 {
                if let Some(p) = last_path {
                    logf!("Export: OK count={} last={}", written, p.display());
                    app.status(format!("Exported {} file(s). Last: {}", written, p.display()));
                } else {
                    logf!("Export: OK count={}", written);
                    app.status(format!("Exported {} file(s).", written));
                }
            } else {
                app.status("Nothing to export");
                logd!("Export: PerTeam produced no files (no rows for chosen teams)");
            }
        }
    }
}

pub fn scrape(app: &mut App) {
    let page = app.current_page();
    let kind = page.kind();

    // Keep scrape options aligned (defensive)
    app.state.options.scrape.page = kind;
    app.sync_gui_selection_into_scrape();

    logf!("Scrape: Begin page={:?} teams={:?}", kind, app.state.options.scrape.teams);

    let mut prog = GuiProgress::new(app.status.clone());
    let ds_res = page.scrape(&app.state, Some(&mut prog));

    match ds_res {
        Ok(new_ds) => {
            logf!(
                "Scrape: OK page={:?}, rows={} headers={}",
                kind,
                new_ds.row_count(),
                new_ds.header_count()
            );

            // Update RAW
            let entry = app.raw_data.entry(kind)
                .or_insert_with(|| crate::data::RawData::new(kind, store::DataSet { headers: None, rows: Vec::new() }));
            entry.merge_from_scrape(page, new_ds);

            // Persist RAW
            if let Some(entry2) = app.raw_data.get_mut(&kind) {
                let save_ref = entry2.dataset_mut_for_io();
                match store::save_dataset(&kind, save_ref) {
                    Ok(p) => logf!("Cache: Saved {:?} → {}", kind, p.display()),
                    Err(e) => loge!("Cache: Save failed {:?}: {}", kind, e),
                }
            }

            // Invalidate any cached row indices for this page (data changed).
            app.row_ix_cache.retain(|(k, _), _| *k != kind);

            // Rebuild current display from canonical raw + selection
            app.rebuild_view();
            app.status("Ready");
        }
        Err(e) => {
            loge!("Scrape: Error page={:?}: {}", kind, e);
            app.status(format!("Error: {e}"));
        }
    }
}
