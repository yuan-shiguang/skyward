//! The MTZ themes on the phone: finding them, reading the colours and
//! drawables the theme editor put in them, and importing one into the
//! folder the pages scan. An MTZ is a zip of module zips; the settings
//! module holds the colours and icons that restyle the settings-like pages.

use anyhow::{Context, Result};
use skyward_ui::custom_theme::ThemePack;
use skyward_ui::Icon;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// Where themes are kept once imported, from the top of shared storage.
pub const FOLDER: &str = "Skyward/themes";
/// Other folders of shared storage looked in for themes that were not
/// imported, so a theme downloaded anywhere can be applied at once.
const ALSO: [&str; 2] = ["Download", "MIUI/theme"];

/// Read one theme's file into what the pages can lay over themselves.
pub fn read(path: &Path) -> Result<ThemePack> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut outer = ZipArchive::new(BufReader::new(file)).context("open the theme's zip")?;
    let description = text_or(&mut outer, "description.xml").unwrap_or_default();
    let (name, author) = describe(&description);

    let mut colors = Vec::new();
    let mut icons = Vec::new();
    if outer.index_for_name("com.android.settings").is_some() {
        let bytes = bytes(&mut outer, "com.android.settings")?;
        let mut inner = ZipArchive::new(Cursor::new(bytes)).context("open the settings module")?;
        if let Ok(values) = text_or(&mut inner, "theme_values.xml") {
            colors = colors_of(&values);
        }
        icons = icons_of(&mut inner);
    }
    Ok(ThemePack { path: path.to_string_lossy().into_owned(), name, author, colors, icons })
}

/// All the themes found in shared storage, newest first, each that could
/// not be read left out with its name as the reason.
pub fn scan(top: &Path) -> Result<Vec<ThemePack>> {
    let mut found = Vec::new();
    for folder in [FOLDER, ALSO[0], ALSO[1]] {
        let dir = top.join(folder);
        if !dir.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                found.push(unreadable(&dir, &e.to_string()));
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let mtz = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("mtz"));
            if !mtz || found.iter().any(|p: &ThemePack| p.path == path.to_string_lossy()) {
                continue;
            }
            match read(&path) {
                Ok(pack) => found.push(pack),
                Err(e) => found.push(unreadable(&path, &format!("{e:#}"))),
            }
        }
    }
    found.sort_by(|a, b| b.path.cmp(&a.path));
    Ok(found)
}

fn unreadable(path: &Path, why: &str) -> ThemePack {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string());
    ThemePack { path: path.to_string_lossy().into_owned(), name: format!("{name}：{why}"), author: String::new(), colors: Vec::new(), icons: Vec::new() }
}

/// Copy the `.mtz` at `from` into the themes folder, making room for it.
pub fn import(top: &Path, from: &Path) -> Result<PathBuf> {
 let folder = top.join(FOLDER);
 fs::create_dir_all(&folder).with_context(|| format!("make {}", folder.display()))?;
 let name = from.file_name().context("the file has no name")?;
 let into = folder.join(name);
 fs::copy(from, &into).with_context(|| format!("copy {} to {}", from.display(), into.display()))?;
 Ok(into)
}

/// The title and designer a theme gives itself in its description.
fn describe(xml: &str) -> (String, String) {
    (tag(xml, "title").unwrap_or_else(|| "未命名主题".into()), tag(xml, "designer").unwrap_or_default())
}

/// The text of the first `name` element in `xml`.
fn tag(xml: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let at = xml.find(&open)? + open.len();
    let end = xml[at..].find("</")? + at;
    Some(xml[at..end].trim().to_string())
}

/// The colour values a settings module names, from `theme_values.xml`.
fn colors_of(xml: &str) -> Vec<(String, skyward_ui::Rgba)> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find("<color") {
        rest = &rest[at..];
        let Some(name_end) = rest.find('"') else { break };
        let rest2 = &rest[name_end + 1..];
        let Some(name_close) = rest2.find('"') else { break };
        let name = &rest2[..name_close];
        let Some(gt) = rest2[name_close..].find('>') else { break };
        let after = &rest2[name_close + gt + 1..];
        let Some(lt) = after.find('<') else { break };
        let value = after[..lt].trim();
        if let Some(rgba) = argb(value) {
            out.push((name.to_string(), rgba));
        }
        rest = after;
    }
    out
}

/// An Android `#AARRGGBB` colour, with alpha packed first.
fn argb(value: &str) -> Option<skyward_ui::Rgba> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 8 {
        return None;
    }
    let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    Some(skyward_ui::Rgba { a: byte(0)?, r: byte(2)?, g: byte(4)?, b: byte(6)? })
}

/// The drawables of the settings module that stand in for built-in icons.
fn icons_of(zip: &mut ZipArchive<Cursor<Vec<u8>>>) -> Vec<(Icon, Vec<u8>)> {
    let mut named: HashMap<String, Vec<u8>> = HashMap::new();
    for i in 0..zip.len() {
        let Ok(mut entry) = zip.by_index(i) else { continue };
        let name = entry.name().rsplit('/').next().unwrap_or_default().to_string();
        if !name.ends_with(".png") {
            continue;
        }
        let mut bytes = Vec::new();
        if entry.read_to_end(&mut bytes).is_ok() {
            named.insert(name.trim_end_matches(".png").to_string(), bytes);
        }
    }
    let mut out = Vec::new();
    for (icon, names) in ICON_DRAWABLES {
        for name in *names {
            if let Some(bytes) = named.get(*name) {
                out.push((*icon, bytes.clone()));
                break;
            }
        }
    }
    out
}

/// Which settings-module drawables stand in for which of the pages' icons.
/// A theme that supplies none of them leaves the built-in glyphs as they
/// are; a theme that supplies one replaces just that glyph.
const ICON_DRAWABLES: &[(Icon, &[&str])] = &[
    (Icon::Gear, &["ic_settings", "bh_ic_setting", "ic_system_app_settings", "ic_other_advanced_settings"]),
    (Icon::Wlan, &["ic_wifi_settings", "ic_portable_wlan_hotspot_settings"]),
    (Icon::Battery, &["ic_battery_settings", "device_description_battery"]),
    (Icon::Lock, &["ic_face_unlock", "ic_finger_unlock", "ic_password_unlock", "flip_lock_screen_icon_new"]),
    (Icon::Usb, &["ic_usb", "ic_usb_mtp_new"]),
    (Icon::Log, &["ic_bugreport_settings"]),
    (Icon::Clock, &["setting_alarm_clock"]),
    (Icon::More, &["ic_miui_home_settings"]),
    (Icon::Shield, &["ic_security_settings_settings", "ic_security_center_settings"]),
    (Icon::Phone, &["ic_settings_dualsim"]),
];

fn bytes<R: Read + std::io::Seek>(zip: &mut ZipArchive<R>, name: &str) -> Result<Vec<u8>> {
    let mut entry = zip.by_name(name).with_context(|| format!("open {name}"))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).with_context(|| format!("read {name}"))?;
    Ok(bytes)
}

fn text_or<R: Read + std::io::Seek>(zip: &mut ZipArchive<R>, name: &str) -> Result<String> {
    let mut entry = zip.by_name(name).with_context(|| format!("open {name}"))?;
    let mut s = String::new();
    entry.read_to_string(&mut s).with_context(|| format!("read {name}"))?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_argb_colour_keeps_alpha_first() {
        let c = argb("#ff3482ff").unwrap();
        assert_eq!((c.a, c.r, c.g, c.b), (255, 0x34, 0x82, 0xff));
        let t = argb("#99000000").unwrap();
        assert_eq!((t.a, t.r, t.g, t.b), (153, 0, 0, 0));
        assert!(argb("#fff").is_none());
        assert!(argb("3482ff").is_none());
    }

    #[test]
    fn a_description_gives_the_title_and_designer() {
        let xml = "<MIUI-Theme>\n<title>test</title>\n<designer>默认</designer>\n</MIUI-Theme>";
        assert_eq!(describe(xml), ("test".into(), "默认".into()));
    }

    #[test]
    fn colours_are_read_from_theme_values() {
        let xml = r#"<?xml version="1.0"?><MIUI_Theme_Values template="LightTemplate">
<color name="main_page_background" >#ffffffff</color>
<color name="miuix_color_blue_light_primary_default" >#ff3482ff</color>
<dimen name="x">18dp</dimen></MIUI_Theme_Values>"#;
        let colors = colors_of(xml);
        assert_eq!(colors.len(), 2);
        assert_eq!(colors[0].0, "main_page_background");
        assert_eq!(colors[1].1, skyward_ui::Rgba::rgb(0x34, 0x82, 0xff));
    }
}
