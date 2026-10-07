//! Appearance: customize the HeroUI theme (every HeroUI program) and HeroBar.
//!
//! Edits are written ~300 ms after the last change, atomically; HeroBar
//! applies them within a second. bar.toml keeps its comments.

mod barconf;
mod launcherconf;
mod presets;
mod saved;
mod wallconf;
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
    Launcher,
    Wallpaper,
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
    launcher_dirty: bool,
    // HeroLauncher
    launcher: Option<launcherconf::LauncherDoc>,
    l_layout: usize,
    l_categories: bool,
    l_width: f64,
    l_height: f64,
    launcher_installed: bool,
    /// Themes the user saved, and the name typed for the next one.
    saved: Vec<(String, Theme)>,
    save_name: String,
    // HeroWallpaper
    wall: Option<wallconf::WallDoc>,
    wall_dirty: bool,
    /// The page was opened (pictures and screens looked up).
    w_loaded: bool,
    w_installed: bool,
    w_running: bool,
    /// "All screens", then each screen's name.
    w_outputs: Vec<String>,
    w_output: usize,
    /// The chosen screen has settings of its own.
    w_own: bool,
    w_path: String,
    w_mode: usize,
    w_theme_color: bool,
    w_color: Color,
    w_transition: f64,
    w_animate: bool,
    /// Pictures found, and their thumbnails (None: not made yet; Some(None):
    /// can't be).
    w_gallery: Vec<(std::path::PathBuf, Option<Option<std::path::PathBuf>>)>,
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
    short_units: bool,
    max_width: f64,
    fixed_width: bool,
    button_width: f64,
    task_workspace: usize,
    folder_style: usize,
    // Workspaces
    ws_show: usize,
    launcher_mode: usize,
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
const LAUNCHER_MODES: &[&str] = &["Drops down from the button", "Middle of the screen"];
const LAUNCHER_MODE_KEYS: [&str; 2] = ["menu", "center"];
const TASK_WS: &[&str] = &["All workspaces", "Current workspace only"];

const ISLAND_STYLES: &[&str] = &["Sharp", "Rounded", "Pill"];
const TASK_SHOW: &[&str] = &["Pinned and running", "Running only", "Pinned only"];
const TASK_SHOW_KEYS: [&str; 3] = ["both", "running", "pinned"];
const TASK_STYLES: &[&str] = &["Icons (one per app)", "Icons and titles (one per window)"];
const TASK_STYLE_KEYS: [&str; 2] = ["icons", "icons-titles"];
const FOLDER_STYLES: &[&str] = &["List (icons and names)", "Grid (icons, names below)", "Icons only"];
const FOLDER_STYLE_KEYS: [&str; 3] = ["list", "grid", "icons"];

#[derive(Clone)]
enum Msg {
    Page(Page),
    // Theme
    Mode(heroui::theme::Mode),
    Accent(u32),
    Preset(usize),
    Color(usize, Color),
    Radius(f64),
    Spacing(f64),
    Padding(f64),
    FontSize(f64),
    Font(String),
    Animations(bool),
    FrameRate(usize),
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
    LauncherMode(usize),
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
    /// Move member `i` of the selected group up (-1) or down (1).
    /// Move a module of the selected (or selected module's) group from one
    /// position to another.
    MemberMoveTo(usize, usize),
    /// Back to the group of the selected member.
    EditGroup,
    /// Edit the module with this name (from the group chips).
    SelectName(String),
    /// Add a module of the "Add" kind to the selected group.
    AddToGroup,
    TaskShow(usize),
    TaskStyle(usize),
    FolderStyle(usize),
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
    ShortUnits(bool),
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
    LLayout(usize),
    LCategories(bool),
    LWidth(f64),
    LHeight(f64),
    /// Open the launcher to see the changes.
    TryLauncher,
    // Saved themes
    SaveName(String),
    SaveTheme,
    LoadSaved(usize),
    DeleteSaved,
    // Wallpaper
    WScanned(Vec<std::path::PathBuf>),
    WThumbs(Vec<(std::path::PathBuf, Option<std::path::PathBuf>)>),
    WOutputs(Vec<String>),
    WRunning(bool),
    WStart,
    WOutput(usize),
    WSameAsAll,
    /// A gallery tile: 0 is "no picture".
    WPick(usize),
    WPathTyped(String),
    WPathSubmit,
    WBrowse,
    WMode(usize),
    WThemeColor(bool),
    WColor(Color),
    WTransition(f64),
    WAnimate(bool),
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
            launcher_dirty: false,
            launcher: launcherconf::LauncherDoc::load(launcherconf::default_path()).ok(),
            l_layout: 0,
            l_categories: true,
            l_width: 0.0,
            l_height: 540.0,
            launcher_installed: installed("herolauncher"),
            saved: saved::load_all(),
            save_name: String::new(),
            wall: wallconf::WallDoc::load(wallconf::default_path()).ok(),
            wall_dirty: false,
            w_loaded: false,
            w_installed: installed("herowallpaper"),
            w_running: false,
            w_outputs: vec!["All screens".into()],
            w_output: 0,
            w_own: false,
            w_path: String::new(),
            w_mode: 0,
            w_theme_color: true,
            w_color: Color::from_rgb(0x14, 0x14, 0x1c),
            w_transition: 450.0,
            w_animate: true,
            w_gallery: Vec::new(),
            generation: 0,
            status: String::new(),
        };
        a.read_bar();
        if let Some(d) = &a.launcher {
            let layout = d.str("layout");
            a.l_layout = launcherconf::LAYOUTS.iter().position(|(k, _)| *k == layout).unwrap_or(0);
            a.l_categories = d.bool("categories", true);
            a.l_width = d.int("width", 0) as f64;
            a.l_height = d.int("height", 540) as f64;
        }
        a.read_wall();
        a
    }

    /// The screen being set (None: all of them).
    fn w_screen(&self) -> Option<String> {
        (self.w_output > 0).then(|| self.w_outputs.get(self.w_output).cloned()).flatten()
    }

    /// The wallpaper settings shown, for the chosen screen.
    fn read_wall(&mut self) {
        let screen = self.w_screen();
        let Some(d) = &self.wall else { return };
        let sc = screen.as_deref();
        self.w_path = d.str(sc, "path");
        let mode = d.str(sc, "mode");
        self.w_mode = wallconf::MODES.iter().position(|(k, _)| *k == mode).unwrap_or(0);
        let color = parse_hex(&d.str(sc, "color"));
        self.w_theme_color = color.is_none();
        self.w_color = color.unwrap_or(self.theme.background);
        self.w_transition = d.int("transition", 450) as f64;
        self.w_animate = d.bool("animate", true);
        self.w_own = sc.is_some_and(|s| d.has_own(s));
    }

    /// Writes a key of HeroWallpaper's config (debounced); `all` keys are
    /// for every screen.
    fn wall_set(&mut self, key: &str, v: impl Into<toml_edit::Value>, all: bool) -> Task<Msg> {
        let screen = if all { None } else { self.w_screen() };
        let Some(d) = self.wall.as_mut() else { return Task::none() };
        d.set(screen.as_deref(), key, v);
        self.w_own = screen.as_deref().is_some_and(|s| d.has_own(s));
        self.wall_touched()
    }

    fn wall_touched(&mut self) -> Task<Msg> {
        self.wall_dirty = true;
        self.generation += 1;
        let g = self.generation;
        Task::perform(move || {
            std::thread::sleep(Duration::from_millis(300));
            Msg::Flush(g)
        })
    }

    /// First visit: find pictures, screens, and whether it's running.
    fn wall_load(&mut self) -> Task<Msg> {
        Task::batch([
            Task::perform(|| Msg::WScanned(wallconf::scan(120))),
            Task::perform(|| {
                let out = std::process::Command::new("herowallpaper").arg("--outputs").stderr(std::process::Stdio::null()).output();
                let names = out.ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::to_owned).collect()).unwrap_or_default();
                Msg::WOutputs(names)
            }),
            check_running(0),
        ])
    }

    /// Thumbnails for the next few pictures without one.
    fn next_thumbs(&mut self) -> Task<Msg> {
        let batch: Vec<std::path::PathBuf> = self.w_gallery.iter().filter(|(_, t)| t.is_none()).take(6).map(|(p, _)| p.clone()).collect();
        if batch.is_empty() || !self.w_installed {
            return Task::none();
        }
        Task::perform(move || {
            let thumbs = wallconf::thumbnails(&batch);
            Msg::WThumbs(batch.into_iter().zip(thumbs).collect())
        })
    }

    /// Shows `path` (and lists it, if it's new).
    fn wall_pick(&mut self, path: String) -> Task<Msg> {
        self.w_path = path.clone();
        let mut more = Task::none();
        if !path.is_empty() {
            let p = std::path::PathBuf::from(&path);
            if !self.w_gallery.iter().any(|(g, _)| *g == p) {
                self.w_gallery.insert(0, (p, None));
                more = self.next_thumbs();
            }
        }
        Task::batch([self.wall_set("path", path, false), more])
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
            short_units: d.module_str(name, "units") == "short",
            max_width: d.module_int(name, "max-width", 600) as f64,
            fixed_width: d.module_bool(name, "fixed-width", false),
            button_width: d.module_int(name, "button-width", 180) as f64,
            task_workspace: usize::from(d.module_str(name, "workspace") == "current"),
            folder_style: FOLDER_STYLE_KEYS.iter().position(|k| *k == d.module_str(name, "folder-style")).unwrap_or(0),
            ws_show: usize::from(d.module_str(name, "show") == "occupied"),
            launcher_mode: usize::from(d.module_str(name, "mode") == "center"),
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

    /// Writes a key of HeroLauncher's config (debounced).
    fn launcher_set(&mut self, key: &str, v: impl Into<toml_edit::Value>) -> Task<Msg> {
        let Some(d) = self.launcher.as_mut() else { return Task::none() };
        d.set(key, v);
        self.launcher_dirty = true;
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
            Msg::Page(p) => {
                self.page = p;
                if p == Page::Wallpaper && !std::mem::replace(&mut self.w_loaded, true) {
                    return self.wall_load();
                }
            }
            Msg::WScanned(found) => {
                // The picture shown comes first, found or not.
                let current = std::path::PathBuf::from(&self.w_path);
                let mut list: Vec<_> = found.into_iter().filter(|p| *p != current).map(|p| (p, None)).collect();
                if !self.w_path.is_empty() {
                    list.insert(0, (current, None));
                }
                self.w_gallery = list;
                return self.next_thumbs();
            }
            Msg::WThumbs(done) => {
                for (p, t) in done {
                    if let Some(g) = self.w_gallery.iter_mut().find(|(g, _)| *g == p) {
                        g.1 = Some(t);
                    }
                }
                return self.next_thumbs();
            }
            Msg::WOutputs(names) => {
                self.w_outputs = std::iter::once("All screens".to_string()).chain(names).collect();
                self.w_output = self.w_output.min(self.w_outputs.len() - 1);
            }
            Msg::WRunning(on) => self.w_running = on,
            Msg::WStart => {
                let _ = heroui::process::launch("herowallpaper");
                return check_running(600);
            }
            Msg::WOutput(i) => {
                self.w_output = i;
                self.read_wall();
            }
            Msg::WSameAsAll => {
                let screen = self.w_screen();
                if let (Some(d), Some(s)) = (self.wall.as_mut(), screen) {
                    d.forget_screen(&s);
                    self.read_wall();
                    return self.wall_touched();
                }
            }
            Msg::WPick(i) => {
                let path = if i == 0 { String::new() } else { self.w_gallery.get(i - 1).map(|(p, _)| p.to_string_lossy().into_owned()).unwrap_or_default() };
                return self.wall_pick(path);
            }
            Msg::WPathTyped(t) => self.w_path = t,
            Msg::WPathSubmit => {
                let path = self.w_path.trim().to_string();
                let p = std::path::PathBuf::from(&path);
                if !path.is_empty() && !p.is_file() {
                    self.status = "No such picture".into();
                    return Task::none();
                }
                let path = if path.is_empty() { path } else { std::fs::canonicalize(&p).unwrap_or(p).to_string_lossy().into_owned() };
                return self.wall_pick(path);
            }
            Msg::WBrowse => {
                use heroui::fltk::dialog::{NativeFileChooser, NativeFileChooserType};
                let mut fc = NativeFileChooser::new(NativeFileChooserType::BrowseFile);
                fc.set_title("Choose a picture or video");
                fc.set_filter(&format!("Pictures and videos\t*.{{{}}}", wallconf::EXTENSIONS.join(",")));
                if let Some(home) = std::env::var_os("HOME") {
                    let _ = fc.set_directory(&std::path::PathBuf::from(home).join("Pictures"));
                }
                fc.show();
                let f = fc.filename();
                if f.is_file() {
                    return self.wall_pick(f.to_string_lossy().into_owned());
                }
            }
            Msg::WMode(i) => {
                self.w_mode = i;
                return self.wall_set("mode", wallconf::MODES[i.min(4)].0, false);
            }
            Msg::WThemeColor(on) => {
                self.w_theme_color = on;
                if on {
                    let screen = self.w_screen();
                    if let Some(d) = self.wall.as_mut() {
                        d.remove(screen.as_deref(), "color");
                    }
                    return self.wall_touched();
                }
                return self.wall_set("color", hex(self.w_color), false);
            }
            Msg::WColor(c) => {
                self.w_color = c;
                self.w_theme_color = false;
                return self.wall_set("color", hex(c), false);
            }
            Msg::WTransition(v) => {
                self.w_transition = v.round();
                return self.wall_set("transition", v.round() as i64, true);
            }
            Msg::WAnimate(on) => {
                self.w_animate = on;
                return self.wall_set("animate", on, true);
            }
            Msg::LLayout(i) => {
                self.l_layout = i;
                return self.launcher_set("layout", launcherconf::LAYOUTS[i.min(2)].0);
            }
            Msg::LCategories(on) => {
                self.l_categories = on;
                return self.launcher_set("categories", on);
            }
            Msg::LWidth(v) => {
                // Below 240 means "a default for the layout" (0).
                let w = if v < 240.0 { 0 } else { v.round() as i64 };
                self.l_width = w as f64;
                return self.launcher_set("width", w);
            }
            Msg::LHeight(v) => {
                self.l_height = v.round();
                return self.launcher_set("height", v.round() as i64);
            }
            Msg::TryLauncher => {
                return Task::perform(|| {
                    // On its own (not our child), like the bar starts it.
                    let _ = heroui::process::launch("herolauncher");
                    Msg::Flush(u64::MAX)
                });
            }

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
            Msg::SaveName(n) => self.save_name = n,
            Msg::SaveTheme => {
                let name = self.save_name.trim().to_string();
                if name.is_empty() {
                    self.status = "Name it first".into();
                    return Task::none();
                }
                self.status = match saved::save(&name, &self.theme) {
                    Ok(()) => format!("Saved \"{name}\""),
                    Err(e) => format!("Couldn't save it: {e}"),
                };
                self.save_name.clear();
                self.saved = saved::load_all();
            }
            Msg::LoadSaved(i) => {
                if let Some((_, t)) = self.saved.get(i) {
                    self.theme = saved::applied(t, &self.theme);
                    return self.touched(true);
                }
            }
            Msg::DeleteSaved => {
                if let Some((name, _)) = self.saved.iter().find(|(_, t)| saved::same(t, &self.theme)) {
                    self.status = match saved::delete(name) {
                        Ok(()) => format!("Deleted \"{name}\""),
                        Err(e) => format!("Couldn't delete it: {e}"),
                    };
                    self.saved = saved::load_all();
                }
            }
            Msg::Preset(i) => {
                self.theme = presets::PRESETS[i].apply(&self.theme);
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
            Msg::FrameRate(i) => {
                self.theme.frame_rate = FRAME_RATES[i].0;
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
            Msg::LauncherMode(i) => {
                self.edit.launcher_mode = i;
                return self.module_set("mode", LAUNCHER_MODE_KEYS[i.min(1)]);
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
            Msg::MemberMoveTo(from, to) => {
                let names = group_of(self);
                let Some((g, members)) = names.split_first() else { return Task::none() };
                let mut members = members.to_vec();
                if from >= members.len() {
                    return Task::none();
                }
                let m = members.remove(from);
                members.insert(to.min(members.len()), m);
                let g = g.clone();
                if let Some(d) = self.bar_mut() {
                    d.set_module_list(&g, "modules", &members);
                }
                if let Some(d) = &self.bar {
                    self.groups = d.groups();
                }
                if self.selected.as_deref() == Some(g.as_str()) {
                    self.edit.members = members;
                }
                return self.touched(false);
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
            Msg::FolderStyle(i) => {
                self.edit.folder_style = i;
                return self.module_set("folder-style", FOLDER_STYLE_KEYS[i.min(2)]);
            }
            Msg::PinSel(k) => self.edit.pin_sel = Some(k),
            Msg::PinNewText(t) => self.edit.pin_new = t,
            Msg::Tooltip(on) => {
                self.edit.tooltip = on;
                return self.module_set("tooltip", on);
            }
            Msg::ShortUnits(on) => {
                self.edit.short_units = on;
                return self.module_set("units", if on { "short" } else { "long" });
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
                if std::mem::take(&mut self.wall_dirty) {
                    if let Some(Err(e)) = self.wall.as_ref().map(|d| d.save()) {
                        errors.push(format!("wallpaper: {e}"));
                    }
                }
                if std::mem::take(&mut self.launcher_dirty) {
                    if let Some(Err(e)) = self.launcher.as_ref().map(|d| d.save()) {
                        errors.push(format!("launcher: {e}"));
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
        const PAGES: [Page; 4] = [Page::Theme, Page::Bar, Page::Launcher, Page::Wallpaper];
        row(vec![
            column(vec![
                heading("Appearance").fixed(40),
                // The highlight slides to the chosen page.
                segmented(
                    &["Theme", "Bar", "Launcher", "Wallpaper"],
                    true,
                    |s: &Appearance| PAGES.iter().position(|&p| p == s.page).unwrap_or(0),
                    |i| Msg::Page(PAGES[i]),
                )
                .fixed(4 * 36 + 3 * 8),
                spacer(),
                text(|s: &Appearance| s.status.clone()).fixed(24),
            ])
            .fixed(170),
            column(vec![
                // Pages rise into place as they're switched to.
                theme_page().transition(|s: &Appearance| s.page == Page::Theme),
                bar_page().transition(|s: &Appearance| s.page == Page::Bar),
                launcher_page().transition(|s: &Appearance| s.page == Page::Launcher),
                wallpaper_page().transition(|s: &Appearance| s.page == Page::Wallpaper),
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

const LAYOUT_LABELS: [&str; 3] = [launcherconf::LAYOUTS[0].1, launcherconf::LAYOUTS[1].1, launcherconf::LAYOUTS[2].1];

/// HeroLauncher's settings. Favorites are added and ordered in the
/// launcher itself (right-click an app).
fn launcher_page() -> Element<Appearance, Msg> {
    scroll(vec![
        heading("Launcher").fixed(36),
        caption("HeroLauncher reads these each time it opens. Favorites: right-click an app in it.").fixed(22),
        setting("Layout", dropdown(|_: &Appearance| &LAYOUT_LABELS[..], |a: &Appearance| a.l_layout, Msg::LLayout), 300),
        toggle("Category buttons (Games, Internet, Office...)", |a: &Appearance| a.l_categories, Msg::LCategories).fixed(30),
        int_slider("Width (left: automatic)", 200.0..=1400.0, |a| if a.l_width < 240.0 { 200.0 } else { a.l_width }, Msg::LWidth),
        int_slider("Height", 300.0..=1200.0, |a| a.l_height, Msg::LHeight),
        row(vec![spacer(), primary_button("Try it", Msg::TryLauncher).fixed(120)]).fixed(34).visible(|a: &Appearance| a.launcher_installed),
        caption("Not installed? Install the herolauncher package.")
            .fixed(20)
            .visible(|a: &Appearance| !a.launcher_installed),
    ])
}

/// Whether a program is on the PATH.
fn installed(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(program).is_file()))
}

/// Asks (after `delay_ms`) whether the wallpaper is running.
fn check_running(delay_ms: u64) -> Task<Msg> {
    Task::perform(move || {
        std::thread::sleep(Duration::from_millis(delay_ms));
        let ok = std::process::Command::new("herowallpaper").arg("--running").status().is_ok_and(|s| s.success());
        Msg::WRunning(ok)
    })
}

const WALL_MODE_LABELS: [&str; 5] = [wallconf::MODES[0].1, wallconf::MODES[1].1, wallconf::MODES[2].1, wallconf::MODES[3].1, wallconf::MODES[4].1];

/// HeroWallpaper's settings: a gallery of the pictures found, or any file.
fn wallpaper_page() -> Element<Appearance, Msg> {
    scroll(vec![
        heading("Wallpaper").fixed(36),
        caption("HeroWallpaper shows changes as soon as they're saved.").fixed(22),
        row(vec![caption("HeroWallpaper isn't running."), spacer(), primary_button("Start it", Msg::WStart).fixed(120)])
            .fixed(34)
            .visible(|a: &Appearance| a.w_installed && !a.w_running),
        caption("Not installed? Install the herowallpaper package.").fixed(20).visible(|a: &Appearance| !a.w_installed),
        setting("Screen", dropdown(|a: &Appearance| &a.w_outputs[..], |a: &Appearance| a.w_output, Msg::WOutput), 260)
            .visible(|a: &Appearance| a.w_outputs.len() > 2),
        row(vec![caption("This screen has its own picture."), spacer(), button("Same as all screens", Msg::WSameAsAll).fixed(180)])
            .fixed(32)
            .visible(|a: &Appearance| a.w_output > 0 && a.w_own),
        gallery(),
        row(vec![
            text_input_submit(|a: &Appearance| a.w_path.clone(), Msg::WPathTyped, Msg::WPathSubmit),
            button("Browse…", Msg::WBrowse).fixed(110),
        ])
        .fixed(34),
        row(vec![
            label("Scaling"),
            segmented(&WALL_MODE_LABELS, false, |a: &Appearance| a.w_mode, Msg::WMode).fixed(5 * 76 + 4 * 8),
        ])
        .fixed(34),
        toggle("The theme's background color around it", |a: &Appearance| a.w_theme_color, Msg::WThemeColor).fixed(30),
        row(vec![
            label("Background color"),
            caption_text(|a: &Appearance| hex(a.w_color)).fixed(76),
            color_button(|a: &Appearance| a.w_color, Msg::WColor).fixed(44),
        ])
        .fixed(36)
        .visible(|a: &Appearance| !a.w_theme_color),
        label("All screens").fixed(24),
        int_slider("Crossfade (ms)", 0.0..=1500.0, |a| a.w_transition, Msg::WTransition),
        toggle("Play videos and animated pictures", |a: &Appearance| a.w_animate, Msg::WAnimate).fixed(30),
        caption("They hold still while animations are off (Theme page), as in battery saver. Videos need FFmpeg.").fixed(20),
    ])
}

const TILE_H: i32 = 96;
const TILE_COLS: i32 = 4;
const TILE_GAP: i32 = 10;

fn gallery_height(pictures: usize) -> i32 {
    let rows = (pictures as i32 + 1 + TILE_COLS - 1) / TILE_COLS;
    rows * TILE_H + (rows - 1).max(0) * TILE_GAP
}

/// The pictures as thumbnails, "no picture" first; the one shown has an
/// accent ring. One widget, drawing every tile.
fn gallery() -> Element<Appearance, Msg> {
    use heroui::fltk::enums::{Event, FrameType};
    use heroui::fltk::frame::Frame;
    use heroui::fltk::image::RgbImage;
    use heroui::fltk::prelude::{ImageExt, WidgetBase};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[derive(Default)]
    struct G {
        items: Vec<(PathBuf, Option<Option<PathBuf>>)>,
        selected: Option<usize>,
        hover: Option<usize>,
        color: Option<Color>,
        /// Thumbnails at tile size (None: unreadable).
        cache: HashMap<(PathBuf, i32, i32), Option<RgbImage>>,
    }
    fn tile_rect(f: &Frame, i: usize) -> (i32, i32, i32, i32) {
        let w = (f.w() - (TILE_COLS - 1) * TILE_GAP) / TILE_COLS;
        let (c, r) = (i as i32 % TILE_COLS, i as i32 / TILE_COLS);
        (f.x() + c * (w + TILE_GAP), f.y() + r * (TILE_H + TILE_GAP), w, TILE_H)
    }
    fn tile_at(f: &Frame, n: usize, x: i32, y: i32) -> Option<usize> {
        (0..=n).find(|&i| {
            let (tx, ty, tw, th) = tile_rect(f, i);
            x >= tx && y >= ty && x < tx + tw && y < ty + th
        })
    }
    /// The thumbnail scaled to cover the tile.
    fn cover(thumb: &std::path::Path, w: i32, h: i32) -> Option<RgbImage> {
        // Not SharedImage: that keeps every original in FLTK's cache.
        let img = heroui::fltk::image::PngImage::load(thumb).ok()?.to_rgb().ok()?;
        let (iw, ih) = (img.data_w().max(1) as f64, img.data_h().max(1) as f64);
        let s = (w as f64 / iw).max(h as f64 / ih);
        Some(img.copy_sized((iw * s).ceil() as i32, (ih * s).ceil() as i32))
    }

    Element::new(|ctx| {
        RgbImage::set_scaling_algorithm(heroui::fltk::image::RgbScaling::Bilinear);
        let g = Rc::new(RefCell::new(G::default()));
        let mut f = Frame::default();
        f.set_frame(FrameType::NoBox);
        {
            let g = g.clone();
            f.draw(move |f| {
                let t = heroui::theme::current();
                let mut g = g.borrow_mut();
                let r = t.radius.min(10);
                for i in 0..=g.items.len() {
                    let (x, y, w, h) = tile_rect(f, i);
                    let ring = if g.selected == Some(i) {
                        Some(t.accent)
                    } else if g.hover == Some(i) {
                        Some(t.text_dim)
                    } else {
                        None
                    };
                    let (x, y, w, h) = (x + 3, y + 3, w - 6, h - 6);
                    if let Some(c) = ring {
                        draw::set_draw_color(c);
                        draw::draw_rounded_rectf(x - 3, y - 3, w + 6, h + 6, r + 3);
                    }
                    draw::set_draw_color(t.border);
                    draw::draw_rounded_rectf(x, y, w, h, r);
                    if i == 0 {
                        draw::set_draw_color(g.color.unwrap_or(t.background));
                        draw::draw_rounded_rectf(x + 1, y + 1, w - 2, h - 2, (r - 1).max(0));
                        draw::set_font(t.font(), t.font_size);
                        draw::set_draw_color(t.text_dim);
                        draw::draw_text2("No picture", x, y, w, h, Align::Center);
                        continue;
                    }
                    draw::set_draw_color(t.surface);
                    draw::draw_rounded_rectf(x + 1, y + 1, w - 2, h - 2, (r - 1).max(0));
                    let (path, thumb) = g.items[i - 1].clone();
                    match thumb {
                        Some(Some(thumb)) => {
                            let (iw, ih) = (w - 2, h - 2);
                            let img = g.cache.entry((thumb.clone(), iw, ih)).or_insert_with(|| cover(&thumb, iw, ih));
                            if let Some(img) = img {
                                let (sw, sh) = (img.w(), img.h());
                                draw::push_clip(x + 1, y + 1, iw, ih);
                                img.draw(x + 1 - (sw - iw) / 2, y + 1 - (sh - ih) / 2, sw, sh);
                                draw::pop_clip();
                            }
                        }
                        // Not made yet, or not a picture it can show.
                        other => {
                            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            draw::set_font(t.font(), (t.font_size - 2).max(9));
                            draw::set_draw_color(t.text_dim);
                            let what = if other.is_none() { "…".to_string() } else { name };
                            draw::draw_text2(&what, x + 4, y, w - 8, h, Align::Center | Align::Clip | Align::Wrap);
                        }
                    }
                }
            });
        }
        {
            let (g, emit) = (g.clone(), ctx.emitter());
            f.handle(move |f, ev| match ev {
                Event::Enter | Event::Move | Event::Leave => {
                    let n = g.borrow().items.len();
                    let at = if ev == Event::Leave { None } else { tile_at(f, n, heroui::fltk::app::event_x(), heroui::fltk::app::event_y()) };
                    if std::mem::replace(&mut g.borrow_mut().hover, at) != at {
                        f.redraw();
                    }
                    true
                }
                Event::Push => true,
                Event::Released => {
                    let n = g.borrow().items.len();
                    if let Some(i) = tile_at(f, n, heroui::fltk::app::event_x(), heroui::fltk::app::event_y()) {
                        emit(Msg::WPick(i));
                    }
                    true
                }
                _ => false,
            });
        }
        let mut w = f.clone();
        ctx.bind(move |a: &Appearance| {
            let selected = if a.w_path.is_empty() { Some(0) } else { a.w_gallery.iter().position(|(p, _)| p.to_str() == Some(a.w_path.as_str())).map(|i| i + 1) };
            let color = (!a.w_theme_color).then_some(a.w_color);
            let mut g = g.borrow_mut();
            if g.items != a.w_gallery || g.selected != selected || g.color != color {
                g.items = a.w_gallery.clone();
                g.selected = selected;
                g.color = color;
                // Only tiles still shown keep their pictures.
                let keep: std::collections::HashSet<PathBuf> = g.items.iter().filter_map(|(_, t)| t.clone().flatten()).collect();
                g.cache.retain(|k, _| keep.contains(&k.0));
                heroui::widgets::repaint(&mut w);
            }
        });
        f.as_base_widget()
    })
    .fixed_with(|a: &Appearance| gallery_height(a.w_gallery.len()))
}

fn theme_page() -> Element<Appearance, Msg> {
    let mut rows: Vec<Element<Appearance, Msg>> = vec![
        heading("Theme").fixed(36),
        caption("Used by every HeroUI program; open ones update right away.").fixed(22),
        preview().fixed(150),
        label("Presets").fixed(24),
        caption("Pick one, then change any color below to make it yours.").fixed(20),
    ];
    for chunk in (0..presets::PRESETS.len()).collect::<Vec<_>>().chunks(4) {
        // A short last row keeps the cards' width.
        let cards = chunk.iter().map(|&i| preset_card(i)).chain(std::iter::repeat_with(spacer).take(4 - chunk.len()));
        rows.push(row(cards.collect()).fixed(60));
    }
    rows.extend([
        label("My themes").fixed(24),
        caption("Save the theme as it is now, to come back to it any time.").fixed(20),
        list(|a: &Appearance| a.saved.len().div_ceil(4), |r| row((0..4).map(|k| saved_card(r * 4 + k)).collect()).fixed(60)),
        row(vec![
            text_input_submit(|a: &Appearance| a.save_name.clone(), Msg::SaveName, Msg::SaveTheme),
            primary_button("Save", Msg::SaveTheme).fixed(90),
        ])
        .fixed(34),
        row(vec![
            caption_text(|a: &Appearance| a.saved.iter().find(|(_, t)| saved::same(t, &a.theme)).map(|(n, _)| format!("This is \"{n}\".")).unwrap_or_default()),
            button("Delete it", Msg::DeleteSaved).fixed(110),
        ])
        .fixed(34)
        .visible(|a: &Appearance| a.saved.iter().any(|(_, t)| saved::same(t, &a.theme))),
    ]);
    rows.extend([
        row(vec![
            label("Mode"),
            segmented(
                &["System", "Dark", "Light"],
                false,
                |a: &Appearance| MODES.iter().position(|&m| m == a.theme.mode).unwrap_or(0),
                |i| Msg::Mode(MODES[i]),
            )
            .fixed(3 * 90 + 2 * 8),
        ])
        .fixed(34),
        caption("System follows your desktop's dark/light setting.").fixed(20),
        row(ACCENTS
            .iter()
            .map(|&c| button_like_swatch(c))
            .chain([spacer()])
            .collect())
        .fixed(34),
    ]);
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
        setting(
            "Animation frame rate",
            dropdown(
                |_: &Appearance| FRAME_RATE_LABELS,
                |a: &Appearance| FRAME_RATES.iter().position(|&(f, _)| f == a.theme.frame_rate).unwrap_or(0),
                Msg::FrameRate,
            ),
            200,
        ),
        caption("Your screen's refresh rate looks smoothest. Only used while something moves.").fixed(20),
    ]);
    scroll(rows)
}

/// A preset theme, drawn in its own colors; the one in use is outlined.
fn preset_card(i: usize) -> Element<Appearance, Msg> {
    use std::cell::Cell;
    use std::rc::Rc;
    Element::new(move |ctx| {
        let p = &presets::PRESETS[i];
        let on = Rc::new(Cell::new(false));
        let mut b = custom_button({
            let on = on.clone();
            move |b| {
                let c = Color::from_hex;
                let t = heroui::theme::current();
                let (x, y, w, h) = (b.x() + 2, b.y() + 2, b.w() - 4, b.h() - 4);
                let r = t.radius.min(10);
                let a = heroui::hover::hover_amount(b);
                let ring = if on.get() { Some(t.accent) } else if a > 0.0 { Some(heroui::widgets::mix(t.background, t.text_dim, a)) } else { None };
                if let Some(ring) = ring {
                    draw::set_draw_color(ring);
                    draw::draw_rounded_rectf(x - 2, y - 2, w + 4, h + 4, r + 2);
                }
                draw::set_draw_color(c(p.border));
                draw::draw_rounded_rectf(x, y, w, h, r);
                draw::set_draw_color(c(p.background));
                draw::draw_rounded_rectf(x + 1, y + 1, w - 2, h - 2, (r - 1).max(0));
                // A card, an accent "button" and the name in its text color.
                draw::set_draw_color(c(p.surface));
                draw::draw_rounded_rectf(x + 8, y + h - 20, w - 16, 12, 4);
                draw::set_draw_color(c(p.accent));
                draw::draw_rounded_rectf(x + w - 34, y + h - 18, 22, 8, 4);
                draw::set_draw_color(c(p.text_dim));
                draw::draw_rounded_rectf(x + 14, y + h - 16, (w / 3).min(40), 4, 2);
                draw::set_font(t.font(), (t.font_size - 1).max(9));
                draw::set_draw_color(c(p.text));
                draw::draw_text2(p.name, x + 8, y + 4, w - 16, h - 26, Align::Left | Align::Inside | Align::Clip);
            }
        });
        let emit = ctx.emitter();
        b.set_callback(move |_| emit(Msg::Preset(i)));
        let mut w = b.clone();
        ctx.bind(move |a: &Appearance| {
            let now = presets::PRESETS[i].matches(&a.theme);
            if on.replace(now) != now {
                heroui::widgets::repaint(&mut w);
            }
        });
        b.as_base_widget()
    })
}

/// A saved theme's card: its colors and name, like the presets'. Empty
/// (and inert) past the end of the list.
fn saved_card(i: usize) -> Element<Appearance, Msg> {
    use std::cell::RefCell;
    use std::rc::Rc;
    Element::new(move |ctx| {
        // The theme shown, and whether it's the current one.
        let shown: Rc<RefCell<Option<(String, Theme, bool)>>> = Rc::default();
        let mut b = custom_button({
            let shown = shown.clone();
            move |b| {
                let Some((name, p, on)) = &*shown.borrow() else { return };
                let t = heroui::theme::current();
                let (x, y, w, h) = (b.x() + 2, b.y() + 2, b.w() - 4, b.h() - 4);
                let r = t.radius.min(10);
                let a = heroui::hover::hover_amount(b);
                let ring = if *on { Some(t.accent) } else if a > 0.0 { Some(heroui::widgets::mix(t.background, t.text_dim, a)) } else { None };
                if let Some(ring) = ring {
                    draw::set_draw_color(ring);
                    draw::draw_rounded_rectf(x - 2, y - 2, w + 4, h + 4, r + 2);
                }
                let pr = p.radius.min(10);
                draw::set_draw_color(p.border);
                draw::draw_rounded_rectf(x, y, w, h, pr);
                draw::set_draw_color(p.background);
                draw::draw_rounded_rectf(x + 1, y + 1, w - 2, h - 2, (pr - 1).max(0));
                draw::set_draw_color(p.surface);
                draw::draw_rounded_rectf(x + 8, y + h - 20, w - 16, 12, 4);
                draw::set_draw_color(p.accent);
                draw::draw_rounded_rectf(x + w - 34, y + h - 18, 22, 8, 4);
                draw::set_draw_color(p.text_dim);
                draw::draw_rounded_rectf(x + 14, y + h - 16, (w / 3).min(40), 4, 2);
                draw::set_font(t.font(), (t.font_size - 1).max(9));
                draw::set_draw_color(p.text);
                draw::draw_text2(name, x + 8, y + 4, w - 16, h - 26, Align::Left | Align::Inside | Align::Clip);
            }
        });
        let emit = ctx.emitter();
        {
            let shown = shown.clone();
            b.set_callback(move |_| {
                if shown.borrow().is_some() {
                    emit(Msg::LoadSaved(i));
                }
            });
        }
        let mut w = b.clone();
        ctx.bind(move |a: &Appearance| {
            let now = a.saved.get(i).map(|(n, t)| (n.clone(), t.clone(), saved::same(t, &a.theme)));
            if *shown.borrow() != now {
                *shown.borrow_mut() = now;
                heroui::widgets::repaint(&mut w);
            }
        });
        b.as_base_widget()
    })
}

/// Animation frame rates offered (frames per second, label).
const FRAME_RATES: [(i32, &str); 6] = [(60, "60 (most screens)"), (75, "75"), (90, "90"), (120, "120"), (144, "144"), (165, "165")];
const FRAME_RATE_LABELS: &[&str] = &["60 (most screens)", "75", "90", "120", "144", "165"];

const MODES: [heroui::theme::Mode; 3] = [heroui::theme::Mode::System, heroui::theme::Mode::Dark, heroui::theme::Mode::Light];

/// A clickable accent color swatch.
fn button_like_swatch(c: u32) -> Element<Appearance, Msg> {
    Element::new(move |ctx| {
        let t = ctx.theme_rc();
        let mut b = custom_button(move |b| {
            let s = b.w().min(b.h()) - 4;
            let (x, y) = (b.x() + (b.w() - s) / 2, b.y() + (b.h() - s) / 2);
            let a = heroui::hover::hover_amount(b);
            if a > 0.0 {
                draw::set_draw_color(heroui::widgets::mix(t.background, t.text_dim, a));
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
    "Launcher (apps menu)",
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

/// A group and its modules as a small bar: tap one to edit it (without
/// scrolling back to the bar preview), drag a module sideways to move it
/// in the group, like modules in the preview.
fn group_chips() -> Element<Appearance, Msg> {
    use heroui::fltk::enums::{Event, FrameType};
    use heroui::fltk::frame::Frame;
    use heroui::fltk::prelude::{WidgetBase, WidgetExt};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct St {
        /// The group, then its modules.
        names: Vec<String>,
        selected: Option<String>,
        hover: heroui::hover::HoverFade,
        /// A module being moved: (its index in `names`, press x, pointer x, moving).
        drag: Option<(usize, i32, i32, bool)>,
    }
    fn label(k: usize, n: &str) -> String {
        if k == 0 {
            format!("Group: {}", barconf::pretty(n))
        } else {
            barconf::pretty(n)
        }
    }
    /// (x, w) of each chip (font set); `skip` is left out, with room made
    /// for it before `gap`.
    fn spans(names: &[String], skip: Option<(usize, usize)>) -> Vec<(usize, i32, i32)> {
        let t = heroui::theme::current();
        draw::set_font(t.font(), t.font_size - 1);
        let width = |k: usize| draw::width(&label(k, &names[k])).ceil() as i32 + 24;
        let mut x = 0;
        let mut out = Vec::new();
        for k in 0..names.len() {
            if let Some((from, gap)) = skip {
                if k == gap {
                    x += width(from) + 6;
                }
                if k == from {
                    continue;
                }
            }
            out.push((k, x, width(k)));
            x += width(k) + 6;
        }
        out
    }
    /// Where module `from` (index in `names`) dragged to `px` goes: its
    /// position among the group's other modules.
    fn target(names: &[String], from: usize, px: i32) -> usize {
        spans(names, Some((from, usize::MAX))).iter().filter(|(k, x, w)| *k > 0 && x + w / 2 < px).count()
    }
    /// The chip (index in `names`) that room is made before, for module
    /// `from` going to position `to` among the others.
    fn gap_before(names: &[String], from: usize, to: usize) -> usize {
        (1..names.len()).filter(|&k| k != from).nth(to).unwrap_or(names.len())
    }
    Element::new(|ctx| {
        let st: Rc<RefCell<St>> = Rc::default();
        let mut f = Frame::default();
        f.set_frame(FrameType::NoBox);
        {
            let st = st.clone();
            f.draw(move |f| {
                let t = heroui::theme::current();
                let Ok(s) = st.try_borrow() else { return };
                let moving = s.drag.filter(|d| d.3);
                // The others make room where the moved one would land.
                let skip = moving.map(|(from, _, px, _)| (from, gap_before(&s.names, from, target(&s.names, from, px - f.x()))));
                let (cy, ch) = (f.y() + 4, f.h() - 8);
                let r = t.radius.min(ch / 2);
                for (k, x, w) in spans(&s.names, skip) {
                    let n = &s.names[k];
                    let on = s.selected.as_deref() == Some(n.as_str());
                    let base = if on { t.accent } else if k == 0 { t.surface } else { t.surface_alt };
                    let a = if on { 0.0 } else { s.hover.amount(k) };
                    draw::set_draw_color(heroui::widgets::mix(base, t.accent, 0.3 * a));
                    draw::draw_rounded_rectf(f.x() + x, cy, w, ch, r);
                    draw::set_draw_color(if on { t.accent_text } else { t.text });
                    draw::draw_text2(&label(k, n), f.x() + x, cy, w, ch, Align::Center);
                }
                if let Some((from, _, px, _)) = moving {
                    let w = spans(&s.names, None).iter().find(|(k, _, _)| *k == from).map_or(60, |c| c.2);
                    let x = (px - w / 2).clamp(f.x(), f.x() + f.w() - w);
                    draw::set_draw_color(t.accent);
                    draw::draw_rounded_rectf(x, cy - 2, w, ch, r);
                    draw::set_draw_color(t.accent_text);
                    draw::draw_text2(&label(from, &s.names[from]), x, cy - 2, w, ch, Align::Center);
                }
            });
        }
        let emit = ctx.emitter();
        {
            let st = st.clone();
            f.handle(move |f, ev| {
                let px = heroui::fltk::app::event_x() - f.x();
                let me = f.as_base_widget();
                let at = |names: &[String]| spans(names, None).into_iter().find(|&(_, x, w)| px >= x && px < x + w).map(|c| c.0);
                match ev {
                    Event::Enter | Event::Move => {
                        let mut s = st.borrow_mut();
                        let h = at(&s.names);
                        s.hover.set(h, &me);
                        true
                    }
                    Event::Leave => {
                        st.borrow_mut().hover.set(None, &me);
                        true
                    }
                    Event::Push => {
                        let mut s = st.borrow_mut();
                        let hit = at(&s.names);
                        s.drag = hit.map(|k| (k, px, heroui::fltk::app::event_x(), false));
                        true
                    }
                    Event::Drag => {
                        let mut s = st.borrow_mut();
                        if let Some(d) = s.drag.as_mut() {
                            d.2 = heroui::fltk::app::event_x();
                            // Modules move; the group stays first.
                            if !d.3 && d.0 > 0 && (px - d.1).abs() > 4 {
                                d.3 = true;
                            }
                            if d.3 {
                                drop(s);
                                heroui::widgets::repaint(&mut me.clone());
                            }
                        }
                        true
                    }
                    Event::Released => {
                        let (drag, names) = {
                            let mut s = st.borrow_mut();
                            (s.drag.take(), s.names.clone())
                        };
                        match drag {
                            Some((from, _, _, true)) => {
                                // Positions among the modules (the group is 0).
                                let to = target(&names, from, px);
                                if to != from - 1 {
                                    emit(Msg::MemberMoveTo(from - 1, to));
                                }
                            }
                            Some((k, _, _, false)) => emit(Msg::SelectName(names[k].clone())),
                            None => {}
                        }
                        heroui::widgets::repaint(&mut me.clone());
                        true
                    }
                    _ => false,
                }
            });
        }
        let mut w = f.clone();
        ctx.bind(move |a: &Appearance| {
            let (names, selected) = (group_of(a), a.selected.clone());
            let mut s = st.borrow_mut();
            if s.names != names || s.selected != selected {
                if s.names != names {
                    s.hover.clear();
                }
                s.names = names;
                s.selected = selected;
                drop(s);
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
            icon_button(move |a: &Appearance| sel_folder(a).map(|f| f.1).unwrap_or_default(), Msg::FolderIcon).fixed(150),
        ])
        .fixed(34)
        .visible(move |a: &Appearance| sel_folder(a).is_some()),
        caption("Folder icon: None shows small icons of its apps.").fixed(20).visible(move |a: &Appearance| sel_folder(a).is_some()),
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
        layout::editor().fixed(layout::HEIGHT),
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
    // Kinds with no text of their own.
    let not_taskbar = |k: &str| !matches!(k, "taskbar" | "spacer" | "group" | "workspaces" | "launcher");
    let not_custom = |k: &str| !matches!(k, "custom" | "taskbar" | "spacer" | "group" | "workspaces" | "launcher");
    let taskbar = |a: &Appearance| a.selected.as_deref().is_some_and(|n| barconf::kind_of(n) == "taskbar");
    fn is(kind: &'static str) -> impl Fn(&Appearance) -> bool + Copy {
        move |a: &Appearance| a.selected.as_deref().is_some_and(|n| barconf::kind_of(n) == kind)
    }
    let plain = |a: &Appearance| {
        a.selected
            .as_deref()
            .is_some_and(|n| !matches!(barconf::kind_of(n), "taskbar" | "spacer" | "group" | "workspaces"))
    };
    // The launcher has an icon and a label, but no format or interval.
    let has_icon = move |a: &Appearance| plain(a) || is("group")(a) || is("launcher")(a);
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
        edit_field("Text (beside the icon)", "text", |e| e.text.clone(), |k| k == "custom" || k == "launcher"),
        setting("Opens", dropdown(|_: &Appearance| LAUNCHER_MODES, |a: &Appearance| a.edit.launcher_mode, Msg::LauncherMode), 260)
            .visible(is("launcher")),
        caption("Needs HeroLauncher. For a shortcut, bind \"herolauncher\" in your compositor (opens centered).")
            .fixed(20)
            .visible(is("launcher")),
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
        edit_field("On click (command)", "on-click", |e| e.on_click.clone(), |k| !matches!(k, "taskbar" | "spacer" | "group" | "workspaces")),
        row(vec![
            label("Icon").fixed(150),
            // "" in the config = the kind's own icon, "none" = no icon.
            icon_button(
                |a: &Appearance| match a.edit.icon.as_str() {
                    "" => barconf::default_icon(barconf::kind_of(a.selected.as_deref().unwrap_or(""))).to_owned(),
                    "none" => String::new(),
                    i => i.to_owned(),
                },
                |n| Msg::Edit("icon", if n.is_empty() { "none".into() } else { n }),
            ),
            button("Default", Msg::Edit("icon", String::new())).fixed(90).enabled(|a: &Appearance| !a.edit.icon.is_empty()),
        ])
        .fixed(34)
        .visible(move |a: &Appearance| has_icon(a) && a.selected.is_some()),
        caption("Choose from the built-in icons and your icon theme's; Default is the module's own.")
            .fixed(20)
            .visible(move |a: &Appearance| has_icon(a) && a.selected.is_some()),
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
        toggle("Click shows time left and screen brightness", |a: &Appearance| a.edit.popup, Msg::Popup).fixed(30).visible(is("battery")),
        caption("With the popup, On click is what its Advanced/settings button runs.")
            .fixed(20)
            .visible(|a: &Appearance| a.edit.popup && a.selected.as_deref().is_some_and(|n| matches!(barconf::kind_of(n), "volume" | "network" | "bluetooth" | "battery"))),
        toggle("Click shows a calendar", |a: &Appearance| a.edit.popup, Msg::Popup).fixed(30).visible(is("clock")),
        toggle("Short speeds (1.7K instead of 1.7 KB/s: narrower)", |a: &Appearance| a.edit.short_units, Msg::ShortUnits)
            .fixed(30)
            .visible(is("network")),
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
        caption("Modules in this group (drag them in the strip above to reorder):").fixed(24).visible(is("group")),
        list(
            |a: &Appearance| if is("group")(a) { a.edit.members.len() } else { 0 },
            |i| {
                row(vec![
                    text(move |a: &Appearance| a.edit.members.get(i).map(|m| barconf::pretty(m)).unwrap_or_default()),
                    button("Edit", Msg::EditMember(i)).fixed(70),
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
        setting(
            "Open folders as",
            dropdown(|_: &Appearance| FOLDER_STYLES, |a: &Appearance| a.edit.folder_style, Msg::FolderStyle),
            260,
        )
        .visible(taskbar),
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
    heroui::simple_args(
        "heroappearance",
        env!("CARGO_PKG_VERSION"),
        "Appearance: the HeroUI theme, HeroBar, HeroLauncher and HeroWallpaper settings.",
    );
    let settings = Settings::new("Appearance")
        .size(820, 640)
        .class("heroappearance");
    heroui::run(Appearance::new(), settings).unwrap();
}
