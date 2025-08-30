// src/gui/table_model.rs
//! TableModel: Filtered view of a `DataSet` for GUI display.
//!
//! Purpose:
//! - Wrap a canonical `DataSet` (headers + rows).
//! - Apply the current team selection filter, so the GUI only renders rows
//!   for the selected teams.
//! - Leave all other options (headers on/off, per-team export, format, etc.)
//!   to the Copy/Export paths — they do not affect the live table view.
//!
//! Why separate it out?
//! - Keeps `app.rs` slimmer by pushing the filtering logic into a dedicated type.
//! - Makes `data_table.rs` simpler: it only consumes `TableModel` and renders it,
//!   without knowing about selections or raw datasets.
//! - Provides a future extension point if we later want to add more lightweight
//!   transformations for display (e.g. highlight rows, hide specific columns).

use std::collections::HashSet;

use crate::store::DataSet;

/// The table model used by the GUI
#[derive(Clone, Debug, Default)]
pub struct TableData {
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

impl TableData {
    pub fn empty() -> Self {
        Self { headers: None, rows: Vec::new() }
    }

    pub fn with(headers: Option<Vec<String>>, rows: Vec<Vec<String>>) -> Self {
        Self { headers, rows }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn ncols(&self) -> usize {
        self.headers
            .as_ref()
            .map(|h| h.len())
            .or_else(|| self.rows.get(0).map(|r| r.len()))
            .unwrap_or(0)
    }

    pub fn nrows(&self) -> usize {
        self.rows.len()
    }

    pub fn into_parts(self) -> (Option<Vec<String>>, Vec<Vec<String>>) {
        (self.headers, self.rows)
    }
}

impl TableData {
    /// Construct a model with no filtering.
    pub fn from_dataset(ds: &DataSet) -> Self {
        Self {
            headers: ds.headers.clone(),
            rows: ds.rows.clone(),
        }
    }

    /// Construct a model with a row predicate filter.
    /// The predicate receives each row (as a slice) and should return `true`
    /// if the row must be kept for display.
    pub fn from_dataset_filtered<F>(ds: &DataSet, keep: F) -> Self
    where
        F: Fn(&[String]) -> bool,
    {
        let rows = ds.rows.iter().cloned().filter(|r| keep(r)).collect();
        Self {
            headers: ds.headers.clone(),
            rows,
        }
    }

    /// Construct a model filtered by team selection.
    ///
    /// - `team_col`: the column index where the *team name* is stored in `ds`.
    /// - `selected_ids`: list of selected team IDs (0..31).
    /// - `teams`: mapping (id, name) so we can convert IDs into display names.
    ///
    /// Any row whose team-name cell matches one of the selected team names
    /// is kept; everything else is dropped.
    pub fn from_dataset_for_teams(
        ds: &DataSet,
        team_col: usize,
        selected_ids: &[u32],
        teams: &[(u32, String)],
    ) -> Self {
        if selected_ids.is_empty() {
            // Show nothing if nothing is selected.
            return Self {
                headers: ds.headers.clone(),
                rows: Vec::new()
            };
        }

        // Build a set of selected team *names*.
        let selected_names: HashSet<&str> = teams
            .iter()
            .filter(|(id, _)| selected_ids.contains(id))
            .map(|(_, name)| name.as_str())
            .collect();

        let rows = ds
            .rows
            .iter()
            .filter(|row| {
                row.get(team_col)
                    .map(|name| selected_names.contains(name.as_str()))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        Self {
            headers: ds.headers.clone(),
            rows,
        }
    }
}