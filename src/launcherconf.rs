//! HeroLauncher's config (`~/.config/hero/launcher.toml`), edited with
//! toml_edit so comments and layout are kept. The launcher reads it each
//! time it opens.

use std::path::PathBuf;

use toml_edit::{value, DocumentMut};

/// The layouts, as written in the config, and how they're shown.
pub const LAYOUTS: [(&str, &str); 3] = [
    ("list", "List (icon, name, what it is)"),
    ("grid", "Grid (icons with names)"),
    ("split", "Split (favorites beside all apps)"),
];

/// Used when the file doesn't exist and herolauncher isn't installed.
const FALLBACK: &str = "# HeroLauncher\nfavorites = []\n";

pub struct LauncherDoc {
    path: PathBuf,
    doc: DocumentMut,
}

pub fn default_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hero/launcher.toml")
}

impl LauncherDoc {
    /// The user's file; if missing, the launcher's commented default.
    /// Nothing is written until an edit.
    pub fn load(path: PathBuf) -> Result<LauncherDoc, String> {
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => std::process::Command::new("herolauncher")
                .arg("--print-default-config")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_else(|| FALLBACK.to_owned()),
        };
        let doc = text.parse::<DocumentMut>().map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(LauncherDoc { path, doc })
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp, self.doc.to_string())?;
        std::fs::rename(&tmp, &self.path)
    }

    pub fn str(&self, key: &str) -> String {
        self.doc.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_owned()
    }

    pub fn int(&self, key: &str, default: i64) -> i64 {
        self.doc.get(key).and_then(|v| v.as_integer()).unwrap_or(default)
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        self.doc.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    /// Sets `key`, keeping the comment after it.
    pub fn set(&mut self, key: &str, v: impl Into<toml_edit::Value>) {
        let decor = self.doc.get(key).and_then(|i| i.as_value()).map(|v| v.decor().clone());
        self.doc[key] = value(v);
        if let (Some(d), Some(v)) = (decor, self.doc[key].as_value_mut()) {
            *v.decor_mut() = d;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_keep_comments() {
        let mut d = LauncherDoc { path: PathBuf::new(), doc: "# mine\nlayout = \"list\"   # how\n".parse().unwrap() };
        d.set("layout", "split");
        d.set("categories", false);
        let t = d.doc.to_string();
        assert!(t.contains("# mine") && t.contains("layout = \"split\"   # how") && t.contains("categories = false"), "{t}");
        assert_eq!((d.str("layout").as_str(), d.bool("categories", true), d.int("height", 540)), ("split", false, 540));
    }
}
