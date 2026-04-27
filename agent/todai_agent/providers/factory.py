"""Build an LlmProvider from config. Add new providers here."""

from __future__ import annotations

from .base import LlmProvider, ProviderError


def build_provider(provider_name: str, model: str, api_key_env: str) -> LlmProvider:
    name = provider_name.strip().lower()
    if name == "anthropic":
        from .anthropic_provider import AnthropicProvider
        return AnthropicProvider(model=model, api_key_env=api_key_env)
    if name in {"openai", "ollama"}:
        raise ProviderError(
            f"provider '{name}' is reserved but not implemented yet; "
            "implement providers/{name}_provider.py and register here"
        )
    raise ProviderError(f"unknown provider '{provider_name}'")
