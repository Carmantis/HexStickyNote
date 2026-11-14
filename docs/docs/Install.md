# Building and installing

## Building

```bash
npm run build:full -- --bundles deb      # notes, calendar and HexTime
npm run tauri build -- --bundles deb     # without HexTime
```

`build:full` first builds the HexTime sidecar from `../HexTime` (or `HEXTIME_DIR`), then the app with `src-tauri/tauri.hextime.conf.json`, which adds the sidecar to the package. The result is in `src-tauri/target/release/bundle/`. Version numbers are in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.

The sidecar is built from whatever HexTime is checked out, uncommitted changes included, so check the HexTime branch before a release ([Time tracking](Time-tracking.md#keeping-it-up-to-date)).

## Arch Linux

`packaging/arch/PKGBUILD` turns the `.deb` into a pacman package:

```bash
npm run build:full -- --bundles deb
cd packaging/arch
makepkg -f
sudo pacman -U hex-sticky-note-*.pkg.tar.zst
```

`makepkg -si` builds and installs in one step. Installing over an older version keeps all data. Node.js (for Claude Desktop) and Ollama (for the assistant) are optional dependencies.

## Debian and Ubuntu

Install the `.deb` from `src-tauri/target/release/bundle/deb/` with `sudo apt install ./HexStickyNote_*.deb`. Build it on the oldest release you want to support: like HexTime's, the bundled sidecar is linked against the build machine's libraries.

## After installing

- **Claude Desktop:** open **Settings → Claude Desktop** and click **Add to Claude Desktop** once, so Claude uses the installed MCP server and HexTime ([Claude Desktop](Claude-Desktop.md)).
- **Hyprland:** on Wayland an app cannot place its own windows, so the orb opens wherever the compositor puts it. A window rule keeps it small, floating and in a corner. With Omarchy's Lua configuration, in `~/.config/hypr/hyprland.lua`:

```lua
o.window({ class = "^hex-sticky-note$", title = "^HexStickyNote Orb$" }, {
  tag = "-default-opacity",
  float = true,
  pin = true,
  size = { 76, 76 },
  move = { "(monitor_w-window_w-40)", "(monitor_h-window_h-40)" },
  border_size = 0,
  rounding = 0,
  no_shadow = true,
  no_blur = true,
  no_initial_focus = true,
  opacity = "1 1",
})
```

## Windows

Build on Windows itself; neither the app nor the sidecar cross-compiles. You need Rust (with the MSVC build tools), Node.js, [uv](https://docs.astral.sh/uv/) and [Git for Windows](https://gitforwindows.org/), whose Git Bash runs the sidecar's build script. In Git Bash:

```bash
npm install
npm run build:full        # .msi and setup .exe in src-tauri/target/release/bundle/
```

If `npm run build:hextime` picks up WSL's `bash` instead of Git Bash, run `bash sidecar/hextime/build.sh` in Git Bash and then `npm run tauri build -- --config src-tauri/tauri.hextime.conf.json`.

The installers include WebView2 and install for the current user. On Windows the data is under `%APPDATA%`: notes in `HexStickyNote\HexStickyNote\data\cards`, the calendar in `com.hexcalendar.app`. Claude Desktop installed from its MSIX package keeps its configuration under `%LOCALAPPDATA%\Packages\Claude_…`; **Add to Claude Desktop** finds it there.

## Not working

| Problem | Fix |
| --- | --- |
| The app starts but no window shows | Start `hex-sticky-note` from a terminal to see its log |
| Blank or black window on Linux | The app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` itself; if you set it to something else, unset it |
| The Time tab does not start | See [Time tracking](Time-tracking.md#when-it-does-not-start) |
| AppImage build fails on Arch | Use the `.deb` and the PKGBUILD instead |
