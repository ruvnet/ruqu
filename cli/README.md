# ruqu

ruqu quantum CLI — agent harness over the quantum-sim + coherence crates

> **Exotic / Self-Evolving** — Hypothesizer → experimenter → federator over a witness-signed evolution log (ADR-014).
>
> Generated with [`create-agent-harness`](https://github.com/ruvnet/agent-harness-generator). WASM kernel, multi-host support, witness-signed releases.

## Install

```bash
npm install -g ruqu
ruqu init
ruqu doctor
```

## Agents

| Agent | Role |
|---|---|
| `hypothesizer` | Proposes a falsifiable self-improvement. |
| `experimenter` | Tests the hypothesis safely and records it. |
| `federator` | Shares vetted improvements across instances. |

This harness ships with the **claude-code** adapter.

## License

MIT
