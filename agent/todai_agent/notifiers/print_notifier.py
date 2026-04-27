"""Stdout notifier for development and --dry-run."""

from __future__ import annotations

import sys

from .base import Notification


class PrintNotifier:
    name = "print"

    def send(self, notification: Notification) -> None:
        out = sys.stdout
        out.write(f"[notify:{notification.priority}] {notification.title}\n")
        out.write(f"  todo_id: {notification.todo_id}\n")
        out.write(f"  body:    {notification.body}\n")
        if notification.actions:
            labels = ", ".join(a.get("title", a.get("action", "?")) for a in notification.actions)
            out.write(f"  actions: {labels}\n")
        out.flush()
