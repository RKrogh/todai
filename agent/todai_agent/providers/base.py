"""LLM provider abstraction. Swap Anthropic for OpenAI/Ollama via config."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol


class ProviderError(RuntimeError):
    """Raised on any provider-side failure (network, auth, rate limit, parse)."""


@dataclass(frozen=True)
class LlmRequest:
    system: str
    user: str
    max_tokens: int = 512
    temperature: float = 0.4


@dataclass(frozen=True)
class LlmResponse:
    text: str
    model: str
    input_tokens: int = 0
    output_tokens: int = 0


class LlmProvider(Protocol):
    """Vendor-neutral LLM interface. Implementations live in providers/<name>.py."""

    name: str

    def reason(self, request: LlmRequest) -> LlmResponse:
        ...
