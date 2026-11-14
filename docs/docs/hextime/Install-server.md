# Installing HexTime on a server

## You need

- A Linux server with **Podman 4.6+** (Ubuntu 24.04, Debian 13, Fedora, RHEL 9) and a user account with `sudo`.
- A **host name**, e.g. `hextime.example.org`, with a DNS record pointing at the server.
- **Ports 80 and 443** open to the internet.
- **SMTP details** for sending email: server, port, user, password, sender address.
- The **email domain** allowed to have accounts, e.g. `example.org`.

## Steps

**1. Install Podman and open the ports** (on the server, with sudo):

```bash
sudo apt install podman                     # Fedora/RHEL: sudo dnf install podman
sudo ufw allow OpenSSH && sudo ufw allow 80,443/tcp && sudo ufw enable
# firewalld: sudo firewall-cmd --permanent --add-service={http,https} && sudo firewall-cmd --reload
```

**2. Sign in as the user who will own HexTime**, directly over SSH (not with `su`):

```bash
ssh hextime@hextime.example.org
```

**3. Download and run the installation script:**

```bash
curl -fsSLO https://raw.githubusercontent.com/Carmantis/HexTime/main/deploy/hextime-install.sh
bash hextime-install.sh
```

While the repository is private, copy the script to the server with `scp` and run `podman login --authfile ~/.config/containers/auth.json ghcr.io` first.

**4. Answer the questions.** Enter accepts the value in brackets. The script asks for the host name, the email domain, the language, the time zone, the first administrator and the SMTP details, then sends a test email. It may ask for your sudo password.

**5. Wait for** `HexTime is running: https://hextime.example.org`. HexTime, PostgreSQL and Caddy (HTTPS) are now running, and the database is backed up every night.

**6. Open the link in your email** and set your password. Then add people under **Users**.

## Updating or changing a setting

Run `bash hextime-install.sh` again. Your answers are the defaults; change what you need and press Enter at the rest.

Backups, logs, troubleshooting and securing the server: [Running the server](Server-admin.md).
