#!/usr/bin/env node
// SPDX-License-Identifier: MIT
// The `ruqu` CLI — an agent harness over the metaharness kernel PLUS real
// quantum-circuit execution via the bundled ruqu-wasm WebAssembly module.
//
// Plain ESM JS: runs as-is via `npx @ruvector/ruqu` with no build step.

import { loadKernel } from '@metaharness/kernel';
import adapter from '@metaharness/host-claude-code';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HARNESS_NAME = 'ruqu';
const require = createRequire(import.meta.url);
const HERE = dirname(fileURLToPath(import.meta.url));

/** Lazily load the bundled nodejs-target quantum WASM (ruqu-wasm crate). */
let _wasm;
function quantum() {
  if (!_wasm) _wasm = require(join(HERE, '..', 'wasm', 'ruqu_wasm.js'));
  return _wasm;
}

function arg(args, flag, def) {
  const i = args.indexOf(flag);
  return i >= 0 && args[i + 1] !== undefined ? args[i + 1] : def;
}
function intArg(args, flag, def) {
  const v = arg(args, flag, undefined);
  return v === undefined ? def : parseInt(v, 10);
}
/** Top-k computational basis states by probability, as [bitstring, p]. */
function topStates(probs, n, k = 8) {
  return probs
    .map((p, i) => [i.toString(2).padStart(n, '0'), p])
    .filter(([, p]) => p > 1e-9)
    .sort((a, b) => b[1] - a[1])
    .slice(0, k);
}

/** `ruqu init` — boot the agent-harness kernel + host adapter. */
async function init() {
  const kernel = await loadKernel();
  const info = kernel.kernelInfo();
  console.log(`${HARNESS_NAME} — kernel ${info.version} (${kernel.backend}) · host ${adapter.name}`);
  console.log(`Quantum WASM: ${quantum().max_qubits()} qubits max. Try \`${HARNESS_NAME} capabilities\`.`);
  return 0;
}

/** `ruqu doctor` — verify the harness kernel AND the quantum WASM end-to-end. */
async function doctor() {
  const kernel = await loadKernel();
  const info = kernel.kernelInfo();
  let maxq = 0;
  try { maxq = quantum().max_qubits(); } catch { maxq = 0; }
  const checks = [
    ['metaharness kernel loads', !!kernel],
    ['kernel reports a version', typeof info.version === 'string' && info.version.length > 0],
    ['host adapter has a name', typeof adapter?.name === 'string' && adapter.name.length > 0],
    ['quantum WASM loads (ruqu-wasm)', maxq > 0],
    [`quantum WASM runs a Bell state`, runsBell()],
  ];
  let ok = true;
  for (const [label, pass] of checks) { console.log(`${pass ? 'PASS' : 'FAIL'} ${label}`); if (!pass) ok = false; }
  console.log(ok
    ? `\n${HARNESS_NAME}: all checks passed (kernel ${info.version} ${kernel.backend}; quantum WASM ${maxq} qubits)`
    : `\n${HARNESS_NAME}: doctor found problems`);
  return ok ? 0 : 1;
}
function runsBell() {
  try {
    const q = quantum();
    const c = new q.WasmQuantumCircuit(2); c.h(0); c.cnot(0, 1);
    const r = q.simulate(c);
    return Math.abs(r.probabilities[0] - 0.5) < 1e-6 && Math.abs(r.probabilities[3] - 0.5) < 1e-6;
  } catch { return false; }
}

/** `ruqu capabilities` — list the quantum capabilities this CLI exposes. */
function capabilities() {
  const q = quantum();
  const mem = q.estimate_memory(q.max_qubits());
  console.log(`${HARNESS_NAME} — quantum capabilities (pure-Rust → WebAssembly)`);
  console.log(`  max qubits        ${q.max_qubits()}`);
  console.log(`  state @ max       ~${(mem / 1e6).toFixed(1)} MB`);
  console.log(`  gates             h x y z s t rx ry rz cnot cz swap rzz measure reset barrier`);
  console.log(`  algorithms        simulate · grover · qaoa`);
  console.log(`  commands          init · doctor · capabilities · simulate · grover · qaoa · version`);
  return 0;
}

/** `ruqu simulate [--qubits N]` — run a GHZ/Bell circuit and show the distribution. */
function simulate(args) {
  const q = quantum();
  const n = intArg(args, '--qubits', 2);
  const c = new q.WasmQuantumCircuit(n);
  c.h(0);
  for (let i = 1; i < n; i++) c.cnot(0, i);
  const r = q.simulate(c);
  console.log(`${HARNESS_NAME} simulate — ${n}-qubit GHZ · ${r.gate_count} gates · ${r.execution_time_ms.toFixed(2)} ms`);
  for (const [bits, p] of topStates(r.probabilities, n)) console.log(`  |${bits}⟩  ${(p * 100).toFixed(1)}%`);
  return 0;
}

/** `ruqu grover [--qubits N] [--target T] [--seed S]` — amplitude amplification. */
function grover(args) {
  const q = quantum();
  const n = intArg(args, '--qubits', 3);
  const target = intArg(args, '--target', (1 << n) - 1);
  const seed = args.includes('--seed') ? intArg(args, '--seed', 0) : null;
  const r = q.grover_search(n, [target], seed);
  console.log(`${HARNESS_NAME} grover — ${n} qubits, target=${target}`);
  console.log(`  iterations         ${r.num_iterations}`);
  console.log(`  success probability ${(r.success_probability * 100).toFixed(1)}%`);
  console.log(`  measured state      ${r.measured_state}`);
  return 0;
}

/** `ruqu qaoa [--nodes N] [--p P]` — QAOA MaxCut on a ring graph. */
function qaoa(args) {
  const q = quantum();
  const nodes = intArg(args, '--nodes', 4);
  const edges = [];
  for (let i = 0; i < nodes; i++) { edges.push(i, (i + 1) % nodes); }
  const p = intArg(args, '--p', 1);
  const gammas = Array.from({ length: p }, () => 0.8);
  const betas = Array.from({ length: p }, () => 0.6);
  const r = q.qaoa_maxcut(nodes, edges, p, gammas, betas, null);
  console.log(`${HARNESS_NAME} qaoa — ${nodes}-node ring MaxCut, p=${p}`);
  for (const [bits, prob] of topStates(r.probabilities, nodes, 5)) console.log(`  cut ${bits}  ${(prob * 100).toFixed(1)}%`);
  return 0;
}

async function version() {
  const k = await loadKernel();
  console.log(`${HARNESS_NAME} CLI — metaharness kernel ${k.kernelInfo().version} (${k.backend}) · quantum WASM ${quantum().max_qubits()}q`);
  return 0;
}

export async function run(argv) {
  const [cmd, ...rest] = argv;
  switch (cmd) {
    case undefined:
    case 'init': return init();
    case 'doctor': return doctor();
    case 'capabilities': case 'caps': return capabilities();
    case 'simulate': case 'sim': return simulate(rest);
    case 'grover': return grover(rest);
    case 'qaoa': return qaoa(rest);
    case 'version': case '--version': return version();
    case 'help': case '--help':
      console.log(`Usage: ${HARNESS_NAME} <command>\n\n  init           boot the kernel + host adapter (default)\n  doctor         verify kernel + quantum WASM end-to-end\n  capabilities   list quantum capabilities\n  simulate       run a GHZ/Bell circuit  [--qubits N]\n  grover         Grover search           [--qubits N --target T --seed S]\n  qaoa           QAOA MaxCut on a ring    [--nodes N --p P]\n  version        print versions`);
      return 0;
    default:
      console.error(`Unknown command: ${cmd}. Try \`${HARNESS_NAME} --help\`.`);
      return 1;
  }
}

// Execute only when invoked directly (not when imported by a test).
import { realpathSync } from 'node:fs';
import { argv } from 'node:process';
const invokedDirectly = (() => {
  try { return realpathSync(fileURLToPath(import.meta.url)) === realpathSync(argv[1]); }
  catch { return false; }
})();
if (invokedDirectly) {
  run(argv.slice(2)).then((code) => process.exit(code)).catch((e) => { console.error(e); process.exit(1); });
}
