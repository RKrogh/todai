"""Notification delivery abstraction. HA today, ntfy/email/print later."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Protocol


class NotifierError(RuntimeError):
    pass


@dataclass(frozen=True)
class Notification:
    todo_id: str
    title: str
    body: str
    priority: str = "normal"
    actions: list[dict[str, str]] = field(default_factory=list)


class Notifier(Protocol):
    name: str

    def send(self, notification: Notification) -> None:
        ...
