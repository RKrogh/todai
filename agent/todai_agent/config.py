"""Reads .todai/config.toml from the todai store root."""

from __future__ import annotations

import os
import tomllib
from dataclasses import dataclass, field
from pathlib import Path


def resolve_root(override: Path | None = None) -> Path:
    if override is not None:
        return override
    env = os.environ.get("TODAI_HOME")
    if env:
        return Path(env)
    return Path.home() / ".todai"


@dataclass
class AiConfig:
    provider: str = "anthropic"
    model: str = "claude-sonnet-4-6"
    api_key_env: str = "ANTHROPIC_API_KEY"
    exclude_contexts: list[str] = field(default_factory=list)


@dataclass
class NotificationsConfig:
    method: str = "homeassistant"
    homeassistant_url: str = "http://homeassistant.local:8123"
    homeassistant_token_env: str = "HA_TOKEN"
    device_name: str = "robert_phone"
    default_notify_mode: str = "smart"


@dataclass
class LoggingConfig:
    path: str = ".todai/logs/agent.log"
    retain_runs: int = 100
    retain_days: int = 14


@dataclass
class Config:
    ai: AiConfig = field(default_factory=AiConfig)
    notifications: NotificationsConfig = field(default_factory=NotificationsConfig)
    logging: LoggingConfig = field(default_factory=LoggingConfig)


def load(root: Path) -> Config:
    path = root / ".todai" / "config.toml"
    if not path.exists():
        return Config()
    with path.open("rb") as fh:
        raw = tomllib.load(fh)
    return _from_dict(raw)


def _from_dict(raw: dict) -> Config:
    ai_raw = raw.get("ai", {})
    notif_raw = raw.get("notifications", {})
    log_raw = raw.get("logging", {})
    return Config(
        ai=AiConfig(
            provider=ai_raw.get("provider", "anthropic"),
            model=ai_raw.get("model", "claude-sonnet-4-6"),
            api_key_env=ai_raw.get("api_key_env", "ANTHROPIC_API_KEY"),
            exclude_contexts=list(ai_raw.get("exclude_contexts", [])),
        ),
        notifications=NotificationsConfig(
            method=notif_raw.get("method", "homeassistant"),
            homeassistant_url=notif_raw.get(
                "homeassistant_url", "http://homeassistant.local:8123"
            ),
            homeassistant_token_env=notif_raw.get("homeassistant_token_env", "HA_TOKEN"),
            device_name=notif_raw.get("device_name", "robert_phone"),
            default_notify_mode=notif_raw.get("default_notify_mode", "smart"),
        ),
        logging=LoggingConfig(
            path=log_raw.get("path", ".todai/logs/agent.log"),
            retain_runs=log_raw.get("retain_runs", 100),
            retain_days=log_raw.get("retain_days", 14),
        ),
    )
