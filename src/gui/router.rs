// src/gui/router.rs
use crate::config::options::PageKind::{ self, * };
use super::pages::{ self, Page };

pub static PAGES: &[&'static dyn Page] = &[
    &pages::players::PAGE,
    &pages::game_results::PAGE,
    &pages::injuries::PAGE,
];

pub fn all_pages() -> &'static [&'static dyn Page] {
    PAGES
}

pub fn page_for(kind: &PageKind) -> &'static dyn Page {
    match kind {
        Players     => &pages::players::PAGE,
        GameResults => &pages::game_results::PAGE,
        Injuries    => &pages::injuries::PAGE,
        // Add more as you implement them.
        _ => &pages::players::PAGE,
    }
}
