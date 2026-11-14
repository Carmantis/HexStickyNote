"""HexTime backend as a HexStickyNote sidecar.

HexStickyNote starts this process when the time tracking view is opened and
shows the UI it serves in the HUD. It runs HexTime's own FastAPI app (API and
compiled frontend) on a loopback port and keeps HexTime's local data directory,
so the standalone HexTime and the sidecar share the same database.

Run as: hextime-server --port 51234 [--watch-stdin]

The database is brought up to date here, before the server starts, so that a
failure ends the process with an exit code the caller can explain to the user
(src-tauri/src/hextime.rs and mcp-server/src/hextime.ts map the same codes):

  10  the database was written by a newer HexTime than this sidecar
  11  the HexTime desktop app is running on an older database version
  12  the database could not be opened or migrated
"""

import argparse
import os
import sys
import threading
import time
import traceback

#: Loopback only: local mode has no authentication.
HOST = "127.0.0.1"

EXIT_DATABASE_TOO_NEW = 10
EXIT_DESKTOP_APP_BUSY = 11
EXIT_DATABASE_FAILED = 12

#: How long to wait for a HexTime desktop app that is migrating right now.
DESKTOP_MIGRATION_WAIT = 10.0


def exit_when_parent_closes_stdin() -> None:
    """Exit when HexStickyNote goes away.

    The parent keeps our stdin open for as long as it runs; the pipe closes
    however the parent ends, crash included, so no server is left orphaned.
    """
    try:
        sys.stdin.read()
    finally:
        os._exit(0)


def lock_file(file) -> bool:
    """Take HexTime's lock without waiting; True if this process got it.

    The same OS lock as `_lock` in HexTime's app/desktop.py. It is repeated
    here because importing app.desktop would pull the desktop window's GUI
    libraries into the sidecar.
    """
    try:
        if sys.platform == "win32":
            import msvcrt

            file.seek(0)
            msvcrt.locking(file.fileno(), msvcrt.LK_NBLCK, 1)
        else:
            import fcntl

            fcntl.flock(file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        return False
    return True


def prepare_database(settings):
    """Migrate the shared database without racing the HexTime desktop app.

    The desktop app holds hextime.lock for as long as it runs and migrates
    under it. A migration here takes the same lock, so two processes never
    migrate at once; when the schema is already current no lock is needed.
    """
    from app.core import startup

    url = settings.resolved_database_url
    sqlite_file = startup.sqlite_file_of(url)
    if sqlite_file is None:
        startup.prepare_local_database(settings)
        return

    config = startup._alembic_config(url)
    if sqlite_file.is_file():
        startup.check_not_newer(config, url)
        if not startup.needs_migration(config, url):
            return

    sqlite_file.parent.mkdir(parents=True, exist_ok=True)
    # Closing the file releases the lock
    with sqlite_file.with_name("hextime.lock").open("a") as file:
        if lock_file(file):
            startup.prepare_local_database(settings)
            return

        # The desktop app has the lock: it may be migrating at this moment
        deadline = time.monotonic() + DESKTOP_MIGRATION_WAIT
        while time.monotonic() < deadline:
            time.sleep(0.5)
            if not startup.needs_migration(config, url):
                return
        print("hextime-server: the HexTime desktop app is open on an older database", file=sys.stderr)
        sys.exit(EXIT_DESKTOP_APP_BUSY)


def main() -> None:
    parser = argparse.ArgumentParser(description="HexTime backend for HexStickyNote")
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--watch-stdin", action="store_true")
    args = parser.parse_args()

    if args.watch_stdin:
        threading.Thread(target=exit_when_parent_closes_stdin, daemon=True).start()

    # Imported after argument parsing so --help works without the app
    import uvicorn

    from app.core.config import get_settings
    from app.core.startup import DatabaseTooNewError
    from app.main import create_app

    settings = get_settings()
    try:
        prepare_database(settings)
    except DatabaseTooNewError as error:
        print(f"hextime-server: {error}", file=sys.stderr)
        sys.exit(EXIT_DATABASE_TOO_NEW)
    except Exception:
        # A locked or read-only file, a full disk, a failed migration
        traceback.print_exc()
        sys.exit(EXIT_DATABASE_FAILED)

    app = create_app(settings.model_copy(update={"auto_migrate": False}))
    uvicorn.run(app, host=HOST, port=args.port, log_level="warning")


if __name__ == "__main__":
    main()
