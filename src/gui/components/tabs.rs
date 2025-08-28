// src/gui/components/tabs.rs
//
// Renders the top tabs and performs the tab switch itself.
// Design: display is literal — built from canonical raw_data (cache) plus
// selection filtering. No option-based projection here, and no view cache.
// On tab switch, we load the page's dataset (if any), apply the page's
// filter_rows_for_selection, and set app.headers/app.rows accordingly.

use eframe::egui;
use crate::{gui::app::App, gui::router};
use crate::data::FilteredData;

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
                let kind = page.kind();
                logf!("UI: Tab switch {:?} → {:?}", prev, kind);

                if let Some(raw) = app.raw_data.get(&kind) {
                    let fd = FilteredData::from_raw(
                        *page,
                        raw,
                        &app.state.gui.selected_team_ids,
                        &app.teams,
                    );
                    app.headers = fd.headers_owned();
                    app.rows = fd.to_owned_rows();
                } else {
                    app.headers = page
                        .default_headers()
                        .map(|hs| hs.iter().map(|s| s!(*s)).collect());
                    app.rows = Vec::new();
                }
            }
        }
    });
}
