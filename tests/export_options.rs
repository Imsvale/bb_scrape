// tests/export_options.rs
//
// Tests for ExportOptions path/extension logic.
//
use std::path::{Path, PathBuf};
use bb_scrape::config::options::{ExportOptions, ExportFormat, ExportType, PageKind};
use bb_scrape::config::options::PageKind::{Players, GameResults};

#[test]
fn default_path_ext_changes_when_fully_default() {
    let mut opts = ExportOptions::default();
    opts.format = ExportFormat::Csv;
    opts.export_type = ExportType::SingleFile;

    // Fresh default for Players
    opts.set_default_dir_for_page(PageKind::Players);
    let p_csv = opts.out_path();
    assert!(p_csv.to_string_lossy().ends_with(".csv"));

    // Switch format; still fully-default → extension should reflect new format
    opts.format = ExportFormat::Tsv;
    let p_tsv = opts.out_path();
    assert!(p_tsv.to_string_lossy().ends_with(".tsv"));
}

fn norm(p: &Path) -> PathBuf { p.components().collect() }

#[test]
fn filename_preserved_on_dir_migration() {
    let mut export = ExportOptions::default();
    export.format = ExportFormat::Csv;
    export.export_type = ExportType::SingleFile;

    // Simulate the text box value before tab switch
    let prev_dir = ExportOptions::default_dir_for(GameResults);
    let text_before: String = prev_dir.join("hello.csv").to_string_lossy().into_owned();

    // --- What tabs.rs does on tab switch ---
    // 1) detect directory shown in the text box
    let text_path = Path::new(&text_before);
    let dir_in_text = text_path.parent().unwrap_or(&prev_dir);

    // 2) if it equals the prev default dir, migrate DIR but keep the filename
    let new_default = ExportOptions::default_dir_for(Players);
    let text_after = if norm(dir_in_text) == norm(&prev_dir) {
        export.set_default_dir_for_page(Players); // update ExportOptions’ dir
        let file_name = text_path.file_name().unwrap_or_default();
        ExportOptions::join_dir_and_filename(&new_default, file_name)
            .to_string_lossy().into_owned()
    } else {
        text_before.clone()
    };
    // --- end UI logic ---

    let expected = norm(&new_default.join("hello.csv"));
    assert_eq!(norm(Path::new(&text_after)), expected,
        "DIR should migrate and filename be preserved");
}

#[test]
fn format_change_keeps_user_extension_when_dirty() {
    let mut export = ExportOptions::default();
    export.format = ExportFormat::Csv;

    // Simulate the textbox holding a custom extension the user typed
    let out_path_text = "out/players/custom.data".to_string();

    // UI: user flips format to TSV
    export.format = ExportFormat::Tsv;

    // Rule: if the textbox is dirty, we leave it alone
    // (The UI never rewrites out_path_text when dirty.)
    assert_eq!(out_path_text, "out/players/custom.data");

    // And ExportOptions isn't updated until the user "applies" the text:
    export.set_path(&out_path_text);
    // Still has .data
    assert!(export.out_path().to_string_lossy().ends_with("custom.data"));
}

#[test]
fn no_dir_migration_when_textbox_dir_is_custom() {
    // Pretend the textbox points to a custom dir that is NOT the prev default
    let text_before = "out/custom/hello.csv".to_string();

    // --- tabs.rs logic mirror ---
    let prev_default = ExportOptions::default_dir_for(GameResults);
    let new_default  = ExportOptions::default_dir_for(Players);

    let text_path = Path::new(&text_before);
    let dir_in_text = text_path.parent().unwrap();

    let text_after = if norm(dir_in_text) == norm(&prev_default) {
        // would migrate… but we expect not to enter here
        let file_name = text_path.file_name().unwrap();
        ExportOptions::join_dir_and_filename(&new_default, file_name)
            .to_string_lossy().into_owned()
    } else {
        text_before.clone()
    };
    // --- end mirror ---

    assert_eq!(text_after, "out/custom/hello.csv");
}