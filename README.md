# HeroAppearance

**Appearance**, the customization app of [HeroineOS](https://github.com/HeroineOS), built
on [HeroUI](https://github.com/HeroineOS/HeroUI). It edits:

- **The HeroUI theme**, used by every HeroUI program: colors (with a live preview, Dark/Light
  presets and accent swatches), corner radius, spacing, padding, font and font size, and
  animations (off = reduced motion, saves battery).
- **[HeroBar](https://github.com/HeroineOS/HeroBar)**: position, height, reserved space,
  padding, spacing and colors, and the modules: add, remove, reorder, move between the
  left/center/right sections, and edit each one (format, interval, text, command, on-click).

Changes are saved ~300 ms after you stop editing, atomically. HeroBar applies them within
a second; other HeroUI programs use the new theme on their next start. `bar.toml` keeps its
comments, so GUI and hand edits mix. It works on other distros too, e.g. to rice HeroBar on
sway or Hyprland. System settings (network, displays, power...) belong to HeroSettings.

Measured: 3 MB of its own memory, 0% CPU when idle, 1.7 MB binary.

## Files

| What | Where |
|---|---|
| HeroUI theme | `~/.config/heroui/theme.conf` |
| HeroBar | `~/.config/hero/bar.toml` (created from HeroBar's default on the first edit) |

## Install

Debian packages for amd64 and arm64 are on the
[releases](https://github.com/HeroineOS/HeroAppearance/releases) page:

```sh
sudo apt install ./heroappearance_0.1.0-1_arm64.deb
```

It shows up as **Appearance** in application menus (`heroappearance` on the command line).

## Building

```sh
cargo build --release
cargo deb          # cargo install cargo-deb
```

Same build dependencies as HeroUI. Cargo.toml patches `fltk-sys` with
[HeroineOS/fltk-sys](https://github.com/HeroineOS/fltk-sys) (Wayland touch input).
Targets: x86_64, aarch64, i686 and armv7 Linux (CI builds x86_64 and aarch64).

## License

MIT OR Apache-2.0.
