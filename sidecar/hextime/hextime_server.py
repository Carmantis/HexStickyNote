"""HexTime backend as a HexStickyNote sidecar.

HexStickyNote starts this process when the time tracking view is opened and
shows the UI it serves in the HUD. It runs HexTime's own FastAPI app (API and
compiled frontend) on a loopback port and keeps HexTime's local data directory,
so the standalone HexTime and the sidecar share the same database.

Run as: hextime-server --port 51234 [--watch-stdin]
"""

import argparse
import os
import sys
import threading

#: Loopback only: local mode has no authentication.
HOST = "127.0.0.1"


def exit_when_parent_closes_stdin() -> None:
    """Exit when HexStickyNote goes away.

    The parent keeps our stdin open for as long as it runs; the pipe closes
    however the parent ends, crash included, so no server is left orphaned.
    """
    try:
        sys.stdin.read()
    finally:
        os._exit(0)


def main() -> None:
    parser = argparse.ArgumentParser(description="HexTime backend for HexStickyNote")
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--watch-stdin", action="store_true")
    args = parser.parse_args()

    if args.watch_stdin:
        threading.Thread(target=exit_when_parent_closes_stdin, daemon=True).start()

    # Imported after argument parsing so --help works without the app
    import uvicorn

    from app.main import app

    uvicorn.run(app, host=HOST, port=args.port, log_level="warning")


if __name__ == "__main__":
    main()
