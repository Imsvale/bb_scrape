// tests/export_stream.rs
//
// Tests for file::stream_write_table_to_path without UI.
//
use std::fs;
use std::path::PathBuf;
use bb_scrape::file::{self, ColumnProjection};
use bb_scrape::store::DataSet;

fn tmp(path: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(path);
    p
}

#[test]
fn stream_projection_csv() {
    let headers = Some(vec!["A".into(), "B".into(), "C".into()]);
    let rows = vec![
        vec!["1".into(), "2".into(), "3".into()],
        vec!["4".into(), "5".into(), "6".into()],
    ];
    let ds = DataSet { headers, rows };
    let row_ix = vec![0,1];

    // KeepAll
    let p1 = tmp("bb_stream_keepall.csv");
    file::stream_write_table_to_path(
        &p1, &ds.headers, &ds.rows, &row_ix, Some(','), ColumnProjection::KeepAll
    ).unwrap();
    let s1 = fs::read_to_string(&p1).unwrap();
    assert!(s1.contains("A,B,C"));
    assert!(s1.contains("1,2,3"));
    assert!(s1.contains("4,5,6"));

    // DropLast
    let p2 = tmp("bb_stream_drop.csv");
    file::stream_write_table_to_path(
        &p2, &ds.headers, &ds.rows, &row_ix, Some(','), ColumnProjection::DropLast
    ).unwrap();
    let s2 = fs::read_to_string(&p2).unwrap();
    assert!(s2.contains("A,B"));
    assert!(s2.contains("1,2\n"));
    assert!(!s2.contains(",3"));
}
