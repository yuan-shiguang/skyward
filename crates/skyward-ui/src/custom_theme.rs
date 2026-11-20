//! The MTZ themes the owner keeps, and what they change in the pages.
//!
//! An MTZ is a MIUI theme: a zip holding a `description.xml` and, for each
//! themed app, another zip with a `theme_values.xml` of colours and drawable
//! PNGs. The runtime reads those and hands what the pages need here, so the
//! pages never open a zip themselves.

use crate::color::Rgba;
use crate::icons::Icon;
use crate::theme::Palette;

/// One theme the runtime found on the phone, with the colours and icons it
/// gives the pages. The PNG bytes are kept as they were in the theme so the
/// icons module can decode and scale them where they are drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemePack {
    /// Where the `.mtz` lies, so it can be opened again on another start.
    pub path: String,
    /// The theme's title from its description.
    pub name: String,
    /// Who made the theme, from its description.
    pub author: String,
    /// Colour values from `com.android.settings`' `theme_values.xml`, under
    /// their MIUI names.
    pub colors: Vec<(String, Rgba)>,
    /// Drawable PNGs that stand in for the built-in glyphs.
    pub icons: Vec<(Icon, Vec<u8>)>,
}

impl ThemePack {
    /// The first colour under any of `keys` that this theme names.
    fn color(&self, keys: &[&str]) -> Option<Rgba> {
        keys.iter().find_map(|key| self.colors.iter().find(|(name, _)| name == key).map(|(_, c)| *c))
    }

    /// Lay the theme's colours over `pal`, leaving every role the theme does
    /// not name as it was. The mapping follows the names MIUI Settings gives
    /// the parts of its own pages, which are what the theme recolours.
    pub fn apply(&self, pal: &mut Palette) {
        if let Some(c) = self.color(&["main_page_background", "miuix_default_color_surface_light"]) {
            pal.bg = c;
        }
        if let Some(c) = self.color(&["card_view_background_color", "device_card_background", "locale_cardview_background_color"]) {
            pal.card = c;
        }
        if let Some(c) = self.color(&["miuix_preference_primary_text_color_light", "default_text_color", "card_view_title_color"]) {
            pal.text = c;
        }
        if let Some(c) = self.color(&["miuix_preference_secondary_text_color_light", "list_secondary_text_color_normal_light", "accessibility_footer_color"]) {
            pal.subtitle = c;
        }
        if let Some(c) = self.color(&["card_view_value_color", "basic_info_item_summary_color"]) {
            pal.value = c;
        }
        if let Some(c) = self.color(&["miuix_color_blue_light_primary_default", "miuix_checked_text_light", "bluetooth_disconnect_text_color"]) {
            pal.accent = c;
        }
        if let Some(c) = self.color(&["miuix_appcompat_arrow_right_normal_fill_color_light", "miuix_appcompat_arrow_right_pressed_fill_color_light"]) {
            pal.chevron = c;
        }
        if let Some(c) = self.color(&["wifi_detail_delete_color", "icon_notification_message_bg_color"]) {
            pal.danger = c;
        }
        if let Some(c) = self.color(&["miuix_appcompat_dropdown_popup_background_color_light", "miuix_appcompat_dialog_default_bg_color_light"]) {
            pal.popup = c;
        }
        if let Some(c) = self.color(&["miuix_appcompat_button_bg_color_normal_light", "bg_btn_normal_color"]) {
            pal.track = c;
        }
        // The search field and the section labels keep the tone of the card
        // and the subtitles: themes rarely name them for themselves.
        if let Some(c) = self.color(&["bg_btn_search_color"]) {
            pal.search = c;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(colors: &[(&str, Rgba)]) -> ThemePack {
        ThemePack {
            path: "/sdcard/Skyward/themes/x.mtz".into(),
            name: "x".into(),
            author: String::new(),
            colors: colors.iter().map(|&(k, c)| (k.to_string(), c)).collect(),
            icons: Vec::new(),
        }
    }

    #[test]
    fn a_theme_only_changes_the_colours_it_names() {
        let mut p = crate::theme::Theme::light(1080, 2248, 2.75).pal;
        let before = p;
        pack(&[("main_page_background", Rgba::rgb(1, 2, 3)), ("miuix_color_blue_light_primary_default", Rgba::rgb(4, 5, 6))]).apply(&mut p);
        assert_eq!((p.bg, p.accent), (Rgba::rgb(1, 2, 3), Rgba::rgb(4, 5, 6)));
        assert_eq!(p.card, before.card, "a colour the theme does not name stays");
        assert_eq!(p.text, before.text);
    }

    #[test]
    fn the_first_name_a_theme_gives_a_role_wins() {
        let mut pal = crate::theme::Theme::light(1080, 2248, 2.75).pal;
        pack(&[
            ("main_page_background", Rgba::rgb(9, 9, 9)),
            ("miuix_default_color_surface_light", Rgba::rgb(7, 7, 7)),
        ])
        .apply(&mut pal);
        assert_eq!(pal.bg, Rgba::rgb(9, 9, 9), "the first of the two names wins");
    }

    #[test]
    fn an_empty_theme_changes_nothing() {
        let mut pal = crate::theme::Theme::light(1080, 2248, 2.75).pal;
        let before = pal;
        pack(&[]).apply(&mut pal);
        assert_eq!(pal.bg, before.bg);
        assert_eq!(pal.accent, before.accent);
    }
}
