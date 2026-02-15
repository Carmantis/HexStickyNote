import path from "path";
import os from "os";

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
      "data",
      "cards"
    );
  } else {
    const dataHome =
      process.env.XDG_DATA_HOME || path.join(os.homedir(), ".local", "share");
    return path.join(dataHome, "HexStickyNote", "HexStickyNote", "data", "cards");
  }
}
