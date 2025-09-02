// src/gui/components/action_buttons.rs

use eframe::egui::{self, Checkbox, widgets::Spinner};
use crate::{
    gui::app::App,
    config::options::{
        ExportFormat,
        ExportType::{PerTeam, SingleFile},
    },
};

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

    // Actions: Copy / Export / Scrape
    use crate::gui::actions;
    ui.horizontal(|ui| {

        // Copy
        let button_copy = ui.button("Copy");
        if button_copy.clicked() {
            actions::copy(app, ui.ctx());
        }

        // Export
        let button_export = ui.button("Export");
        if button_export.clicked() {
            actions::export(app);
        }

        // Scrape
        let red = egui::Color32::from_rgb(220, 30, 30);
        let black = egui::Color32::BLACK;

        let button_scrape = ui.add_enabled(
            !app.running,
            egui::Button::new(
                egui::RichText::new("SCRAPE")
                .color(black)
                .strong())
            .fill(red));

        if button_scrape.clicked() { 
            actions::scrape(app); 
        }

        if app.running {
            ui.add(Spinner::new().size(16.0));
        }

        let status = app.status.lock().unwrap().clone();

        ui.label(status);
    });
}
