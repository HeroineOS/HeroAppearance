//! The bar layout editor: the bar's three sections, each a row of its
//! own, with the modules as chips. Tap a chip to select it (its settings show below);
//! drag it to reorder it or move it to another section. One widget with
//! one event handler; horizontal drags move chips, vertical drags still
//! scroll the page.

use std::cell::RefCell;
use std::rc::Rc;

use heroui::fltk::draw;
use heroui::fltk::enums::{Align, Event, FrameType};
use heroui::fltk::frame::Frame;
use heroui::fltk::prelude::*;
use heroui::prelude::*;

use crate::{barconf, Appearance, Msg};

pub const SECTION_NAMES: [&str; 3] = ["Left", "Center", "Right"];

/// Movement before a press becomes a drag.
const DRAG_START: i32 = 4;
const CHIP_PAD: i32 = 12;
const GAP: i32 = 6;

type Chip3 = (String, String, String);

/// Icon size in chips.
const ICON: i32 = 16;
const ICON_GAP: i32 = 5;

/// Padding of icon-only chips.
const COMPACT_PAD: i32 = 9;

/// A chip's width for its label and icon; `compact` chips with an icon
/// show only the icon.
fn chip_width(label: &str, icon: &str, compact: bool) -> i32 {
    if compact && !icon.is_empty() {
        return ICON + 2 * COMPACT_PAD;
    }
    let icon = if icon.is_empty() { 0 } else { ICON + ICON_GAP };
    draw::width(label).ceil() as i32 + icon + 2 * CHIP_PAD
}

/// Draws a chip's icon and label centered in (x, y, w, h).
fn chip_content(label: &str, icon: &str, compact: bool, (x, y, w, h): (i32, i32, i32, i32), color: heroui::fltk::enums::Color) {
    if compact && !icon.is_empty() {
        heroui::icons::draw(icon, x + (w - ICON) / 2, y + (h - ICON) / 2, ICON, color);
        return;
    }
    let content = chip_width(label, icon, false) - 2 * CHIP_PAD;
    let mut cx = x + ((w - content) / 2).max(CHIP_PAD.min(w / 4));
    draw::push_clip(x, y, w, h);
    if !icon.is_empty() {
        heroui::icons::draw(icon, cx, y + (h - ICON) / 2, ICON, color);
        cx += ICON + ICON_GAP;
    }
    draw::set_draw_color(color);
    draw::draw_text2(label, cx, y, x + w - cx, h, Align::Left | Align::Inside);
    draw::pop_clip();
}

#[derive(Default)]
struct State {
    /// (config name, label, icon) per section.
    sections: [Vec<Chip3>; 3],
    selected: Option<String>,
    drag: Option<Drag>,
    /// Icon-only chips in a section, when full ones don't fit (see `fit`).
    compact: std::cell::Cell<[bool; 3]>,
    /// The chip under the pointer (section * 1000 + index), fading.
    hover: heroui::hover::HoverFade,
    /// Chips glide to new spots after a move; new ones pop in, removed
    /// ones fade (keyed by module name; ghosts keep label and icon).
    glide: heroui::glide::Glides<(String, String, bool)>,
}

/// A chip's hover key.
fn key(section: usize, index: usize) -> usize {
    section * 1000 + index
}

/// Each section is a row of its own: its box's height, the gap between
/// boxes, the room for its name, and the chips' height.
const ROW_H: i32 = 46;
const ROW_GAP: i32 = 8;
const NAME_W: i32 = 64;
const CHIP_H: i32 = 32;

/// The editor's height.
pub const HEIGHT: i32 = 3 * ROW_H + 2 * ROW_GAP;

/// Section `s`'s box.
fn row_rect(s: usize, (x, y, w): (i32, i32, i32)) -> (i32, i32, i32, i32) {
    (x, y + s as i32 * (ROW_H + ROW_GAP), w, ROW_H)
}

/// Where a section's chips go (x, width).
fn chip_area(s: usize, f: (i32, i32, i32)) -> (i32, i32) {
    let (bx, _, bw, _) = row_rect(s, f);
    (bx + NAME_W, bw - NAME_W - GAP)
}

impl State {
    /// Picks full or icon-only chips for each section's row.
    fn fit(&self, f: (i32, i32, i32)) {
        self.compact.set([false; 3]);
        let widths = chip_widths(self, None);
        self.compact.set(std::array::from_fn(|s| {
            let need: i32 = widths[s].iter().map(|(_, cw)| cw + GAP).sum::<i32>();
            need > chip_area(s, f).1
        }));
    }
}

struct Drag {
    from: (usize, usize),
    /// Pointer x at the press, and now.
    start_x: i32,
    x: i32,
    y: i32,
    active: bool,
}

#[derive(Clone, Copy)]
struct Chip {
    section: usize,
    index: usize,
    x: i32,
    w: i32,
}

/// Natural chip widths per section (skipping the dragged chip).
fn chip_widths(st: &State, skip: Option<(usize, usize)>) -> [Vec<(usize, i32)>; 3] {
    let t = heroui::theme::current();
    draw::set_font(t.font(), t.font_size - 1);
    let compact = st.compact.get();
    std::array::from_fn(|s| {
        st.sections[s]
            .iter()
            .enumerate()
            .filter(|(i, _)| skip != Some((s, *i)))
            .map(|(i, (_, label, icon))| (i, chip_width(label, icon, compact[s])))
            .collect()
    })
}

/// Chip positions, with `skip` (the dragged chip) left out. Chips shrink
/// when a row is still too full with icons only.
fn layout(st: &State, f: (i32, i32, i32), skip: Option<(usize, usize)>) -> Vec<Chip> {
    let widths = chip_widths(st, skip);
    let mut out = Vec::new();
    for (s, ws) in widths.iter().enumerate() {
        let (ax, aw) = chip_area(s, f);
        let natural = ws.iter().map(|(_, w)| w).sum::<i32>() + GAP * (ws.len() as i32 - 1).max(0);
        let scale = if natural > aw { aw as f64 / natural.max(1) as f64 } else { 1.0 };
        let ws: Vec<(usize, i32)> = ws.iter().map(|&(i, cw)| (i, ((cw as f64 * scale) as i32).max(24))).collect();
        let total = ws.iter().map(|(_, w)| w).sum::<i32>() + GAP * (ws.len() as i32 - 1).max(0);
        // Left packs left, center centers, right packs right, like the bar.
        let mut cx = match s {
            0 => ax,
            1 => ax + (aw - total) / 2,
            _ => ax + aw - total,
        };
        for (i, cw) in ws {
            out.push(Chip { section: s, index: i, x: cx, w: cw });
            cx += cw + GAP;
        }
    }
    out
}

/// The chip's top.
fn chip_y(section: usize, f: (i32, i32, i32)) -> i32 {
    let (_, by, _, bh) = row_rect(section, f);
    by + (bh - CHIP_H) / 2
}

/// The section whose row is nearest `py`.
fn section_at(f: (i32, i32, i32), py: i32) -> usize {
    (0..3).min_by_key(|&s| {
        let (_, by, _, bh) = row_rect(s, f);
        (py - (by + bh / 2)).abs()
    }).unwrap_or(0)
}

/// Where a chip dropped at (px, py) goes: (section, index among the others).
fn drop_target(st: &State, f: (i32, i32, i32), (px, py): (i32, i32), from: (usize, usize)) -> (usize, usize) {
    let s = section_at(f, py);
    let chips = layout(st, f, Some(from));
    let before = chips.iter().filter(|c| c.section == s && c.x + c.w / 2 < px).count();
    (s, before)
}

/// The chip at (px, py).
fn chip_at(st: &State, f: (i32, i32, i32), (px, py): (i32, i32)) -> Option<Chip> {
    layout(st, f, None).into_iter().find(|c| {
        let cy = chip_y(c.section, f);
        px >= c.x && px < c.x + c.w && py >= cy - 4 && py < cy + CHIP_H + 4
    })
}

/// A chip's label: short ones for spacers and workspaces.
fn chip_label(name: &str) -> String {
    match barconf::kind_of(name) {
        "spacer" => "Space".into(),
        "workspaces" => "1 2 3".into(),
        _ => barconf::pretty(name),
    }
}

/// The icon a module shows: its own `icon`, else its kind's.
fn chip_icon(a: &Appearance, name: &str) -> String {
    a.bar
        .as_ref()
        .and_then(|d| d.module_icon(name))
        .unwrap_or_else(|| barconf::default_icon(barconf::kind_of(name)).to_owned())
}

pub fn editor() -> Element<Appearance, Msg> {
    Element::new(|ctx| {
        let st: Rc<RefCell<State>> = Rc::default();
        let mut f = Frame::default();
        f.set_frame(FrameType::NoBox);
        {
            let st = st.clone();
            f.draw(move |f| {
                let t = heroui::theme::current();
                let st = st.borrow();
                let fr = (f.x(), f.y(), f.w());
                st.fit(fr);
                let compact = st.compact.get();
                let r = t.radius.min(10);
                // A box per section, its name at the left.
                draw::set_font(t.font(), t.font_size - 2);
                for (s, name) in SECTION_NAMES.iter().enumerate() {
                    let (bx, by, bw, bh) = row_rect(s, fr);
                    draw::set_draw_color(t.border);
                    draw::draw_rounded_rectf(bx, by, bw, bh, r);
                    draw::set_draw_color(t.background);
                    draw::draw_rounded_rectf(bx + 1, by + 1, bw - 2, bh - 2, (r - 1).max(0));
                    draw::set_draw_color(t.text_dim);
                    draw::draw_text2(name, bx + 12, by, NAME_W - 12, bh, Align::Left | Align::Inside);
                    if st.sections[s].is_empty() && st.drag.is_none() {
                        let (ax, aw) = chip_area(s, fr);
                        draw::draw_text2("Empty", ax, by, aw, bh, Align::Center);
                    }
                }
                let dragging = st.drag.as_ref().filter(|d| d.active);
                let skip = dragging.map(|d| d.from);
                draw::set_font(t.font(), t.font_size - 1);
                let paint_chip = |(label, icon, selected): &(String, String, bool), a: f32, (cx, cy, cw): (i32, i32, i32), compact: bool| {
                    draw::set_draw_color(if *selected { heroui::widgets::mix(t.accent, t.text, 0.15 * a) } else { heroui::widgets::mix(t.surface_alt, t.accent, 0.3 * a) });
                    draw::draw_rounded_rectf(cx, cy, cw, CHIP_H, t.radius.min(CHIP_H / 2));
                    chip_content(label, icon, compact, (cx, cy, cw, CHIP_H), if *selected { t.accent_text } else { t.text });
                };
                st.glide.begin(&f.as_base_widget());
                for c in layout(&st, fr, skip) {
                    let (name, label, icon) = &st.sections[c.section][c.index];
                    let item = (label.clone(), icon.clone(), st.selected.as_deref() == Some(name.as_str()));
                    let a = if dragging.is_some() { 0.0 } else { st.hover.amount(key(c.section, c.index)) };
                    let cy = chip_y(c.section, fr);
                    let ((gx, gy, gw), pop) = st.glide.place(name, (c.x - fr.0, cy - fr.1, c.w), &item);
                    let (gx, gy) = (fr.0 + gx, fr.1 + gy);
                    let small = compact[c.section];
                    heroui::fx::draw_scaled((gx, gy, gw, CHIP_H), 0.6 + 0.4 * pop, pop.clamp(0.0, 1.0), || paint_chip(&item, a, (gx, gy, gw), small));
                }
                if let Some(d) = dragging {
                    st.glide.keep(&st.sections[d.from.0][d.from.1].0);
                }
                st.glide.end(|item, (gx, gy, gw), a| {
                    let (gx, gy) = (fr.0 + gx, fr.1 + gy);
                    heroui::fx::draw_scaled((gx, gy, gw, CHIP_H), 0.5 + 0.5 * a, a, || paint_chip(item, 0.0, (gx, gy, gw), false));
                });
                // The dragged chip follows the pointer; a marker shows where
                // it will land.
                if let Some(d) = dragging {
                    let (ts, ti) = drop_target(&st, fr, (d.x, d.y), d.from);
                    let others = layout(&st, fr, Some(d.from));
                    let in_row: Vec<&Chip> = others.iter().filter(|c| c.section == ts).collect();
                    let mx = match in_row.get(ti) {
                        Some(c) => c.x - GAP / 2 - 1,
                        None => in_row.last().map(|c| c.x + c.w + GAP / 2 - 1).unwrap_or_else(|| {
                            let (ax, aw) = chip_area(ts, fr);
                            match ts {
                                0 => ax,
                                1 => ax + aw / 2,
                                _ => ax + aw - 3,
                            }
                        }),
                    };
                    let ty = chip_y(ts, fr);
                    draw::set_draw_color(t.accent);
                    draw::draw_rectf(mx, ty - 2, 3, CHIP_H + 4);
                    let (_, label, icon) = &st.sections[d.from.0][d.from.1];
                    let small = compact[d.from.0];
                    let cw = chip_width(label, icon, small);
                    let dx = (d.x - cw / 2).clamp(fr.0, fr.0 + fr.2 - cw);
                    let dy = (d.y - CHIP_H / 2).clamp(fr.1, fr.1 + HEIGHT - CHIP_H);
                    draw::set_draw_color(t.accent);
                    draw::draw_rounded_rectf(dx, dy, cw, CHIP_H, t.radius.min(CHIP_H / 2));
                    chip_content(label, icon, small, (dx, dy, cw, CHIP_H), t.accent_text);
                }
            });
        }
        let emit = ctx.emitter();
        {
            let st = st.clone();
            f.handle(move |f, ev| {
                let (px, py) = (heroui::fltk::app::event_x(), heroui::fltk::app::event_y());
                let fr = (f.x(), f.y(), f.w());
                match ev {
                    Event::Enter | Event::Move => {
                        let mut s = st.borrow_mut();
                        let h = chip_at(&s, fr, (px, py)).map(|c| key(c.section, c.index));
                        s.hover.set(h, &f.as_base_widget());
                        true
                    }
                    Event::Leave => {
                        st.borrow_mut().hover.set(None, &f.as_base_widget());
                        true
                    }
                    Event::Push => {
                        let mut s = st.borrow_mut();
                        s.fit(fr);
                        let hit = chip_at(&s, fr, (px, py));
                        s.drag = hit.map(|c| Drag { from: (c.section, c.index), start_x: px, x: px, y: py, active: false });
                        true
                    }
                    Event::Drag => {
                        let mut s = st.borrow_mut();
                        if let Some(d) = s.drag.as_mut() {
                            d.x = px;
                            d.y = py;
                            if !d.active && (px - d.start_x).abs() > DRAG_START {
                                d.active = true;
                            }
                            if d.active {
                                f.redraw();
                            }
                        }
                        true
                    }
                    Event::Released => {
                        let (drag, target) = {
                            let mut s = st.borrow_mut();
                            let d = s.drag.take();
                            let target = d.as_ref().filter(|d| d.active).map(|d| drop_target(&s, fr, (d.x, d.y), d.from));
                            (d, target)
                        };
                        if let Some(d) = drag {
                            match target {
                                Some((ts, ti)) => emit(Msg::MoveTo(d.from.0, d.from.1, ts, ti)),
                                None => emit(Msg::Select(d.from.0, d.from.1)),
                            }
                        }
                        f.redraw();
                        true
                    }
                    _ => false,
                }
            });
        }
        let mut w = f.clone();
        ctx.bind(move |a: &Appearance| {
            let sections: [Vec<Chip3>; 3] = std::array::from_fn(|s| {
                a.sections[s].iter().map(|n| (n.clone(), chip_label(n), chip_icon(a, n))).collect()
            });
            let mut s = st.borrow_mut();
            if s.sections != sections || s.selected != a.selected {
                if s.sections != sections {
                    s.hover.clear();
                }
                s.sections = sections;
                s.selected = a.selected.clone();
                w.redraw();
            }
        });
        f.as_base_widget()
    })
}
