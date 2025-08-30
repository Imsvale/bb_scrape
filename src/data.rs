// src/data.rs
//
// Light wrappers around canonical and view-layer table data.
//
// - RawData: read-only holder for canonical page data (cache + post-scrape).
//            Only SCRAPE is allowed to mutate it, via an explicit method.
// - SelectionView: derived (view) data produced from RawData by applying
//                 page-specific selection filtering for on-screen display.
//
// Common utilities that make sense at the dataset level live as methods
// on DataSet itself (see src/store.rs).

use std::io;
use std::path::PathBuf;

use crate::store::DataSet;
use crate::gui::pages::Page;
use crate::config::options::PageKind;

/// Authoritative, canonical page dataset.
/// Loaded from cache at startup; updated only by SCRAPE.
#[derive(Clone, Debug)]
pub struct RawData {
    kind: PageKind,
    ds: DataSet,
}

impl RawData {
    /// Build from a freshly loaded cache dataset.
    pub fn new(kind: PageKind, ds: DataSet) -> Self { Self { kind, ds } }
    pub fn kind(&self) -> PageKind { self.kind }

    /// Read-only view of the dataset.
    pub fn dataset(&self) -> &DataSet { &self.ds }

    pub fn save(&self) -> io::Result<PathBuf> {
        crate::store::save_dataset(&self.kind, &self.ds)
    }

    /// Merge in newly scraped data using the page's merge policy.
    /// This is the *only* mutator; keeps the rest of the app read-only.
    pub fn merge_from_scrape(&mut self, page: &dyn Page, new: DataSet) {
        page.merge(&mut self.ds, new);
    }

    /// Mutable access for I/O boundaries that require &mut DataSet (rare).
    /// Prefer `apply_scrape_merge()` for scrape updates instead of mutating directly.
    pub fn dataset_mut_for_io(&mut self) -> &mut DataSet {
        &mut self.ds
    }
}

#[derive(Clone, Copy)]
pub struct Selection<'a> {
    pub ids: &'a [u32],
    pub teams: &'a [(u32, String)],
}

impl<'a> Selection<'a> {
    #[inline] pub fn is_none(&self) -> bool { self.ids.is_empty() }
    #[inline] pub fn is_all(&self) -> bool { self.ids.len() == self.teams.len() }

    pub fn to_key_mask(&self) -> u32 {

        // Guard against teams space exceeding bitmask space
        debug_assert!(self.teams.len() <= 32);
        debug_assert!(self.ids.iter().all(|&id| id < 32));
        // If #teams ever exceeds 32, change to u64

        let mut mask = 0u32;
        for &id in self.ids {
            // If ids ever drift >31, this just ignores the overflow safely
            if id < 32 { mask |= 1u32 << id; }
        }
        mask
    }

    // If #teams ever exceeds 64:
    // pub fn to_key(&self, teams_version: u64) -> SelectionKey {
    //     use std::hash::{Hash, Hasher};
    //     use std::collections::hash_map::DefaultHasher;

    //     let mut ids = self.ids.to_vec();
    //     ids.sort_unstable();
    //     ids.dedup();

    //     let mut h = DefaultHasher::new();
    //     ids.hash(&mut h);
    //     SelectionKey { ids_hash: h.finish(), teams_version }
    // }
}

/// Zero-copy filtered view for display.
/// Holds list of row indexes into RawData.
#[derive(Clone, Debug)]
pub struct SelectionView<'a> {
    /// Positions of kept rows in the raw dataset
    pub row_ix: Vec<usize>,
    /// Borrowed pointer to the canonical dataset
    raw: &'a DataSet,
}

impl<'a> SelectionView<'a> {
    /// Build a filtered view from RawData and a page's selection filter.
    /// NOTE: Since the Page trait exposes only a row-vector filter and not
    /// a per-row predicate, we compute indices by comparing filtered rows
    /// back against the raw rows. This is O(n^2) worst-case but avoids
    /// retaining duplicate owned data in App. If needed later, we can add
    /// a predicate API to Page for true O(n) zero-copy selection.
    pub fn from_raw(
        page: &dyn Page,
        raw: &'a RawData,
        sel: Selection<'_>
    ) -> Self {
        let ds = raw.dataset();

        if sel.is_none() { return Self { row_ix: vec![], raw: ds }; }
        if sel.is_all()  { return Self { row_ix: (0..ds.rows.len()).collect(), raw: ds }; }

        // Fast path: ask the page for row indices directly
        if let Some(ix) = page.filter_row_indices_for_selection(sel.ids, sel.teams, &ds.rows) {
            return Self { row_ix: ix, raw: ds };
        }

        // Fallback: Get owned rows, then remap to indices: O(n²)
        let filtered_rows = page.filter_rows_for_selection(sel.ids, sel.teams, &ds.rows);

        let mut row_ix = Vec::with_capacity(filtered_rows.len());
        let mut used = vec![false; ds.rows.len()];
        'outer: for fr in &filtered_rows {
            for (i, rr) in ds.rows.iter().enumerate() {
                if !used[i] && rr == fr {
                    used[i] = true;
                    row_ix.push(i);
                    continue 'outer;
                }
            }
            // If no match, skip silently (defensive)
        }

        Self { row_ix, raw: ds }
    }

    /// Number of rows in the projection.
    pub fn len(&self) -> usize { self.row_ix.len() }
    pub fn is_empty(&self) -> bool { self.row_ix.is_empty() }

    /// Borrow a single row by projected index (no cloning).
    pub fn row(&self, i: usize) -> Option<&[String]> {
        self.row_ix.get(i).and_then(|&ix| self.raw.rows.get(ix).map(|r| r.as_slice()))
    }

    /// Materialize owned rows (for UI/export boundaries).
    pub fn to_owned_rows(&self) -> Vec<Vec<String>> {
        self.row_ix.iter().map(|&ix| self.raw.rows[ix].clone()).collect()
    }

    /// Build a view directly from precomputed indices (cache hit path).
    pub fn from_indices(raw: &'a RawData, row_ix: Vec<usize>) -> Self {
        Self { row_ix, raw: raw.dataset() }
    }
}