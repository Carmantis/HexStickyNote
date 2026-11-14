# Following HexTime's logs on a server

On a server installed with `deploy/hextime-install.sh`, every HexTime service writes to the systemd journal. Run these as the user who owns HexTime. `-f` follows the log as it grows (Ctrl+C stops); leave it out to page through what is already there.

| Command | What it shows |
|---|---|
| `journalctl --user -u hextime -f` | HexTime itself: every request with its status code, start-up and database migrations, and the emails it sends (printed in full when the mail server is `console`). |
| `journalctl --user -u hextime-caddy -f` | The web server in front: HTTPS certificates being obtained and renewed, and connections it could not serve. |
| `journalctl --user -u hextime-postgres -f` | The database: start-up, shutdown and errors such as a full disk or a refused connection. |
| `journalctl --user -u 'hextime*' -f` | All of the above interleaved, which shows how one problem travels from service to service. |
| `journalctl --user -u hextime-backup` | The nightly database backups: when each ran and whether it succeeded. |
| `journalctl --user -u hextime -b` | HexTime's log since the server last started. |
| `journalctl --user -u hextime --since "1 hour ago"` | HexTime's log for a recent stretch of time; `--since today` or `--since "2026-10-24 09:00"` work too. |
| `journalctl --user -u hextime \| grep -i "sending to"` | Emails the mail server refused, with the reason. |
| `systemctl --user status hextime hextime-caddy hextime-postgres` | Whether each service is running, since when, and its last few log lines. |

Installing is covered in [Installing on a server](Install-server.md); updates, backups and common problems in [Running the server](Server-admin.md).
