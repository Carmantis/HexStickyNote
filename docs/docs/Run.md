# Development and testing

## You need

- [Node.js](https://nodejs.org/) 22.13 or newer and [Rust](https://rustup.rs/) (stable).
- On Linux, Tauri's system libraries: on Arch `sudo pacman -S --needed base-devel webkit2gtk-4.1 libayatana-appindicator librsvg openssl`, on Debian and Ubuntu `sudo apt install build-essential libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libxdo-dev`.
- For the Time tab: [uv](https://docs.astral.sh/uv/) and a HexTime checkout next to this repository (`../HexTime`) or in `HEXTIME_DIR`.
- For the assistant: [Ollama](https://ollama.com) and a model with tool support.

## Running from the source

```bash
npm install
npm run build:hextime     # once, and after each HexTime change
npm run tauri dev
```

`npm run tauri dev` starts Vite and the app; the UI reloads on every change and the Rust side rebuilds on save. Without `build:hextime` everything works except the Time tab, which says HexTime is not bundled.

## Tests and checks

| Command | Checks |
| --- | --- |
| `cd src-tauri && cargo test --lib` | The backend: notes and folders, calendar queries, the assistant loop and its tools (with a fake Ollama), saving PDFs |
| `cd src-tauri && cargo test --lib -- --ignored` | Also the tests that run the real HexTime sidecar, each on its own port and temporary database, and one that asks a real Ollama model (`ASSISTANT_TEST_MODEL`, default `qwen3.8:27b`) |
| `npm run check` | svelte-check and TypeScript |
| `cd mcp-server && npm run build` | The MCP server compiles |

The tests never use your own data: notes are written to temporary directories, and HexTime runs from `HexTime::isolated()` with a database of its own.

## Testing without your own data

The app itself always uses the real directories. To try something out on test data, give it its own data directory:

```bash
XDG_DATA_HOME=/tmp/hsn-test npm run tauri dev
```

Notes, the calendar and HexTime's database then live under `/tmp/hsn-test`, so you can create, move and delete freely. Put a few `.md` files (and folders) in `/tmp/hsn-test/hexstickynote/cards/` first to have notes to look at. Settings stay in `~/.config/hexstickynote/`. Two cautions:

- Close the installed HexStickyNote first. The Time tab reuses any HexTime already answering on port 47613, and that one uses your real database.
- Do not send the assistant requests that change things unless the Time tab's HexTime is also the test one.

## The MCP server

The MCP server for Claude Desktop is a separate Node project in `mcp-server/`:

```bash
cd mcp-server
npm install
npm run build             # TypeScript to dist/, then one bundled file
cp bundle/hexstickynote-mcp.mjs ../src-tauri/resources/
```

The app installs the copy in `src-tauri/resources/`, so copy the bundle there after every change. The notes code in `mcp-server/src/cards.ts` mirrors `src-tauri/src/card_manager.rs`: the two must agree on the file format, file names and folders.

## HexTime

HexTime is developed in its own repository; see its [development guide](hextime/Run.md). After a change there, run `npm run build:hextime` here and restart the app. How the two fit together is in [Time tracking](Time-tracking.md).
