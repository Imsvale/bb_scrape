// tests/selection_view.rs
//
// Minimal Page impl to test SelectionView behavior without UI.
//
use std::error::Error;
use bb_scrape::data::{RawData, Selection, SelectionView};
use bb_scrape::store::DataSet;
use bb_scrape::config::options::PageKind;
use bb_scrape::config::state::AppState;
use bb_scrape::progress::Progress;
use bb_scrape::gui::pages::Page;

struct TestPage;
impl Page for TestPage {
    fn title(&self) -> &'static str { "Test" }
    fn kind(&self) -> PageKind { PageKind::Players }
    fn scrape(
        &self,
        _state: &AppState,
        _progress: Option<&mut dyn Progress>,
    ) -> Result<DataSet, Box<dyn Error>> {
        Ok(DataSet { headers: None, rows: Vec::new() })
    }
    fn filter_rows_for_selection(
        &self,
        selected_ids: &[u32],
        teams: &[(u32, String)],
        rows: &Vec<Vec<String>>,
    ) -> Vec<Vec<String>> {
        // TEAM_COL = 3 (same as Players)
        use std::collections::HashSet;
        let sel: HashSet<&str> = selected_ids.iter()
            .filter_map(|id| teams.iter().find(|(tid, _)| tid == id))
            .map(|(_, name)| name.as_str())
            .collect();
        rows.iter()
            .filter(|r| r.get(3).map(|t| sel.contains(t.as_str())).unwrap_or(false))
            .cloned()
            .collect()
    }
    
}

#[test]
fn selection_view_none_all_partial() {
    // Teams and rows
    let teams = vec![(0, "Alpha".into()), (1, "Beta".into()), (2, "Gamma".into())];
    let rows = vec![
        vec!["p1".into(), "X".into(), "Y".into(), "Alpha".into()],
        vec!["p2".into(), "X".into(), "Y".into(), "Beta".into()],
        vec!["p3".into(), "X".into(), "Y".into(), "Gamma".into()],
        vec!["p4".into(), "X".into(), "Y".into(), "Alpha".into()],
    ];
    let ds = DataSet { headers: None, rows };
    let raw = RawData::new(PageKind::Players, ds);
    let page = TestPage;

    // None
    let sel = Selection { ids: &[], teams: &teams };
    let view = SelectionView::from_raw(&page, &raw, sel);
    assert!(view.is_empty());

    // All
    let all = vec![0,1,2];
    let sel = Selection { ids: &all, teams: &teams };
    let view = SelectionView::from_raw(&page, &raw, sel);
    assert_eq!(view.len(), raw.dataset().rows.len());

    // Partial
    let pick = vec![0,2]; // Alpha + Gamma
    let sel = Selection { ids: &pick, teams: &teams };
    let view = SelectionView::from_raw(&page, &raw, sel);
    // Expect rows 0,2,3 (two Alpha, one Gamma)
    let idx: Vec<usize> = view.row_ix.clone();
    assert_eq!(idx, vec![0,2,3]);
}
