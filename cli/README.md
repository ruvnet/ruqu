# ruqu

**Agent-harness CLI for the [ruqu](https://github.com/ruvnet/ruqu) quantum project.** Boots the
[metaharness](https://github.com/ruvnet/agent-harness-generator) kernel + a Claude Code host
adapter, with a self-evolving agent loop (hypothesizer → experimenter → federator) over a
witness-signed evolution log.

```bash
npx @ruvector/ruqu init      # boot the kernel + host adapter
npx @ruvector/ruqu doctor    # verify the install end-to-end
```

Or install globally:

```bash
npm install -g @ruvector/ruqu
ruqu doctor
```

## Agents

| Agent | Role |
|---|---|
| `hypothesizer` | Proposes a falsifiable self-improvement. |
| `experimenter` | Tests the hypothesis safely and records it. |
| `federator` | Shares vetted improvements across instances. |

Ships with the **claude-code** host adapter.

## Kernel backend

The harness loads `@metaharness/kernel`, which resolves a backend in order **native → wasm → js**.
The published `@metaharness/kernel@0.1.0` beta ships only the **pure-JS** floor backend; the native
(NAPI) and WASM artifacts are produced by separate upstream build jobs and are not in the npm
package yet, so `ruqu doctor` currently reports the **`js`** backend. It will pick up native/WASM
automatically once those kernel artifacts are published — no change needed here.

## License

MIT © Ruvector Team
