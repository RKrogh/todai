"""Provider-neutral prompt builders.

Pure strings, no Anthropic-specific blocks. The provider layer adapts these to
whatever wire format the vendor expects.
"""

from __future__ import annotations

from typing import Any

SYSTEM_PROMPT = """You are todai, a calm, terse assistant who notifies a busy software engineer
about upcoming todos. You receive structured JSON describing a todo and a specific reminder
that is firing now. Write a single push notification body, 1-2 short sentences, in the user's
voice. Be concrete: reference the todo's title and any due date. Do not greet, do not sign off.
If the todo's notes contain context worth surfacing (a person, a place, a deadline), weave it in.
Avoid em dashes; prefer commas and periods.
"""


def build_user_prompt(todo: dict[str, Any], reminder: dict[str, Any]) -> str:
    lines = [
        "Reminder firing now for this todo. Compose the notification body.",
        "",
        "Todo:",
        f"  title:    {todo.get('title')}",
        f"  context:  {todo.get('context')}",
        f"  priority: {todo.get('priority')}",
        f"  due:      {todo.get('due') or '(none)'}",
    ]
    tags = todo.get("tags") or []
    if tags:
        lines.append(f"  tags:     {', '.join(tags)}")
    body = (todo.get("body") or "").strip()
    if body:
        lines.append("  notes:")
        for ln in body.splitlines():
            lines.append(f"    {ln}")
    lines.extend(
        [
            "",
            "Reminder:",
            f"  at:      {reminder.get('at')}",
            f"  message: {reminder.get('message')}",
        ]
    )
    return "\n".join(lines)
