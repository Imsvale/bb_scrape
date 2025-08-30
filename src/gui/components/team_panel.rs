// src/gui/components/team_panel.rs
//
// Renders the left team list and applies selection changes directly to `app`.
// Handles ctrl/shift range behavior, status text, and marks current page dirty.

use eframe::egui;
use crate::gui::app::App;

pub fn draw(ui: &mut egui::Ui, app: &mut App) {
    ui.heading("Teams");

    // Apply current selection → scrape options, rebuild table, set status.
    let apply_selection_change = |app: &mut App| {
        app.sync_gui_selection_into_scrape();
        app.rebuild_view();
        app.set_selection_message();
    };

    ui.horizontal(|ui| {
        if ui.button("All").clicked() {
            app.state.gui.selected_team_ids = app.teams.iter().map(|(id, _)| *id).collect();
            apply_selection_change(app);
        }
        if ui.button("None").clicked() {
            app.state.gui.selected_team_ids.clear();
            apply_selection_change(app);
        }
    });

    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut changed = false;

        for (idx, (id, name)) in app.teams.iter().enumerate() {
            let is_selected = app.state.gui.selected_team_ids.contains(id);
            let resp = ui.selectable_label(is_selected, name);

            if resp.clicked() && !app.running {
                let input = ui.input(|i| i.clone());
                let sel = &mut app.state.gui.selected_team_ids;

                if input.modifiers.ctrl {
                    if is_selected { sel.retain(|x| x != id); } else { sel.push(*id); }
                    app.last_clicked = Some(idx);
                } else if input.modifiers.shift {
                    if let Some(last) = app.last_clicked {
                        let (lo, hi) = if last <= idx { (last, idx) } else { (idx, last) };
                        sel.clear();
                        for j in lo..=hi { sel.push(app.teams[j].0); }
                    }
                } else {
                    sel.clear();
                    sel.push(*id);
                    app.last_clicked = Some(idx);
                }
                changed = true;
            }
        }

        if changed {
            apply_selection_change(app);
            logf!(
                "UI: Selection changed ({} teams) — {:?}",
                app.state.gui.selected_team_ids.len(),
                &app.state.gui.selected_team_ids
            );
        }
    });
}
