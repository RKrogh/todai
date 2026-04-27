# todai-agent

Python loop that drives the todai AI notifications. Runs on the Raspberry Pi 5
under a systemd timer; can also run locally for development with `--dry-run`.

## Quick start (local dev)

```bash
cd agent
python3 -m venv .venv
source .venv/bin/activate
pip install -e .

# Without ANTHROPIC_API_KEY set, all notifications use dumb (plain message) mode.
# Without HA_TOKEN set, use --dry-run to print to stdout.
todai-agent --dry-run
```

## Architecture

- `cli.py` shells out to the `todai` binary for all reads and writes (no markdown
  parsing in Python).
- `providers/` is the LLM abstraction. `anthropic_provider.py` is the default;
  `openai`, `ollama`, etc. are reserved names with stub errors. Pick via
  `[ai].provider` in `.todai/config.toml`.
- `notifiers/` is the delivery abstraction. `home_assistant.py` is the default;
  `print_notifier.py` is used for `--dry-run` and dev.
- `agent.py` is the main loop. Pull JSON, find due reminders, compose a message
  (smart via LLM, or dumb fallback), notify, mark notified.
- Secrets live only in env vars. The agent reads the env-var name from config
  and looks the value up at runtime.

## Falls back to dumb mode

The agent never silently skips a reminder. If the LLM provider is unavailable
(no key, network error, rate limit), the dumb path uses the reminder's own
`message` field. Per-reminder opt-in: set `notify_mode: dumb` in the todo
frontmatter to bypass the LLM permanently.

## Privacy: exclude_contexts

Contexts listed in `[ai].exclude_contexts` are never sent to the LLM. Their
reminders always go through the dumb path; content stays on the Pi.
