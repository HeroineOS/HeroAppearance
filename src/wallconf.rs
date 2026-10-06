//! HeroWallpaper's config (`~/.config/hero/wallpaper.toml`), edited with
//! toml_edit so comments and layout are kept. The wallpaper follows the
//! file as soon as it's saved. Picture, fill mode and color can be set for
//! one screen, in `[output."NAME"]`; the rest is for all of them.

use std::path::{Path, PathBuf};

use toml_edit::{value, DocumentMut, Item, Table};

/// Fill modes, as written in the config, and how they're shown.
pub const MODES: [(&str, &str); 5] = [("cover", "Fill"), ("contain", "Fit"), ("stretch", "Stretch"), ("center", "Center"), ("tile", "Tile")];

/// Used when the file doesn't exist and herowallpaper isn't installed.
const FALLBACK: &str = "# HeroWallpaper\nmode = \"cover\"\n";

/// Picture files the wallpaper shows.
pub const EXTENSIONS: [&str; 11] = ["jpg", "jpeg", "png", "apng", "gif", "webp", "avif", "bmp", "tif", "tiff", "qoi"];

pub struct WallDoc {
    path: PathBuf,
    doc: DocumentMut,
}

pub fn default_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hero/wallpaper.toml")
}

impl WallDoc {
    /// The user's file; if missing, the wallpaper's commented default.
    /// Nothing is written until an edit.
    pub fn load(path: PathBuf) -> Result<WallDoc, String> {
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => std::process::Command::new("herowallpaper")
                .arg("--print-default-config")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_else(|| FALLBACK.to_owned()),
        };
        let doc = text.parse::<DocumentMut>().map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(WallDoc { path, doc })
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp, self.doc.to_string())?;
        std::fs::rename(&tmp, &self.path)
    }

    fn screen(&self, name: &str) -> Option<&Table> {
        self.doc.get("output")?.as_table()?.get(name)?.as_table()
    }

    /// `key`'s own value at the top (`screen` None) or for one screen.
    fn own(&self, screen: Option<&str>, key: &str) -> Option<&toml_edit::Value> {
        match screen {
            None => self.doc.get(key)?.as_value(),
            Some(s) => self.screen(s)?.get(key)?.as_value(),
        }
    }

    /// What applies: the screen's own value, else the top one.
    fn value(&self, screen: Option<&str>, key: &str) -> Option<&toml_edit::Value> {
        self.own(screen, key).or_else(|| self.own(None, key))
    }

    pub fn str(&self, screen: Option<&str>, key: &str) -> String {
        self.value(screen, key).and_then(|v| v.as_str()).unwrap_or_default().to_owned()
    }

    pub fn int(&self, key: &str, default: i64) -> i64 {
        self.own(None, key).and_then(|v| v.as_integer()).unwrap_or(default)
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        self.own(None, key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    /// Whether a screen has settings of its own.
    pub fn has_own(&self, screen: &str) -> bool {
        self.screen(screen).is_some_and(|t| !t.is_empty())
    }

    fn table_mut(&mut self, screen: Option<&str>) -> &mut Table {
        let Some(name) = screen else { return self.doc.as_table_mut() };
        let outs = self.doc.entry("output").or_insert_with(|| {
            let mut t = Table::new();
            t.set_implicit(true);
            Item::Table(t)
        });
        if !outs.is_table() {
            *outs = Item::Table(Table::new());
        }
        let outs = outs.as_table_mut().unwrap();
        let t = outs.entry(name).or_insert_with(|| Item::Table(Table::new()));
        if !t.is_table() {
            *t = Item::Table(Table::new());
        }
        t.as_table_mut().unwrap()
    }

    /// Sets `key`, keeping the comment after it.
    pub fn set(&mut self, screen: Option<&str>, key: &str, v: impl Into<toml_edit::Value>) {
        let t = self.table_mut(screen);
        let decor = t.get(key).and_then(|i| i.as_value()).map(|v| v.decor().clone());
        t[key] = value(v);
        if let (Some(d), Some(v)) = (decor, t[key].as_value_mut()) {
            *v.decor_mut() = d;
        }
    }

    pub fn remove(&mut self, screen: Option<&str>, key: &str) {
        self.table_mut(screen).remove(key);
    }

    /// The screen goes back to what all screens show.
    pub fn forget_screen(&mut self, name: &str) {
        if let Some(outs) = self.doc.get_mut("output").and_then(|o| o.as_table_mut()) {
            outs.remove(name);
        }
    }
}

/// Folders wallpapers usually are in.
fn folders() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"));
    vec![
        home.join("Pictures/Wallpapers"),
        home.join("Pictures/wallpapers"),
        home.join("Pictures"),
        home.join("Wallpapers"),
        data.join("backgrounds"),
        data.join("wallpapers"),
        PathBuf::from("/usr/share/backgrounds"),
        PathBuf::from("/usr/share/wallpapers"),
    ]
}

fn is_picture(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Pictures in the usual wallpaper folders (a few levels down), at most
/// `max`. A KDE-style wallpaper (`NAME/contents/images/WxH.jpg`, one file
/// per screen size) counts once, at its largest.
pub fn scan(max: usize) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![];
    fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>, max: usize) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        entries.sort();
        if dir.ends_with("contents/images") {
            let size = |p: &PathBuf| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            if let Some(p) = entries.iter().filter(|p| is_picture(p)).max_by_key(|p| size(p)) {
                out.push(p.clone());
            }
            return;
        }
        for p in entries {
            if out.len() >= max {
                return;
            }
            let hidden = p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'));
            if hidden {
                continue;
            }
            if p.is_dir() {
                if depth > 0 {
                    walk(&p, depth - 1, out, max);
                }
            } else if is_picture(&p) {
                out.push(p);
            }
        }
    }
    for f in folders() {
        // ~/Pictures itself only at the top (photo libraries go deep).
        let depth = if f.ends_with("Pictures") { 0 } else { 3 };
        let mut found = vec![];
        walk(&f, depth, &mut found, max);
        for p in found {
            let p = std::fs::canonicalize(&p).unwrap_or(p);
            if out.len() < max && !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

/// Thumbnails (from `herowallpaper thumbnail`), in order; None where one
/// couldn't be made.
pub fn thumbnails(pictures: &[PathBuf]) -> Vec<Option<PathBuf>> {
    let out = std::process::Command::new("herowallpaper").arg("thumbnail").args(pictures).stderr(std::process::Stdio::null()).output();
    let text = out.ok().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let mut lines = text.lines().map(|l| (!l.is_empty()).then(|| PathBuf::from(l)));
    pictures.iter().map(|_| lines.next().flatten()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_inherit_and_override() {
        let mut d = WallDoc { path: PathBuf::new(), doc: "# mine\npath = \"/a.jpg\"  # pic\nmode = \"cover\"\n".parse().unwrap() };
        assert_eq!(d.str(Some("DP-1"), "path"), "/a.jpg");
        d.set(Some("DP-1"), "mode", "tile");
        d.set(None, "path", "/b.png");
        assert_eq!(d.str(Some("DP-1"), "mode"), "tile");
        assert_eq!(d.str(None, "mode"), "cover");
        assert_eq!(d.str(Some("DP-1"), "path"), "/b.png");
        assert!(d.has_own("DP-1"));
        let t = d.doc.to_string();
        assert!(t.starts_with("# mine\npath = \"/b.png\"  # pic"), "{t}");
        assert!(t.contains("[output.DP-1]") || t.contains("[output.\"DP-1\"]"), "{t}");
        d.forget_screen("DP-1");
        assert!(!d.has_own("DP-1"));
        d.set(None, "color", "#102030");
        d.remove(None, "color");
        assert_eq!(d.str(None, "color"), "");
    }
}
