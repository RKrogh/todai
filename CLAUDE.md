# todai

AI-powered cross-machine todo and reminder system. Local-first, markdown-based, synced via Syncthing.

## Project overview

See `PLAN.md` for full architecture, design decisions, and implementation phases.

**Core idea:** Markdown todos with YAML frontmatter, managed by a Rust CLI, synced across devices
via Syncthing, with a Python AI agent on a Raspberry Pi 5 that reads todos and sends contextual
notifications through Home Assistant.

## Tech stack

- **CLI:** Rust (clap for args, serde for serialization, colored for output)
- **Storage:** Markdown files with YAML frontmatter in a Syncthing-shared folder
- **AI agent:** Python, runs on Raspberry Pi 5, uses Claude API (Sonnet for cost efficiency)
- **Notifications:** Home Assistant Companion app
- **Sync:** Syncthing (not git, not cloud)

## Development notes

- Primary dev environment is WSL (Linux). Target platforms: x86_64-linux and aarch64-linux (Pi).
- Use `cross` for ARM64 compilation: `cross build --target aarch64-unknown-linux-gnu --release`
- Todo data folder is separate from this repo. Default: `~/.todai/` or configured in `.todai/config.toml`
- The AI agent is a separate Python project in `agent/`, not part of the Rust binary.
- Keep the CLI fast and dependency-light. No network calls from the CLI itself.

## Conventions

- File-scoped error handling with `anyhow` for the CLI
- Todo filenames are slugified from the title (e.g., "Book 1-on-1 with Erik" -> `book-1on1-with-erik.md`)
- Contexts use colon-separated hierarchy: `work:staff`, `private:garden`
- Contexts map to folder structure: `work/staff/book-1on1-with-erik.md`

## Current phase

Phase 1 (POC): Basic CLI with `add`, `list`, `done`, `show`. Simple agent script.

## NDA / sensitivity

No sensitive or client-specific information should be stored in todos. This folder syncs
unencrypted across devices. Future phases may add per-context encryption.
