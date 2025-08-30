// src/gui/components/export_bar.rs

use std::fs;
use std::path::PathBuf;
use eframe::egui::{self, Checkbox};
use crate::{
    data::RawData,
    gui::app::App,
    gui::progress::GuiProgress,
    config::options::{
        ExportFormat,
        ExportType::{PerTeam, SingleFile},
        ExportOptions, PageKind,
    },
    file, store,
};
use crate::core::sanitize::sanitize_team_filename;
use crate::data::{Selection, SelectionView};
use file::ColumnProjection;

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiFormat { Csv, Tsv }

pub fn draw(ui: &mut egui::Ui, app: &mut App) {

    let page = app.current_page();
    let per_team_applicable = page.per_team_applicable();
    let cur_kind = app.current_page_kind();
    {
        let export = &mut app.state.options.export;

        // --- Format + Include headers ---
        let prev_fmt = match export.format {
            ExportFormat::Csv => UiFormat::Csv,
            ExportFormat::Tsv => UiFormat::Tsv,
        };
        let mut fmt = prev_fmt;

        ui.horizontal(|ui| {
            ui.label("Format:");
            ui.selectable_value(&mut fmt, UiFormat::Csv, "CSV");
            ui.selectable_value(&mut fmt, UiFormat::Tsv, "TSV");
        });

        if fmt != prev_fmt {
            export.format = match fmt {
                UiFormat::Csv => ExportFormat::Csv,
                UiFormat::Tsv => ExportFormat::Tsv,
            };
            logf!("UI: Export format → {:?}", export.format);

            // If the entire path is still default and the user hasn't typed,
            // refresh the text field to reflect the new extension.
            if !app.out_path_dirty && export.is_fully_default_for(cur_kind) {
                app.out_path_text = export.out_path().to_string_lossy().into_owned();
                logd!("UI: out_path_text refreshed to match format (default path)");
            }
        }

        let before_headers = export.include_headers;
        ui.checkbox(&mut export.include_headers, "Include headers");
        if export.include_headers != before_headers {
            logf!("UI: Include_headers → {}", export.include_headers);
        }
    }

    // Page-specific controls
    let _changed = page.draw_controls(ui, &mut app.state);
    // Needs re-binding because of mut/borrow conflict from the line above
    let export = &mut app.state.options.export;

    // --- Per-team toggle + Output field ---
    ui.horizontal(|ui| {
        // Keep layout stable: always show the checkbox, gray it out if not applicable.
        let mut single = matches!(export.export_type, SingleFile);
        let changed = ui.add_enabled(
            per_team_applicable,
            Checkbox::new(&mut single, "All teams in one file"))
            .changed();

        // If the checkbox was interactable and changed, update the export type.
        if per_team_applicable && changed {
            export.export_type = if single { SingleFile } else { PerTeam };
            if !app.out_path_dirty {
                app.out_path_text = export.out_path().to_string_lossy().into_owned();
            }
            logf!("UI: export_type → {:?}", export.export_type);
        }

        // If not applicable, force SingleFile silently (no layout shift).
        if !per_team_applicable && !matches!(export.export_type, SingleFile) {
            export.export_type = SingleFile;
        }

        ui.label("Output:");
        if ui
            .add(egui::TextEdit::singleline(&mut app.out_path_text)
                .font(egui::TextStyle::Monospace))
            .changed()
        {
            app.out_path_dirty = true;
            logd!("UI: out_path_text changed (dirty=true) → {}", app.out_path_text);
        }
    });

    // --- Actions (Copy / Export / SCRAPE) ---
    ui.horizontal(|ui| {
        // Get RAW dataset for current page (canonical cache)
        let raw_opt = app.raw_data.get(&cur_kind).map(|rd| rd.dataset());

        // Copy
        if ui.button("Copy").clicked() {
            if app.row_ix.is_empty() {
                app.status("Nothing to copy");
                logd!("Copy: Clicked, but there's nothing to copy");
            } else if let Some(raw_ds) = raw_opt {
                let page = app.current_page();

                // Build a selected-rows buffer (clone only what we’re exporting)
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
                ui.ctx().copy_text(txt);
                app.status("Copied to clipboard");
            } else {
                app.status("Nothing to copy (no cached data)");
            }
            
        }

        // Export
        if ui.button("Export").clicked() {
            if app.row_ix.is_empty() && matches!(app.state.options.export.export_type, SingleFile) {
                app.status("Nothing to export");
                logd!("Export: Clicked, but there's nothing to export");
            } else {
                if app.out_path_dirty {
                    app.state.options.export.set_path(&app.out_path_text);
                    logf!(
                        "Export: Out path set → {}",
                        app.state.options.export.out_path().display()
                    );
                    app.out_path_dirty = false;
                }

                if let Some(raw_ds) = raw_opt {
                    let page    = app.current_page();
                    let options = &app.state.options;
                    let export  = &options.export;

                    match export.export_type {
                        SingleFile => {
                            // (existing SingleFile path)
                            let (h, r) = page.view_for_export(&app.state, &raw_ds.headers, &raw_ds.rows);
                            logf!(
                                "Export: Begin page={:?}, rows={}, headers={}, type=SingleFile",
                                cur_kind,
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

                        PerTeam => {
                            // Determine target teams: if "All" selected -> all, else selected subset.
                            let selected_ids = &app.state.gui.selected_team_ids;
                            let all_team_ids: Vec<u32> = app.teams.iter().map(|(id, _)| *id).collect();
                            let ids_to_export: &[u32] = if selected_ids.len() == app.teams.len() {
                                &all_team_ids
                            } else {
                                selected_ids
                            };

                            // Ensure the target directory exists (PerTeam out_path() is a dir)
                            let dir = export.out_path();
                            if let Err(e) = fs::create_dir_all(&dir) {
                                loge!("Export: create_dir_all failed: {}", e);
                                app.status(format!("Export error: {}", e));
                                return;
                            }

                            // Column projection mirrors the table/export rules
                            let proj = if matches!(cur_kind, PageKind::GameResults)
                                && !app.state.gui.game_results_show_match_id
                            {
                                ColumnProjection::DropLast
                            } else {
                                ColumnProjection::KeepAll
                            };

                            let mut written = 0usize;
                            let mut last_path: Option<PathBuf> = None;

                            for &team_id in ids_to_export {
                                // Lookup name (skip if unknown id)
                                let team_name = match app.teams.iter().find(|(id, _)| *id == team_id) {
                                    Some((_, name)) => name.as_str(),
                                    None => continue,
                                };

                                // Build a one-team Selection -> SelectionView
                                let one = [team_id];
                                let sel  = Selection { ids: &one, teams: &app.teams };
                                let view = SelectionView::from_raw(page, &app.raw_data[&cur_kind], sel);

                                if view.row_ix.is_empty() {
                                    continue; // no rows for this team, skip file
                                }

                                // Build file name: sanitized team name + current format extension
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
                } else {
                    app.status("Nothing to export (no cached data)");
                }
            }
        }

        // SCRAPE
        let red = egui::Color32::from_rgb(220, 30, 30);
        let black = egui::Color32::BLACK;
        if ui
            .add(
                egui::Button::new(egui::RichText::new("SCRAPE").color(black).strong())
                    .fill(red),
            )
            .clicked()
        {
            let page = app.current_page();
            let kind = page.kind();
            app.state.options.scrape.page = kind;
            app.sync_gui_selection_into_scrape();

            logf!("Scrape: Begin page={:?} teams={:?}", kind, app.state.options.scrape.teams);

            let mut prog = GuiProgress::new(app.status.clone());

            // Run the scrape! → Result(DataSet)
            let ds_res = page.scrape(&app.state, Some(&mut prog));

            // Evaluate scrape results
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
                        .or_insert_with(|| RawData::new(kind, store::DataSet { headers: None, rows: Vec::new() }));
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

        let status = app.status.lock().unwrap().clone();
        ui.label(format!("Status: {status}"));
    });
}
