// src/config/app_state.rs
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
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub options: AppOptions,
    pub gui: GuiState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            options: AppOptions::default(),
            gui: GuiState::default(),
        }
    }
}
