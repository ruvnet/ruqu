# ruqu

ruqu quantum CLI — agent harness over the quantum-sim + coherence crates

> Exotic / Self-Evolving harness · domain: `exotic/self-evolution`. Generated with [create-agent-harness](https://github.com/ruvnet/agent-harness-generator).

## Behavioral rules

- Use the harness's MCP tools (`mcp__ruqu__*`) for orchestration
- Memory and routing are handled by the kernel — you don't need to learn them
- Defer destructive operations to the user

## Agents

| Agent | Tier | Role |
|---|---|---|
| `hypothesizer` | opus | Proposes a falsifiable self-improvement. |
| `experimenter` | opus | Tests the hypothesis safely and records it. |
| `federator` | sonnet | Shares vetted improvements across instances. |
## Skills

- `/memory-inspect` — Search and inspect the harness memory namespace (HNSW + emergent-time decay).
- `/evolve` — Run one safe self-improvement cycle: hypothesize → experiment → record → (maybe) federate.

## Commands

- `doctor` — Health-check the harness: kernel load, MCP wiring, memory backend, host adapter.

## Architecture

This harness uses [@metaharness/kernel](https://www.npmjs.com/package/@metaharness/kernel) — a Rust-compiled WASM module with a NAPI-RS native fallback — so the same code runs identically on every platform.
