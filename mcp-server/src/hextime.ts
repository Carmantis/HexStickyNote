import { spawn, type ChildProcess } from "child_process";
import net from "net";

// HexTime time tracking through its HTTP API. HexStickyNote runs HexTime as a
// sidecar on port 47613; if it is not running, this server starts the same
// sidecar binary itself (path from HEXTIME_SERVER, written into the Claude
// Desktop config by HexStickyNote) and keeps it for as long as it runs.

const PREFERRED_PORT = 47613;
const STARTUP_TIMEOUT_MS = 30_000;

let baseUrl: string | null = null;
let child: ChildProcess | null = null;
/** The startup in progress; parallel tool calls wait for the same one */
let starting: Promise<string> | null = null;

export interface EntrySummary {
  description: string | null;
  project: string | null;
  date: string | null;
  start: string | null;
  end: string | null;
  running: boolean;
  duration_minutes: number | null;
  billable: boolean;
}

async function isHexTime(url: string): Promise<boolean> {
  try {
    const response = await fetch(`${url}/api/v1/config`, { signal: AbortSignal.timeout(1000) });
    return response.ok && "mode" in ((await response.json()) as object);
  } catch {
    return false;
  }
}

function portIsFree(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const probe = net.createServer();
    probe.once("error", () => resolve(false));
    probe.listen(port, "127.0.0.1", () => probe.close(() => resolve(true)));
  });
}

function anyFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const probe = net.createServer();
    probe.once("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const port = (probe.address() as net.AddressInfo).port;
      probe.close(() => resolve(port));
    });
  });
}

/** URL of a running HexTime, starting the sidecar if needed */
async function ensureServer(): Promise<string> {
  if (baseUrl && (await isHexTime(baseUrl))) return baseUrl;
  starting ??= findOrStartServer().finally(() => {
    starting = null;
  });
  return starting;
}

async function findOrStartServer(): Promise<string> {
  const preferred = `http://127.0.0.1:${PREFERRED_PORT}`;
  if (await isHexTime(preferred)) {
    baseUrl = preferred;
    return baseUrl;
  }

  const binary = process.env.HEXTIME_SERVER;
  if (!binary) {
    throw new Error(
      "Time tracking is not available. Open HexStickyNote, or click 'Add to Claude Desktop' in its Settings again so Claude can start HexTime itself."
    );
  }

  const port = (await portIsFree(PREFERRED_PORT)) ? PREFERRED_PORT : await anyFreePort();
  const url = `http://127.0.0.1:${port}`;

  console.error(`[hextime] Starting ${binary} on port ${port}`);
  // stdin stays open while this process runs; HexTime exits when it closes.
  // stdout is ignored: it would corrupt the MCP protocol on our stdout.
  child = spawn(binary, ["--port", String(port), "--watch-stdin"], {
    stdio: ["pipe", "ignore", "inherit"],
    windowsHide: true,
  });
  const started = child;
  let exited = false;
  started.once("exit", () => {
    exited = true;
    if (child === started) child = null;
  });

  const deadline = Date.now() + STARTUP_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (exited) throw new Error("HexTime stopped during startup");
    if (await isHexTime(url)) {
      baseUrl = url;
      return url;
    }
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  started.kill();
  throw new Error("HexTime did not start in time");
}

async function api(method: string, path: string, query?: Record<string, string>, body?: unknown): Promise<any> {
  const url = new URL(`${await ensureServer()}/api/v1${path}`);
  for (const [key, value] of Object.entries(query ?? {})) url.searchParams.set(key, value);

  const response = await fetch(url, {
    method,
    headers: body === undefined ? undefined : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  const data = text ? JSON.parse(text) : null;
  if (!response.ok) {
    // HexTime errors look like {"error": {"code": ..., "message": ...}}
    throw new Error(data?.error?.message ?? `HexTime returned ${response.status}`);
  }
  return data;
}

async function projects(): Promise<Map<string, string>> {
  const list = (await api("GET", "/projects")) as Array<{ id: string; name: string }>;
  return new Map(list.map((p) => [p.id, p.name]));
}

const pad = (n: number) => String(n).padStart(2, "0");

function summarize(entry: any, projectNames: Map<string, string>): EntrySummary {
  const started = entry.started_at ? new Date(entry.started_at) : null;
  const ended = entry.ended_at ? new Date(entry.ended_at) : null;
  return {
    description: entry.description ?? null,
    project: entry.project_id ? projectNames.get(entry.project_id) ?? null : null,
    date: started ? `${started.getFullYear()}-${pad(started.getMonth() + 1)}-${pad(started.getDate())}` : null,
    start: started ? `${pad(started.getHours())}:${pad(started.getMinutes())}` : null,
    end: ended ? `${pad(ended.getHours())}:${pad(ended.getMinutes())}` : null,
    running: !entry.ended_at,
    duration_minutes: typeof entry.duration_seconds === "number" ? Math.floor(entry.duration_seconds / 60) : null,
    billable: Boolean(entry.billable),
  };
}

function localDayStart(date: string, dayOffset = 0): Date {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date.trim());
  if (!match) throw new Error(`'${date}' is not a YYYY-MM-DD date`);
  return new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]) + dayOffset);
}

/** Time entries started on the days from `fromDate` to `toDate` (inclusive) */
export async function listTimeEntries(fromDate: string, toDate?: string) {
  const from = localDayStart(fromDate);
  const to = localDayStart(toDate?.trim() ? toDate : fromDate, 1);
  if (to <= from) throw new Error("to_date is before from_date");

  const entries = (await api("GET", "/entries", { from: from.toISOString(), to: to.toISOString() })) as any[];
  const names = await projects();
  const summaries = entries.map((e) => summarize(e, names));
  const totalMinutes = summaries.reduce((sum, e) => sum + (e.duration_minutes ?? 0), 0);
  return { entries: summaries, total_minutes: totalMinutes };
}

export async function getTimer() {
  const running = await api("GET", "/timer");
  if (!running) return { running: false };
  return { running: true, entry: summarize(running, await projects()) };
}

export async function startTimer(description?: string, project?: string) {
  let projectId: string | null = null;
  const names = await projects();
  const wanted = project?.trim().toLowerCase();
  if (wanted) {
    const all = [...names.entries()];
    const found =
      all.find(([, name]) => name.toLowerCase() === wanted) ?? all.find(([, name]) => name.toLowerCase().includes(wanted));
    if (!found) {
      const existing = all.map(([, name]) => name).join(", ") || "none";
      throw new Error(`No project named '${project}'. Existing projects: ${existing}`);
    }
    projectId = found[0];
  }

  const entry = await api("POST", "/timer/start", undefined, {
    description: description?.trim() || null,
    project_id: projectId,
  });
  return summarize(entry, names);
}

export async function stopTimer() {
  return summarize(await api("POST", "/timer/stop"), await projects());
}
