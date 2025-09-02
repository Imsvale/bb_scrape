// src/gui/components/data_table.rs
//
// Draws the live table. Fills headers from defaults if None.
// Purely a view; reads/writes App where needed for headers.

use eframe::egui::{self, Align, Layout, RichText, TextWrapMode};
use egui_extras::{Column, TableBuilder};
use crate::gui::app::App;

pub fn draw(ui: &mut egui::Ui, app: &mut App) {
    let page = app.current_page();

    // Prefer live headers; fall back to the page's known headers.
    let hdrs = app.headers.clone()
        .or_else(|| page.default_headers().map(|s| s.iter().map(|x| s!(*x)).collect()));
    app.headers = hdrs;

    let cols = app.headers.as_ref()
        .map(|h| h.len())
        .or_else(|| {
            let kind = app.current_page_kind();
            app.raw_data.get(&kind)
                .and_then(|raw| raw.dataset().rows.get(0))
                .map(|r| r.len())
        })
        .unwrap_or_else(|| page.default_headers().map(|h| h.len()).unwrap_or(0));

    let widths = page.preferred_column_widths();

    let mut table = TableBuilder::new(ui).striped(true).min_scrolled_height(0.0);

    if let Some(ws) = widths {
        for (i, w) in ws.iter().copied().enumerate() {
            let mut col = Column::initial(w as f32).resizable(true);
            if i <= 1 { col = col.at_least(w as f32); }
            table = table.column(col);
        }
    } else {
        table = table
            .column(Column::initial(60.0).at_least(180.0).resizable(true))
            .column(Column::initial(30.0).at_least(30.0).resizable(true))
            .column(Column::initial(140.0).at_least(120.0).resizable(true))
            .column(Column::initial(160.0).at_least(140.0).resizable(true));
        for _ in 4..cols {
            table = table.column(Column::initial(30.0).at_least(30.0).resizable(true));
        }
    }

    // Determine numeric columns from the Page's static hints.
    let kind = app.current_page_kind();
    let raw_opt = app.raw_data.get(&kind).map(|r| r.dataset());

    let non_numeric = page.non_numeric_columns();
    let numeric_cols: Vec<bool> = (0..cols)
        .map(|ci| !non_numeric.contains(&ci))
        .collect();

    table
        .header(24.0, |mut header| {
            if let Some(hs) = app.headers.as_ref() {
                for (ci, h) in hs.iter().enumerate() {
                    header.col(|ui| {
                        ui.scope(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                            if numeric_cols.get(ci).copied().unwrap_or(false) {
                                ui.centered_and_justified(|ui| {
                                    ui.label(RichText::new(h).strong());
                                });
                            } else {
                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                    ui.label(RichText::new(h).strong());
                                });
                            }
                        });
                    });
                }
            } else {
                for i in 0..cols {
                    header.col(|ui| {
                        ui.scope(|ui| {
                            ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                            if numeric_cols.get(i).copied().unwrap_or(false) {
                                ui.centered_and_justified(|ui| {
                                    ui.label(RichText::new(format!("Col {}", i + 1)).strong());
                                });
                            } else {
                                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                    ui.label(RichText::new(format!("Col {}", i + 1)).strong());
                                });
                            }
                        });
                    });
                }
            }
        })
        .body(|body| {
            body.rows(20.0, app.row_ix.len(), |mut row| {
                let row_idx = row.index();
                if let (Some(raw), Some(&src_ix)) = (raw_opt, app.row_ix.get(row_idx)) {
                    if let Some(data) = raw.rows.get(src_ix) {
                        for (ci, cell) in data.iter().enumerate() {
                            row.col(|ui| {
                                ui.scope(|ui| {
                                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                                    let rt = RichText::new(cell);
                                    if numeric_cols.get(ci).copied().unwrap_or(false) {
                                        ui.centered_and_justified(|ui| { ui.label(rt); });
                                    } else {
                                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| { ui.label(rt); });
                                    }
                                });
                            });
                        }
                    }
                }
            });
        });
}
