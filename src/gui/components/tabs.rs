// src/gui/components/tabs.rs
//
// Renders the top tabs and performs the tab switch itself.
// Design: display is literal — built from canonical raw_data (cache) plus
// selection filtering. No option-based projection here, and no view cache.
// On tab switch, we load the page's dataset (if any), apply the page's
// filter_rows_for_selection, and set app.headers/app.rows accordingly.

use eframe::egui;
use std::path::{Path, PathBuf};
use crate::gui::{app::App, router};
use crate::config::options::{ExportOptions, ExportType};

fn norm(p: &Path) -> PathBuf { p.components().collect() }

pub fn draw(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        let pages = router::all_pages();
        let cur = app.current_index();

        for (idx, page) in pages.iter().enumerate() {
            let selected = idx == cur;

            if ui.selectable_label(selected, page.title()).clicked() && !selected {
                let prev = app.current_page_kind();
                app.set_current_index(idx);
                let new_kind = page.kind();
                logf!("UI: Tab switch {:?} → {:?}", prev, new_kind);

                // Keep scrape options aligned and rebuild the table.
                app.state.options.scrape.page = new_kind;
                app.rebuild_view();

                // ----- DIR migration logic -----
                // If the *user-visible* DIR is still the default for the previous tab,
                // move DIR to the new tab's default, but preserve the filename/ext
                // from the text field (even if the field is dirty).
                let export = &mut app.state.options.export;
                let prev_default = ExportOptions::default_dir_for(prev);
                let new_default = ExportOptions::default_dir_for(new_kind);

                // Determine DIR as shown in the text field.
                let dir_in_text: PathBuf = match export.export_type {
                    ExportType::SingleFile => {
                        let p = Path::new(&app.out_path_text);
                        p.parent().map(|pp| pp.to_path_buf())
                            .unwrap_or_else(|| export.current_dir().to_path_buf())
                    }
                    ExportType::PerTeam => PathBuf::from(&app.out_path_text),
                };

                if norm(&dir_in_text) == norm(&prev_default) {
                    // Update ExportOptions' DIR to the new default (only DIR).
                    export.set_default_dir_for_page(new_kind);

                    // Recompose the text field path, preserving filename/ext from the textbox.
                    app.out_path_text = match export.export_type {
                        ExportType::SingleFile => {
                            let file_name = Path::new(&app.out_path_text)
                                .file_name()
                                .map(|s| s.to_owned())
                                // Fallback to whatever ExportOptions would produce.
                                .unwrap_or_else(|| export.out_path().file_name()
                                    .unwrap_or_default()
                                    .to_owned());
                            ExportOptions::join_dir_and_filename(&new_default, PathBuf::from(file_name))
                                .to_string_lossy()
                                .into_owned()
                        }
                        ExportType::PerTeam => new_default.to_string_lossy().into_owned(),
                    };

                    // Important: do NOT touch app.out_path_dirty here.
                    // User edits remain "dirty" until they export or otherwise apply.
                }
            }
        }
    });
}