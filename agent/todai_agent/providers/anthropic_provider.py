"""Anthropic implementation of LlmProvider.

The api key is never read from synced files. It is taken from the env var
named in `[ai].api_key_env` in .todai/config.toml.
"""

from __future__ import annotations

import os

from .base import LlmProvider, LlmRequest, LlmResponse, ProviderError


class AnthropicProvider:
    name = "anthropic"

    def __init__(self, model: str, api_key_env: str) -> None:
        self.model = model
        self.api_key_env = api_key_env
        api_key = os.environ.get(api_key_env)
        if not api_key:
            raise ProviderError(
                f"env var {api_key_env} is not set; cannot initialize Anthropic provider"
            )
        try:
            import anthropic
        except ImportError as exc:
            raise ProviderError(
                "anthropic package not installed; run `pip install anthropic`"
            ) from exc
        self._client = anthropic.Anthropic(api_key=api_key)

    def reason(self, request: LlmRequest) -> LlmResponse:
        try:
            msg = self._client.messages.create(
                model=self.model,
                system=request.system,
                messages=[{"role": "user", "content": request.user}],
                max_tokens=request.max_tokens,
                temperature=request.temperature,
            )
        except Exception as exc:
            raise ProviderError(f"anthropic call failed: {exc}") from exc
        text_parts: list[str] = []
        for block in getattr(msg, "content", []) or []:
            if getattr(block, "type", None) == "text":
                text_parts.append(getattr(block, "text", ""))
        text = "\n".join(p for p in text_parts if p).strip()
        usage = getattr(msg, "usage", None)
        return LlmResponse(
            text=text,
            model=self.model,
            input_tokens=getattr(usage, "input_tokens", 0) if usage else 0,
            output_tokens=getattr(usage, "output_tokens", 0) if usage else 0,
        )
