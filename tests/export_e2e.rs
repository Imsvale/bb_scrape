// tests/export_e2e.rs
use std::fs;
use std::path::PathBuf;

use bb_scrape::config::options::{AppOptions, ExportFormat, ExportType, PageKind};
use bb_scrape::file::{self, export_dataset};

fn tmp_dir(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("bb_e2e_{}", name));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn cli_respects_user_extension_when_format_changes() {
    let mut opts = AppOptions::default();
    opts.export.format = ExportFormat::Csv;
    // single-file path with explicit extension .txt
    let dir = tmp_dir("ext");
    let mut file_path = dir.clone();
    file_path.push("hello.txt");
    opts.export.set_path(file_path.to_str().unwrap());

    // flip format to TSV; extension should remain .txt
    opts.export.format = ExportFormat::Tsv;
    let out = opts.export.out_path();
    assert!(out.to_string_lossy().ends_with("hello.txt"));

    // Write single file for Players
    let headers = Some(vec!["Name".into(), "#Number".into(), "Race".into(), "Team".into()]);
    let rows = vec![vec!["A".into(), "#7".into(), "Elf".into(), "Alpha".into()]];
    let written = export_dataset(&opts, PageKind::Players, &headers, &rows).unwrap();
    assert_eq!(written.len(), 1);
    assert!(written[0].to_string_lossy().ends_with("hello.txt"));
}

#[test]
fn results_per_team_writes_both_teams() {
    let mut opts = AppOptions::default();
    opts.export.export_type = ExportType::PerTeam;
    opts.export.format = ExportFormat::Csv;
    // per-team requires directory path
    let dir = tmp_dir("results_per_team");
    opts.export.set_path(dir.to_str().unwrap());

    let headers = Some(vec![
        "S".into(), "W".into(), "Home team".into(), "Home".into(),
        "Away".into(), "Away team".into(), "Match id".into()
    ]);
    let rows = vec![
        vec!["5".into(), "1".into(), "Alpha".into(), "7".into(), "3".into(), "Beta".into(), "100".into()],
        vec!["5".into(), "2".into(), "Gamma".into(), "2".into(), "1".into(), "Alpha".into(), "101".into()],
    ];

    let written = export_dataset(&opts, PageKind::GameResults, &headers, &rows).unwrap();
    // Expect two files: Alpha and Beta and Gamma (Alpha appears twice, Beta and Gamma once)
    // Exact filenames depend on sanitize; check count >= 2
    assert!(written.len() >= 2);

    // Read Alpha file should contain both rows involving Alpha
    let alpha_file = written.iter().find(|p| p.file_name().unwrap().to_string_lossy().contains("Alpha")).unwrap();
    let content = fs::read_to_string(alpha_file).unwrap();
    assert!(content.contains("Alpha"));
    assert!(content.contains("Beta") || content.contains("Gamma"));
}

#[test]
fn players_skip_optional_strips_hash() {
    let mut opts = AppOptions::default();
    opts.export.export_type = ExportType::SingleFile;
    opts.export.format = ExportFormat::Csv;
    opts.export.skip_optional = true; // page-agnostic: removes '#'
    let dir = tmp_dir("players_skip");
    let mut file_path = dir.clone();
    file_path.push("skip.csv");
    opts.export.set_path(file_path.to_str().unwrap());

    let headers = Some(vec!["Name".into(), "#Number".into(), "Race".into(), "Team".into()]);
    let rows = vec![vec!["A".into(), "#27".into(), "Elf".into(), "Alpha".into()]];
    let written = export_dataset(&opts, PageKind::Players, &headers, &rows).unwrap();
    let s = fs::read_to_string(&written[0]).unwrap();
    assert!(s.contains("Name,#Number,Race,Team"));
    assert!(s.contains(",27,")); // no '#'
}

