# todai - AI-powered cross-machine todo & reminder system

## Vision

A local-first, markdown-based todo system with hierarchical contexts, synced across devices via
Syncthing, with an AI agentic loop running on a Raspberry Pi 5 that proactively reasons about
upcoming tasks and sends context-aware notifications.

## Architecture Overview

```
┌─────────────┐     ┌─────────────┐     ┌──────────────────┐
│  PC (WSL)   │     │   Mobile    │     │  Raspberry Pi 5  │
│             │     │  (Android)  │     │  (HomeAssistant)  │
│  todai CLI  │◄───►│  Syncthing  │◄───►│  Syncthing        │
│  edit/view  │     │  view only? │     │  AI agentic loop  │
│             │     │  HA notifs  │     │  agent add-on     │
└─────────────┘     └─────────────┘     └──────────────────┘
       │                                        │
       └──────── Syncthing (encrypted) ─────────┘
```

All devices share the same `todai/` folder via Syncthing. The Pi runs the AI loop on a schedule,
reads the todo files, calls Claude API for reasoning, and sends notifications via Home Assistant
Companion app to Robert's phone.

## Core Components

### 1. Todo Storage Format

Markdown files with YAML frontmatter. One file per todo, stored in a flat or context-based
folder structure.

```
~/.todai/                          # or ~/projects/todai-data/ (syncthing root)
├── work/
│   ├── assignments/
│   │   └── review-pr-backend.md
│   ├── staff/
│   │   └── book-1on1-erik.md
│   └── generategroup/
│       └── update-website.md
├── private/
│   ├── garden/
│   │   └── plant-carrots.md
│   ├── shoppinglist/
│   │   └── weekly-groceries.md
│   └── kids/
│       └── birthday-party-gift.md
└── .todai/
    └── config.toml                # local config (notification prefs, API keys, etc.)
```

#### Todo file format

All datetimes are stored in **UTC** (ISO 8601 with trailing `Z`). The CLI converts to the local
timezone on display. Rationale: travel and multi-device work should not cause "why does this
reminder fire at 3 AM?" surprises. Bare dates (no time) are allowed for "some time today"
todos; the agent treats them as `00:00` local on the Pi for scheduling.

```markdown
---
id: book-1on1-with-erik                           # slug; suffixed with -a3f2 nanoid on collision
title: "Book 1-on-1 with Erik"
context: work:staff
status: pending                                   # pending | in_progress | done | cancelled
priority: normal                                  # low | normal | high | urgent
created: 2026-04-16T09:12:00Z
due: 2026-05-16T14:00:00Z                         # UTC; displayed as local time
completed: null                                   # set when status=done
completed_by: null                                # hostname of the machine that ran `todai done`
remind:
  - at: 2026-05-09T07:00:00Z
    message: "Schedule the 1-on-1 with Erik for next week"
    notified_at: null                             # agent writes this after sending
    notify_mode: smart                            # smart (Claude) | dumb (plain text)
  - at: 2026-05-15T07:00:00Z
    message: "1-on-1 with Erik tomorrow. Review his recent PRs and project status."
    notified_at: null
    notify_mode: smart
tags: [meeting, erik, recurring]
recur:
  rule: monthly                                   # none | daily | weekly | monthly | yearly
  next_due: 2026-06-16T14:00:00Z                  # what the spawned successor will use
  prev_id: null                                   # id of the instance this was spawned from
---

Book a 1-on-1 with Erik to discuss project progress and career development.

## Notes
- Last 1-on-1: discussed migration timeline
- He mentioned wanting to explore more frontend work
```

**Slug collisions.** Default `id` is the slug. If a file with that name already exists in the
same context, the CLI suffixes with a short nanoid: `book-1on1-with-erik-a3f2`. The full `id`
in frontmatter is what everything else references (reminders, recur links, action buttons).

**Notification idempotency.** The agent writes `notified_at` back into the file when a
reminder fires. Next scan sees it's already handled and skips. This makes the agent a
writer, not just a reader — worth knowing from day one so the read/write contract is clear.

**Recurring lifecycle.** When `todai done <id>` completes a todo with `recur.rule != none`:
1. Set `status=done`, `completed=<now UTC>`, `completed_by=<hostname>`.
2. Spawn a new file for the next instance, with `due = recur.next_due`, `prev_id = <this id>`,
   and a freshly computed `next_due` based on the rule.
3. Copy over `title`, `context`, `priority`, `tags`, `remind` (with `notified_at: null`),
   and the body.
The chain of `prev_id` links forms the history of a recurring task. `todai list` hides the
completed ancestors by default; `todai show <id> --history` traverses the chain.

### 2. CLI Tool (`todai`)

Rust CLI for managing todos. Core commands:

```bash
# First-time setup
todai init                              # see "todai init" section below

# Creating todos
todai add "Plant carrots" --context private:garden --due 2026-05-01 --remind 2026-04-25
todai add "Buy birthday gift" --context private:kids --due 2026-04-20 --priority high

# Listing and filtering
todai today                             # shortcut: due today + overdue + today's reminders
todai list                              # all pending todos
todai list --context work               # all work todos (any sub-context)
todai list --context work:staff         # specific sub-context
todai list --due today                  # due today
todai list --due this-week              # due this week
todai list --overdue                    # past due
todai list --tag meeting                # by tag
todai list --json                       # machine-readable; used by the agent

# Managing todos
todai done "plant-carrots"              # mark as done (by id, slug, or fuzzy match)
todai archive                           # sweep done/cancelled items past archive_after_days into .archive/
todai archive --dry-run                 # preview what would be swept, move nothing
todai edit "plant-carrots"              # open in $EDITOR
todai show "plant-carrots"              # display full todo (times in local TZ)
todai show "plant-carrots" --history    # follow prev_id chain for recurring tasks
todai rm "plant-carrots"                # remove/archive

# Context management
todai contexts                          # list all contexts
todai switch work:staff                 # set default context for new todos

# Reminders
todai remind "plant-carrots" --at 2026-04-28T09:00 --message "Time to plant!"
todai upcoming                          # show upcoming reminders
todai snooze "plant-carrots" --for 1h   # push all pending reminders out by 1h

# Search (delegates to oi if available)
todai search "carrots"

# Conflict handling (Syncthing)
todai conflicts                         # list .sync-conflict-*.md, show diff, pick a side
```

#### `todai init`

Bootstraps a fresh todo store. Idempotent — running it on an existing store prints status
rather than clobbering anything.

```bash
todai init                              # uses ~/.todai/ (or $TODAI_HOME if set)
todai init --path ~/projects/todai-data # explicit location for the Syncthing folder
```

What it does:
1. Creates the store directory if missing.
2. Writes `.todai/config.toml` with commented defaults (see Configuration section).
3. Drops a `.stignore` template so Syncthing skips `.archive/`, logs, and `.sync-conflict-*` noise.
4. Scaffolds the context subfolders listed in `[contexts.allowed]` (if any).
5. Prints next-step hints: share the folder via Syncthing, set `ANTHROPIC_API_KEY` /
   `HA_TOKEN`, install the binary on the Pi (`/share/todai/bin/`), enable the agent add-on.

#### CLI design principles
- Minimal flags for common operations, rich flags for power use
- Fuzzy matching on todo slugs (don't force exact names)
- Color-coded output (priority, overdue status, context)
- Shell completions (bash, zsh, fish)
- Every `list`-style command supports `--json` for scripting and for the agent to consume

#### CLI design principles
- Minimal flags for common operations, rich flags for power use
- Fuzzy matching on todo slugs (don't force exact names)
- Color-coded output (priority, overdue status, context)
- Shell completions (bash, zsh, fish)

### 3. AI Agentic Loop

Runs on the Raspberry Pi 5 as a custom HA add-on with an internal scheduler (the Pi runs HAOS,
so there are no user systemd timers; see Prerequisites).

#### Schedule
- **Morning run (07:00):** Daily briefing. What's due today, what's coming this week,
  any contextual observations.
- **Evening check (18:00):** Anything missed today? Anything to prep for tomorrow?
- **Hourly scan (working hours):** Quick check for urgent/high-priority items approaching
  their reminder times.

#### What the AI does
1. Shells out to `todai list --json --all` to get a structured snapshot (single source of
   truth for parsing — the agent doesn't re-implement YAML frontmatter handling).
2. Pre-filters: drops todos whose reminders aren't due, or where `notified_at` is already set
   for the relevant reminder. This keeps cost proportional to actual work, not inventory size.
3. For each reminder due now, decides `smart` vs `dumb`:
   - `notify_mode: dumb` → skip Claude, send a plain text notification.
   - `notify_mode: smart` (default) → call Claude for a context-aware message.
4. Calls Claude with the relevant todos and context. Claude reasons about:
   - What needs attention right now
   - Contextual connections ("you're in the city tomorrow, the gift shop is nearby")
   - Proactive suggestions ("Erik's 1-on-1 is next week, want me to check his recent activity?")
   - Seasonal/recurring awareness ("It's May, garden todos are relevant now")
5. Sends notification via Home Assistant REST API to Companion app (with action buttons — see
   "HA action buttons" below).
6. Writes `notified_at: <now UTC>` back into the todo file so the next scan doesn't re-fire.

#### Offline / API failure fallback

If the Anthropic API is unreachable (no network, quota, outage), the agent does **not** silently
skip. It falls back to the dumb-notification path for every reminder that would have been
`smart`, using the reminder's own `message` field as the body. One blunt push beats zero.
The agent logs the degradation so it's visible on the next successful run.

Per-task opt-in to the cheap path: set `notify_mode: dumb` on a reminder (or globally via
`[notifications].default_notify_mode = "dumb"` in config) to bypass Claude entirely for that
item. Useful for things where "milk is due" is all you need.

#### Cost posture

No hard cap for the POC. We're gathering usage data first — strangling the feature before we
know what it costs is a worse error than a surprise bill. Mitigations we get for free from the
design: pre-filter before calling Claude, `notify_mode: dumb` toggle, and Anthropic prompt
caching on the static portion of the system prompt (the schema / instructions). Revisit once
we have a month of real telemetry.

#### AI loop script (Python, runs on Pi)

The agent is **provider-agnostic**: the LLM call is wrapped behind a `LlmProvider` interface,
so swapping Anthropic for OpenAI, Ollama, or anything else is a config change
(`[ai].provider = "anthropic"`), not a code change. Notification delivery is similarly
abstracted via a `Notifier` interface (HA today, ntfy/email/print later).

```
agent/
├── agent.py                # main loop: read CLI JSON, decide, notify, write back
├── config.py               # reads .todai/config.toml
├── cli.py                  # subprocess wrapper around the `todai` binary
├── providers/              # LLM provider abstraction
│   ├── __init__.py         # LlmProvider Protocol + build_provider() factory
│   ├── anthropic.py        # AnthropicProvider
│   └── (future: openai.py, ollama.py)
├── notifiers/              # notification delivery abstraction
│   ├── __init__.py         # Notifier Protocol + build_notifier() factory
│   ├── home_assistant.py   # HomeAssistantNotifier
│   └── print.py            # PrintNotifier (dev / dry-run)
├── prompts.py              # system prompt + prompt builders (provider-neutral text)
└── requirements.txt        # anthropic, httpx, tomli (3.11+ uses stdlib tomllib)
```

Secrets are **never read from synced files**. The agent reads the env-var name from
`[ai].api_key_env` / `[notifications].homeassistant_token_env` and looks the value up at
runtime. Setup of those env vars happens via the add-on configuration / HA secrets,
documented in `DEPLOY.md` (script-driven, user runs locally).

#### Home Assistant notification integration

The HA REST API can send notifications to the Companion app:

```python
# POST to /api/services/notify/mobile_app_<device_name>
{
    "message": "1-on-1 with Erik tomorrow. He recently worked on the migration PR.",
    "title": "todai: work:staff",
    "data": {
        "priority": "high",
        "channel": "todai",
        "tag": "todai:<todo-id>",           # used by the action handler to route the tap
        "actions": [
            {"action": "TODAI_DONE",        "title": "Done"},
            {"action": "TODAI_SNOOZE_1H",   "title": "Snooze 1h"},
            {"action": "TODAI_SNOOZE_1D",   "title": "Tomorrow"}
        ]
    }
}
```

Benefits of using HA:
- Already running on the Pi
- Companion app already on phone
- Notification channels, priorities, grouping built in
- Can correlate with other HA data (location, calendar, etc.)

#### HA action buttons → write-back plumbing

The agent writes. The phone (via HA) writes too. Both go through the same CLI binary on the
Pi, which means the CLI is the **single source of truth for write operations across the whole
system** — a design decision worth internalizing up front.

Flow when the user taps an action button on the phone:

```
Phone tap "Done"
      │
      ▼
Companion app fires "mobile_app_notification_action" event to HA
      │        (event data: {action: "TODAI_DONE", tag: "todai:<todo-id>"})
      ▼
HA automation matches the event, extracts the todo id from the tag
      │
      ▼
HA shell_command: `/usr/local/bin/todai done <todo-id>`   (binary on the Pi)
      │
      ▼
CLI edits the markdown file in the Syncthing folder
      │
      ▼
Syncthing propagates → PC (WSL) and phone see the update on next sync
      │
      ▼
Next agent tick sees status=done (or snoozed reminders), does not re-notify
```

Snooze actions map to `todai snooze <id> --for 1h` (or `--for 1d`). Because the CLI owns the
write path, HA never touches markdown directly and we don't have two codepaths that can
disagree. The CLI must therefore be deployable to both the PC (WSL) and the Pi — cross-compile
`aarch64-unknown-linux-musl` (static) is already in the prerequisites.

#### Syncthing conflict handling (keep it simple)

Syncthing produces `.sync-conflict-<date>-<hash>.md` when two machines touch the same file.
For POC: `todai conflicts` lists them, shows a diff against the "winning" file, and lets the
user pick a side or open `$EDITOR`. The agent skips any file matching `.sync-conflict-*.md`
when reading. AI-driven conflict resolution is a later problem once we see what real
conflicts look like in practice.

#### Observability

The agent runs unattended on the Pi. We want to be able to debug it without SSHing in.

- **journald on the Pi** for the service lifecycle (`journalctl -u todai-agent`). Standard.
- **Rolling log inside the synced folder**: `.todai/logs/agent.log` — last N runs, readable
  from any device that has the share. Captures timestamp, list of todos considered, which
  ones matched, the Claude request/response pair (truncated), and the outcome.
- **Retention**: keep the last 100 runs or 14 days, whichever is smaller. Log file is in
  `.stignore` so it doesn't round-trip back and cause sync churn (well, actually it does
  need to sync out from the Pi but shouldn't sync back in — use Syncthing "send only" on the
  Pi's copy if this becomes a problem, otherwise accept the churn).
- **`todai logs`** CLI command: tail the latest agent log entries from the local copy.

### 4. Configuration

```toml
# .todai/config.toml

[general]
default_context = "work:assignments"
editor = "nvim"                        # or $EDITOR
archive_done = true                    # move done items to .archive/
archive_after_days = 7                 # archive done items after N days

[contexts]
# Define valid contexts (optional, for autocomplete/validation)
allowed = [
    "work:assignments",
    "work:staff",
    "work:generategroup",
    "private:garden",
    "private:shoppinglist",
    "private:kids",
    "private:home",
]

[notifications]
method = "homeassistant"               # homeassistant | ntfy | email | none
homeassistant_url = "http://homeassistant.local:8123"
homeassistant_token_env = "HA_TOKEN"   # env var name, not the token itself
device_name = "robert_phone"
default_notify_mode = "smart"          # smart | dumb (per-reminder override wins)

[ai]
model = "claude-sonnet-4-6"            # keep costs reasonable for frequent runs
api_key_env = "ANTHROPIC_API_KEY"      # env var name
morning_briefing = "07:00"
evening_check = "18:00"
hourly_scan = true
hourly_scan_range = "08:00-17:00"
# Contexts the AI agent is forbidden from reading. Reminders in these contexts will
# always use notify_mode=dumb, regardless of the per-reminder setting. Todo content
# never leaves the Pi for these contexts. Use for health, finance, personal notes, etc.
exclude_contexts = []

[logging]
path = ".todai/logs/agent.log"
retain_runs = 100
retain_days = 14
```

#### Time, dates, and timezones

- All datetimes in frontmatter: **UTC, ISO 8601, trailing Z**.
- CLI display: converted to the machine's local timezone via `chrono-tz` / `time` crate, with
  the timezone abbreviation shown (`due: 2026-05-16 16:00 CEST`).
- `todai add --due 2026-05-16T14:00` — interpreted as local time, stored as UTC. Add
  `--tz Europe/Stockholm` to override when scripting from somewhere else.
- `todai today` uses the machine's local midnight-to-midnight window, not UTC, so "today" is
  what the human thinks it is regardless of which timezone the Pi is in.
- Bare dates without time (e.g., `due: 2026-05-16`) are allowed for "some time that day" items;
  the agent treats them as the Pi's local 00:00 for scheduling decisions but doesn't fabricate
  a fake time in the file.

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| CLI language | Rust | Matches oi, Robert's preference, cross-compiles to ARM64 |
| Todo format | Markdown + YAML frontmatter | oi-indexable, human-readable, editor-friendly |
| YAML parsing | `serde-saphyr` | `serde_yaml` is archived (2024); `serde-saphyr` is the actively maintained serde-compatible replacement as of 2026-04, avoids `unsafe-libyaml`, passes full yaml-test-suite |
| CLI args | `clap` (derive) | Standard, excellent completions generation |
| Datetime | `chrono` + `chrono-tz` | UTC storage, local display, IANA tz names |
| ID generation | `nanoid` | Short collision-resistant suffixes for slug conflicts |
| Sync | Syncthing | Local-first, encrypted in transit, no cloud dependency |
| AI loop | Python | Lighter weight for the Pi, Anthropic SDK is excellent |
| Notifications | Home Assistant | Already deployed on the Pi, Companion app on phone |
| AI model | Claude Sonnet | Cost-effective for frequent automated runs |
| Config | TOML | Rust ecosystem standard, readable |

## Prerequisites

### On development machine (WSL)
- Rust toolchain (already installed)
- Cross-compilation target: `rustup target add aarch64-unknown-linux-musl` (static; must run in HA's Alpine container)
- Build with rust-lld, no Docker or `cross` needed (deps are pure Rust):
  `RUSTFLAGS="-C linker=rust-lld" cargo build --target aarch64-unknown-linux-musl --release`
  Validated 2026-06-10: fully static ELF, full add/list/done round-trip passes under qemu-aarch64.

### On Raspberry Pi 5 (HAOS 17.x)
The Pi runs **Home Assistant OS**, not general-purpose Linux. No `apt`, no user systemd units:
everything deploys as HA add-ons.

- **Syncthing**: community add-on, data dir on `/share/todai` so other containers can reach the store
- **Agent**: custom local add-on (container bundling the Python agent + `todai` binary; scheduling
  is a loop/cron *inside* the add-on, replacing the earlier systemd-timer idea)
- **`todai` binary**: build for `aarch64-unknown-linux-musl` (static). `shell_command` executes
  inside the HA core container (Alpine), where a glibc build won't run. Place the binary at
  `/share/todai/bin/todai` so both HA (action buttons) and the agent add-on call the same one.
- Home Assistant Companion app configured on the phone
- **Tailscale add-on** (installed 2026-06): remote reach for HA Assist capture and admin away from
  home. Not part of the sync path; Syncthing owns store-and-forward.
- Claude API key via add-on configuration / HA secrets, surfaced to the agent as an env var

### On Android phone
- Syncthing app (for todo file sync, optional, could be view-only)
- Home Assistant Companion app (for notifications)

## Implementation Phases

### Phase 1: POC (done except sync verification)
- [x] Cargo project scaffolding with `clap`, `serde`, `serde-saphyr`, `chrono`, `chrono-tz`, `nanoid`, `anyhow`
- [x] Frontmatter schema as a typed `Todo` struct with round-trip tests
- [x] Filesystem layer: read/write markdown+frontmatter, UTC serialization, slug + nanoid collision handling
- [x] CLI commands: `init`, `add`, `list`, `today`, `show`, `done` (with completion metadata), `edit`
- [x] `--json` output on `list` / `today` / `show`
- [x] Spawn-on-done recurring logic with `prev_id` chain
- [x] Simple Python agent that shells out to `todai list --json`, sends a test HA notification
- [x] Agent writes `notified_at` back to the file after sending
- [x] Offline fallback: dumb notification if Anthropic API call fails
- [ ] Verify Syncthing sync between WSL and Pi

### Phase 2: Core features
- [x] CLI: `edit` (plus `archive`, `snooze`, `notified` beyond original plan)
- [ ] Remaining CLI: `rm`, `search`, `remind`, `contexts`, `upcoming`
- [ ] Fuzzy matching for todo slugs
- [ ] Shell completions
- [x] AI reasoning with Claude API (provider abstraction, key via env var only)
- [ ] Agent add-on on the Pi (HAOS local add-on with internal scheduler)
- [x] HA notification with action buttons (done, snooze) — HA automation wiring per DEPLOY.md, unverified on device

### Phase 3: Intelligence
- [ ] Calendar integration (Google Calendar via API or HA integration)
- [ ] Location awareness via HA (phone location for "you're in the city" nudges)
- [ ] Recurring todo generation
- [ ] Weekly summary/review prompt
- [ ] Learn from patterns (e.g., "you usually do garden tasks on weekends")

### Phase 4: Polish
- [ ] TUI interface (ratatui) for browsing todos
- [ ] Mobile-friendly viewer (simple web UI or Obsidian plugin)
- [ ] Encryption at rest for sensitive contexts
- [ ] Multi-user support (family shared lists)
- [ ] oi integration (search todos alongside knowledge base)

## Security & Privacy

- **No sensitive info in todos.** Hard rule for now. Sync folder is unencrypted at rest on
  trusted devices. Future phases may add per-context encryption.
- **API keys** stored as environment variables or HA secrets, never in synced files.
- **Syncthing** encrypts in transit (TLS 1.3). For untrusted relay devices, enable
  "untrusted device" encryption.
- **What the AI sees.** By default the agent sends all matching todos (title, body, notes,
  tags, context) to the Anthropic API when computing `smart` messages. Use
  `[ai].exclude_contexts` in the config to prevent specific contexts from ever reaching the
  API — those reminders fall back to `dumb` mode and the content stays on the Pi.

## Resolved design decisions

Recording these so we don't relitigate them later. If we change our minds, update here.

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Agent writes `notified_at` back into the todo file | Single source of truth, no hidden state dir, already synced everywhere |
| 2 | Recurring tasks use spawn-on-done with `prev_id` chain + `next_due` field | Visible in the file ("when is the next one?"), cleanly links history |
| 3 | All datetimes stored as UTC ISO 8601, displayed as local TZ | Travel-safe, no "what zone is this?" ambiguity |
| 4 | Slug collisions resolved with nanoid suffix on the filename | Reusable pattern — use nanoid anywhere we hit collision headaches |
| 5 | No AI cost cap yet | Collect data first. Mitigations available: pre-filter, `dumb` mode, prompt caching |
| 6 | Offline fallback: send dumb notifications when Claude is unreachable | One blunt push beats zero; per-reminder `notify_mode: dumb` also opts in |
| 7 | Syncthing conflicts: simple `todai conflicts` CLI for now, AI resolution later | See what real conflicts look like before building for them |
| 8 | `todai init` bootstraps store + config + stignore + context scaffolding | Low-cost, big onboarding win, idempotent |
| 9 | `todai today` as a first-class command | Most-used view earns its shortcut |
| 10 | On `done`: record `completed` (UTC) and `completed_by` (hostname) | Enables analytics, pattern learning, "where was this closed?" debugging |
| 11 | Agent consumes `todai list --json`, doesn't re-parse markdown | CLI is single source of truth for reading too |
| 12 | Observability: journald + rolling `.todai/logs/agent.log` + `todai logs` | Debug without SSHing the Pi |
| 13 | `[ai].exclude_contexts` in config = hard privacy boundary | Explicit opt-out for contexts that must not hit the API |
| 14 | YAML crate: `serde-saphyr` | `serde_yaml` archived 2024; `serde-saphyr` is the current actively maintained replacement |
| 15 | HA action buttons shell out to `todai` on the Pi, which owns writes | CLI is single source of truth for writes too — no two-codepath drift |
| 16 | Agent abstracts over LLM providers via `LlmProvider` interface | Vendor-agnostic; switching to OpenAI/Ollama is a config change, not a code change |
| 17 | Secrets never live in synced files or on Claude's surface | API keys / HA tokens read from env vars at runtime; setup via user-run scripts only |
| 18 | Archive is two-stage: `done` flags (status + `completed`), a separate `todai archive` sweep moves items done/cancelled > `archive_after_days` into `.archive/`, mirroring the context tree | Keeps an "undo" window after completion; sweep is idempotent and agent-driven (daily), not bolted onto `done` (which would never re-touch an old item). `archive_done = false` = flag-only. Known limitation: `.archive/` is in `.stignore`, so archives live only on the machine that swept them; drop it from `.stignore` if cross-device archive history is ever wanted |
| 19 | Mobile is read-only Syncthing + HA action buttons (Path A); no direct markdown editing on the phone (Path B) | Editing YAML frontmatter on a phone keyboard is miserable and corruption-prone, and phone-side writes are the likeliest conflict source. Action buttons (done/snooze) cover the 90% case and still route through the CLI on the Pi (decision #15). Future mobile quick-capture should be an HA button that shells `todai add`, not Path B |
| 20 | Pi runs HAOS (17.x), so deployment is add-on-based: Syncthing community add-on, agent as a custom local add-on with internal scheduler, `todai` built static (`aarch64-unknown-linux-musl`) at `/share/todai/bin/todai` | No `apt`/systemd on HAOS. `shell_command` runs inside HA's Alpine container, so the binary must be static musl, and one shared binary on `/share` keeps the single-write-path rule (#15) intact. Tailscale add-on provides remote reach (Assist/admin away from home); it is reachability only, Syncthing remains the store-and-forward transport |

## Still-open questions

*(none currently — see decisions #18 and #19 for the two that were here)*

## File Structure (this repo)

```
todai/
├── PLAN.md                 # this file
├── CLAUDE.md               # instructions for Claude sessions working on this project
├── Cargo.toml              # Rust CLI
├── src/
│   ├── main.rs
│   ├── cli.rs              # clap argument parsing
│   ├── todo.rs             # todo model and serialization
│   ├── store.rs            # filesystem operations
│   ├── context.rs          # context hierarchy logic
│   └── display.rs          # terminal output formatting
├── agent/                  # Python AI agent (runs on Pi)
│   ├── agent.py
│   ├── config.py
│   ├── notifier.py
│   ├── reasoning.py
│   └── requirements.txt
└── addon/                  # HA local add-on packaging for the agent (Pi runs HAOS)
    ├── config.yaml         # add-on manifest (schema for API key env, schedule, etc.)
    ├── Dockerfile          # bundles agent + todai musl binary
    └── run.sh              # loop/cron entrypoint (replaces systemd timer)
```
