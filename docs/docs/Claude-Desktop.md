# Claude Desktop

HexStickyNote includes an [MCP](https://modelcontextprotocol.io) server, so Claude Desktop can read and change the same notes, calendar and time tracking as the [assistant](Assistant.md).

## Connecting

1. Install [Node.js](https://nodejs.org/) 22.13 or newer (the calendar tools use `node:sqlite`).
2. In HexStickyNote, open **Settings → Claude Desktop** and click **Add to Claude Desktop**.
3. Restart Claude Desktop.

The button writes a `hexstickynote` entry into Claude Desktop's `claude_desktop_config.json`: `node` with the MCP server that is installed with the app, and `HEXTIME_SERVER` pointing at the bundled HexTime. After installing a new version somewhere else, click the button again so the paths are right. **Remove from Claude Desktop** takes the entry out.

## Tools

| Area | Tools |
| --- | --- |
| Notes | `create_note` (optionally into a folder), `list_notes`, `read_note`, `update_note`, `delete_note` |
| Calendar | `list_events`, `create_event` |
| Time tracking | `list_time_entries`, `get_timer`, `start_timer`, `stop_timer` |

Claude Desktop asks before it runs a tool. Unlike the built-in assistant, Claude can delete notes.

## How it reaches the data

| Data | Through |
| --- | --- |
| Notes | The Markdown files directly, folders included |
| Calendar | The HexCalendar SQLite database |
| Time tracking | HexTime's API on port 47613: the HexTime that HexStickyNote runs, or, when HexStickyNote is closed, one the MCP server starts itself from `HEXTIME_SERVER` |

Notes changed by Claude show up in HexStickyNote after the refresh button in the top bar. If HexTime cannot start, the time tools answer with the same reasons as the Time tab ([Time tracking](Time-tracking.md#when-it-does-not-start)).

The server is in `mcp-server/`; `npm run build` there builds it, and the bundle is copied to `src-tauri/resources/hexstickynote-mcp.mjs` for the app ([Development and testing](Run.md#the-mcp-server)).
