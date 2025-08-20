// src/engine/types.rs
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub enum TaskKind { Players, SeasonStats, CareerStats, Season, Injuries }

#[derive(Clone, Copy)]
pub enum Mode { Single(u32), All }

pub struct RunOpts {
    pub keep_hash: bool,
    pub include_headers: bool,
    pub mode: Mode,
    pub out_dir_or_file: Option<PathBuf>, // interpreted per mode/kind
}

pub struct OutputBundle {
    pub filename_stem: String,
    pub headers: Option<Vec<String>>,
    pub rows: Vec<Vec<String>>,
}

/* Generic table spec */
pub enum Locator {
    TagWithAttr { tag: &'static str, attr: &'static str, value_sub: &'static str },
    FirstTableWithHeader(&'static str),
}

pub enum HeaderMode { ConsecutiveTh, None }

pub enum RowSelector { TrClassAny(&'static [&'static str]) }

pub enum CellSelector { TdOnly, ThThenTd }

pub struct TableSpec {
    pub path_tmpl: &'static str,   // e.g., "team.php?i={id}"
    pub locator: Locator,
    pub header_mode: HeaderMode,
    pub row_selector: RowSelector,
    pub cell_selector: CellSelector,
    pub header_ops: &'static [&'static str], // prepend fixed headers
    pub drop_first_header: bool,
    pub split_fused_first_cell: bool,
    pub insert_team_at: Option<usize>,
}
