# HexStickyNote

A modern desktop workspace for sticky notes, a calendar and time tracking, with a unique 3D carousel interface and a local AI assistant. The assistant runs on [Ollama](https://ollama.com) on your own machine, or you can let Claude manage your notes through MCP – your data stays on your computer.

![HexStickyNote Screenshot](docs/image.png)

## Features

- **3D Carousel Interface**: Navigate your notes in an intuitive 3D carousel with smooth animations.
- **Calendar and Time Tracking**: HexCalendar and HexTime built in, one tab away from your notes.
- **AI Assistant**: Ask about your notes, events and tracked time, or ask it to create events, write notes or start the timer. Every change waits for your approval.
- **Markdown Editor**: Full-featured CodeMirror 6 editor with syntax highlighting and live preview.
- **Claude Desktop Integration**: Built-in **MCP (Model Context Protocol)** server that allows Claude Desktop to read and manage your notes.
- **Portable Data**: Notes are stored as human-readable `.md` files with YAML metadata.

## Calendar and Time Tracking

The HUD has three views, switched from the tabs in the top bar:

- **Notes** – the sticky note carousel.
- **Calendar** – HexCalendar (month, week and day views, day notes and reminders). It uses the same
  database as the standalone HexCalendar.
- **Time** – HexTime time tracking. HexTime is a Python app; it is bundled as a sidecar binary that
  starts when the tab is first opened and stops with HexStickyNote. It uses the same database as the
  standalone HexTime.

## AI Assistant

Open the assistant from the chat button in the top bar. It can look up notes, calendar events and
time entries, and propose changes: creating or editing notes, creating events, and starting or
stopping the timer. Each change is shown as a card that you approve or decline; nothing is deleted.

The assistant runs on a local [Ollama](https://ollama.com) (`OLLAMA_HOST` is respected). In
**Settings** -> **AI Model**:

- **Model in use**: pick an installed model. The assistant needs a model with tool support, such as
  `qwen3:8b` or `llama3.1:8b`.
- **Add model from the Ollama library**: type a name from [ollama.com/library](https://ollama.com/library)
  or a Hugging Face GGUF repository (`hf.co/user/repo:Q4_K_M`) and click **Download**; the model is
  pulled into Ollama.

## Installation

### For Developers

#### Prerequisites
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/) (Stable)
- [Ollama](https://ollama.com) for the assistant

#### Linux
Install the Tauri system libraries:
```bash
# Arch
sudo pacman -S --needed base-devel webkit2gtk-4.1 libayatana-appindicator librsvg openssl

# Debian / Ubuntu
sudo apt install build-essential libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
  libssl-dev libxdo-dev
```

#### Setup
1. Clone the repository:
   ```bash
   git clone git@github.com:Carmantis/HexStickyNote.git
   cd HexStickyNote
   ```
2. Install dependencies:
   ```bash
   npm install
   ```
3. Run in development mode:
   ```bash
   npm run tauri dev
   ```

## Claude Desktop (MCP)

HexStickyNote includes an MCP server. You can connect it to Claude Desktop to let Claude manage your notes, calendar and time tracking:

1. Open **Settings** in HexStickyNote.
2. Scroll to **Claude Desktop**.
3. Click **Add to Claude Desktop**.
4. Restart Claude Desktop.

Claude can then use these tools (Claude Desktop asks before running them):

- **Notes**: `create_note`, `list_notes`, `read_note`, `update_note`, `delete_note`
- **Calendar**: `list_events`, `create_event`
- **Time tracking**: `list_time_entries`, `get_timer`, `start_timer`, `stop_timer`

The time tracking tools use the HexTime server that HexStickyNote runs; when HexStickyNote is closed,
the MCP server starts HexTime itself. The calendar tools need Node.js 22.13 or newer (`node:sqlite`).

**Tip**: When Claude creates or modifies notes, click the refresh button (🔄) in HexStickyNote to see the changes immediately.


## Build & Release

To create a production installer:
```bash
npm run tauri build
```

To include HexTime, build its sidecar first (needs [uv](https://docs.astral.sh/uv/) and a HexTime
checkout next to this repository, or set `HEXTIME_DIR`):
```bash
npm run build:full          # builds the HexTime sidecar, then the app
```
In development, `npm run build:hextime` once is enough; `npm run tauri dev` finds the sidecar in
`src-tauri/binaries/`.

Installers are written to `src-tauri/target/release/bundle/` (`.msi`/`.exe` on Windows,
`.deb`/`.rpm` on Linux).

### Arch Linux

`packaging/arch/PKGBUILD` turns the `.deb` build into a pacman package:
```bash
npm run build:full -- --bundles deb  # or without HexTime: npm run tauri build -- --bundles deb
cd packaging/arch
makepkg -si
```
After installing, open **Settings** -> **Claude Desktop** once and click **Add to Claude Desktop**
so Claude uses the installed MCP server path.

## License
MIT - HexStickyNote

---
