# HeroAppearance

**Appearance**, the customization app of [HeroineOS](https://github.com/HeroineOS), built
on [HeroUI](https://github.com/HeroineOS/HeroUI). It edits:

- **The HeroUI theme**, used by every HeroUI program: mode (System follows the desktop's
  dark/light setting, or Dark, or Light; switching keeps your accent), accent swatches,
  every color (tap a color for a picker; a hex field is there for pasting), corner radius, spacing, padding, font and font size, and animations (off =
  reduced motion, saves battery). Appearance and every open HeroUI program re-skin live.
- **[HeroBar](https://github.com/HeroineOS/HeroBar)**: position, height, reserved space,
  padding, spacing, colors and islands (sharp, rounded or pill), and the modules: a preview
  of the bar where you tap a module to edit it and drag it to reorder it or move it between
  the left/center/right sections; add and remove modules; edit each one (format, interval,
  text, command, on-click, icon), and the taskbar (pinned apps, per-app or per-window
  buttons, width).

Changes are saved ~300 ms after you stop editing, atomically. Open HeroUI programs re-skin
right away; HeroBar applies layout changes within a second. `bar.toml` keeps its
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
sudo apt install ./heroappearance_0.1.2-1_arm64.deb
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
