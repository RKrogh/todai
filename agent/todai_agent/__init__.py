"""todai agent: provider-agnostic AI loop that reads todos via the todai CLI
and sends contextual notifications.

Architecture:
- `cli.TodaiCli` shells out to the `todai` binary for all reads and writes.
- `providers.LlmProvider` is the vendor-neutral interface; Anthropic is the
  default implementation. Switch via `[ai].provider` in .todai/config.toml.
- `notifiers.Notifier` is the delivery interface; Home Assistant Companion
  app is the default. `PrintNotifier` is for dev / dry-run.
- Secrets (API keys, HA tokens) are read from env vars at runtime, never
  from synced files.
"""

__version__ = "0.1.0"
