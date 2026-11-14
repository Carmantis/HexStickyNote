# Running the server

Run the commands as the user who owns HexTime, unless they start with `sudo`.

## Everyday

| Task | Command |
| --- | --- |
| Update HexTime or change a setting | `bash hextime-install.sh` |
| Is it running? | `systemctl --user status hextime hextime-postgres hextime-caddy` |
| HexTime's log | `journalctl --user -u hextime -f` (more in [Server logs](Logs.md)) |
| Restart | `systemctl --user restart hextime` |
| Automatic updates | `systemctl --user enable --now podman-auto-update.timer` |
| Settings | `~/.config/hextime/` (edit by running the script again) |

## Backups

A backup is taken every night to `~/.local/share/hextime/backups/` (14 kept). **Copy them to another machine**: they are on the same disk as the database.

Take one now: `systemctl --user start hextime-backup.service`

Restore one (replaces all data):

```bash
systemctl --user stop hextime-caddy hextime
podman exec hextime-postgres dropdb -U hextime hextime
podman exec hextime-postgres createdb -U hextime hextime
podman exec -i hextime-postgres pg_restore -U hextime -d hextime < ~/.local/share/hextime/backups/FILE.dump
systemctl --user start hextime hextime-caddy
```

## Before production

- [ ] Ask your IT support whether a password alone is enough for sign-in (HexTime has no two-factor authentication).
- [ ] Backups are copied off the server regularly.
- [ ] Only ports 80 and 443 are open; SSH uses keys only (`PasswordAuthentication no`).
- [ ] Operating system updates install automatically (`unattended-upgrades` or `dnf-automatic`).

## When something goes wrong

| Problem | Fix |
| --- | --- |
| Certificate warning, or the site does not open | Check the DNS record and ports 80/443; `journalctl --user -u hextime-caddy` |
| Emails do not arrive | Check spam; run the script again to fix SMTP (it sends a test email) |
| HexTime does not start | `journalctl --user -u hextime -n 50` shows why |
| Services stop when you sign out | `sudo loginctl enable-linger hextime` (from an account with sudo) |
| `podman pull` refused (private image) | `podman login --authfile ~/.config/containers/auth.json ghcr.io`, or install with `--build` |

## Removing

```bash
systemctl --user disable --now hextime-backup.timer
systemctl --user stop hextime-caddy hextime hextime-postgres
rm ~/.config/containers/systemd/hextime* ~/.config/systemd/user/hextime-backup.*
systemctl --user daemon-reload
podman volume rm hextime-postgres-data hextime-caddy-data hextime-caddy-config
podman network rm hextime
rm -r ~/.config/hextime
```

This deletes all data. The backups stay in `~/.local/share/hextime/`.
