//! Appearance: customize the HeroUI theme (every HeroUI program) and HeroBar.
//!
//! Edits are written ~300 ms after the last change, atomically; HeroBar
//! applies them within a second. bar.toml keeps its comments.

mod barconf;
mod layout;

use std::time::Duration;

use heroui::fltk::draw;
use heroui::fltk::enums::{Align, Color};
use heroui::fltk::prelude::WidgetExt;
use heroui::prelude::*;

use barconf::{BarDoc, KINDS, SECTIONS};

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Theme,
    Bar,
}

/// Editable theme colors: (label, getter, setter).
type ColorField = (&'static str, fn(&Theme) -> Color, fn(&mut Theme, Color));

const COLORS: [ColorField; 8] = [
    ("Accent", |t| t.accent, |t, c| t.accent = c),
    (
        "Text on accent",
        |t| t.accent_text,
        |t, c| t.accent_text = c,
    ),
    ("Background", |t| t.background, |t, c| t.background = c),
    (
        "Surface (cards, buttons)",
        |t| t.surface,
        |t, c| t.surface = c,
    ),
    (
        "Hover, fields, tracks",
        |t| t.surface_alt,
        |t, c| t.surface_alt = c,
    ),
    ("Text", |t| t.text, |t, c| t.text = c),
    ("Secondary text", |t| t.text_dim, |t, c| t.text_dim = c),
    ("Border", |t| t.border, |t, c| t.border = c),
];

const ACCENTS: [u32; 8] = [
    0xb46cff, 0x8a3ffc, 0x4c9aff, 0x3ec99a, 0xffb347, 0xff6b8b, 0xe5484d, 0x9aa0a6,
];

const POSITIONS: &[&str] = &["Top", "Bottom"];

struct Appearance {
    page: Page,
    theme: Theme,
    bar: Option<BarDoc>,
    bar_error: Option<String>,
    // [bar] values shown in the UI.
    position: usize,
    height: f64,
    reserve: bool,
    padding: f64,
    spacing: f64,
    bar_bg: String,
    bar_fg: String,
    islands: bool,
    island_style: usize,
    /// [style] sizes; 0 = default.
    style_padding: f64,
    style_margin: f64,
    style_icon: f64,
    style_font: f64,
    /// Groups and their modules.
    groups: Vec<(String, Vec<String>)>,
    /// Their names as shown.
    group_labels: Vec<String>,
    /// The group chosen in "Put in group".
    join_group: usize,
    sections: [Vec<String>; 3],
    add_kind: usize,
    add_section: usize,
    /// The module whose settings are shown.
    selected: Option<String>,
    edit: ModuleEdit,
    // Pending writes, debounced.
    theme_dirty: bool,
    bar_dirty: bool,
    generation: u64,
    status: String,
}

#[derive(Default, Clone)]
struct ModuleEdit {
    format: String,
    disconnected: String,
    interval: String,
    text: String,
    exec: String,
    on_click: String,
    /// As typed: "" = the kind's default, "none" = no icon.
    icon: String,
    // Taskbar
    show: usize,
    style: usize,
    pinned: Vec<barconf::Pinned>,
    /// The selected pinned entry (row of the flattened tree).
    pin_sel: Option<usize>,
    /// Typed in "Add app".
    pin_new: String,
    /// Details on hover.
    tooltip: bool,
    /// Clock calendar: weeks start on Sunday.
    sunday_first: bool,
    max_width: f64,
    fixed_width: bool,
    button_width: f64,
    task_workspace: usize,
    // Workspaces
    ws_show: usize,
    // Spacer
    width: f64,
    expand: bool,
    spacer_style: usize,
    // Group
    drawer: bool,
    members: Vec<String>,
    /// The group this module is in.
    in_group: Option<String>,
    /// volume/network/bluetooth: a click opens the popup.
    popup: bool,
    // Sizes; 0 = default
    padding: f64,
    icon_size: f64,
    font_size: f64,
}

const NET_SHOWS: &[&str] = &[
    "Network name",
    "Name and speeds",
    "Speeds",
    "Data used",
    "Icon only (signal strength / wired)",
    "Custom (Format below)",
];
const NET_FORMATS: [&str; 5] = [
    "{name}",
    "{name}  {icon:arrow-down}{down} {icon:arrow-up}{up}",
    "{icon:arrow-down}{down} {icon:arrow-up}{up}",
    "{icon:arrow-down}{down-total} {icon:arrow-up}{up-total}",
    "",
];

const SPACER_STYLES: &[&str] = &["Empty", "Line", "Dots"];
const SPACER_STYLE_KEYS: [&str; 3] = ["none", "line", "dots"];
const WS_SHOW: &[&str] = &["All", "With windows (and the shown one)"];
const TASK_WS: &[&str] = &["All workspaces", "Current workspace only"];

const ISLAND_STYLES: &[&str] = &["Sharp", "Rounded", "Pill"];
const TASK_SHOW: &[&str] = &["Pinned and running", "Running only", "Pinned only"];
const TASK_SHOW_KEYS: [&str; 3] = ["both", "running", "pinned"];
const TASK_STYLES: &[&str] = &["Icons (one per app)", "Icons and titles (one per window)"];
const TASK_STYLE_KEYS: [&str; 2] = ["icons", "icons-titles"];

#[derive(Clone)]
enum Msg {
    Page(Page),
    // Theme
    Mode(heroui::theme::Mode),
    Accent(u32),
    Color(usize, Color),
    Radius(f64),
    Spacing(f64),
    Padding(f64),
    FontSize(f64),
    Font(String),
    Animations(bool),
    // Bar
    Position(usize),
    Height(f64),
    Reserve(bool),
    BarPadding(f64),
    BarSpacing(f64),
    /// None: use the theme's.
    BarBg(Option<Color>),
    BarFg(Option<Color>),
    Islands(bool),
    IslandStyle(usize),
    /// A [style] size (key, value; 0 = default).
    StyleSize(&'static str, f64),
    /// A size of the selected module (key, value; 0 = default).
    ModSize(&'static str, f64),
    SpacerWidth(f64),
    SpacerExpand(bool),
    SpacerStyle(usize),
    WsShow(usize),
    TaskWorkspace(usize),
    Drawer(bool),
    Popup(bool),
    NetShow(usize),
    JoinGroupPick(usize),
    /// Move the selected module into the picked group.
    JoinGroup,
    /// Move the selected module out of its group, after the group.
    LeaveGroup,
    /// Edit member `i` of the selected group.
    EditMember(usize),
    /// Back to the group of the selected member.
    EditGroup,
    /// Edit the module with this name (from the group chips).
    SelectName(String),
    /// Add a module of the "Add" kind to the selected group.
    AddToGroup,
    TaskShow(usize),
    TaskStyle(usize),
    PinSel(usize),
    /// Move the selected pinned entry up (-1) or down (1) among its siblings.
    PinMove(i32),
    /// Into the folder just above it.
    PinIn,
    /// Out of its folder.
    PinOut,
    PinRemove,
    PinNewText(String),
    /// Add the typed app (into the selected folder, if one is).
    PinAdd,
    PinNewFolder,
    FolderName(String),
    FolderIcon(String),
    Tooltip(bool),
    SundayFirst(bool),
    MaxWidth(f64),
    FixedWidth(bool),
    ButtonWidth(f64),
    /// Move module (section, index) to (section, index among the others).
    MoveTo(usize, usize, usize, usize),
    RemoveSelected,
    Select(usize, usize),
    AddKind(usize),
    AddSection(usize),
    Add,
    Edit(&'static str, String),
    // Saving
    Flush(u64),
}

fn hex(c: Color) -> String {
    let (r, g, b) = c.to_rgb();
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn parse_hex(s: &str) -> Option<Color> {
    let h = s.trim().strip_prefix('#')?;
    let v = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
    match h.len() {
        3 => Some(Color::from_rgb(v(0, 1)? * 17, v(1, 1)? * 17, v(2, 1)? * 17)),
        6 => Some(Color::from_rgb(v(0, 2)?, v(2, 2)?, v(4, 2)?)),
        _ => None,
    }
}

impl Appearance {
    fn new() -> Appearance {
        let theme = Theme::load();
        let (bar, bar_error) = match BarDoc::load(barconf::default_path()) {
            Ok(d) => (Some(d), None),
            Err(e) => (None, Some(e)),
        };
        let mut a = Appearance {
            page: Page::Theme,
            theme,
            bar,
            bar_error,
            position: 0,
            height: 34.0,
            reserve: true,
            padding: 6.0,
            spacing: 4.0,
            bar_bg: String::new(),
            bar_fg: String::new(),
            islands: false,
            island_style: 1,
            style_padding: 0.0,
            style_margin: 0.0,
            style_icon: 0.0,
            style_font: 0.0,
            groups: Vec::new(),
            group_labels: Vec::new(),
            join_group: 0,
            sections: Default::default(),
            add_kind: 0,
            add_section: 2,
            selected: None,
            edit: ModuleEdit::default(),
            theme_dirty: false,
            bar_dirty: false,
            generation: 0,
            status: String::new(),
        };
        a.read_bar();
        a
    }

    fn read_bar(&mut self) {
        let Some(d) = &self.bar else { return };
        self.position = usize::from(d.bar_str("position", "top") == "bottom");
        self.height = d.bar_int("height", 34) as f64;
        self.reserve = d.bar_bool("reserve-space", true);
        self.padding = d.bar_int("padding", 6) as f64;
        self.spacing = d.bar_int("spacing", 4) as f64;
        self.bar_bg = d.style("background");
        self.bar_fg = d.style("foreground");
        self.islands = d.bar_bool("islands", false);
        self.island_style = match d.bar_str("island-style", "rounded").as_str() {
            "sharp" => 0,
            "pill" => 2,
            _ => 1,
        };
        // Unset sizes show what HeroBar uses then.
        let size = |k: &str, default: i32| d.style_int(k).unwrap_or(default as i64) as f64;
        let font = self.theme.font_size;
        self.style_font = size("font-size", font);
        self.style_icon = size("icon-size", self.style_font as i32 + 2);
        self.style_padding = size("module-padding", 10);
        self.style_margin = size("module-margin", 3);
        for (i, key) in SECTIONS.iter().enumerate() {
            self.sections[i] = d.section(key);
        }
        self.groups = d.groups();
        self.group_labels = self.groups.iter().map(|(g, _)| barconf::pretty(g)).collect();
    }

    fn load_edit(&mut self) {
        let (Some(d), Some(name)) = (&self.bar, &self.selected) else {
            return;
        };
        self.edit = ModuleEdit {
            // An unset network format shows the name; set to "" it's icon-only.
            format: if barconf::kind_of(name) == "network" && !d.module_has(name, "format") {
                "{name}".into()
            } else {
                d.module_str(name, "format")
            },
            disconnected: d.module_str(name, "format-disconnected"),
            interval: d.module_str(name, "interval"),
            text: d.module_str(name, "text"),
            exec: d.module_str(name, "exec"),
            on_click: d.module_str(name, "on-click"),
            icon: match d.module_icon(name) {
                None => String::new(),
                Some(i) if i.is_empty() => "none".into(),
                Some(i) => i,
            },
            show: TASK_SHOW_KEYS.iter().position(|k| *k == d.module_str(name, "show")).unwrap_or(0),
            style: TASK_STYLE_KEYS.iter().position(|k| *k == d.module_str(name, "style")).unwrap_or(0),
            pinned: d.pinned(name),
            pin_sel: None,
            pin_new: String::new(),
            tooltip: d.module_bool(name, "tooltip", true),
            sunday_first: d.module_str(name, "first-weekday") == "sunday",
            max_width: d.module_int(name, "max-width", 600) as f64,
            fixed_width: d.module_bool(name, "fixed-width", false),
            button_width: d.module_int(name, "button-width", 180) as f64,
            task_workspace: usize::from(d.module_str(name, "workspace") == "current"),
            ws_show: usize::from(d.module_str(name, "show") == "occupied"),
            width: d.module_int(name, "width", 12) as f64,
            expand: d.module_bool(name, "expand", false),
            spacer_style: SPACER_STYLE_KEYS.iter().position(|k| *k == d.module_str(name, "style")).unwrap_or(0),
            drawer: d.module_bool(name, "drawer", false),
            members: d.module_list(name, "modules"),
            in_group: d.groups().into_iter().find(|(_, m)| m.contains(name)).map(|(g, _)| g),
            popup: d.module_bool(name, "popup", true),
            padding: d.module_int(name, "padding", 0) as f64,
            icon_size: d.module_int(name, "icon-size", 0) as f64,
            font_size: d.module_int(name, "font-size", 0) as f64,
        };
    }

    /// Schedules a write ~300 ms after the last change. Theme edits also
    /// apply to this window right away.
    fn touched(&mut self, theme: bool) -> Task<Msg> {
        if theme {
            heroui::theme::set_current(self.theme.clone());
            self.theme_dirty = true;
        } else {
            self.bar_dirty = true;
        }
        self.generation += 1;
        let g = self.generation;
        Task::perform(move || {
            std::thread::sleep(Duration::from_millis(300));
            Msg::Flush(g)
        })
    }

    /// A new accent, with text on it kept readable.
    fn set_accent(&mut self, c: Color) {
        self.theme.accent = c;
        self.theme.accent_text = heroui::theme::contrast_text(c);
    }

    /// Creates a module of the "Add" kind (with a fresh name) and returns
    /// its name.
    fn new_module(&mut self) -> Option<String> {
        let kind = KINDS[self.add_kind].0;
        let d = self.bar.as_mut()?;
        let name = d.new_name(kind);
        match kind {
            "custom" => d.set_module(&name, "text", "New"),
            "group" => d.set_module_list(&name, "modules", &[]),
            _ => {}
        }
        Some(name)
    }

    /// Writes a key of the selected module.
    fn module_set(&mut self, key: &str, v: impl Into<toml_edit::Value>) -> Task<Msg> {
        if let Some(name) = self.selected.clone() {
            if let Some(d) = self.bar_mut() {
                d.set_module_value(&name, key, v);
            }
            return self.touched(false);
        }
        Task::none()
    }

    fn bar_mut(&mut self) -> Option<&mut BarDoc> {
        self.bar.as_mut()
    }

    fn write_sections(&mut self) -> Task<Msg> {
        let sections = self.sections.clone();
        if let Some(d) = self.bar_mut() {
            for (i, key) in SECTIONS.iter().enumerate() {
                d.set_section(key, &sections[i]);
            }
        }
        if let Some(d) = &self.bar {
            self.groups = d.groups();
            self.group_labels = self.groups.iter().map(|(g, _)| barconf::pretty(g)).collect();
        }
        self.touched(false)
    }
}

impl App for Appearance {
    type Message = Msg;

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Page(p) => self.page = p,

            Msg::Mode(mode) => {
                // A new base palette; the accent and the other settings stay.
                let old = self.theme.clone();
                self.theme = Theme::for_mode(mode);
                self.theme.accent = old.accent;
                self.theme.accent_text = heroui::theme::contrast_text(old.accent);
                self.theme.radius = old.radius;
                self.theme.spacing = old.spacing;
                self.theme.padding = old.padding;
                self.theme.font_size = old.font_size;
                self.theme.font = old.font;
                self.theme.animations = old.animations;
                self.theme.icon_theme = old.icon_theme;
                return self.touched(true);
            }
            Msg::Accent(c) => {
                self.set_accent(Color::from_hex(c));
                return self.touched(true);
            }
            Msg::Color(i, c) => {
                if i == 0 {
                    self.set_accent(c);
                } else {
                    (COLORS[i].2)(&mut self.theme, c);
                }
                return self.touched(true);
            }
            Msg::Radius(v) => {
                self.theme.radius = v.round() as i32;
                return self.touched(true);
            }
            Msg::Spacing(v) => {
                self.theme.spacing = v.round() as i32;
                return self.touched(true);
            }
            Msg::Padding(v) => {
                self.theme.padding = v.round() as i32;
                return self.touched(true);
            }
            Msg::FontSize(v) => {
                self.theme.font_size = v.round() as i32;
                return self.touched(true);
            }
            Msg::Font(f) => {
                self.theme.font = f;
                return self.touched(true);
            }
            Msg::Animations(on) => {
                self.theme.animations = on;
                return self.touched(true);
            }

            Msg::Position(i) => {
                self.position = i;
                let v = if i == 1 { "bottom" } else { "top" };
                if let Some(d) = self.bar_mut() {
                    d.set_bar("position", v);
                }
                return self.touched(false);
            }
            Msg::Height(v) => {
                self.height = v.round();
                let v = self.height as i64;
                if let Some(d) = self.bar_mut() {
                    d.set_bar("height", v);
                }
                return self.touched(false);
            }
            Msg::Reserve(on) => {
                self.reserve = on;
                if let Some(d) = self.bar_mut() {
                    d.set_bar("reserve-space", on);
                }
                return self.touched(false);
            }
            Msg::BarPadding(v) => {
                self.padding = v.round();
                let v = self.padding as i64;
                if let Some(d) = self.bar_mut() {
                    d.set_bar("padding", v);
                }
                return self.touched(false);
            }
            Msg::BarSpacing(v) => {
                self.spacing = v.round();
                let v = self.spacing as i64;
                if let Some(d) = self.bar_mut() {
                    d.set_bar("spacing", v);
                }
                return self.touched(false);
            }
            Msg::BarBg(c) => {
                let s = c.map(hex).unwrap_or_default();
                if let Some(d) = self.bar_mut() {
                    d.set_style("background", &s);
                }
                self.bar_bg = s;
                return self.touched(false);
            }
            Msg::BarFg(c) => {
                let s = c.map(hex).unwrap_or_default();
                if let Some(d) = self.bar_mut() {
                    d.set_style("foreground", &s);
                }
                self.bar_fg = s;
                return self.touched(false);
            }
            Msg::Islands(on) => {
                self.islands = on;
                if let Some(d) = self.bar_mut() {
                    d.set_bar("islands", on);
                }
                return self.touched(false);
            }
            Msg::IslandStyle(i) => {
                self.island_style = i;
                let v = ["sharp", "rounded", "pill"][i.min(2)];
                if let Some(d) = self.bar_mut() {
                    d.set_bar("island-style", v);
                }
                return self.touched(false);
            }
            Msg::StyleSize(key, v) => {
                let v = v.round();
                match key {
                    "module-padding" => self.style_padding = v,
                    "module-margin" => self.style_margin = v,
                    "icon-size" => self.style_icon = v,
                    _ => self.style_font = v,
                }
                if let Some(d) = self.bar_mut() {
                    d.set_style_int(key, Some(v as i64));
                }
                return self.touched(false);
            }
            Msg::ModSize(key, v) => {
                let v = v.round();
                match key {
                    "padding" => self.edit.padding = v,
                    "icon-size" => self.edit.icon_size = v,
                    _ => self.edit.font_size = v,
                }
                if v <= 0.0 {
                    let name = self.selected.clone();
                    if let (Some(name), Some(d)) = (name, self.bar_mut()) {
                        d.remove_module_key(&name, key);
                    }
                    return self.touched(false);
                }
                return self.module_set(key, v as i64);
            }
            Msg::SpacerWidth(v) => {
                self.edit.width = v.round();
                return self.module_set("width", v.round() as i64);
            }
            Msg::SpacerExpand(on) => {
                self.edit.expand = on;
                return self.module_set("expand", on);
            }
            Msg::SpacerStyle(i) => {
                self.edit.spacer_style = i;
                return self.module_set("style", SPACER_STYLE_KEYS[i.min(2)]);
            }
            Msg::WsShow(i) => {
                self.edit.ws_show = i;
                return self.module_set("show", if i == 1 { "occupied" } else { "all" });
            }
            Msg::TaskWorkspace(i) => {
                self.edit.task_workspace = i;
                return self.module_set("workspace", if i == 1 { "current" } else { "all" });
            }
            Msg::Drawer(on) => {
                self.edit.drawer = on;
                return self.module_set("drawer", on);
            }
            Msg::Popup(on) => {
                self.edit.popup = on;
                let name = self.selected.clone();
                if let (Some(name), Some(d)) = (name, self.bar_mut()) {
                    // On is the default: no key.
                    if on {
                        d.remove_module_key(&name, "popup");
                    } else {
                        d.set_module_value(&name, "popup", false);
                    }
                    return self.touched(false);
                }
            }
            Msg::NetShow(i) => {
                if let Some(f) = NET_FORMATS.get(i) {
                    self.edit.format = (*f).to_owned();
                    return self.module_set("format", *f);
                }
            }
            Msg::JoinGroupPick(i) => self.join_group = i,
            Msg::JoinGroup => {
                let (Some(name), Some((g, _))) = (self.selected.clone(), self.groups.get(self.join_group).cloned()) else {
                    return Task::none();
                };
                // Out of its section, to the end of the group.
                for sec in self.sections.iter_mut() {
                    if let Some(i) = sec.iter().position(|n| *n == name) {
                        sec.remove(i);
                        break;
                    }
                }
                if let Some(d) = self.bar_mut() {
                    let mut members = d.module_list(&g, "modules");
                    members.push(name);
                    d.set_module_list(&g, "modules", &members);
                }
                let t = self.write_sections();
                self.read_bar();
                self.load_edit();
                return t;
            }
            Msg::LeaveGroup => {
                let (Some(name), Some(g)) = (self.selected.clone(), self.edit.in_group.clone()) else { return Task::none() };
                if let Some(d) = self.bar_mut() {
                    let mut members = d.module_list(&g, "modules");
                    members.retain(|m| *m != name);
                    d.set_module_list(&g, "modules", &members);
                }
                // Right after the group in its section.
                let at = self.sections.iter().enumerate().find_map(|(s, sec)| sec.iter().position(|n| *n == g).map(|i| (s, i + 1)));
                let (s, i) = at.unwrap_or((2, self.sections[2].len()));
                self.sections[s].insert(i, name);
                let t = self.write_sections();
                self.read_bar();
                self.load_edit();
                return t;
            }
            Msg::EditMember(i) => {
                if let Some(m) = self.edit.members.get(i).cloned() {
                    self.selected = Some(m);
                    self.load_edit();
                }
            }
            Msg::SelectName(n) => {
                self.selected = Some(n);
                self.load_edit();
            }
            Msg::EditGroup => {
                if let Some(g) = self.edit.in_group.clone() {
                    self.selected = Some(g);
                    self.load_edit();
                }
            }
            Msg::AddToGroup => {
                let Some(g) = self.selected.clone() else { return Task::none() };
                if KINDS[self.add_kind].0 == "group" {
                    self.status = "Groups can't contain groups".into();
                    return Task::none();
                }
                let Some(name) = self.new_module() else { return Task::none() };
                if let Some(d) = self.bar_mut() {
                    let mut members = d.module_list(&g, "modules");
                    members.push(name.clone());
                    d.set_module_list(&g, "modules", &members);
                }
                self.read_bar();
                self.selected = Some(name);
                self.load_edit();
                return self.touched(false);
            }
            Msg::TaskShow(i) => {
                self.edit.show = i;
                return self.module_set("show", TASK_SHOW_KEYS[i.min(2)]);
            }
            Msg::TaskStyle(i) => {
                self.edit.style = i;
                return self.module_set("style", TASK_STYLE_KEYS[i.min(1)]);
            }
            Msg::PinSel(k) => self.edit.pin_sel = Some(k),
            Msg::PinNewText(t) => self.edit.pin_new = t,
            Msg::Tooltip(on) => {
                self.edit.tooltip = on;
                return self.module_set("tooltip", on);
            }
            Msg::SundayFirst(on) => {
                self.edit.sunday_first = on;
                return self.module_set("first-weekday", if on { "sunday" } else { "monday" });
            }
            Msg::PinMove(_) | Msg::PinIn | Msg::PinOut | Msg::PinRemove | Msg::PinAdd | Msg::PinNewFolder | Msg::FolderName(_) | Msg::FolderIcon(_) => {
                let rows = barconf::flatten(&self.edit.pinned);
                let sel = self.edit.pin_sel.and_then(|k| rows.get(k)).map(|(p, _)| p.clone());
                let list = &mut self.edit.pinned;
                // The entry to select afterwards.
                let mut after = sel.clone();
                match (msg, sel) {
                    (Msg::PinMove(d), Some(p)) => {
                        if let Some((parent, i)) = barconf::parent_mut(list, &p) {
                            let j = i as i32 + d;
                            if j >= 0 && (j as usize) < parent.len() {
                                parent.swap(i, j as usize);
                                let mut np = p.clone();
                                *np.last_mut().expect("non-empty") = j as usize;
                                after = Some(np);
                            }
                        }
                    }
                    (Msg::PinIn, Some(p)) => {
                        let i = *p.last().expect("non-empty");
                        if i > 0 {
                            let mut above = p.clone();
                            *above.last_mut().expect("non-empty") = i - 1;
                            if matches!(barconf::get(list, &above), Some(barconf::Pinned::Folder { .. })) {
                                if let Some((parent, i)) = barconf::parent_mut(list, &p) {
                                    let e = parent.remove(i);
                                    if let barconf::Pinned::Folder { apps, .. } = &mut parent[i - 1] {
                                        apps.push(e);
                                        let mut np = above.clone();
                                        np.push(apps.len() - 1);
                                        after = Some(np);
                                    }
                                }
                            }
                        }
                    }
                    (Msg::PinOut, Some(p)) if p.len() > 1 => {
                        let folder = p[..p.len() - 1].to_vec();
                        let e = barconf::parent_mut(list, &p).map(|(parent, i)| parent.remove(i));
                        if let (Some(e), Some((parent, fi))) = (e, barconf::parent_mut(list, &folder)) {
                            parent.insert(fi + 1, e);
                            let mut np = folder.clone();
                            *np.last_mut().expect("non-empty") = fi + 1;
                            after = Some(np);
                        }
                    }
                    (Msg::PinRemove, Some(p)) => {
                        if let Some((parent, i)) = barconf::parent_mut(list, &p) {
                            // A folder's apps stay, where it was.
                            if let barconf::Pinned::Folder { apps, .. } = parent.remove(i) {
                                for (k, a) in apps.into_iter().enumerate() {
                                    parent.insert(i + k, a);
                                }
                            }
                        }
                        after = None;
                    }
                    (Msg::PinAdd, sel) => {
                        let id = self.edit.pin_new.trim().trim_end_matches(".desktop").to_owned();
                        if id.is_empty() {
                            return Task::none();
                        }
                        self.edit.pin_new.clear();
                        let into = sel.filter(|p| matches!(barconf::get(list, p), Some(barconf::Pinned::Folder { .. })));
                        match into.as_ref().and_then(|p| barconf::parent_mut(list, p)) {
                            Some((parent, i)) => {
                                if let barconf::Pinned::Folder { apps, .. } = &mut parent[i] {
                                    apps.push(barconf::Pinned::App(id));
                                }
                            }
                            None => list.push(barconf::Pinned::App(id)),
                        }
                    }
                    (Msg::PinNewFolder, _) => {
                        list.push(barconf::Pinned::Folder { name: "New folder".into(), icon: None, apps: vec![] });
                        after = Some(vec![list.len() - 1]);
                    }
                    (Msg::FolderName(n), Some(p)) => {
                        if let Some((parent, i)) = barconf::parent_mut(list, &p) {
                            if let barconf::Pinned::Folder { name, .. } = &mut parent[i] {
                                *name = n;
                            }
                        }
                    }
                    (Msg::FolderIcon(v), Some(p)) => {
                        if let Some((parent, i)) = barconf::parent_mut(list, &p) {
                            if let barconf::Pinned::Folder { icon, .. } = &mut parent[i] {
                                *icon = (!v.trim().is_empty()).then(|| v.trim().to_owned());
                            }
                        }
                    }
                    _ => return Task::none(),
                }
                self.edit.pin_sel = after.and_then(|p| barconf::flatten(&self.edit.pinned).iter().position(|(q, _)| *q == p));
                let pinned = self.edit.pinned.clone();
                let name = self.selected.clone();
                if let (Some(name), Some(d)) = (name, self.bar_mut()) {
                    d.set_pinned(&name, &pinned);
                    return self.touched(false);
                }
            }
            Msg::MaxWidth(v) => {
                self.edit.max_width = v.round();
                return self.module_set("max-width", v.round() as i64);
            }
            Msg::FixedWidth(on) => {
                self.edit.fixed_width = on;
                return self.module_set("fixed-width", on);
            }
            Msg::ButtonWidth(v) => {
                self.edit.button_width = v.round();
                return self.module_set("button-width", v.round() as i64);
            }
            Msg::MoveTo(fs, fi, ts, ti) if fi < self.sections[fs].len() => {
                let m = self.sections[fs].remove(fi);
                let ti = ti.min(self.sections[ts].len());
                self.sections[ts].insert(ti, m.clone());
                self.selected = Some(m);
                self.load_edit();
                return self.write_sections();
            }
            Msg::RemoveSelected if self.edit.in_group.is_some() => {
                let (Some(name), Some(g)) = (self.selected.take(), self.edit.in_group.clone()) else { return Task::none() };
                if let Some(d) = self.bar_mut() {
                    let mut members = d.module_list(&g, "modules");
                    members.retain(|m| *m != name);
                    d.set_module_list(&g, "modules", &members);
                }
                self.selected = Some(g);
                self.read_bar();
                self.load_edit();
                return self.touched(false);
            }
            Msg::RemoveSelected => {
                if let Some(name) = self.selected.take() {
                    // The first occurrence; its [modules] settings stay in the
                    // file, so re-adding it keeps them.
                    for sec in self.sections.iter_mut() {
                        if let Some(i) = sec.iter().position(|n| *n == name) {
                            sec.remove(i);
                            break;
                        }
                    }
                    return self.write_sections();
                }
            }
            Msg::Select(s, i) => {
                self.selected = self.sections[s].get(i).cloned();
                self.load_edit();
            }
            Msg::AddKind(k) => self.add_kind = k,
            Msg::AddSection(sec) => self.add_section = sec,
            Msg::Add => {
                let s = self.add_section;
                let Some(name) = self.new_module() else { return Task::none() };
                self.sections[s].push(name.clone());
                self.selected = Some(name);
                self.load_edit();
                return self.write_sections();
            }
            Msg::Edit(key, v) => {
                match key {
                    "format" => self.edit.format = v.clone(),
                    "format-disconnected" => self.edit.disconnected = v.clone(),
                    "interval" => self.edit.interval = v.clone(),
                    "text" => self.edit.text = v.clone(),
                    "exec" => self.edit.exec = v.clone(),
                    "icon" => self.edit.icon = v.clone(),
                    _ => self.edit.on_click = v.clone(),
                }
                if key == "icon" {
                    // "" = the kind's default (no key), "none" = no icon.
                    let name = self.selected.clone();
                    if let (Some(name), Some(d)) = (name, self.bar_mut()) {
                        match v.trim() {
                            "" => d.remove_module_key(&name, "icon"),
                            "none" => d.set_module(&name, "icon", ""),
                            i => d.set_module(&name, "icon", i),
                        }
                        return self.touched(false);
                    }
                    return Task::none();
                }
                if key == "interval" && !v.trim().is_empty() && v.trim().parse::<f64>().is_err() {
                    return Task::none();
                }
                if let Some(name) = self.selected.clone() {
                    if let Some(d) = self.bar_mut() {
                        d.set_module(&name, key, &v);
                    }
                    return self.touched(false);
                }
            }

            Msg::Flush(g) if g == self.generation => {
                let mut errors = Vec::new();
                if std::mem::take(&mut self.theme_dirty) {
                    if let Err(e) = self.theme.save() {
                        errors.push(format!("theme: {e}"));
                    }
                }
                if std::mem::take(&mut self.bar_dirty) {
                    if let Some(Err(e)) = self.bar.as_ref().map(|d| d.save()) {
                        errors.push(format!("bar: {e}"));
                    }
                }
                self.status = if errors.is_empty() {
                    "Saved".into()
                } else {
                    errors.join("; ")
                };
            }
            _ => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<Self, Msg> {
        let nav = |label: &str, page: Page| {
            let l = label.to_owned();
            column(vec![
                primary_button(&l, Msg::Page(page))
                    .visible(move |s: &Appearance| s.page == page)
                    .fixed(36),
                button(&l, Msg::Page(page))
                    .visible(move |s: &Appearance| s.page != page)
                    .fixed(36),
            ])
            .fixed(36)
            .spacing(0)
        };
        row(vec![
            column(vec![
                heading("Appearance").fixed(40),
                nav("Theme", Page::Theme),
                nav("Bar", Page::Bar),
                spacer(),
                text(|s: &Appearance| s.status.clone()).fixed(24),
            ])
            .fixed(170),
            column(vec![
                theme_page().visible(|s: &Appearance| s.page == Page::Theme),
                bar_page().visible(|s: &Appearance| s.page == Page::Bar),
            ]),
        ])
        .padding(16)
        .spacing(20)
    }
}

/// Label on the left, control on the right.
fn setting(name: &str, control: Element<Appearance, Msg>, width: i32) -> Element<Appearance, Msg> {
    row(vec![label(name), control.fixed(width)]).fixed(34)
}

fn int_slider(
    name: &str,
    range: std::ops::RangeInclusive<f64>,
    get: fn(&Appearance) -> f64,
    msg: fn(f64) -> Msg,
) -> Element<Appearance, Msg> {
    row(vec![
        label(name).fixed(170),
        slider(range, get, msg),
        text(move |s: &Appearance| format!("{}", get(s).round() as i64)).fixed(40),
    ])
    .fixed(30)
}

/// The edited theme drawn as a small sample window.
fn preview() -> Element<Appearance, Msg> {
    canvas(
        |s: &Appearance| s.theme.clone(),
        |t: &Theme, x, y, w, h, _| {
            let r = t.radius;
            draw::set_draw_color(t.background);
            draw::draw_rounded_rectf(x, y, w, h, r);
            let (cx, cy, cw, ch) = (x + 14, y + 14, w - 28, h - 28);
            draw::set_draw_color(t.surface);
            draw::draw_rounded_rectf(cx, cy, cw, ch, r);
            draw::set_font(heroui::fltk::enums::Font::HelveticaBold, t.font_size + 2);
            draw::set_draw_color(t.text);
            draw::draw_text2("Preview", cx + 14, cy + 10, cw - 28, 24, Align::Left);
            draw::set_font(heroui::fltk::enums::Font::Helvetica, t.font_size - 2);
            draw::set_draw_color(t.text_dim);
            draw::draw_text2("Secondary text", cx + 14, cy + 34, cw - 28, 18, Align::Left);
            // A primary and a normal button.
            let by = cy + ch - 46;
            draw::set_draw_color(t.accent);
            draw::draw_rounded_rectf(cx + 14, by, 100, 32, r.min(16));
            draw::set_font(heroui::fltk::enums::Font::Helvetica, t.font_size);
            draw::set_draw_color(t.accent_text);
            draw::draw_text2("Primary", cx + 14, by, 100, 32, Align::Center);
            draw::set_draw_color(t.surface_alt);
            draw::draw_rounded_rectf(cx + 124, by, 90, 32, r.min(16));
            draw::set_draw_color(t.text);
            draw::draw_text2("Button", cx + 124, by, 90, 32, Align::Center);
            // A toggle, on.
            let (tw, th) = (40, 22);
            let (tx, ty) = (cx + cw - tw - 14, by + 5);
            draw::set_draw_color(t.accent);
            draw::draw_rounded_rectf(tx, ty, tw, th, th / 2);
            draw::set_draw_color(t.accent_text);
            draw::draw_pie(tx + tw - th + 3, ty + 3, th - 6, th - 6, 0.0, 360.0);
        },
    )
}

fn theme_page() -> Element<Appearance, Msg> {
    let mut rows: Vec<Element<Appearance, Msg>> = vec![
        heading("Theme").fixed(36),
        caption("Used by every HeroUI program; open ones update right away.").fixed(22),
        preview().fixed(150),
        row(vec![
            label("Mode"),
            mode_button("System", heroui::theme::Mode::System),
            mode_button("Dark", heroui::theme::Mode::Dark),
            mode_button("Light", heroui::theme::Mode::Light),
        ])
        .fixed(34),
        caption("System follows your desktop's dark/light setting.").fixed(20),
        row(ACCENTS
            .iter()
            .map(|&c| button_like_swatch(c))
            .chain([spacer()])
            .collect())
        .fixed(34),
    ];
    for (i, (name, get, _)) in COLORS.iter().enumerate() {
        let get = *get;
        rows.push(
            row(vec![
                label(name),
                caption_text(move |s: &Appearance| hex(get(&s.theme))).fixed(76),
                color_button(move |s: &Appearance| get(&s.theme), move |c| Msg::Color(i, c)).fixed(44),
            ])
            .fixed(36),
        );
    }
    rows.extend([
        int_slider(
            "Corner radius",
            0.0..=20.0,
            |s| s.theme.radius as f64,
            Msg::Radius,
        ),
        int_slider(
            "Spacing",
            0.0..=24.0,
            |s| s.theme.spacing as f64,
            Msg::Spacing,
        ),
        int_slider(
            "Card padding",
            0.0..=32.0,
            |s| s.theme.padding as f64,
            Msg::Padding,
        ),
        int_slider(
            "Font size",
            9.0..=22.0,
            |s| s.theme.font_size as f64,
            Msg::FontSize,
        ),
        setting(
            "Font (family name, empty = default)",
            text_input(|s: &Appearance| s.theme.font.clone(), Msg::Font),
            200,
        ),
        toggle(
            "Animations",
            |s: &Appearance| s.theme.animations,
            Msg::Animations,
        )
        .fixed(30),
        caption("Off: changes happen instantly (reduced motion, saves battery).").fixed(20),
    ]);
    scroll(rows)
}

/// One option of the mode selector; the chosen one is highlighted.
fn mode_button(name: &str, mode: heroui::theme::Mode) -> Element<Appearance, Msg> {
    column(vec![
        primary_button(name, Msg::Mode(mode)).visible(move |a: &Appearance| a.theme.mode == mode).fixed(34),
        button(name, Msg::Mode(mode)).visible(move |a: &Appearance| a.theme.mode != mode).fixed(34),
    ])
    .spacing(0)
    .fixed(90)
}

/// A clickable accent color swatch.
fn button_like_swatch(c: u32) -> Element<Appearance, Msg> {
    Element::new(move |ctx| {
        let t = ctx.theme_rc();
        let mut b = custom_button(move |b| {
            let s = b.w().min(b.h()) - 4;
            let (x, y) = (b.x() + (b.w() - s) / 2, b.y() + (b.h() - s) / 2);
            if heroui::hover::is_hovered(b) {
                draw::set_draw_color(t.text_dim);
                draw::draw_pie(x - 2, y - 2, s + 4, s + 4, 0.0, 360.0);
            }
            draw::set_draw_color(Color::from_hex(c));
            draw::draw_pie(x, y, s, s, 0.0, 360.0);
        });
        let emit = ctx.emitter();
        b.set_callback(move |_| emit(Msg::Accent(c)));
        b.as_base_widget()
    })
    .fixed(34)
}

const KIND_LABELS: &[&str] = &[
    "Clock",
    "CPU",
    "Memory",
    "Battery",
    "Network",
    "Volume",
    "Bluetooth",
    "Taskbar (apps and windows)",
    "Workspaces",
    "Spacer (space, line or dots)",
    "Group (modules together, or a drawer)",
    "Custom (text or command)",
];

/// Secondary (dim) text that changes with the state.
fn caption_text(f: impl Fn(&Appearance) -> String + 'static) -> Element<Appearance, Msg> {
    canvas(f, |s: &String, x, y, w, h, t: &Theme| {
        draw::set_font(t.font(), t.font_size - 1);
        draw::set_draw_color(t.text_dim);
        draw::draw_text2(s, x, y, w, h, Align::Right | Align::Inside);
    })
}

/// A bar color: the bar's own if set, else the theme's (shown, and
/// "Theme" resets to it).
fn bar_color(
    name: &str,
    own: fn(&Appearance) -> Option<Color>,
    theme: fn(&Appearance) -> Color,
    set: fn(Color) -> Msg,
    reset: Msg,
) -> Element<Appearance, Msg> {
    row(vec![
        label(name),
        caption_text(move |a| if own(a).is_some() { String::new() } else { "from the theme".into() }).fixed(110),
        button("Use theme", reset).fixed(110).visible(move |a: &Appearance| own(a).is_some()),
        color_button(move |a: &Appearance| own(a).unwrap_or_else(|| theme(a)), set).fixed(44),
    ])
    .fixed(36)
}

fn edit_field(
    name: &str,
    key: &'static str,
    get: fn(&ModuleEdit) -> String,
    shown: fn(&str) -> bool,
) -> Element<Appearance, Msg> {
    row(vec![
        label(name).fixed(150),
        text_input(
            move |a: &Appearance| get(&a.edit),
            move |v| Msg::Edit(key, v),
        ),
    ])
    .fixed(34)
    .visible(move |a: &Appearance| {
        a.selected
            .as_deref()
            .is_some_and(|n| shown(barconf::kind_of(n)))
    })
}

/// The selected module's group and its members (group first), when the
/// selection is a group or in one.
fn group_of(a: &Appearance) -> Vec<String> {
    let Some(sel) = a.selected.as_deref() else { return vec![] };
    let group = if barconf::kind_of(sel) == "group" { Some(sel.to_owned()) } else { a.edit.in_group.clone() };
    let Some(g) = group else { return vec![] };
    let members = a.groups.iter().find(|(n, _)| *n == g).map(|(_, m)| m.clone()).unwrap_or_default();
    std::iter::once(g).chain(members).collect()
}

/// Chips for a group and its modules: tap one to edit it, without
/// scrolling back to the bar preview.
fn group_chips() -> Element<Appearance, Msg> {
    use heroui::fltk::enums::{Event, FrameType};
    use heroui::fltk::frame::Frame;
    use heroui::fltk::prelude::{WidgetBase, WidgetExt};
    Element::new(|ctx| {
        // (names, selected)
        let st: std::rc::Rc<std::cell::RefCell<(Vec<String>, Option<String>)>> = Default::default();
        let chips = |names: &[String]| -> Vec<(i32, i32)> {
            let t = heroui::theme::current();
            draw::set_font(t.font(), t.font_size - 1);
            let mut x = 0;
            names
                .iter()
                .enumerate()
                .map(|(k, n)| {
                    let label = if k == 0 { format!("Group: {}", barconf::pretty(n)) } else { barconf::pretty(n) };
                    let w = draw::width(&label).ceil() as i32 + 24;
                    let r = (x, w);
                    x += w + 6;
                    r
                })
                .collect()
        };
        let mut f = Frame::default();
        f.set_frame(FrameType::NoBox);
        {
            let st = st.clone();
            f.draw(move |f| {
                let t = heroui::theme::current();
                let (names, sel) = &*st.borrow();
                let spans = chips(names);
                draw::set_font(t.font(), t.font_size - 1);
                for (k, (n, (x, w))) in names.iter().zip(spans).enumerate() {
                    let on = sel.as_deref() == Some(n.as_str());
                    let (cx, cy, ch) = (f.x() + x, f.y() + 4, f.h() - 8);
                    draw::set_draw_color(if on { t.accent } else if k == 0 { t.surface } else { t.surface_alt });
                    draw::draw_rounded_rectf(cx, cy, w, ch, t.radius.min(ch / 2));
                    draw::set_draw_color(if on { t.accent_text } else { t.text });
                    let label = if k == 0 { format!("Group: {}", barconf::pretty(n)) } else { barconf::pretty(n) };
                    draw::draw_text2(&label, cx, cy, w, ch, Align::Center);
                }
            });
        }
        let emit = ctx.emitter();
        {
            let st = st.clone();
            f.handle(move |f, ev| {
                if ev != Event::Push {
                    return false;
                }
                let px = heroui::fltk::app::event_x() - f.x();
                let names = st.borrow().0.clone();
                if let Some(k) = chips(&names).iter().position(|&(x, w)| px >= x && px < x + w) {
                    emit(Msg::SelectName(names[k].clone()));
                }
                true
            });
        }
        let mut w = f.clone();
        ctx.bind(move |a: &Appearance| {
            let now = (group_of(a), a.selected.clone());
            if *st.borrow() != now {
                *st.borrow_mut() = now;
                heroui::widgets::repaint(&mut w);
            }
        });
        f.as_base_widget()
    })
}

/// The taskbar's pinned apps and folders, as an indented list: select an
/// entry to move it (up, down, into the folder above, out of its folder)
/// or remove it; a selected folder can be renamed and given an icon.
fn pinned_editor() -> Element<Appearance, Msg> {
    let sel_folder = |a: &Appearance| {
        let rows = barconf::flatten(&a.edit.pinned);
        a.edit.pin_sel.and_then(|k| rows.get(k)).and_then(|(p, _)| match barconf::get(&a.edit.pinned, p) {
            Some(barconf::Pinned::Folder { name, icon, .. }) => Some((name.clone(), icon.clone().unwrap_or_default())),
            _ => None,
        })
    };
    column(vec![
        caption("Pinned apps and folders (tap one to move it, rename a folder...)").fixed(22),
        list(|a: &Appearance| barconf::flatten(&a.edit.pinned).len(), pinned_row),
        row(vec![
            button("Up", Msg::PinMove(-1)),
            button("Down", Msg::PinMove(1)),
            button("Into folder above", Msg::PinIn),
            button("Out of folder", Msg::PinOut),
            button("Remove", Msg::PinRemove),
        ])
        .fixed(34)
        .visible(|a: &Appearance| a.edit.pin_sel.is_some()),
        row(vec![
            label("Folder name").fixed(110),
            text_input(move |a: &Appearance| sel_folder(a).map(|f| f.0).unwrap_or_default(), Msg::FolderName),
            label("Icon").fixed(40),
            text_input(move |a: &Appearance| sel_folder(a).map(|f| f.1).unwrap_or_default(), Msg::FolderIcon).fixed(120),
        ])
        .fixed(34)
        .visible(move |a: &Appearance| sel_folder(a).is_some()),
        caption("Folder icon: empty shows small icons of its apps.").fixed(20).visible(move |a: &Appearance| sel_folder(a).is_some()),
        row(vec![
            text_input_submit(|a: &Appearance| a.edit.pin_new.clone(), Msg::PinNewText, Msg::PinAdd),
            primary_button("Add app", Msg::PinAdd).fixed(100),
            button("New folder", Msg::PinNewFolder).fixed(110),
        ])
        .fixed(34),
        caption("Apps by .desktop file name (foot, firefox-esr); into the selected folder if one is.").fixed(20),
    ])
    .spacing(6)
    .fixed_with(move |a: &Appearance| {
        let rows = barconf::flatten(&a.edit.pinned).len() as i32;
        let sel = if a.edit.pin_sel.is_some() { 40 } else { 0 };
        let folder = if sel_folder(a).is_some() { 66 } else { 0 };
        28 + rows * (32 + a.theme.spacing) + sel + folder + 40 + 26 + 10
    })
}

/// How a pinned row looks: (depth, label, icon, is a folder, selected).
type PinRow = (usize, String, String, bool, bool);

/// One entry of the pinned list.
fn pinned_row(k: usize) -> Element<Appearance, Msg> {
    Element::new(move |ctx| {
        let cur: std::rc::Rc<std::cell::RefCell<PinRow>> = Default::default();
        let mut b = custom_button({
            let cur = cur.clone();
            move |b| {
                let t = heroui::theme::current();
                let (depth, label, icon, _, selected) = &*cur.borrow();
                let x0 = b.x() + 6 + *depth as i32 * 22;
                if *selected {
                    draw::set_draw_color(t.accent);
                    draw::draw_rounded_rectf(x0 - 6, b.y(), b.x() + b.w() - x0 + 6, b.h(), t.radius.min(10));
                } else {
                    let a = heroui::hover::hover_amount(b);
                    if a > 0.0 {
                        draw::set_draw_color(heroui::widgets::mix(t.background, t.surface_alt, a));
                        draw::draw_rounded_rectf(x0 - 6, b.y(), b.x() + b.w() - x0 + 6, b.h(), t.radius.min(10));
                    }
                }
                let fg = if *selected { t.accent_text } else { t.text };
                if !heroui::icons::draw(icon, x0, b.y() + (b.h() - 18) / 2, 18, fg) {
                    heroui::icons::draw("app", x0, b.y() + (b.h() - 18) / 2, 18, fg);
                }
                draw::set_font(t.font(), t.font_size);
                draw::set_draw_color(fg);
                draw::draw_text2(label, x0 + 26, b.y(), b.w(), b.h(), Align::Left | Align::Inside);
            }
        });
        let emit = ctx.emitter();
        b.set_callback(move |_| emit(Msg::PinSel(k)));
        let mut w = b.clone();
        ctx.bind(move |a: &Appearance| {
            let rows = barconf::flatten(&a.edit.pinned);
            let Some((path, depth)) = rows.get(k) else { return };
            let (label, icon, folder) = match barconf::get(&a.edit.pinned, path) {
                Some(barconf::Pinned::App(id)) => (id.clone(), barconf::app_icon(id), false),
                Some(barconf::Pinned::Folder { name, icon, .. }) => (name.clone(), icon.clone().unwrap_or_else(|| "folder".into()), true),
                None => return,
            };
            let now = (*depth, label, icon, folder, a.edit.pin_sel == Some(k));
            if *cur.borrow() != now {
                *cur.borrow_mut() = now;
                heroui::widgets::repaint(&mut w);
            }
        });
        b.as_base_widget()
    })
    .fixed(32)
}

fn bar_page() -> Element<Appearance, Msg> {
    let mut rows: Vec<Element<Appearance, Msg>> = vec![
        heading("Bar").fixed(36),
        caption("HeroBar applies changes within a second. Comments in bar.toml are kept.")
            .fixed(22),
        text(|a: &Appearance| a.bar_error.clone().unwrap_or_default())
            .fixed(24)
            .visible(|a: &Appearance| a.bar_error.is_some()),
        setting(
            "Position",
            dropdown(
                |_: &Appearance| POSITIONS,
                |a: &Appearance| a.position,
                Msg::Position,
            ),
            140,
        ),
        int_slider("Height", 20.0..=64.0, |a| a.height, Msg::Height),
        toggle(
            "Reserve space (windows stay clear of the bar)",
            |a: &Appearance| a.reserve,
            Msg::Reserve,
        )
        .fixed(30),
        int_slider("Edge padding", 0.0..=32.0, |a| a.padding, Msg::BarPadding),
        int_slider("Module spacing", 0.0..=24.0, |a| a.spacing, Msg::BarSpacing),
        bar_color(
            "Background",
            |a| parse_hex(&a.bar_bg),
            |a| a.theme.background,
            |c| Msg::BarBg(Some(c)),
            Msg::BarBg(None),
        ),
        bar_color(
            "Text color",
            |a| parse_hex(&a.bar_fg),
            |a| a.theme.text,
            |c| Msg::BarFg(Some(c)),
            Msg::BarFg(None),
        ),
        toggle(
            "Islands (each module on its own background, see-through gaps)",
            |a: &Appearance| a.islands,
            Msg::Islands,
        )
        .fixed(30),
        setting(
            "Island corners",
            dropdown(|_: &Appearance| ISLAND_STYLES, |a: &Appearance| a.island_style, Msg::IslandStyle),
            140,
        )
        .visible(|a: &Appearance| a.islands),
        caption("Module sizes").fixed(24),
        int_slider("Font size", 8.0..=28.0, |a| a.style_font, |v| Msg::StyleSize("font-size", v)),
        int_slider("Icon size", 8.0..=40.0, |a| a.style_icon, |v| Msg::StyleSize("icon-size", v)),
        int_slider("Padding inside", 0.0..=32.0, |a| a.style_padding, |v| Msg::StyleSize("module-padding", v)),
        int_slider("Space above/below", 0.0..=16.0, |a| a.style_margin, |v| Msg::StyleSize("module-margin", v)),
        heading("Modules").fixed(36),
    ];
    rows.extend([
        caption("Tap a module to edit it; drag it to move it, also between sections.").fixed(20),
        layout::editor().fixed(76),
        row(vec![
            label("Add").fixed(40),
            dropdown(|_: &Appearance| KIND_LABELS, |a: &Appearance| a.add_kind, Msg::AddKind),
            label("to").fixed(24),
            dropdown(|_: &Appearance| &layout::SECTION_NAMES[..], |a: &Appearance| a.add_section, Msg::AddSection).fixed(120),
            primary_button("Add", Msg::Add).fixed(70),
        ])
        .fixed(34),
    ]);
    // Settings of the selected module.
    let not_taskbar = |k: &str| !matches!(k, "taskbar" | "spacer" | "group" | "workspaces");
    let not_custom = |k: &str| !matches!(k, "custom" | "taskbar" | "spacer" | "group" | "workspaces");
    let taskbar = |a: &Appearance| a.selected.as_deref().is_some_and(|n| barconf::kind_of(n) == "taskbar");
    fn is(kind: &'static str) -> impl Fn(&Appearance) -> bool + Copy {
        move |a: &Appearance| a.selected.as_deref().is_some_and(|n| barconf::kind_of(n) == kind)
    }
    let plain = |a: &Appearance| {
        a.selected
            .as_deref()
            .is_some_and(|n| !matches!(barconf::kind_of(n), "taskbar" | "spacer" | "group" | "workspaces"))
    };
    rows.extend([
        text(|a: &Appearance| {
            a.selected
                .as_deref()
                .map(|n| format!("Module: {}", barconf::pretty(n)))
                .unwrap_or_default()
        })
        .fixed(36)
        .visible(|a: &Appearance| a.selected.is_some()),
        group_chips().fixed(40).visible(|a: &Appearance| !group_of(a).is_empty()),
        row(vec![spacer(), button("Remove module", Msg::RemoveSelected).fixed(150)])
            .fixed(34)
            .visible(|a: &Appearance| a.selected.is_some()),
        edit_field("Format", "format", |e| e.format.clone(), not_custom),
        edit_field(
            "When offline",
            "format-disconnected",
            |e| e.disconnected.clone(),
            |k| k == "network",
        ),
        edit_field("Text", "text", |e| e.text.clone(), |k| k == "custom"),
        edit_field(
            "Command (shows output)",
            "exec",
            |e| e.exec.clone(),
            |k| k == "custom",
        ),
        edit_field(
            "Interval (seconds)",
            "interval",
            |e| e.interval.clone(),
            not_taskbar,
        ),
        edit_field(
            "On click (command)",
            "on-click",
            |e| e.on_click.clone(),
            not_taskbar,
        ),
        row(vec![
            label("Icon").fixed(150),
            text_input(|a: &Appearance| a.edit.icon.clone(), |v| Msg::Edit("icon", v)),
            icon(
                |a: &Appearance| match a.edit.icon.as_str() {
                    "" => barconf::default_icon(barconf::kind_of(a.selected.as_deref().unwrap_or(""))).to_owned(),
                    "none" => String::new(),
                    i => i.to_owned(),
                },
                20,
            )
            .fixed(34),
        ])
        .fixed(34)
        .visible(move |a: &Appearance| (plain(a) || is("group")(a)) && a.selected.is_some()),
        caption("Icon: a built-in name (cpu, terminal, apps, power...), an app icon name or a file. Empty = default, none = no icon.")
            .fixed(20)
            .visible(move |a: &Appearance| (plain(a) || is("group")(a)) && a.selected.is_some()),
        // Network
        setting(
            "Show",
            dropdown(
                |_: &Appearance| NET_SHOWS,
                |a: &Appearance| NET_FORMATS.iter().position(|f| *f == a.edit.format || (a.edit.format.is_empty() && *f == "{name}")).unwrap_or(4),
                Msg::NetShow,
            ),
            220,
        )
        .visible(is("network")),
        // Popups
        toggle("Click opens a popup (otherwise: runs On click)", |a: &Appearance| a.edit.popup, Msg::Popup)
            .fixed(30)
            .visible(|a: &Appearance| a.selected.as_deref().is_some_and(|n| matches!(barconf::kind_of(n), "volume" | "network" | "bluetooth"))),
        caption("With the popup, On click is what its Advanced/settings button runs.")
            .fixed(20)
            .visible(|a: &Appearance| a.edit.popup && a.selected.as_deref().is_some_and(|n| matches!(barconf::kind_of(n), "volume" | "network" | "bluetooth"))),
        toggle("Click shows a calendar", |a: &Appearance| a.edit.popup, Msg::Popup).fixed(30).visible(is("clock")),
        toggle("Weeks start on Sunday", |a: &Appearance| a.edit.sunday_first, Msg::SundayFirst)
            .fixed(30)
            .visible(move |a: &Appearance| is("clock")(a) && a.edit.popup),
        toggle("Details when the mouse rests on it", |a: &Appearance| a.edit.tooltip, Msg::Tooltip)
            .fixed(30)
            .visible(move |a: &Appearance| plain(a)),
        // Spacer
        int_slider("Width", 0.0..=200.0, |a| a.edit.width, Msg::SpacerWidth).visible(move |a: &Appearance| is("spacer")(a) && !a.edit.expand),
        toggle("Expand: share the free space (centers what's between)", |a: &Appearance| a.edit.expand, Msg::SpacerExpand)
            .fixed(30)
            .visible(is("spacer")),
        setting("Looks", dropdown(|_: &Appearance| SPACER_STYLES, |a: &Appearance| a.edit.spacer_style, Msg::SpacerStyle), 140)
            .visible(is("spacer")),
        // Workspaces
        setting("Show", dropdown(|_: &Appearance| WS_SHOW, |a: &Appearance| a.edit.ws_show, Msg::WsShow), 300)
            .visible(is("workspaces")),
        // Group
        toggle("Drawer: show only the icon until clicked", |a: &Appearance| a.edit.drawer, Msg::Drawer)
            .fixed(30)
            .visible(is("group")),
        caption("Modules in this group:").fixed(24).visible(is("group")),
        list(
            |a: &Appearance| if is("group")(a) { a.edit.members.len() } else { 0 },
            |i| {
                row(vec![
                    text(move |a: &Appearance| a.edit.members.get(i).map(|m| barconf::pretty(m)).unwrap_or_default()),
                    button("Edit", Msg::EditMember(i)).fixed(80),
                ])
                .fixed(34)
            },
        ),
        row(vec![
            label("Add").fixed(40),
            dropdown(|_: &Appearance| KIND_LABELS, |a: &Appearance| a.add_kind, Msg::AddKind),
            primary_button("Add to group", Msg::AddToGroup).fixed(130),
        ])
        .fixed(34)
        .visible(is("group")),
        // Membership
        row(vec![
            label("Put in group").fixed(150),
            dropdown(|a: &Appearance| &a.group_labels, |a: &Appearance| a.join_group, Msg::JoinGroupPick),
            button("Move", Msg::JoinGroup).fixed(80),
        ])
        .fixed(34)
        .visible(|a: &Appearance| {
            a.edit.in_group.is_none() && !a.groups.is_empty() && a.selected.as_deref().is_some_and(|n| barconf::kind_of(n) != "group")
        }),
        row(vec![
            text(|a: &Appearance| a.edit.in_group.as_deref().map(|g| format!("In group {}", barconf::pretty(g))).unwrap_or_default()),
            button("Back to group", Msg::EditGroup).fixed(140),
            button("Take out", Msg::LeaveGroup).fixed(110),
        ])
        .fixed(34)
        .visible(|a: &Appearance| a.edit.in_group.is_some()),
        // Sizes of this module
        int_slider("Padding (0 = default)", 0.0..=32.0, |a| a.edit.padding, |v| Msg::ModSize("padding", v)).visible(plain),
        int_slider("Icon size", 0.0..=40.0, |a| a.edit.icon_size, |v| Msg::ModSize("icon-size", v)).visible(plain),
        int_slider("Font size", 0.0..=28.0, |a| a.edit.font_size, |v| Msg::ModSize("font-size", v))
            .visible(move |a: &Appearance| plain(a) || is("workspaces")(a)),
        setting(
            "Show",
            dropdown(|_: &Appearance| TASK_SHOW, |a: &Appearance| a.edit.show, Msg::TaskShow),
            220,
        )
        .visible(taskbar),
        setting(
            "Buttons",
            dropdown(|_: &Appearance| TASK_STYLES, |a: &Appearance| a.edit.style, Msg::TaskStyle),
            260,
        )
        .visible(taskbar),
        pinned_editor().visible(taskbar),
        int_slider("Most room it takes", 100.0..=1600.0, |a| a.edit.max_width, Msg::MaxWidth).visible(taskbar),
        toggle("Always take that room (other modules never move)", |a: &Appearance| a.edit.fixed_width, Msg::FixedWidth)
            .fixed(30)
            .visible(taskbar),
        setting("Windows from", dropdown(|_: &Appearance| TASK_WS, |a: &Appearance| a.edit.task_workspace, Msg::TaskWorkspace), 260)
            .visible(taskbar),
        int_slider("Widest window button", 60.0..=400.0, |a| a.edit.button_width, Msg::ButtonWidth)
            .visible(move |a: &Appearance| taskbar(a) && a.edit.style == 1),
        text(
            |a: &Appearance| match a.selected.as_deref().map(barconf::kind_of) {
                Some("clock") => "Format: strftime, e.g. %a %d %b  %H:%M".into(),
                Some("cpu") => "Format placeholders: {usage}".into(),
                Some("memory") => "Format placeholders: {used} {total} {percent}".into(),
                Some("battery") => "Format placeholders: {capacity} {status}".into(),
                Some("network") => "Format: {name} {essid} {ifname} {signal} {down} {up} {down-total} {up-total}".into(),
                Some("volume") => "Format placeholders: {volume}".into(),
                Some("bluetooth") => "Format placeholders: {device} {count}".into(),
                Some("taskbar") => "Click: open or focus (again: next window); middle click: close.".into(),
                Some("workspaces") => "Click to switch; the mouse wheel steps through them.".into(),
                Some("spacer") => "Expanding spacers center the modules between them and the section's edge.".into(),
                Some("group") => "One background for all its modules. Groups can't contain groups.".into(),
                Some(_) => "Shows Text, or the first line the Command prints.".into(),
                None => String::new(),
            },
        )
        .fixed(24)
        .visible(|a: &Appearance| a.selected.is_some()),
        spacer().fixed(16),
    ]);
    scroll(rows)
}

fn main() {
    let settings = Settings::new("Appearance")
        .size(820, 640)
        .class("heroappearance");
    heroui::run(Appearance::new(), settings).unwrap();
}
