"""Home Assistant Companion app notifier.

The HA token is read from the env var named in `[notifications].homeassistant_token_env`,
never from a synced file.
"""

from __future__ import annotations

import os

import httpx

from .base import Notification, NotifierError


class HomeAssistantNotifier:
    name = "homeassistant"

    def __init__(
        self,
        base_url: str,
        device_name: str,
        token_env: str,
        timeout_seconds: float = 10.0,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.device_name = device_name
        self.token_env = token_env
        self.timeout = timeout_seconds
        token = os.environ.get(token_env)
        if not token:
            raise NotifierError(f"env var {token_env} is not set")
        self._token = token

    def send(self, notification: Notification) -> None:
        url = f"{self.base_url}/api/services/notify/mobile_app_{self.device_name}"
        priority_map = {"urgent": "high", "high": "high", "normal": "normal", "low": "low"}
        payload = {
            "message": notification.body,
            "title": notification.title,
            "data": {
                "priority": priority_map.get(notification.priority, "normal"),
                "channel": "todai",
                "tag": f"todai:{notification.todo_id}",
            },
        }
        if notification.actions:
            payload["data"]["actions"] = notification.actions
        headers = {
            "Authorization": f"Bearer {self._token}",
            "Content-Type": "application/json",
        }
        try:
            response = httpx.post(url, json=payload, headers=headers, timeout=self.timeout)
            response.raise_for_status()
        except httpx.HTTPError as exc:
            raise NotifierError(f"HA notify failed: {exc}") from exc
