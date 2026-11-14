# Running HexTime in development

Start the backend with `cd backend && uv run uvicorn app.main:app --reload`. It runs on port 8000 and migrates the local SQLite database on startup.
In a second terminal, start the frontend with `cd frontend && npm install && npm run dev`, then open http://localhost:5173. Vite reloads the UI on every change and forwards `/api` requests to the backend.

## Server mode on your own machine

`./scripts/dev-server.sh` runs the organisation version locally: it builds the UI, sets server mode, migrates its own database in `backend/.dev-server/`, adds demo users and a team, and starts on http://localhost:8000. Every demo user's password is `hextime-dev-123`; the script lists who is who. Emails are printed in the terminal instead of being sent. `--reset` starts over from an empty database, `--empty` skips the demo data, `--no-build` skips the UI build and `--port` picks another port. One browser holds one sign-in, so use a private window, another browser or http://127.0.0.1:8000 for a second person at the same time.

## The server stack in containers

`deploy/compose.yaml` runs what a server runs: HexTime, PostgreSQL and Caddy in front with HTTPS. Copy `deploy/.env.example` to `deploy/.env`, fill in `SECRET_KEY` and `POSTGRES_PASSWORD` (`openssl rand -base64 48`), and start it with `podman compose -f deploy/compose.yaml up --build` (Docker works the same). With `HEXTIME_DOMAIN=localhost` Caddy makes its own certificate, so the browser warns once. Create the first administrator with `podman exec -it hextime-hextime-1 hextime-entrypoint python -m app.cli create-admin --email you@example.org --name "You" --send-link`; with `SMTP_HOST=console` the link is printed right there. `podman compose -f deploy/compose.yaml down -v` removes everything, data included.

Installing on a real server is described in [Install-server.md](Install-server.md).

## End-to-end tests

`cd frontend && npm run e2e` builds the UI and runs the Playwright tests in `frontend/e2e/` against server mode, started by `dev-server.sh` with its own database in `backend/.dev-server/e2e/` (yours is left alone) and fresh demo data every run. They need a Chromium: `npx playwright install chromium` once, or point them at the system's with `PLAYWRIGHT_CHROMIUM=/usr/bin/chromium npm run e2e`.

## Desktop window

Build the UI once with `cd frontend && npm run build`, then start the window with `cd backend && uv run --extra desktop python -m app.desktop`. The `desktop` extra installs pywebview (and PyGObject on Linux, which needs the system's GTK and WebKitGTK); the server build leaves it out.

## Installing on Linux for yourself

`./packaging/build-app.sh` builds the app into `backend/dist/hextime/`; it does not install anything. `./packaging/linux/install.sh` then installs that build under `~/.local` with a menu entry and icon, so HexTime shows up in the application launcher (Omarchy's included). Run it again after a new build to update, or with `--uninstall` to remove the app; your data in `~/.local/share/HexTime/` is kept either way.

## Building the .deb

After `./packaging/build-app.sh`, `./packaging/linux/build-deb.sh` writes `packaging/output/hextime_<version>_amd64.deb` (needs [nfpm](https://nfpm.goreleaser.com/install/)). Build release packages on Ubuntu 24.04, as CI does: the bundle's PyGObject is compiled against the build machine's GLib, so a .deb built on a newer distribution (Arch, for one) will not run on Ubuntu 24.04. The package installs to `/opt/hextime` and pulls in GTK and WebKitGTK through apt.
