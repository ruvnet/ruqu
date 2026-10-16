# Cognitum MetaLLM and MetaHarness integration

Cognitum proposes a quantum circuit from a plain language goal. ruqu validates the proposal and computes exact statevector probabilities using its bundled Rust WASM. MetaHarness supplies the host configuration and kernel identity. The existing kernel is not represented as an autonomous optimizer or security sandbox.

## Run from this checkout

```sh
cd cli
npm ci
node bin/cli.js ai-harness > ruqu-harness.json
# Supply COGNITUM_API_KEY securely in your host environment.
node bin/cli.js ai-plan --goal 'Prepare a Bell pair' --model MODEL_ID > proposal.json
node bin/cli.js ai-evaluate --input plan.json
```

`MODEL_ID` must be selected from your live Cognitum `/v1/models` catalog. Each planning call checks this catalog before making one nonstreaming completion request, capped at 1,200 output tokens. The endpoint is fixed to `https://api.cognitum.one/v1`; redirects are rejected. No retries, shell execution, arbitrary tools, cloud quantum submission, or global ruOS routing changes occur.

`ai-evaluate` takes a raw plan with exactly `qubits`, `gates`, and `explanation`. Plans contain 2 to 8 qubits and at most 64 H/X/Z/S/T/CNOT gates. Example:

```json
{"qubits":2,"gates":[{"name":"h","qubit":0},{"name":"cnot","qubit":0,"target":1}],"explanation":"Bell preparation."}
```

Planning produces a receipt containing the proposal, canonical circuit field hash, actual kernel backend, measured probabilities, request latency and provider token usage when returned. Evaluation does not establish whether the circuit meets the user's objective, whether the explanation is correct, or whether hardware will reproduce the result. No automatic self improvement or quantum advantage is claimed.

## Explorer workflow

Use **Import AI proposal JSON** in the Explorer to load either the raw plan or the planning receipt. It loads gates and displays the explanation as plain text; it does not execute a job. Press Run circuit to compute probabilities locally. Imported claims of verification are never trusted. Export your reviewed circuit to the cloud bridge separately if needed.

## MetaHarness host configuration

`ai-harness` returns the actual host adapter's generated files as a JSON map, together with the harness specification and kernel backend. Review the map before installing files into a host. Tool descriptions document CLI entrypoints; they are not an MCP server registration. The generated host deny rules are advisory to that host, not an OS sandbox. Actual plan validation is enforced inside ruqu independently of host permissions.

## Validation and current limits

The integration tests use the real MetaHarness kernel/host adapter and real ruqu WASM, with a mocked HTTP transport for catalog/completion failures and success. They cover invalid gates, indexes, oversized output, missing credentials, unknown models, redirect policy, token bounds and secret omission. The Cognitum connector's live model catalog was verified on 2026-10-16. ruOS currently reports no Cognitum `llm` key source, so live completion remains unverified. Existing Anthropic and other provider keys are not substituted or repurposed.

These commands are available in this repository checkout. The previously published npm 0.3.0 package does not include this addition until a new npm release is published.
