// src/config/state.rs
use super::options::AppOptions;

#[derive(Clone, Debug)]
pub struct GuiState {
    /// Which teams are selected in the left panel
    pub selected_team_ids: Vec<u32>,
    
    pub window_w: u32,
    pub window_h: u32,
    pub last_browse_dir: String,

    /// Active tab index into router::PAGES
    pub current_page_index: usize,

    /// Game Results page -> show/hide Match id column
    pub game_results_show_match_id: bool,

    /// Fixed width for the Teams side panel
    pub team_panel_width: f32,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            selected_team_ids: Vec::new(),
            window_w: 1100,
            window_h: 700,
            last_browse_dir: s!(),
            current_page_index: 0,
            game_results_show_match_id: true,
            team_panel_width: 200.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub options: AppOptions,
    pub gui: GuiState,
    pub season: Option<u32>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            options: AppOptions::default(),
            gui: GuiState::default(),
            season: None,
        }
    }
}
