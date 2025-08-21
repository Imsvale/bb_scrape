// src/config/app_state.rs
use super::app_options::AppOptions;

#[derive(Clone, Debug)]
pub struct GuiState {
    pub selected_team_ids: Vec<u32>,
    pub window_w: u32,
    pub window_h: u32,
    pub last_browse_dir: String,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            selected_team_ids: Vec::new(),
            window_w: 1100,
            window_h: 700,
            last_browse_dir: s!(),
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
