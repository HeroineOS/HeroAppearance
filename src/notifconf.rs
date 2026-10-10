//! HeroNotify's settings (`~/.config/hero/notifications.toml`), edited with
//! toml_edit so comments are kept. HeroBar's do-not-disturb switch writes
//! the same file, so every change reads it fresh, changes one thing and
//! saves (no copy kept here to overwrite the bar's change with).

use std::path::PathBuf;

use toml_edit::{value, Array, DocumentMut, InlineTable, Item, Table};

/// Pop-up positions, as written and as shown.
pub const POSITIONS: [(&str, &str); 6] = [
    ("top-right", "Top right"),
    ("top-left", "Top left"),
    ("bottom-right", "Bottom right"),
    ("bottom-left", "Bottom left"),
    ("top", "Top middle"),
    ("bottom", "Bottom middle"),
];

/// Days a schedule can be on, as shown and as written.
pub const DAYS: [(&str, &[&str]); 3] = [("Every day", &[]), ("Weekdays (Mon-Fri)", &["mon", "tue", "wed", "thu", "fri"]), ("Weekends (Sat, Sun)", &["sat", "sun"])];

/// A do-not-disturb time.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Quiet {
    pub from: String,
    pub to: String,
    pub days: Vec<String>,
}

impl Quiet {
    /// "22:00 to 07:00, every day".
    pub fn describe(&self) -> String {
        let days = match DAYS.iter().find(|(_, d)| d.iter().map(|s| s.to_string()).collect::<Vec<_>>() == self.days) {
            Some((label, _)) => label.split(" (").next().unwrap_or(label).to_lowercase(),
            None => self.days.join(", "),
        };
        format!("{} to {}, {days}", self.from, self.to)
    }
}

/// What the page shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub position: usize,
    pub timeout: i64,
    pub max_shown: i64,
    pub width: i64,
    pub dnd: bool,
    pub urgent: bool,
    pub allow: Vec<String>,
    pub schedule: Vec<Quiet>,
}

pub fn path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hero/notifications.toml")
}

/// The file; if missing, HeroNotify's commented default.
fn doc() -> Result<DocumentMut, String> {
    let text = std::fs::read_to_string(path()).ok().or_else(|| {
        std::process::Command::new("heronotify").arg("--print-default-config").output().ok().filter(|o| o.status.success()).and_then(|o| String::from_utf8(o.stdout).ok())
    });
    text.unwrap_or_default().parse::<DocumentMut>().map_err(|e| format!("{}: {e}", path().display()))
}

fn save(doc: &DocumentMut) -> Result<(), String> {
    let p = path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension("toml.tmp");
    std::fs::write(&tmp, doc.to_string()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

pub fn read() -> Settings {
    let d = doc().unwrap_or_default();
    let dnd = d.get("do-not-disturb").and_then(Item::as_table_like);
    let get = |k: &str| dnd.and_then(|t| t.get(k));
    let strings = |v: Option<&Item>| -> Vec<String> { v.and_then(Item::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default() };
    let schedule = get("schedule")
        .and_then(Item::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_inline_table())
                .map(|t| Quiet {
                    from: t.get("from").and_then(|v| v.as_str()).unwrap_or("").into(),
                    to: t.get("to").and_then(|v| v.as_str()).unwrap_or("").into(),
                    days: t.get("days").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect()).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    let position = d.get("position").and_then(Item::as_str).unwrap_or("top-right");
    Settings {
        position: POSITIONS.iter().position(|(k, _)| *k == position).unwrap_or(0),
        timeout: d.get("timeout").and_then(Item::as_integer).unwrap_or(6),
        max_shown: d.get("max-shown").and_then(Item::as_integer).unwrap_or(3),
        width: d.get("width").and_then(Item::as_integer).unwrap_or(360),
        dnd: get("on").and_then(Item::as_bool).unwrap_or(false),
        urgent: get("urgent").and_then(Item::as_bool).unwrap_or(true),
        allow: strings(get("allow")),
        schedule,
    }
}

/// Sets a top-level key, keeping its comment.
pub fn set(key: &str, v: impl Into<toml_edit::Value>) -> Result<(), String> {
    let mut d = doc()?;
    let decor = d.get(key).and_then(|i| i.as_value()).map(|v| v.decor().clone());
    d[key] = value(v);
    if let (Some(dec), Some(v)) = (decor, d[key].as_value_mut()) {
        *v.decor_mut() = dec;
    }
    save(&d)
}

/// Sets a key of `[do-not-disturb]`, keeping its comment.
pub fn set_dnd(key: &str, v: impl Into<toml_edit::Value>) -> Result<(), String> {
    let mut d = doc()?;
    if !d.contains_table("do-not-disturb") {
        d["do-not-disturb"] = Item::Table(Table::new());
    }
    let t = &mut d["do-not-disturb"];
    let decor = t.get(key).and_then(|i| i.as_value()).map(|v| v.decor().clone());
    t[key] = value(v);
    if let (Some(dec), Some(v)) = (decor, t[key].as_value_mut()) {
        *v.decor_mut() = dec;
    }
    save(&d)
}

pub fn set_allow(apps: &[String]) -> Result<(), String> {
    let a: Array = apps.iter().map(String::as_str).collect();
    set_dnd("allow", a)
}

pub fn set_schedule(times: &[Quiet]) -> Result<(), String> {
    let mut a = Array::new();
    for q in times {
        let mut t = InlineTable::new();
        t.insert("from", q.from.as_str().into());
        t.insert("to", q.to.as_str().into());
        if !q.days.is_empty() {
            t.insert("days", toml_edit::Value::Array(q.days.iter().map(String::as_str).collect()));
        }
        a.push(t);
    }
    // One time per line once there are a few.
    if a.len() > 1 {
        for v in a.iter_mut() {
            v.decor_mut().set_prefix("\n  ");
        }
        a.set_trailing("\n");
        a.set_trailing_comma(true);
    }
    set_dnd("schedule", a)
}

/// "7:5" or "0705" → "07:05"; None if it's no time.
pub fn time(s: &str) -> Option<String> {
    let s = s.trim();
    let (h, m) = match s.split_once(':') {
        Some((h, m)) => (h, m),
        None if s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()) => s.split_at(2),
        None => (s, "0"),
    };
    let (h, m): (u32, u32) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    (h < 24 && m < 60).then(|| format!("{h:02}:{m:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times() {
        assert_eq!(time("22:00").as_deref(), Some("22:00"));
        assert_eq!(time("7:5").as_deref(), Some("07:05"));
        assert_eq!(time("0705").as_deref(), Some("07:05"));
        assert_eq!(time("7").as_deref(), Some("07:00"));
        assert_eq!(time("24:00"), None);
        assert_eq!(time("noon"), None);
    }

    #[test]
    fn descriptions() {
        assert_eq!(Quiet { from: "22:00".into(), to: "07:00".into(), days: vec![] }.describe(), "22:00 to 07:00, every day");
        let days = DAYS[2].1.iter().map(|s| s.to_string()).collect();
        assert_eq!(Quiet { from: "09:00".into(), to: "12:00".into(), days }.describe(), "09:00 to 12:00, weekends");
        assert_eq!(Quiet { from: "1:00".into(), to: "2:00".into(), days: vec!["fri".into()] }.describe(), "1:00 to 2:00, fri");
    }
}
