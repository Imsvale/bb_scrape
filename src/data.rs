// src/data.rs
//
// Light wrappers around canonical and view-layer table data.
//
// - RawData: read-only holder for canonical page data (cache + post-scrape).
//            Only SCRAPE is allowed to mutate it, via an explicit method.
// - FilteredData: derived (view) data produced from RawData by applying
//                 page-specific selection filtering for on-screen display.
//
// Common utilities that make sense at the dataset level live as methods
// on DataSet itself (see src/store.rs).

use std::borrow::Cow;
use std::io::Result;
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

    pub fn save(&self) -> Result<PathBuf> {
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

#[derive(Clone)]
pub struct Selection<'a> {
    pub ids: &'a [u32],
    pub teams: &'a [(u32, String)],
}

/// Zero-copy filtered view for display.
/// Holds borrowed headers and a list of row indexes into RawData.
#[derive(Clone, Debug)]
pub struct FilteredData<'a> {
    /// Borrowed headers (Cow: either borrowed from raw or owned defaults)
    pub headers: Option<Cow<'a, [String]>>,
    /// Positions of kept rows in the raw dataset
    pub row_ix: Vec<usize>,
    /// Borrowed pointer to the canonical dataset
    raw: &'a DataSet,
}

impl<'a> FilteredData<'a> {
    /// Build a filtered view from RawData and a page's selection filter.
    /// NOTE: Since the Page trait exposes only a row-vector filter and not
    /// a per-row predicate, we compute indices by comparing filtered rows
    /// back against the raw rows. This is O(n^2) worst-case but avoids
    /// retaining duplicate owned data in App. If needed later, we can add
    /// a predicate API to Page for true O(n) zero-copy selection.
    pub fn from_raw(
        page: &dyn Page,
        raw: &'a RawData,
        selected_team_ids: &[u32],
        teams: &[(u32, String)],
    ) -> Self {
        let ds = raw.dataset();

        // Borrow headers if present; otherwise use page defaults (owned).
        let headers = match &ds.headers {
            Some(h) => Some(Cow::Borrowed(h.as_slice())),
            None => page
                .default_headers()
                .map(|hs| Cow::Owned(hs.iter().map(|s| s.to_string()).collect::<Vec<_>>())),
        };

        // Use existing page API to get owned filtered rows, then map to indices.
        let filtered_rows = page.filter_rows_for_selection(selected_team_ids, teams, &ds.rows);

        let mut row_ix = Vec::with_capacity(filtered_rows.len());
        if filtered_rows.len() == ds.rows.len() {
            // Fast path: keep all
            row_ix.extend(0..ds.rows.len());
        } else {
            // Map filtered rows back to first unused matching raw row index.
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
        }

        Self { headers, row_ix, raw: ds }
    }

    /// Number of rows in the projection.
    pub fn len(&self) -> usize { self.row_ix.len() }
    pub fn is_empty(&self) -> bool { self.row_ix.is_empty() }

    /// Borrow a single row by projected index (no cloning).
    pub fn row(&self, i: usize) -> Option<&[String]> {
        self.row_ix.get(i).and_then(|&ix| self.raw.rows.get(ix).map(|r| r.as_slice()))
    }

    /// Materialize owned headers (for UI/export boundaries).
    pub fn headers_owned(&self) -> Option<Vec<String>> {
        self.headers.as_ref().map(|c| c.to_vec())
    }

    /// Materialize owned rows (for UI/export boundaries).
    pub fn to_owned_rows(&self) -> Vec<Vec<String>> {
        self.row_ix.iter().map(|&ix| self.raw.rows[ix].clone()).collect()
    }
}
