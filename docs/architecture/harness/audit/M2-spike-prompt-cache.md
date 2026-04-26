# M2 Spike Prompt Cache

Status: mock validation in place. Live validation is manual-only.

## Scope

This spike covers Anthropic prompt cache behavior for the L1-A model provider.

The supported style remains `AnthropicCacheMode::SystemAnd3`. The provider injects `cache_control: {"type":"ephemeral"}` into the system prompt and selected message content blocks when explicit cache breakpoints are present.

## Mock Coverage

`crates/octopus-harness-model/tests/spike_prompt_cache.rs` verifies:

- cache breakpoint injection for system and message content blocks
- `cache_creation_input_tokens` mapping to `UsageSnapshot.cache_write_tokens`
- `cache_read_input_tokens` mapping to `UsageSnapshot.cache_read_tokens`

## Live Coverage

The live test is ignored by default and must not run in CI or normal verification.

Manual command:

```bash
ANTHROPIC_API_KEY=... cargo test -p octopus-harness-model --features anthropic --test spike_prompt_cache -- --ignored
```

The manual test sends three Anthropic requests with a stable cached anchor. A live run is considered useful when a later request reports `cache_read_tokens > 0`.

## Notes

The spike does not add Anthropic beta prompt-cache headers. It validates the default ephemeral cache path only.
