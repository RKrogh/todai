"""Main agent loop. Pulls todos via the CLI, decides what fires, sends, writes back."""

from __future__ import annotations

import logging
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .cli import TodaiCli
from .config import Config
from .notifiers import Notifier, Notification, NotifierError, build_notifier
from .prompts import SYSTEM_PROMPT, build_user_prompt
from .providers import LlmProvider, LlmRequest, ProviderError, build_provider

log = logging.getLogger(__name__)


class AgentRunSummary:
    def __init__(self) -> None:
        self.scanned: int = 0
        self.fired: int = 0
        self.smart: int = 0
        self.dumb: int = 0
        self.errors: list[str] = []


def run_once(
    root: Path,
    config: Config,
    *,
    dry_run: bool = False,
    binary: str = "todai",
) -> AgentRunSummary:
    cli = TodaiCli(root, binary=binary)
    notifier = build_notifier(config.notifications, dry_run=dry_run)

    provider: LlmProvider | None
    try:
        provider = build_provider(
            config.ai.provider,
            model=config.ai.model,
            api_key_env=config.ai.api_key_env,
        )
    except ProviderError as exc:
        log.warning("LLM provider unavailable, all notifications will use dumb mode: %s", exc)
        provider = None

    summary = AgentRunSummary()
    items = cli.list_all()
    summary.scanned = len(items)
    now_utc = datetime.now(timezone.utc)

    for todo in items:
        if todo.get("status") not in {"pending", "in_progress"}:
            continue
        for idx, reminder in enumerate(todo.get("remind") or []):
            if not _is_due(reminder, now_utc):
                continue
            mode = _resolve_mode(todo, reminder, config)
            body = _compose_body(todo, reminder, provider, mode, summary)
            notification = Notification(
                todo_id=todo["id"],
                title=f"todai: {todo.get('context', 'inbox')}",
                body=body,
                priority=str(todo.get("priority", "normal")),
                actions=[
                    {"action": "TODAI_DONE", "title": "Done"},
                    {"action": "TODAI_SNOOZE_1H", "title": "Snooze 1h"},
                    {"action": "TODAI_SNOOZE_1D", "title": "Tomorrow"},
                ],
            )
            try:
                notifier.send(notification)
                summary.fired += 1
            except NotifierError as exc:
                summary.errors.append(f"notify {todo['id']}#{idx}: {exc}")
                log.error("notify failed for %s reminder %s: %s", todo["id"], idx, exc)
                continue
            if not dry_run:
                try:
                    cli.mark_notified(todo["id"], idx)
                except Exception as exc:
                    summary.errors.append(f"mark_notified {todo['id']}#{idx}: {exc}")
                    log.error("mark_notified failed for %s reminder %s: %s", todo["id"], idx, exc)
    return summary


def _is_due(reminder: dict[str, Any], now_utc: datetime) -> bool:
    if reminder.get("notified_at"):
        return False
    at_str = reminder.get("at")
    if not at_str:
        return False
    try:
        at_dt = datetime.fromisoformat(at_str.replace("Z", "+00:00"))
    except ValueError:
        return False
    if at_dt.tzinfo is None:
        at_dt = at_dt.replace(tzinfo=timezone.utc)
    return at_dt <= now_utc


def _resolve_mode(todo: dict[str, Any], reminder: dict[str, Any], config: Config) -> str:
    ctx = str(todo.get("context", ""))
    if ctx and ctx in config.ai.exclude_contexts:
        return "dumb"
    return reminder.get("notify_mode") or "smart"


def _compose_body(
    todo: dict[str, Any],
    reminder: dict[str, Any],
    provider: LlmProvider | None,
    mode: str,
    summary: AgentRunSummary,
) -> str:
    fallback = reminder.get("message") or todo.get("title") or "(reminder)"
    if mode == "dumb" or provider is None:
        summary.dumb += 1
        return fallback
    try:
        response = provider.reason(
            LlmRequest(system=SYSTEM_PROMPT, user=build_user_prompt(todo, reminder))
        )
        summary.smart += 1
        return response.text or fallback
    except ProviderError as exc:
        summary.dumb += 1
        summary.errors.append(f"llm {todo.get('id')}: {exc}")
        log.warning("LLM call failed for %s, falling back to dumb: %s", todo.get("id"), exc)
        return fallback
