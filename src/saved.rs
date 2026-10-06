//! Themes the user saved, to come back to: `~/.config/heroui/themes/NAME.conf`,
//! in the theme file's own format. A saved theme is the whole look (mode,
//! colors, corners, spacing, fonts, icons), but not the animation settings,
//! which belong to the device (battery saver) rather than to a look.

use std::path::PathBuf;

use heroui::theme::Theme;

fn dir() -> Option<PathBuf> {
    Some(Theme::path()?.parent()?.join("themes"))
}

/// A name as a file name.
fn file_name(name: &str) -> String {
    let n: String = name.trim().chars().map(|c| if c == '/' || c == '\0' { '-' } else { c }).collect();
    format!("{}.conf", n.trim_start_matches('.'))
}

/// The look alone, without the device's animation settings.
fn look(t: &Theme) -> Theme {
    Theme { animations: true, frame_rate: 60, ..t.clone() }
}

/// Whether two themes look the same.
pub fn same(a: &Theme, b: &Theme) -> bool {
    look(a) == look(b)
}

/// `theme` with `saved`'s look (its own animation settings kept).
pub fn applied(saved: &Theme, theme: &Theme) -> Theme {
    Theme { animations: theme.animations, frame_rate: theme.frame_rate, ..saved.clone() }
}

/// The saved themes, by name.
pub fn load_all() -> Vec<(String, Theme)> {
    let Some(rd) = dir().and_then(|d| std::fs::read_dir(d).ok()) else { return vec![] };
    let mut out: Vec<(String, Theme)> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "conf"))
        .filter_map(|p| {
            let name = p.file_stem()?.to_string_lossy().into_owned();
            Some((name, Theme::from_conf(&std::fs::read_to_string(&p).ok()?)))
        })
        .collect();
    out.sort_by_key(|(n, _)| n.to_lowercase());
    out
}

pub fn save(name: &str, theme: &Theme) -> std::io::Result<()> {
    let dir = dir().ok_or_else(|| std::io::Error::other("no home directory"))?;
    std::fs::create_dir_all(&dir)?;
    let text: String = look(theme).to_conf().lines().filter(|l| !l.starts_with("animations") && !l.starts_with("frame_rate")).map(|l| format!("{l}\n")).collect();
    let path = dir.join(file_name(name));
    let tmp = path.with_extension("conf.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(tmp, path)
}

pub fn delete(name: &str) -> std::io::Result<()> {
    let dir = dir().ok_or_else(|| std::io::Error::other("no home directory"))?;
    std::fs::remove_file(dir.join(file_name(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_the_look() {
        let home = std::env::temp_dir().join(format!("heroappearance-saved-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &home);
        let mut t = Theme::light();
        t.radius = 3;
        t.accent = heroui::fltk::enums::Color::from_rgb(10, 200, 30);
        t.animations = false;
        save("Mine/1", &t).unwrap();
        let all = load_all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "Mine-1");
        assert!(same(&all[0].1, &t));
        // Animations stay as the device has them.
        let mut now = Theme::dark();
        now.animations = false;
        assert!(!applied(&all[0].1, &now).animations);
        delete("Mine/1").unwrap();
        assert!(load_all().is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }
}
