// src/gui_config.rs
use std::{fs, path::Path};

pub struct GuiConfig {
    pub include_headers: bool,
    pub keep_hash: bool,
    pub per_team: bool,
    pub out_path: String,
    pub selected_ids: Vec<u32>,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            include_headers: false,
            keep_hash: false,
            per_team: false,
            out_path: "NOT ACTIVE".into(),
            selected_ids: Vec::new(),
        }
    }
}

pub fn load(path: &str) -> GuiConfig {
    if !Path::new(path).exists() {
        return GuiConfig::default();
    }
    let text = match fs::read_to_string(path) { Ok(t) => t, Err(_) => return GuiConfig::default() };
    let mut cfg = GuiConfig::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(eq) = line.find('=') {
            let key = &line[..eq].trim();
            let val = &line[eq+1..].trim();
            match *key {
                "include_headers" => cfg.include_headers = val == "1" || val.eq_ignore_ascii_case("true"),
                "keep_hash" => cfg.keep_hash = val == "1" || val.eq_ignore_ascii_case("true"),
                "per_team" => cfg.per_team = val == "1" || val.eq_ignore_ascii_case("true"),
                "out_path" => cfg.out_path = val.to_string(),
                "teams" => {
                    cfg.selected_ids = val.split(',')
                        .filter_map(|s| s.trim().parse::<u32>().ok())
                        .collect();
                }
                _ => {}
            }
        }
    }
    cfg
}

pub fn save(path: &str, cfg: &GuiConfig) {
    let mut s = String::new();
    s.push_str(&format!("include_headers={}\n", if cfg.include_headers {1}else{0}));
    s.push_str(&format!("keep_hash={}\n", if cfg.keep_hash {1}else{0}));
    s.push_str(&format!("per_team={}\n", if cfg.per_team {1}else{0}));
    s.push_str(&format!("out_path={}\n", cfg.out_path));
    if !cfg.selected_ids.is_empty() {
        let list = cfg.selected_ids.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
        s.push_str(&format!("teams={}\n", list));
    }
    let _ = fs::write(path, s);
}
