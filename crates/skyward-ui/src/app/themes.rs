//! What the runtime tells the pages about the MTZ themes on the phone.

use super::*;
use crate::icons;

impl App {
    /// The themes found, in the places the runtime looked, or why none
    /// could be read. The active one is kept whatever the scan found.
    pub fn set_themes(&mut self, found: Result<Vec<crate::custom_theme::ThemePack>, String>) {
        self.state.themes = Some(found);
        self.refresh();
    }

    /// Lay `theme` over the pages' colours and glyphs, or drop it for the
    /// built-in ones when it is `None`.
    pub fn set_theme(&mut self, theme: Option<crate::custom_theme::ThemePack>, now: u64) {
        match &theme {
            Some(pack) => {
                let count = icons::set_overrides(&pack.icons);
                self.report(&format!("已应用主题 {}（{} 个图标）", pack.name, count), now);
            }
            None => {
                icons::clear_overrides();
                self.report("已恢复默认主题", now);
            }
        }
        self.state.theme = theme;
        self.refresh();
    }

    /// An import finished, so the page can stop saying it is under way.
    pub fn theme_imported(&mut self) {
        self.state.importing_theme = false;
        self.refresh();
    }
}
