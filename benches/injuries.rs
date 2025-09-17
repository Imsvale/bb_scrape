// benches/injuries.rs
use criterion::{criterion_group, criterion_main, Criterion, black_box};

use bb_scrape::{
    scrape,
};

fn load_sample() -> String {
    std::fs::read_to_string(".ignore/page_samples/injury.txt")
        .expect("read .ignore/page_samples/injury.txt")
}

fn load_teams() -> Vec<(u32, String)> {
    // Best-effort: use cached teams if present; otherwise fall back to synthetic team names.
    bb_scrape::get_teams::load().unwrap_or_else(|_| scrape::list_teams())
}

fn bench_injuries(c: &mut Criterion) {
    let doc = load_sample();
    let teams = load_teams();
    let season = ""; // use cached mapping inside the parser if needed

    c.bench_function("injuries_current", |b| {
        b.iter(|| {
            let rows = scrape::injuries::parse_doc_current(black_box(&doc), season, black_box(&teams));
            black_box(rows.len())
        })
    });

    c.bench_function("injuries_fast_base", |b| {
        b.iter(|| {
            let rows = scrape::injuries::parse_doc_fast_base(black_box(&doc), season, black_box(&teams));
            black_box(rows.len())
        })
    });

    c.bench_function("injuries_fast_idx", |b| {
        b.iter(|| {
            let rows = scrape::injuries::parse_doc_fast_idx(black_box(&doc), season, black_box(&teams));
            black_box(rows.len())
        })
    });

    c.bench_function("injuries_fast", |b| {
        b.iter(|| {
            let rows = scrape::injuries::parse_doc_fast(black_box(&doc), season, black_box(&teams));
            black_box(rows.len())
        })
    });
}

criterion_group!(benches, bench_injuries);
criterion_main!(benches);
