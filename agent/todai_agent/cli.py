"""Subprocess wrapper around the `todai` binary.

The CLI is the single source of truth for reading and writing todo files.
This module never parses markdown directly; it always shells out.
"""

from __future__ import annotations

import json
import logging
import shutil
import subprocess
from pathlib import Path
from typing import Any

log = logging.getLogger(__name__)


class TodaiCliError(RuntimeError):
    pass


class TodaiCli:
    def __init__(self, root: Path, binary: str = "todai") -> None:
        self.root = root
        self.binary = binary
        if shutil.which(binary) is None:
            log.warning("`%s` binary not on PATH; CLI calls will fail until installed", binary)

    def list_all(self) -> list[dict[str, Any]]:
        out = self._run(["list", "--all", "--json"])
        return json.loads(out) if out.strip() else []

    def show(self, todo_id: str) -> dict[str, Any]:
        out = self._run(["show", todo_id, "--json"])
        return json.loads(out)

    def mark_notified(self, todo_id: str, reminder_index: int) -> None:
        self._run(["notified", todo_id, "--reminder", str(reminder_index)])

    def snooze(self, todo_id: str, duration: str) -> None:
        self._run(["snooze", todo_id, "--for", duration])

    def done(self, todo_id: str) -> None:
        self._run(["done", todo_id])

    def _run(self, args: list[str]) -> str:
        cmd = [self.binary, "--path", str(self.root), *args]
        log.debug("running %s", " ".join(cmd))
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            raise TodaiCliError(
                f"todai {' '.join(args)} failed (exit {result.returncode}): "
                f"{result.stderr.strip() or result.stdout.strip()}"
            )
        return result.stdout
