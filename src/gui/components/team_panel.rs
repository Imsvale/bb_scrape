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

        // Don't overwrite progress messages with team selection info
        if !app.running {
            app.set_selection_message();
        }
        
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

    // Match the scroll bar aesthetics used in the main table
    {
        let s = &mut ui.style_mut().spacing.scroll;
        s.floating = false;
        s.bar_width = 10.0;
        s.bar_inner_margin = 0.0; // small gap between content and bar
        s.bar_outer_margin = -6.0; // 1 px between separator line and bar
        s.handle_min_length = 48.0;
        s.foreground_color = true;
        // Make the background lighter to blend with the window
        let visuals = &mut ui.style_mut().visuals;
        visuals.extreme_bg_color = visuals.panel_fill;
    }

    egui::ScrollArea::vertical()
        .id_salt("teams_panel_scroll")
        .show(ui, |ui| {
            // Ensure the scroll area uses the full panel width so the bar hugs the edge
            let w = ui.available_width();
            ui.set_min_width(w);
            ui.set_width(w);
            let mut changed = false;

        for (idx, (id, name)) in app.teams.iter().enumerate() {
            let is_selected = app.state.gui.selected_team_ids.contains(id);
            let resp = ui.selectable_label(is_selected, name);

            if resp.clicked() && !app.running {
                let input = ui.input(|i| i.clone());
                let sel = &mut app.state.gui.selected_team_ids;
                let ctrl = input.modifiers.ctrl;
                let shift = input.modifiers.shift;

                if ctrl && shift {
                    if let Some(last) = app.last_clicked {
                        let (lo, hi) = if last <= idx { (last, idx) } else { (idx, last) };
                        for j in lo..=hi {
                            let tid = app.teams[j].0;
                            if !sel.contains(&tid) { sel.push(tid); }
                        }
                        app.last_clicked = Some(idx);
                    } else {
                        // No anchor: fall back to ctrl-toggle on single item
                        if is_selected { sel.retain(|x| x != id); } else { sel.push(*id); }
                        app.last_clicked = Some(idx);
                    }
                } else if ctrl {
                    if is_selected { sel.retain(|x| x != id); } else { sel.push(*id); }
                    app.last_clicked = Some(idx);
                } else if shift {
                    if let Some(last) = app.last_clicked {
                        let (lo, hi) = if last <= idx { (last, idx) } else { (idx, last) };
                        sel.clear();
                        for j in lo..=hi { sel.push(app.teams[j].0); }
                        app.last_clicked = Some(idx);
                    } else {
                        // No anchor: behave like single click
                        sel.clear();
                        sel.push(*id);
                        app.last_clicked = Some(idx);
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
