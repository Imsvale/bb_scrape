// src/gui/actions/export.rs
use crate::{gui::app::App, config::options::{ExportOptions, ExportType, PageKind},
            data::{Selection, SelectionView}, core::sanitize::sanitize_team_filename,
            file::{self, ColumnProjection}};
use std::{fs, path::PathBuf};
use super::current_raw;

pub fn export(app: &mut App) {
    // normalize out_path first (mutates app) before any &app borrows
    if app.out_path_dirty {
        app.state.options.export.set_path(&app.out_path_text);
        logf!(
            "Export: Out path set → {}",
            app.state.options.export.out_path().display()
        );
        app.out_path_dirty = false;
    }

    let kind   =  app.current_page_kind();
    let page   =  app.current_page();
    let opts   = &app.state.options;
    let export = &opts.export;

    let status_msg = match export.export_type {
        ExportType::SingleFile => {
            if app.row_ix.is_empty() {
                logd!("Export: Clicked, but there's nothing to export");
                "Nothing to export".to_string()
            } else if current_raw(app).is_none() {
                logd!("Export: Clicked, but there's no cached dataset");
                "Nothing to export (no cached data)".to_string()
            } else {
                // do all work with &DataSet inside this block
                let result: Result<PathBuf, Box<dyn std::error::Error>> = {
                    let raw_ds = current_raw(app).unwrap();

                    let selected_rows: Vec<Vec<String>> = app
                        .row_ix
                        .iter()
                        .filter_map(|&ix| raw_ds.rows.get(ix).cloned())
                        .collect();

                    let (headers, rows) = page.view_for_export(&app.state, &raw_ds.headers, &selected_rows);

                    logf!(
                        "Export: Begin page={:?}, rows={}, headers={}, type=SingleFile",
                        kind,
                        rows.len(),
                        headers.as_ref().map(|hh| hh.len()).unwrap_or(0)
                    );

                    file::write_export_single(opts, &headers, &rows)
                };

                match result {
                    Ok(path) => {
                        logf!("Export: OK count=1 last={}", path.display());
                        format!("Exported 1 file. Last: {}", path.display())
                    }
                    Err(e) => {
                        loge!("Export: Error: {}", e);
                        format!("Export error: {e}")
                    }
                }
            }
        }

        ExportType::PerTeam => {
            if current_raw(app).is_none() {
                logd!("Export: PerTeam but no cached dataset");
                s!("Nothing to export (no cached data)")
            } else {
                // keep all borrows immutable inside this block
                let (written, last): (usize, Option<PathBuf>) = {
                    let raw_ds = current_raw(app).unwrap();

                    // target teams: if ALL selected → all; else the subset
                    let selected_ids = &app.state.gui.selected_team_ids;
                    let all_ids: Vec<u32> = app.teams.iter().map(|(id, _)| *id).collect();
                    let ids_to_export: &[u32] = if selected_ids.len() == app.teams.len() {
                        &all_ids
                    } else {
                        selected_ids
                    };

                    // ensure target dir
                    let dir = export.out_path();
                    if let Err(e) = fs::create_dir_all(&dir) {
                        loge!("Export: create_dir_all failed: {}", e);
                        return app.status(&format!("Export error: {e}")); // early status + return
                    }

                    // column projection matches table toggle
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
                        let team_name = match app.teams.iter().find(|(id, _)| *id == team_id) {
                            Some((_, name)) => name.as_str(),
                            None => continue,
                        };

                        // one-team selection view
                        let one = [team_id];
                        let sel  = Selection { ids: &one, teams: &app.teams };
                        let view = SelectionView::from_raw(page, &app.raw_data[&kind], sel);

                        if view.row_ix.is_empty() {
                            continue;
                        }

                        // file path
                        let stem = sanitize_team_filename(team_name, team_id);
                        let ext  = export.format.ext();
                        let file_name = if ext.is_empty() { stem.clone() } else { format!("{stem}.{ext}") };
                        let path = ExportOptions::join_dir_and_filename(&dir, &file_name);

                        // stream selection → file (no row cloning)
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

                    (written, last_path)
                };

                if written > 0 {
                    if let Some(p) = last {
                        logf!("Export: OK count={} last={}", written, p.display());
                        format!("Exported {} file(s). Last: {}", written, p.display())
                    } else {
                        logf!("Export: OK count={}", written);
                        format!("Exported {} file(s).", written)
                    }
                } else {
                    logd!("Export: PerTeam produced no files (no rows for chosen teams)");
                    "Nothing to export".to_string()
                }
            }
        }
    };

    // mutate app only after the dataset borrows are gone
    app.status(status_msg);
}
