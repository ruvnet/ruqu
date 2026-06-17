# ADR-001: metaharness-generated agent-harness CLI for ruqu

- **Status**: accepted
- **Date**: 2026-06-17

## Context

`ruqu` is a pure-Rust quantum circuit simulator plus a min-cut coherence
engine (the ruqu crates), extracted standalone from `ruvnet/ruvector` per
ADR-257. The Rust crates build and ship on their own. We additionally want an
agent-facing entry point: a CLI that an agent host (Claude Code) can drive to
boot a kernel, verify its environment, and run harness commands — without
re-implementing that scaffolding by hand for every crate.

rUv's `npx metaharness` generator produces exactly this: a non-interactive
scaffold of a CLI harness (`bin/cli.js` loading `@metaharness/kernel`, a host
adapter, `package.json`, `CLAUDE.md`, `src/init.ts`, vitest smoke tests, and a
signed `.harness/manifest.json`). The kernel is cross-platform with a
`native | wasm | js` backend chain and a `doctor` command that verifies it.

## Decision

Ship a metaharness-generated agent-harness CLI for `ruqu`, configured to use
the kernel's WASM backend (with native and pure-JS fallbacks).

- **Location**: the CLI lives in `cli/`, deliberately kept out of the repo root
  so it does not clash with the Rust Cargo workspace. The Rust crates are
  untouched — this change is purely additive.
- **Generator**: `npx metaharness ruqu --target cli --template vertical:exotic
  --host claude-code --description "..." --force`. It produced 17 files.
- **Commands**: `init` (boot the kernel + host adapter, report status) and
  `doctor` (verify the install end-to-end: kernel loads, reports a version,
  backend is one of native/wasm/js, host adapter resolves).
- **Dependencies**: `@metaharness/kernel` and `@metaharness/host-claude-code`,
  both beta `^0.1.0`.
- **Backend selection**: the kernel resolves `native > wasm > js`. WASM is the
  intended primary; the pure-JS backend is the always-available floor so the
  harness is never dead on arrival.

## Consequences

- The kernel is a beta `0.1.x` package; its API surface and backend packaging
  may change. We pin `^0.1.0` and accept that churn until it stabilizes.
- The native and WASM backends are effectively **optional**: the NAPI
  per-platform packages are optional deps, and the WASM `pkg/` artifact is
  produced by a separate CI job and is absent from the published 0.1.0 beta.
  When neither is present, the kernel transparently falls back to the pure-JS
  backend. `doctor` still passes (the JS backend mirrors the Rust kernel's
  `mcpValidate` rules byte-for-byte), so installs succeed everywhere.
- This is **additive** to the Rust crates: no Cargo manifests, sources, or the
  workspace layout change. `cli/node_modules` and `cli/dist` are gitignored.
- Agents (via Claude Code) get a stable, signed harness entry point to ruqu
  without bespoke per-crate plumbing.
