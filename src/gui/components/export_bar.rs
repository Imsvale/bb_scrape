// src/gui/components/export_bar.rs

use eframe::egui;
use crate::{
    gui::app::App,
    config::options::{
        ExportFormat,
        ExportType::{PerTeam, SingleFile},
        PageKind,
    },
    file, store,
};
use crate::data::{RawData, FilteredData};

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiFormat { Csv, Tsv }

pub fn draw(ui: &mut egui::Ui, app: &mut App) {
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
            if !app.out_path_dirty {
                app.out_path_text = export.out_path().to_string_lossy().into_owned();
            }
        }

        let before_headers = export.include_headers;
        ui.checkbox(&mut export.include_headers, "Include headers");
        if export.include_headers != before_headers {
            logf!("UI: Include_headers → {}", export.include_headers);
        }
    }

    // --- Page-specific controls (after headers) ---
    let cur_kind = app.current_page_kind();
    {
        let page = app.current_page();
        let _changed = page.draw_controls(ui, &mut app.state);
        // Display stays literal; no projection or rebuild needed here.
    }

    let export = &mut app.state.options.export;

    // --- Per-team toggle + Output field ---
    let per_team_allowed = matches!(cur_kind, PageKind::Players);

    ui.horizontal(|ui| {
        if per_team_allowed {
            let mut single = matches!(export.export_type, SingleFile);
            if ui.checkbox(&mut single, "All teams in one file").changed() {
                export.export_type = if single { SingleFile } else { PerTeam };
                if !app.out_path_dirty {
                    app.out_path_text = export.out_path().to_string_lossy().into_owned();
                }
                logf!("UI: export_type → {:?}", export.export_type);
            }
        } else {
            export.export_type = SingleFile;
            ui.label("All teams in one file");
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
        let mut get_raw = || {
            app.raw_data.get(&cur_kind).map(|rd| rd.dataset().clone())
        };

        // Copy
        if ui.button("Copy").clicked() {
            if app.rows.is_empty() {
                app.status("Nothing to copy");
                logd!("Copy: Clicked, but there's nothing to copy");
            } else {
                let page = app.current_page();

                if let Some(raw_ds) = get_raw() {
                    let (h, r) = page.view_for_export(&app.state, &raw_ds.headers, &raw_ds.rows);
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
        }

        // Export
        if ui.button("Export").clicked() {
            if app.rows.is_empty() {
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

                let page = app.current_page();

                if let Some(raw_ds) = get_raw() {
                    let (h, r) = page.view_for_export(&app.state, &raw_ds.headers, &raw_ds.rows);

                    let options = &app.state.options;
                    let export = &options.export;

                    logf!(
                        "Export: Begin page={:?}, rows={}, headers={}, type={:?}",
                        cur_kind,
                        r.len(),
                        h.as_ref().map(|hh| hh.len()).unwrap_or(0),
                        export.export_type
                    );

                    let res: Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> =
                        match export.export_type {
                            SingleFile => file::write_export_single(options, &h, &r).map(|p| vec![p]),
                            PerTeam => file::write_export_per_team(options, &h, &r, 3),
                        };

                    match res {
                        Ok(paths) => {
                            if let Some(last) = paths.last() {
                                logf!("Export: OK count={} last={}", paths.len(), last.display());
                                app.status(format!(
                                    "Exported {} file(s). Last: {}",
                                    paths.len(),
                                    last.display()
                                ));
                            } else {
                                logf!("Export: OK count=0");
                                app.status("Export done");
                            }
                        }
                        Err(e) => {
                            loge!("Export: Error: {}", e);
                            app.status(format!("Export error: {e}"));
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

            let mut prog = crate::gui::progress::GuiProgress::new(app.status.clone());
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

                    // Refresh CURRENT display literally (raw + selection filter)
                    if let Some(raw) = app.raw_data.get(&kind) {
                        let fd = FilteredData::from_raw(page, raw, &app.state.gui.selected_team_ids, &app.teams);
                        app.headers = fd.headers_owned();
                        app.rows = fd.to_owned_rows();
                    }

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
