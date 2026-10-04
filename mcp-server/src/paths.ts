import path from "path";
import os from "os";

// Must match the Rust side: directories::ProjectDirs::from("com", "HexStickyNote", "HexStickyNote")
// .data_dir().join("cards") in src-tauri/src/card_manager.rs
export function getCardsDirectory(): string {
  const platform = process.platform;

  if (platform === "win32") {
    const appData = process.env.APPDATA;
    if (!appData) {
      throw new Error("APPDATA environment variable is not set");
    }
    return path.join(appData, "HexStickyNote", "HexStickyNote", "data", "cards");
  } else if (platform === "darwin") {
    return path.join(
      os.homedir(),
      "Library",
      "Application Support",
      "com.HexStickyNote.HexStickyNote",
      "cards"
    );
  } else {
    const xdgDataHome = process.env.XDG_DATA_HOME;
    const dataHome =
      xdgDataHome && path.isAbsolute(xdgDataHome)
        ? xdgDataHome
        : path.join(os.homedir(), ".local", "share");
    return path.join(dataHome, "hexstickynote", "cards");
  }
}

// Must match the Rust side: directories::BaseDirs::data_dir().join("com.hexcalendar.app")
// in src-tauri/src/calendar/db/mod.rs (shared with the standalone HexCalendar)
export function getCalendarDatabasePath(): string {
  const platform = process.platform;

  let dataDir: string;
  if (platform === "win32") {
    const appData = process.env.APPDATA;
    if (!appData) {
      throw new Error("APPDATA environment variable is not set");
    }
    dataDir = appData;
  } else if (platform === "darwin") {
    dataDir = path.join(os.homedir(), "Library", "Application Support");
  } else {
    const xdgDataHome = process.env.XDG_DATA_HOME;
    dataDir =
      xdgDataHome && path.isAbsolute(xdgDataHome)
        ? xdgDataHome
        : path.join(os.homedir(), ".local", "share");
  }
  return path.join(dataDir, "com.hexcalendar.app", "hexcalendar.db");
}
