//! Ready-made color themes: pick one, then tweak any single color. A
//! preset sets the colors (and dark or light); corners, spacing, fonts
//! and the rest stay as they are.

use heroui::fltk::enums::Color;
use heroui::theme::{Mode, Theme};

pub struct Preset {
    pub name: &'static str,
    pub dark: bool,
    pub background: u32,
    pub surface: u32,
    pub surface_alt: u32,
    pub text: u32,
    pub text_dim: u32,
    pub accent: u32,
    pub border: u32,
}

const fn p(name: &'static str, dark: bool, [background, surface, surface_alt, text, text_dim, accent, border]: [u32; 7]) -> Preset {
    Preset { name, dark, background, surface, surface_alt, text, text_dim, accent, border }
}

/// background, surface, surface_alt, text, text_dim, accent, border
pub const PRESETS: &[Preset] = &[
    p("Heroine Dark", true, [0x14141c, 0x1f1f2b, 0x2a2a3a, 0xe6e6f0, 0x9090a8, 0xb46cff, 0x33334a]),
    p("Heroine Light", false, [0xf4f4f8, 0xffffff, 0xe8e8f0, 0x1a1a24, 0x6a6a80, 0x8a3ffc, 0xd4d4e0]),
    p("Catppuccin Mocha", true, [0x1e1e2e, 0x313244, 0x45475a, 0xcdd6f4, 0xa6adc8, 0xcba6f7, 0x45475a]),
    p("Catppuccin Latte", false, [0xeff1f5, 0xe6e9ef, 0xccd0da, 0x4c4f69, 0x6c6f85, 0x8839ef, 0xbcc0cc]),
    p("Nord", true, [0x2e3440, 0x3b4252, 0x434c5e, 0xeceff4, 0xd8dee9, 0x88c0d0, 0x4c566a]),
    p("Gruvbox", true, [0x282828, 0x3c3836, 0x504945, 0xebdbb2, 0xa89984, 0xfe8019, 0x665c54]),
    p("Dracula", true, [0x282a36, 0x343746, 0x44475a, 0xf8f8f2, 0xa4a8c4, 0xff79c6, 0x4d5066]),
    p("Tokyo Night", true, [0x1a1b26, 0x24283b, 0x2f3549, 0xc0caf5, 0x9aa5ce, 0x7aa2f7, 0x3b4261]),
    p("Rosé Pine", true, [0x191724, 0x1f1d2e, 0x26233a, 0xe0def4, 0x908caa, 0xebbcba, 0x403d52]),
    p("Everforest", true, [0x2d353b, 0x343f44, 0x3d484d, 0xd3c6aa, 0x9da9a0, 0xa7c080, 0x475258]),
    p("Solarized Light", false, [0xfdf6e3, 0xeee8d5, 0xe4ddc8, 0x586e75, 0x839496, 0x268bd2, 0xd6cfb8]),
    p("Midnight", true, [0x0a0a0a, 0x141414, 0x1c1c1c, 0xf0f0f0, 0x8a8a8a, 0x4f9dff, 0x2a2a2a]),
];

impl Preset {
    /// `theme` with this preset's colors.
    pub fn apply(&self, theme: &Theme) -> Theme {
        let c = Color::from_hex;
        Theme {
            mode: if self.dark { Mode::Dark } else { Mode::Light },
            background: c(self.background),
            surface: c(self.surface),
            surface_alt: c(self.surface_alt),
            text: c(self.text),
            text_dim: c(self.text_dim),
            accent: c(self.accent),
            accent_text: heroui::theme::contrast_text(c(self.accent)),
            border: c(self.border),
            ..theme.clone()
        }
    }

    /// Whether `theme` has exactly these colors (it's highlighted then).
    pub fn matches(&self, theme: &Theme) -> bool {
        let t = self.apply(theme);
        (t.background, t.surface, t.surface_alt, t.text, t.text_dim, t.accent, t.border)
            == (theme.background, theme.surface, theme.surface_alt, theme.text, theme.text_dim, theme.accent, theme.border)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_themes_are_presets() {
        assert!(PRESETS[0].matches(&Theme::dark()));
        assert!(PRESETS[1].matches(&Theme::light()));
        let t = PRESETS[4].apply(&Theme { radius: 3, ..Theme::light() });
        assert_eq!((t.mode, t.radius), (Mode::Dark, 3));
        assert!(PRESETS[4].matches(&t) && !PRESETS[0].matches(&t));
    }
}
