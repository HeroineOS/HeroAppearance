//! Reading and editing HeroBar's `bar.toml` while keeping its comments and
//! formatting (toml_edit), so GUI edits and hand edits can be mixed.

use std::path::PathBuf;

use toml_edit::{value, Array, DocumentMut, Item, Table};

/// Used when there's no bar.toml yet and `herobar --print-default-config`
/// isn't available.
const FALLBACK: &str = r#"# HeroBar configuration, written by Appearance.

[bar]
position = "top"
height = 34
reserve-space = true
padding = 6
spacing = 4
modules-left = []
modules-center = ["clock"]
modules-right = ["cpu", "memory"]

[style]

[modules.clock]
format = "%a %d %b  %H:%M"
"#;

pub const SECTIONS: [&str; 3] = ["modules-left", "modules-center", "modules-right"];

/// Module kinds that can be added, as (config name, label).
pub const KINDS: [(&str, &str); 8] = [
    ("clock", "Clock"),
    ("cpu", "CPU"),
    ("memory", "Memory"),
    ("battery", "Battery"),
    ("network", "Network"),
    ("volume", "Volume"),
    ("taskbar", "Taskbar (apps and windows)"),
    ("custom", "Custom (text or command)"),
];

/// Module keys stored as numbers.
const NUMBER_KEYS: [&str; 3] = ["interval", "max-width", "button-width"];

pub struct BarDoc {
    pub path: PathBuf,
    doc: DocumentMut,
}

/// `$XDG_CONFIG_HOME/hero/bar.toml`, else `~/.config/hero/bar.toml`.
pub fn default_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("hero").join("bar.toml")
}

impl BarDoc {
    /// The user's bar.toml; if missing or unreadable, HeroBar's commented
    /// default (or a minimal fallback). Nothing is written until an edit.
    pub fn load(path: PathBuf) -> Result<BarDoc, String> {
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => std::process::Command::new("herobar")
                .arg("--print-default-config")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .unwrap_or_else(|| FALLBACK.to_owned()),
        };
        let doc = text
            .parse::<DocumentMut>()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(BarDoc { path, doc })
    }

    /// Writes atomically (temp file + rename), so HeroBar's reload never
    /// sees a half-written file.
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("toml.tmp");
        std::fs::write(&tmp, self.doc.to_string())?;
        std::fs::rename(&tmp, &self.path)
    }

    fn table(&mut self, name: &str) -> &mut Table {
        if !self.doc.contains_table(name) {
            self.doc[name] = Item::Table(Table::new());
        }
        self.doc[name].as_table_mut().expect("is a table")
    }

    // --- [bar] -------------------------------------------------------------

    pub fn bar_str(&self, key: &str, default: &str) -> String {
        self.doc
            .get("bar")
            .and_then(|b| b.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_owned()
    }

    pub fn bar_int(&self, key: &str, default: i64) -> i64 {
        self.doc
            .get("bar")
            .and_then(|b| b.get(key))
            .and_then(|v| v.as_integer())
            .unwrap_or(default)
    }

    pub fn bar_bool(&self, key: &str, default: bool) -> bool {
        self.doc
            .get("bar")
            .and_then(|b| b.get(key))
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }

    pub fn set_bar(&mut self, key: &str, v: impl Into<toml_edit::Value>) {
        self.table("bar")[key] = value(v);
    }

    pub fn section(&self, key: &str) -> Vec<String> {
        self.doc
            .get("bar")
            .and_then(|b| b.get(key))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_section(&mut self, key: &str, names: &[String]) {
        let mut a = Array::new();
        for n in names {
            a.push(n.as_str());
        }
        // Keep the comment and spacing around an existing key.
        let bar = self.table("bar");
        match bar.get_mut(key).and_then(|i| i.as_value_mut()) {
            Some(v) => {
                let decor = v.decor().clone();
                *v = toml_edit::Value::Array(a);
                *v.decor_mut() = decor;
            }
            None => bar[key] = value(a),
        }
    }

    // --- [style] -----------------------------------------------------------

    pub fn style(&self, key: &str) -> String {
        self.doc
            .get("style")
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned()
    }

    /// Empty removes the override (the theme's value is used).
    pub fn set_style(&mut self, key: &str, v: &str) {
        let t = self.table("style");
        if v.is_empty() {
            t.remove(key);
        } else {
            t[key] = value(v);
        }
    }

    // --- [modules."name"] --------------------------------------------------

    pub fn module_str(&self, name: &str, key: &str) -> String {
        let item = self
            .doc
            .get("modules")
            .and_then(|m| m.get(name))
            .and_then(|t| t.get(key));
        match item {
            Some(i) if i.is_str() => i.as_str().unwrap_or("").to_owned(),
            Some(i) if i.is_integer() => i.as_integer().unwrap_or(0).to_string(),
            Some(i) if i.is_float() => i.as_float().unwrap_or(0.0).to_string(),
            // on-click = { action = "run-command", arg = "..." }
            Some(i) => i
                .get("arg")
                .and_then(|a| a.as_str())
                .unwrap_or("")
                .to_owned(),
            None => String::new(),
        }
    }

    /// Sets a module key; empty removes it. `interval` is stored as a number.
    pub fn set_module(&mut self, name: &str, key: &str, v: &str) {
        let modules = self.table("modules");
        modules.set_implicit(true);
        if !modules.contains_key(name) {
            modules[name] = Item::Table(Table::new());
        }
        let t = modules[name].as_table_mut().expect("module is a table");
        let v = v.trim_end_matches('\n');
        if v.trim().is_empty() && key != "icon" {
            t.remove(key);
        } else if NUMBER_KEYS.contains(&key) {
            match v.trim().parse::<f64>() {
                Ok(n) if n.fract() == 0.0 => t[key] = value(n as i64),
                Ok(n) => t[key] = value(n),
                Err(_) => {}
            }
        } else {
            t[key] = value(v);
        }
    }

    /// Sets a module key to any TOML value (bools, numbers, lists).
    pub fn set_module_value(&mut self, name: &str, key: &str, v: impl Into<toml_edit::Value>) {
        let modules = self.table("modules");
        modules.set_implicit(true);
        if !modules.contains_key(name) {
            modules[name] = Item::Table(Table::new());
        }
        modules[name][key] = value(v);
    }

    /// Removes a module key (the module's default applies).
    pub fn remove_module_key(&mut self, name: &str, key: &str) {
        if let Some(t) = self.doc.get_mut("modules").and_then(|m| m.get_mut(name)).and_then(|t| t.as_table_mut()) {
            t.remove(key);
        }
    }

    fn module_item(&self, name: &str, key: &str) -> Option<&Item> {
        self.doc.get("modules").and_then(|m| m.get(name)).and_then(|t| t.get(key))
    }

    /// The module's `icon`: None if not set (the kind's default applies),
    /// Some("") for no icon.
    pub fn module_icon(&self, name: &str) -> Option<String> {
        self.module_item(name, "icon").and_then(|i| i.as_str()).map(str::to_owned)
    }

    pub fn module_list(&self, name: &str, key: &str) -> Vec<String> {
        self.module_item(name, key)
            .and_then(|i| i.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
            .unwrap_or_default()
    }

    pub fn module_int(&self, name: &str, key: &str, default: i64) -> i64 {
        self.module_item(name, key).and_then(|i| i.as_integer()).unwrap_or(default)
    }

    pub fn module_bool(&self, name: &str, key: &str, default: bool) -> bool {
        self.module_item(name, key).and_then(|i| i.as_bool()).unwrap_or(default)
    }

    /// A name for a new custom module that isn't used yet.
    pub fn new_custom_name(&self) -> String {
        let used = |n: &str| self.doc.get("modules").is_some_and(|m| m.get(n).is_some());
        (1..)
            .map(|i| format!("custom/item{i}"))
            .find(|n| !used(n))
            .expect("unbounded")
    }

    #[cfg(test)]
    pub fn text(&self) -> String {
        self.doc.to_string()
    }
}

/// "custom/menu" → "menu", "cpu" → "CPU".
pub fn pretty(name: &str) -> String {
    if let Some(c) = name.strip_prefix("custom/") {
        return c.to_owned();
    }
    KINDS
        .iter()
        .find(|(k, _)| *k == name)
        // Without the explanation in parentheses.
        .map(|(_, l)| l.split(" (").next().unwrap_or(l).to_owned())
        .unwrap_or_else(|| name.to_owned())
}

/// The icon HeroBar shows for a module when its config doesn't set one
/// (the layout editor shows the clock and taskbar ones too).
pub fn default_icon(kind: &str) -> &'static str {
    match kind {
        "clock" => "clock",
        "cpu" => "cpu",
        "memory" => "memory",
        "battery" => "battery-80",
        "network" => "network-wireless",
        "volume" => "volume-high",
        "taskbar" => "app",
        _ => "",
    }
}

pub fn kind_of(name: &str) -> &str {
    if name.starts_with("custom/") {
        "custom"
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(text: &str) -> BarDoc {
        BarDoc {
            path: PathBuf::from("/nonexistent"),
            doc: text.parse().unwrap(),
        }
    }

    #[test]
    fn edits_keep_comments() {
        let mut d = doc("# top comment\n[bar]\n# where\nposition = \"top\" # inline\nmodules-left = [\"clock\"] # mine\n");
        d.set_bar("position", "bottom");
        d.set_section("modules-left", &["cpu".into(), "custom/a".into()]);
        d.set_module("custom/a", "text", "Hi");
        d.set_module("custom/a", "interval", "5");
        d.set_style("background", "#112233");
        let t = d.text();
        assert!(
            t.contains("# top comment") && t.contains("# where") && t.contains("# mine"),
            "{t}"
        );
        assert!(t.contains("position = \"bottom\""), "{t}");
        assert!(t.contains("modules-left = [\"cpu\", \"custom/a\"]"), "{t}");
        assert!(
            t.contains("[modules.\"custom/a\"]")
                && t.contains("text = \"Hi\"")
                && t.contains("interval = 5"),
            "{t}"
        );
        assert_eq!(d.module_str("custom/a", "interval"), "5");
        d.set_module("custom/a", "text", "");
        assert!(!d.text().contains("text = \"Hi\""));
    }

    #[test]
    fn reads_action_tables() {
        let d = doc(
            "[modules.\"custom/x\"]\non-click = { action = \"run-command\", arg = \"foot\" }\n",
        );
        assert_eq!(d.module_str("custom/x", "on-click"), "foot");
    }

    #[test]
    fn fallback_parses_and_names() {
        let d = doc(FALLBACK);
        assert_eq!(d.section("modules-center"), ["clock"]);
        assert_eq!(d.new_custom_name(), "custom/item1");
        assert_eq!(pretty("custom/menu"), "menu");
        assert_eq!(pretty("cpu"), "CPU");
    }

    #[test]
    fn module_values() {
        let mut d = doc("[modules.taskbar]\npinned = [\"foot\"]\n");
        assert_eq!(d.module_list("taskbar", "pinned"), ["foot"]);
        let mut a = Array::new();
        a.push("foot");
        a.push("firefox-esr");
        d.set_module_value("taskbar", "pinned", a);
        d.set_module_value("taskbar", "fixed-width", true);
        d.set_module("taskbar", "max-width", "400");
        assert_eq!(d.module_list("taskbar", "pinned"), ["foot", "firefox-esr"]);
        assert!(d.module_bool("taskbar", "fixed-width", false));
        assert_eq!(d.module_int("taskbar", "max-width", 600), 400);
        assert_eq!(d.module_icon("cpu"), None);
        d.set_module("cpu", "icon", "");
        assert_eq!(d.module_icon("cpu").as_deref(), Some(""), "empty icon = none, kept");
        d.remove_module_key("cpu", "icon");
        assert_eq!(d.module_icon("cpu"), None);
    }
}
