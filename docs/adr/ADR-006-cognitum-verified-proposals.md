# ADR 006: Cognitum proposals with local numerical verification

Use Cognitum's fixed HTTPS OpenAI compatible endpoint and runtime model catalog to propose schema restricted circuits. Use the existing MetaHarness kernel and host adapter to generate a reviewable host configuration; never invent kernel optimizer APIs. The model cannot run tools or submit quantum jobs. ruqu validates and simulates plans locally.

The current installed kernel only exposes identity and MCP transport shape validation, so model policy and circuit checks belong to ruqu. A kernel backend of `js` is reported honestly. Host configuration permissions are not a sandbox.

Inputs: 2,000 character goal, explicit catalog model, host API key. Outputs: bounded proposal, circuit hash, exact probabilities, token usage and latency. Remote reads and completion responses are bounded to 64 KiB with 45 second request timeouts, no redirects or automatic retries. The CLI file reader is bounded. Credentials never enter artifacts or the browser. Provider errors expose HTTP status rather than response bodies.

Failure modes: valid circuit can fail the user's scientific goal; explanation remains unverified. A model may output invalid JSON and the request fails closed. Timeout can still incur provider cost; no retry is automatic. The goal is transmitted to Cognitum only when ai-plan is explicitly invoked. No credentials found in the configured ruOS Cognitum slot means live completion is unverified, not silently routed elsewhere.

Acceptance: real kernel adapter generation, Bell WASM probabilities, strict invalid proposal rejection, catalog admission and bounded mock completion tests; full CLI and browser regressions. Rollback: revert this commit; no global routes or provider accounts are changed.
