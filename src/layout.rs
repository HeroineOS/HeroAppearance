//! The bar layout editor: a preview of the bar's three sections with the
//! modules as chips. Tap a chip to select it (its settings show below);
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

#[derive(Default)]
struct State {
    /// (config name, label) per section.
    sections: [Vec<(String, String)>; 3],
    selected: Option<String>,
    drag: Option<Drag>,
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

/// Smallest width of a section (enough for its name when empty).
const MIN_ZONE: i32 = 70;

/// Natural chip widths per section (skipping the dragged chip).
fn chip_widths(st: &State, skip: Option<(usize, usize)>) -> [Vec<(usize, i32)>; 3] {
    let t = heroui::theme::current();
    draw::set_font(t.font(), t.font_size - 1);
    std::array::from_fn(|s| {
        st.sections[s]
            .iter()
            .enumerate()
            .filter(|(i, _)| skip != Some((s, *i)))
            .map(|(i, (_, label))| (i, draw::width(label).ceil() as i32 + 2 * CHIP_PAD))
            .collect()
    })
}

/// Section boundaries: each section gets its content's width (at least
/// MIN_ZONE) plus an equal share of what's left. When everything is too
/// wide, chips shrink by `scale`.
fn zones(widths: &[Vec<(usize, i32)>; 3], x: i32, w: i32) -> ([(i32, i32); 3], f64) {
    let content: Vec<i32> =
        widths.iter().map(|ws| ws.iter().map(|(_, w)| w + GAP).sum::<i32>() + GAP).collect();
    let need: Vec<i32> = content.iter().map(|&c| c.max(MIN_ZONE)).collect();
    let total: i32 = need.iter().sum();
    let (scale, extra) = if total > w {
        let chips: i32 = content.iter().sum();
        let room = (w - 3 * GAP * 2).max(1);
        ((room as f64 / chips.max(1) as f64).min(1.0), 0)
    } else {
        (1.0, (w - total) / 3)
    };
    let mut zx = x;
    let mut out = [(0, 0); 3];
    for s in 0..3 {
        let zw = if scale < 1.0 { (content[s] as f64 * scale) as i32 + GAP * 2 } else { need[s] + extra };
        out[s] = (zx, zw);
        zx += zw;
    }
    // The last section ends at the edge.
    out[2].1 = x + w - out[2].0;
    (out, scale)
}

/// Chip positions, with `skip` (the dragged chip) left out.
fn layout(st: &State, x: i32, w: i32, skip: Option<(usize, usize)>) -> Vec<Chip> {
    let widths = chip_widths(st, skip);
    let (zs, scale) = zones(&chip_widths(st, None), x, w);
    let mut out = Vec::new();
    for (s, ws) in widths.iter().enumerate() {
        let ws: Vec<(usize, i32)> = ws.iter().map(|&(i, cw)| (i, ((cw as f64 * scale) as i32).max(24))).collect();
        let total = ws.iter().map(|(_, w)| w).sum::<i32>() + GAP * (ws.len() as i32 - 1).max(0);
        let (zx, zw) = zs[s];
        // Left packs left, center centers, right packs right, like the bar.
        let mut cx = match s {
            0 => zx + GAP,
            1 => zx + (zw - total) / 2,
            _ => zx + zw - total - GAP,
        };
        for (i, cw) in ws {
            out.push(Chip { section: s, index: i, x: cx, w: cw });
            cx += cw + GAP;
        }
    }
    out
}

/// The section under `px`.
fn section_at(st: &State, x: i32, w: i32, px: i32) -> usize {
    let (zs, _) = zones(&chip_widths(st, None), x, w);
    zs.iter().position(|&(zx, zw)| px < zx + zw).unwrap_or(2)
}

/// Where a chip dropped at `px` goes: (section, index among the others).
fn drop_target(st: &State, x: i32, w: i32, px: i32, from: (usize, usize)) -> (usize, usize) {
    let s = section_at(st, x, w, px);
    let chips = layout(st, x, w, Some(from));
    let before = chips.iter().filter(|c| c.section == s && c.x + c.w / 2 < px).count();
    (s, before)
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
                let (x, y, w, h) = (f.x(), f.y(), f.w(), f.h());
                // The bar.
                draw::set_draw_color(t.border);
                draw::draw_rounded_rectf(x, y, w, h, t.radius.min(10));
                draw::set_draw_color(t.background);
                draw::draw_rounded_rectf(x + 1, y + 1, w - 2, h - 2, t.radius.min(10));
                // Section labels and dividers.
                let (zs, _) = zones(&chip_widths(&st, None), x, w);
                draw::set_font(t.font(), t.font_size - 3);
                for s in 0..3 {
                    let (zx, zw) = zs[s];
                    draw::set_draw_color(t.text_dim);
                    draw::draw_text2(SECTION_NAMES[s], zx, y + h - 16, zw, 14, Align::Center);
                    if s > 0 {
                        draw::set_draw_color(t.border);
                        draw::draw_line(zx, y + 8, zx, y + h - 8);
                    }
                }
                let dragging = st.drag.as_ref().filter(|d| d.active);
                let skip = dragging.map(|d| d.from);
                let (cy, ch) = (y + 8, h - 30);
                draw::set_font(t.font(), t.font_size - 1);
                for c in layout(&st, x, w, skip) {
                    let (name, label) = &st.sections[c.section][c.index];
                    let selected = st.selected.as_deref() == Some(name.as_str());
                    draw::set_draw_color(if selected { t.accent } else { t.surface_alt });
                    draw::draw_rounded_rectf(c.x, cy, c.w, ch, t.radius.min(ch / 2));
                    draw::set_draw_color(if selected { t.accent_text } else { t.text });
                    draw::push_clip(c.x, cy, c.w, ch);
                    draw::draw_text2(label, c.x, cy, c.w, ch, Align::Center);
                    draw::pop_clip();
                }
                // The dragged chip follows the pointer; a marker shows where
                // it will land.
                if let Some(d) = dragging {
                    let (ts, ti) = drop_target(&st, x, w, d.x, d.from);
                    let others = layout(&st, x, w, Some(d.from));
                    let in_zone: Vec<&Chip> = others.iter().filter(|c| c.section == ts).collect();
                    let mx = match in_zone.get(ti) {
                        Some(c) => c.x - GAP / 2 - 1,
                        None => in_zone.last().map(|c| c.x + c.w + GAP / 2 - 1).unwrap_or_else(|| {
                            let (zx, zw) = zs[ts];
                            zx + zw / 2
                        }),
                    };
                    draw::set_draw_color(t.accent);
                    draw::draw_rectf(mx, cy - 2, 3, ch + 4);
                    let label = &st.sections[d.from.0][d.from.1].1;
                    let cw = draw::width(label).ceil() as i32 + 2 * CHIP_PAD;
                    let dx = (d.x - cw / 2).clamp(x, x + w - cw);
                    let _ = d.y;
                    draw::set_draw_color(t.accent);
                    draw::draw_rounded_rectf(dx, cy - 3, cw, ch, t.radius.min(ch / 2));
                    draw::set_draw_color(t.accent_text);
                    draw::draw_text2(label, dx, cy - 3, cw, ch, Align::Center);
                }
            });
        }
        let emit = ctx.emitter();
        {
            let st = st.clone();
            f.handle(move |f, ev| {
                let (px, py) = (heroui::fltk::app::event_x(), heroui::fltk::app::event_y());
                match ev {
                    Event::Push => {
                        let mut s = st.borrow_mut();
                        let hit = layout(&s, f.x(), f.w(), None)
                            .into_iter()
                            .find(|c| px >= c.x && px < c.x + c.w && py >= f.y() + 4 && py < f.y() + f.h() - 20);
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
                            let target = d.as_ref().filter(|d| d.active).map(|d| drop_target(&s, f.x(), f.w(), d.x, d.from));
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
            let sections: [Vec<(String, String)>; 3] = std::array::from_fn(|s| {
                a.sections[s].iter().map(|n| (n.clone(), barconf::pretty(n))).collect()
            });
            let mut s = st.borrow_mut();
            if s.sections != sections || s.selected != a.selected {
                s.sections = sections;
                s.selected = a.selected.clone();
                w.redraw();
            }
        });
        f.as_base_widget()
    })
}
