// src/gui/components/data_table.rs
//
// Draws the live table. Fills headers from defaults if None.
// Purely a view; reads/writes App where needed for headers.

use eframe::egui::{self, Align, Layout, RichText, TextWrapMode, Sense, CursorIcon, Pos2, Vec2, Stroke, StrokeKind};
use egui_extras::{Column, TableBuilder};
use crate::gui::app::App;

pub fn draw(ui: &mut egui::Ui, app: &mut App) {
    let page = app.current_page();
    let ctx = ui.ctx().clone();

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

    // Page kind
    let kind = app.current_page_kind();

    // Visual column order for this page (initialize/reset to identity if needed)
    let mut ord_local = app
        .col_order
        .entry(kind)
        .or_insert_with(|| (0..cols).collect())
        .clone();
    if ord_local.len() != cols { ord_local = (0..cols).collect(); }

    // Keep columns fixed during drag; reorder only on drop
    // (order will be used and possibly updated inside inner_table)

    // Column widths following source columns across reorders
    let widths_entry = app.col_widths.entry(kind).or_insert_with(|| {
        if let Some(ws) = page.preferred_column_widths() {
            ws.iter().map(|&w| w as f32).collect()
        } else {
            let mut v = Vec::with_capacity(cols);
            if cols >= 1 { v.push(180.0); }
            if cols >= 2 { v.push(30.0); }
            if cols >= 3 { v.push(140.0); }
            if cols >= 4 { v.push(160.0); }
            for _ in v.len()..cols { v.push(30.0); }
            v
        }
    });
    if widths_entry.len() != cols {
        let new = if let Some(ws) = page.preferred_column_widths() {
            ws.iter().map(|&w| w as f32).collect::<Vec<f32>>()
        } else {
            vec![80.0; cols]
        };
        *widths_entry = new;
    }
    let per_source_widths = widths_entry.clone();

    // Ensure scroll bars allocate space (not floating over content), and tune size
    {
        let s = &mut ui.style_mut().spacing.scroll;
        s.floating = false;           // reserve space instead of overlaying content
        s.bar_width = 10.0;           // slightly slimmer bar
        s.bar_inner_margin = 7.0;     // minimal gap to content (avoid overlap)
        s.bar_outer_margin = 0.0;     // flush to the outside edge
        s.handle_min_length = 48.0;   // keep handle usable even in small windows
        s.foreground_color = true;    // slightly darker handle
        // Make the scroll bars blend better with the window
        let visuals = &mut ui.style_mut().visuals;
        // Use the panel/window fill as the bar background base (lighter than default extreme_bg_color)
        visuals.extreme_bg_color = visuals.panel_fill;
    }

    let avail_h = ui.available_height();
    // logd!("Table: inner h-scroll mode; avail_h={}", avail_h);
    egui::ScrollArea::new([true, false])
        .id_salt("inner_table_hscroll")
        .min_scrolled_height(avail_h)
        .max_height(avail_h)
        .show(ui, |ui| {
            inner_table(ui, app, &ctx, page, kind, &mut ord_local, per_source_widths.clone(), cols, false);
        });
    app.col_order.insert(kind, ord_local);
    return;
}

fn inner_table(
    ui: &mut egui::Ui,
    app: &mut App,
    ctx: &egui::Context,
    page: &dyn crate::gui::pages::Page,
    kind: crate::config::options::PageKind,
    ord: &mut Vec<usize>,
    per_source_widths: Vec<f32>,
    cols: usize,
    outer_scroll: bool,
) {
    let dragging = app.dragging_source_col.is_some();
    let display_ord = ord.clone();
    let mut table = TableBuilder::new(ui)
        .striped(true)
        .min_scrolled_height(0.0)
        // Reset egui_extras table state when column order changes so
        // widths come from our per-source cache instead of staying with positions.
        .id_salt(("table_state", kind, &*ord));
    if outer_scroll { table = table.vscroll(false); }
    for (_disp_ix, &src_ci) in display_ord.iter().enumerate() {
        let w = per_source_widths.get(src_ci).copied().unwrap_or(80.0);
        let col = if dragging {
            // Lock width while dragging so other columns don't auto-grow/shrink
            Column::exact(w).resizable(false).clip(true)
        } else {
            Column::initial(w).resizable(true).clip(true).at_least(20.0)
        };
        table = table.column(col);
    }
    // Add a right-side gutter equal to the allocated scroll bar width so the bar sits
    // outside the last real column
    // no extra gutter column; let the scroll bar sit right of the last column

    // Determine numeric columns from the Page's static hints.
    let raw_opt = app.raw_data.get(&kind).map(|r| r.dataset());

    let non_numeric = page.non_numeric_columns();
    let numeric_cols: Vec<bool> = (0..cols)
        .map(|ci| !non_numeric.contains(&ci))
        .collect();

    table
        .header(24.0, |mut header| {
            // Keep columns static during drag; draw overlays instead
            let display_ord = ord.clone();

            let mut any_drag_stopped = false;
            let mut col_rects: Vec<egui::Rect> = Vec::with_capacity(cols);

            for disp_ix in 0..cols {
                let src_ci = display_ord.get(disp_ix).copied().unwrap_or(disp_ix);
                header.col(|ui| {
                    ui.scope(|ui| {
                        ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);

                        // Cursor and label
                        let label_text = if let Some(hs) = app.headers.as_ref() {
                            hs.get(src_ci).cloned().unwrap_or_else(|| format!("Col {}", src_ci + 1))
                        } else { format!("Col {}", src_ci + 1) };

                        // alignment
                        let is_numeric = numeric_cols.get(src_ci).copied().unwrap_or(false);
                        let draw_label = |ui: &mut egui::Ui, text: String| {
                            let resp = ui.add(egui::Label::new(RichText::new(text).strong()).selectable(false));
                            resp.on_hover_cursor(CursorIcon::Default);
                        };
                        if is_numeric {
                            ui.centered_and_justified(|ui| { draw_label(ui, label_text.clone()); });
                        } else {
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| { draw_label(ui, label_text.clone()); });
                        }

                        // Drag area
                        let id = ui.id().with("colhdr").with(kind).with(src_ci as u64);
                        let rect = ui.max_rect();
                        let resp = ui.interact(rect, id, Sense::click_and_drag());
                        col_rects.push(rect);

                        if resp.drag_started() {
                            app.dragging_source_col = Some(src_ci);
                            app.dragging_preview_to = Some(disp_ix);
                            app.dragging_ghost_width = rect.width();
                            if let Some(pos) = ctx.pointer_interact_pos().or_else(|| ctx.pointer_hover_pos()) {
                                app.dragging_ghost_offset_x = (pos.x - rect.min.x).clamp(0.0, rect.width());
                            } else {
                                app.dragging_ghost_offset_x = rect.width() * 0.5;
                            }
                        }
                        if app.dragging_source_col.is_some() || resp.dragged() {
                            ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
                        }
                        if resp.drag_stopped() { any_drag_stopped = true; }

                        // Persist realized width ONLY when not dragging to avoid
                        // dragging-time oscillations and accidental width adoption.
                        if !dragging {
                            if let Some(ws) = app.col_widths.get_mut(&kind) {
                                if let Some(slot) = ws.get_mut(src_ci) {
                                    *slot = rect.width().max(20.0);
                                }
                            }
                        }
                    });
                });
            }
            // no header cell for gutter

            // While dragging, compute snap edge and draw ghost + insert line
            if app.dragging_source_col.is_some() {
                if let Some(pos) = ctx.pointer_hover_pos().or_else(|| ctx.pointer_interact_pos()) {
                    if !col_rects.is_empty() {
                        let mut edges: Vec<f32> = Vec::with_capacity(cols + 1);
                        edges.push(col_rects[0].left());
                        for r in &col_rects { edges.push(r.right()); }

                        let mut ins_ix = cols; // default end
                        for i in 0..cols {
                            let mid = 0.5 * (edges[i] + edges[i + 1]);
                            if pos.x < mid { ins_ix = i; break; }
                        }
                        app.dragging_preview_to = Some(ins_ix);

                        // Snap vertically to the table: from header top down to window bottom
                        let top = col_rects.iter().map(|r| r.top()).fold(f32::INFINITY, f32::min);
                        let bot = ctx.screen_rect().bottom();
                        let painter = ctx.debug_painter();
                        let x = edges[ins_ix];
                        let visuals = ctx.style();
                        let stroke = Stroke::new(4.0, visuals.visuals.selection.stroke.color);
                        painter.line_segment([Pos2::new(x, top), Pos2::new(x, bot)], stroke);

                        let gw = app.dragging_ghost_width.max(20.0);
                        let gx = pos.x - app.dragging_ghost_offset_x;
                        let ghost = egui::Rect::from_min_size(Pos2::new(gx, top), Vec2::new(gw, bot - top));
                        let fill = visuals.visuals.selection.bg_fill.linear_multiply(0.30);
                        let border = Stroke::new(1.5, visuals.visuals.selection.stroke.color);
                        painter.rect_filled(ghost, 3.0, fill);
                        painter.rect_stroke(ghost, 3.0, border, StrokeKind::Inside);
                    }
                }
                ctx.request_repaint();
            }

            // Commit on drop using edge index
            if any_drag_stopped {
                if let Some(src_col) = app.dragging_source_col {
                    if let Some(from_ix) = ord.iter().position(|&c| c == src_col) {
                        let mut edge_ix = app.dragging_preview_to.unwrap_or(from_ix);
                        if edge_ix > from_ix { edge_ix -= 1; }
                        let mut new_ord = ord.clone();
                        let moved = new_ord.remove(from_ix);
                        new_ord.insert(edge_ix.min(new_ord.len()), moved);
                        *ord = new_ord;
                    }
                }
                app.dragging_source_col = None;
                app.dragging_preview_to = None;
                app.dragging_ghost_width = 0.0;
                app.dragging_ghost_offset_x = 0.0;
            }
        })
        .body(|body| {
            body.rows(20.0, app.row_ix.len(), |mut row| {
                let row_idx = row.index();
                if let (Some(raw), Some(&src_ix)) = (raw_opt, app.row_ix.get(row_idx)) {
                    if let Some(data) = raw.rows.get(src_ix) {
                        // Use committed order for body (no live reordering)
                        let display_ord = ord.clone();

                        for disp_ix in 0..cols {
                            let ci = display_ord.get(disp_ix).copied().unwrap_or(disp_ix);
                            let cell_opt = data.get(ci);
                            row.col(|ui| {
                                ui.scope(|ui| {
                                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                                    if let Some(cell) = cell_opt {
                                        let mut rt = RichText::new(cell);
                                        // Per-page coloring: Injuries -> Type and Bounty columns
                        if kind == crate::config::options::PageKind::Injuries {
                            if ci == 7 { // Type
                                                let u = cell.to_ascii_uppercase();
                                                // Colors matched to site CSS (from brustyle3.css sample):
                                                // text_blue ≈ #64B4FF, text_yellow ≈ #F0D23C, kill/red ≈ #DC6149
                                                if u.contains("SEASON ENDING") { rt = rt.color(egui::Color32::from_rgb(0x64,0xB4,0xFF)); }
                                                else if u.contains("KILL") { rt = rt.color(egui::Color32::from_rgb(0xDC,0x61,0x49)); }
                                                else { rt = rt.color(egui::Color32::from_rgb(0xF0,0xD2,0x3C)); }
                                            } else if ci == 11 { // Bounty
                                                if cell.to_ascii_uppercase().contains("BOUNTY") {
                                                    // text_orange ≈ #FFA500
                                                    rt = rt.color(egui::Color32::from_rgb(0xFF,0xA5,0x00));
                                                }
                                            }
                                        }
                                        if numeric_cols.get(ci).copied().unwrap_or(false) {
                                            ui.centered_and_justified(|ui| { ui.label(rt); });
                                        } else {
                                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| { ui.label(rt); });
                                        }
                                    }
                                });
                            });
                        }
                        // no body cell for gutter
                    }
                }
            });
        });
}
