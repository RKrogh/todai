"""Build a Notifier from config."""

from __future__ import annotations

from ..config import NotificationsConfig
from .base import Notifier, NotifierError


def build_notifier(cfg: NotificationsConfig, dry_run: bool = False) -> Notifier:
    if dry_run:
        from .print_notifier import PrintNotifier
        return PrintNotifier()
    method = cfg.method.strip().lower()
    if method == "homeassistant":
        from .home_assistant import HomeAssistantNotifier
        return HomeAssistantNotifier(
            base_url=cfg.homeassistant_url,
            device_name=cfg.device_name,
            token_env=cfg.homeassistant_token_env,
        )
    if method == "print":
        from .print_notifier import PrintNotifier
        return PrintNotifier()
    if method == "none":
        from .print_notifier import PrintNotifier
        return PrintNotifier()
    raise NotifierError(f"unknown notifier method '{cfg.method}'")
