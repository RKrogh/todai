# todai deploy guide (Raspberry Pi 5)

User-runnable, step-by-step. Claude does not touch secrets; you set them yourself
where indicated. All commands are idempotent unless noted.

Assumes:
- Pi 5 running Home Assistant OS or HA Supervised on Debian Bookworm/Trixie
- `~/.todai/` already exists on the PC (created by `todai init`)
- Syncthing already configured (or about to be)
- HA Companion app installed on your phone and registered with HA

The architecture you are building:

```
PC (WSL)  ──Syncthing──▶  ~/.todai/  ──Syncthing──▶  Pi 5
                                                       │
                                  ┌────────────────────┼─────────────────┐
                                  │                    │                 │
                          /usr/local/bin/todai   todai-agent.service   HA shell_command
                                (writes)          (reads + writes)     (writes via CLI)
```

---

## 1. Cross-compile the CLI on your PC

Pick one of the two approaches. **(A) is faster on WSL.**

### (A) Native GNU toolchain
```bash
sudo apt install -y gcc-aarch64-linux-gnu
rustup target add aarch64-unknown-linux-gnu
cd ~/projects/todai
cargo build --release --target aarch64-unknown-linux-gnu
ls -lh target/aarch64-unknown-linux-gnu/release/todai
```

### (B) `cross` (Docker-based)
```bash
cargo install cross
cd ~/projects/todai
cross build --release --target aarch64-unknown-linux-gnu
```

Either way you get the binary at:
`target/aarch64-unknown-linux-gnu/release/todai`

---

## 2. Copy the binary to the Pi

```bash
PI_HOST=raspberrypi.local           # change to your Pi's hostname or IP
PI_USER=robert                       # change to your Pi user

scp target/aarch64-unknown-linux-gnu/release/todai \
    ${PI_USER}@${PI_HOST}:/tmp/todai

ssh ${PI_USER}@${PI_HOST} 'sudo install -m 0755 /tmp/todai /usr/local/bin/todai && /usr/local/bin/todai --version'
```

Expected output: `todai 0.1.0`.

---

## 3. Make sure Syncthing is mirroring `~/.todai/` to the Pi

On the PC and the Pi, in Syncthing's web UI:
- Add the folder `~/.todai/` on both sides, link them, accept the share.
- Pi side path is up to you; recommended: `/home/<piuser>/.todai/`.
- Verify the `.todai/config.toml` and at least one todo file appear on the Pi.

```bash
ssh ${PI_USER}@${PI_HOST} 'ls -la ~/.todai && cat ~/.todai/.todai/config.toml | head -10'
```

If you keep todai's store at `~/.todai/` on the Pi too, you can skip `--path` everywhere.

---

## 4. Set up the agent (Python, on the Pi)

```bash
ssh ${PI_USER}@${PI_HOST}
```

On the Pi:
```bash
# 4a. System Python 3.11+ (Bookworm has 3.11; Trixie has 3.12)
python3 --version    # confirm >= 3.11

# 4b. Drop the agent code on the Pi. Easiest: rsync it from your PC.
#     (run this from your PC, NOT the Pi)
exit   # back to PC
rsync -avh --exclude '__pycache__' --exclude '.venv' \
      ~/projects/todai/agent/ ${PI_USER}@${PI_HOST}:~/todai-agent/

# back on the Pi
ssh ${PI_USER}@${PI_HOST}
cd ~/todai-agent
python3 -m venv .venv
.venv/bin/pip install --upgrade pip
.venv/bin/pip install -e .
.venv/bin/todai-agent --version
```

---

## 5. Set up secrets (you do this; Claude never sees the values)

The agent reads two env vars at runtime:
- `ANTHROPIC_API_KEY` (from `[ai].api_key_env` in the config)
- `HA_TOKEN`           (from `[notifications].homeassistant_token_env`)

**Do not paste these into chat.** Set them up using the script below or by editing
the EnvironmentFile yourself.

### Option A: systemd EnvironmentFile (recommended)

On the Pi:
```bash
sudo install -m 0700 -d /etc/todai-agent
sudo touch /etc/todai-agent/secrets.env
sudo chmod 0600 /etc/todai-agent/secrets.env
sudo nano /etc/todai-agent/secrets.env
```

Add (with your real values, on the Pi only):
```
ANTHROPIC_API_KEY=sk-ant-...
HA_TOKEN=eyJhbGciOi...
```

The systemd unit in step 6 picks these up via `EnvironmentFile=`.

### How to get the HA long-lived token
1. Open Home Assistant in a browser, click your profile (bottom left).
2. Scroll to "Long-Lived Access Tokens", click "Create Token".
3. Name it `todai-agent`. Copy the token shown ONCE; paste into `secrets.env`.

### How to get the Anthropic key
From <https://console.anthropic.com/settings/keys>. Create a key, paste into `secrets.env`.

---

## 6. systemd service + timer (on the Pi)

Create `/etc/systemd/system/todai-agent.service`:
```ini
[Unit]
Description=todai AI agent (one-shot)
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
User=robert
WorkingDirectory=/home/robert/todai-agent
EnvironmentFile=/etc/todai-agent/secrets.env
Environment=TODAI_HOME=/home/robert/.todai
ExecStart=/home/robert/todai-agent/.venv/bin/todai-agent
Nice=10
```

Create `/etc/systemd/system/todai-agent.timer`:
```ini
[Unit]
Description=todai AI agent schedule

[Timer]
# Morning briefing
OnCalendar=*-*-* 07:00:00
# Evening check
OnCalendar=*-*-* 18:00:00
# Hourly during working hours
OnCalendar=*-*-* 08..17:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

Enable:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now todai-agent.timer
sudo systemctl list-timers | grep todai
```

Tail logs:
```bash
journalctl -u todai-agent.service -f
```

---

## 7. HA action buttons → CLI write-back

In Home Assistant, edit `automations.yaml` (or via UI → Settings → Automations):

```yaml
- alias: todai - mark done from notification
  trigger:
    - platform: event
      event_type: mobile_app_notification_action
      event_data:
        action: TODAI_DONE
  action:
    - service: shell_command.todai_done
      data:
        todo_id: "{{ trigger.event.data.tag | regex_replace('^todai:', '') }}"

- alias: todai - snooze 1h from notification
  trigger:
    - platform: event
      event_type: mobile_app_notification_action
      event_data:
        action: TODAI_SNOOZE_1H
  action:
    - service: shell_command.todai_snooze
      data:
        todo_id: "{{ trigger.event.data.tag | regex_replace('^todai:', '') }}"
        duration: "1h"

- alias: todai - snooze 1d from notification
  trigger:
    - platform: event
      event_type: mobile_app_notification_action
      event_data:
        action: TODAI_SNOOZE_1D
  action:
    - service: shell_command.todai_snooze
      data:
        todo_id: "{{ trigger.event.data.tag | regex_replace('^todai:', '') }}"
        duration: "1d"
```

In `configuration.yaml`:
```yaml
shell_command:
  todai_done: "/usr/local/bin/todai --path /home/robert/.todai done {{ todo_id }}"
  todai_snooze: "/usr/local/bin/todai --path /home/robert/.todai snooze {{ todo_id }} --for {{ duration }}"
```

Restart HA (or reload Shell Commands + Automations). When you tap "Done" on a
todai push notification, HA fires the event → automation matches → shell command
runs the CLI on the Pi → markdown file updates → Syncthing propagates to your PC.

---

## 8. Verify the full loop

1. On the PC: `todai add "Test push" --due tomorrow --remind <something past>`
2. Wait for Syncthing to mirror to the Pi (seconds).
3. Trigger a manual run: `ssh pi 'sudo systemctl start todai-agent.service'`
4. Watch the phone for a notification with Done / Snooze 1h / Tomorrow buttons.
5. Tap "Done" → HA event → CLI on Pi marks done → Syncthing → PC sees `status: done`.
6. `todai list` on the PC should now show no pending "Test push".

If any step fails, `journalctl -u todai-agent.service -e` is the first place to look.

---

## What Claude touched and what you did

- Claude wrote: every config file, every line of code, this guide, the systemd
  unit templates, the HA automation/shell_command snippets.
- You did: create the secrets file, paste the real `ANTHROPIC_API_KEY` and
  `HA_TOKEN` values into it, run the deploy commands.

This division is by policy, not by laziness. Secrets stay off Claude's surface
on purpose; see `~/.claude/CLAUDE.md` ("Don't touch secrets directly").
