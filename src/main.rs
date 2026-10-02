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
    /// What's typed in each color field (applied when it's a valid color).
    color_text: Vec<String>,
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
}

#[derive(Clone)]
enum Msg {
    Page(Page),
    // Theme
    Mode(heroui::theme::Mode),
    Accent(u32),
    ColorText(usize, String),
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
    BarBg(String),
    BarFg(String),
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
            color_text: COLORS.iter().map(|(_, get, _)| hex(get(&theme))).collect(),
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
        for (i, key) in SECTIONS.iter().enumerate() {
            self.sections[i] = d.section(key);
        }
    }

    fn load_edit(&mut self) {
        let (Some(d), Some(name)) = (&self.bar, &self.selected) else {
            return;
        };
        self.edit = ModuleEdit {
            format: d.module_str(name, "format"),
            disconnected: d.module_str(name, "format-disconnected"),
            interval: d.module_str(name, "interval"),
            text: d.module_str(name, "text"),
            exec: d.module_str(name, "exec"),
            on_click: d.module_str(name, "on-click"),
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
        self.color_text[0] = hex(c);
        self.color_text[1] = hex(self.theme.accent_text);
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
                self.color_text = COLORS.iter().map(|(_, get, _)| hex(get(&self.theme))).collect();
                return self.touched(true);
            }
            Msg::Accent(c) => {
                self.set_accent(Color::from_hex(c));
                return self.touched(true);
            }
            Msg::ColorText(i, s) => {
                if let Some(c) = parse_hex(&s) {
                    if i == 0 {
                        self.set_accent(c);
                    } else {
                        (COLORS[i].2)(&mut self.theme, c);
                    }
                    self.color_text[i] = s;
                    return self.touched(true);
                }
                self.color_text[i] = s;
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
            // Not a color (yet): keep what's typed, don't write.
            Msg::BarBg(s) if !s.is_empty() && parse_hex(&s).is_none() => self.bar_bg = s,
            Msg::BarFg(s) if !s.is_empty() && parse_hex(&s).is_none() => self.bar_fg = s,
            Msg::BarBg(s) => {
                if let Some(d) = self.bar_mut() {
                    d.set_style("background", &s);
                }
                self.bar_bg = s;
                return self.touched(false);
            }
            Msg::BarFg(s) => {
                if let Some(d) = self.bar_mut() {
                    d.set_style("foreground", &s);
                }
                self.bar_fg = s;
                return self.touched(false);
            }
            Msg::MoveTo(fs, fi, ts, ti) if fi < self.sections[fs].len() => {
                let m = self.sections[fs].remove(fi);
                let ti = ti.min(self.sections[ts].len());
                self.sections[ts].insert(ti, m.clone());
                self.selected = Some(m);
                self.load_edit();
                return self.write_sections();
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
                let kind = KINDS[self.add_kind].0;
                let name = match (kind, &self.bar) {
                    ("custom", Some(d)) => d.new_custom_name(),
                    _ => kind.to_owned(),
                };
                if kind == "custom" {
                    if let Some(d) = self.bar_mut() {
                        d.set_module(&name, "text", "New");
                    }
                }
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
                    _ => self.edit.on_click = v.clone(),
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

fn swatch(c: impl Fn(&Appearance) -> u32 + 'static) -> Element<Appearance, Msg> {
    canvas(c, |rgb: &u32, x, y, w, h, t: &Theme| {
        let s = w.min(h) - 6;
        draw::set_draw_color(t.border);
        draw::draw_rounded_rectf(x + (w - s) / 2 - 1, y + (h - s) / 2 - 1, s + 2, s + 2, 6);
        draw::set_draw_color(Color::from_hex(*rgb));
        draw::draw_rounded_rectf(x + (w - s) / 2, y + (h - s) / 2, s, s, 5);
    })
}

fn color_u32(c: Color) -> u32 {
    let (r, g, b) = c.to_rgb();
    (r as u32) << 16 | (g as u32) << 8 | b as u32
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
                swatch(move |s: &Appearance| color_u32(get(&s.theme))).fixed(34),
                text_input(
                    move |s: &Appearance| s.color_text[i].clone(),
                    move |v| Msg::ColorText(i, v),
                )
                .fixed(110),
            ])
            .fixed(34),
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
    "Custom (text or command)",
];

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
        setting(
            "Background (#rrggbb, empty = theme)",
            text_input(|a: &Appearance| a.bar_bg.clone(), Msg::BarBg),
            120,
        ),
        setting(
            "Text color (#rrggbb, empty = theme)",
            text_input(|a: &Appearance| a.bar_fg.clone(), Msg::BarFg),
            120,
        ),
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
    let any = |_: &str| true;
    let not_custom = |k: &str| k != "custom";
    rows.extend([
        text(|a: &Appearance| {
            a.selected
                .as_deref()
                .map(|n| format!("Module: {}", barconf::pretty(n)))
                .unwrap_or_default()
        })
        .fixed(36)
        .visible(|a: &Appearance| a.selected.is_some()),
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
            any,
        ),
        edit_field(
            "On click (command)",
            "on-click",
            |e| e.on_click.clone(),
            any,
        ),
        text(
            |a: &Appearance| match a.selected.as_deref().map(barconf::kind_of) {
                Some("clock") => "Format: strftime, e.g. %a %d %b  %H:%M".into(),
                Some("cpu") => "Format placeholders: {usage}".into(),
                Some("memory") => "Format placeholders: {used} {total} {percent}".into(),
                Some("battery") => "Format placeholders: {capacity} {status}".into(),
                Some("network") => "Format placeholders: {ifname} {state}".into(),
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
