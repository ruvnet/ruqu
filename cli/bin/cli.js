#!/usr/bin/env node
// SPDX-License-Identifier: MIT
// The `ruqu` CLI — an agent harness over the metaharness kernel PLUS real
// quantum-circuit execution via the bundled ruqu-wasm WebAssembly module.
//
// Plain ESM JS: runs as-is via `npx @ruvector/ruqu` with no build step.

import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HARNESS_NAME = 'ruqu';
const require = createRequire(import.meta.url);
const HERE = dirname(fileURLToPath(import.meta.url));
async function harness() {
  const [{loadKernel}, {default:adapter}] = await Promise.all([
    import('@metaharness/kernel'), import('@metaharness/host-claude-code'),
  ]);
  return {loadKernel,adapter};
}

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
  const {loadKernel,adapter} = await harness();
  const kernel = await loadKernel();
  const info = kernel.kernelInfo();
  console.log(`${HARNESS_NAME} — kernel ${info.version} (${kernel.backend}) · host ${adapter.name}`);
  console.log(`Quantum WASM: ${quantum().max_qubits()} qubits max. Try \`${HARNESS_NAME} capabilities\`.`);
  return 0;
}

/** `ruqu doctor` — verify the harness kernel AND the quantum WASM end-to-end. */
async function doctor() {
  const {loadKernel,adapter} = await harness();
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
  console.log(`  commands          init · doctor · capabilities · simulate · grover · qaoa · observer-lab · magic-factory · research · research-memory · version`);
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
  const {loadKernel} = await harness();
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
    case 'research-memory': {
      const memory = require(join(HERE, '..', 'src', 'research-memory.cjs'));
      console.log(JSON.stringify(await memory.runCLI(rest), null, 2));
      return 0;
    }
    case 'research': {
      // Read at most 1 MiB; reject malformed argument combinations and oversized
      // files before allocating an unbounded input string.
      if (!(rest.length === 0 || rest.length === 2 && ['--input','--json'].includes(rest[0]))) throw Error('Usage: research [--input file.json | --json JSON] (or JSON on stdin)');
      let text;
      if (rest[0] === '--json') {
        text = rest[1];
      } else if (rest.length === 0) {
        const chunks=[];let size=0;
        for await (const chunk of process.stdin) {size+=chunk.length;if(size>1048576)throw Error('Research request exceeds 1 MiB');chunks.push(chunk);}
        text=Buffer.concat(chunks,size).toString('utf8');
      } else {
        const {open} = await import('node:fs/promises');
        const handle = await open(rest[1], 'r');
        try {
          const bytes=Buffer.alloc(1048577);let offset=0;
          while(offset<bytes.length) {const {bytesRead}=await handle.read(bytes,offset,bytes.length-offset,null);if(!bytesRead)break;offset+=bytesRead;}
          if(offset>1048576)throw Error('Research request exceeds 1 MiB');
          text=bytes.subarray(0,offset).toString('utf8');
        } finally { await handle.close(); }
      }
      if (Buffer.byteLength(text)>1048576) throw Error('Research request exceeds 1 MiB');
      // Pass original text to Rust so duplicate JSON fields remain detectable.
      const result=quantum().research_run_json(text);
      console.log(JSON.stringify(JSON.parse(result), null, 2));
      return 0;
    }
    case 'magic-factory': {
      const factory = require(join(HERE, '..', 'src', 'magic-factory.cjs'));
      const options = factory.parse(rest);
      const result = options.research ? factory.research() : options.benchmark ? factory.benchmark() : factory.run(options);
      console.log(JSON.stringify(result, null, 2));
      return result.pass === false ? 1 : 0;
    }
    case 'observer-lab': {
      const lab = require(join(HERE, '..', 'src', 'observer-lab.cjs'));
      if(rest.length===1 && rest[0]==='--benchmark') {
        console.log(JSON.stringify(lab.benchmark(),null,2)); return 0;
      }
      const options={};
      for(let i=0;i<rest.length;i+=2) {
        const key=rest[i];
        if(!['--count','--seed'].includes(key) || options[key.slice(2)]!==undefined ||
           !/^[0-9]+$/.test(rest[i+1] ?? '')) throw Error('Usage: observer-lab [--count N --seed N] | --benchmark');
        options[key.slice(2)]=Number(rest[i+1]);
      }
      console.log(JSON.stringify(lab.run(options),null,2)); return 0;
    }
    case 'ai-plan': case 'ai-evaluate': case 'ai-harness': {
      const ai=await import('../src/cognitum.mjs');
      if(cmd==='ai-harness'){if(rest.length)throw Error('Usage: ai-harness');console.log(JSON.stringify(await ai.harnessSpec(),null,2));return 0;}
      const options={};const allowed=cmd==='ai-plan'?['--goal','--model']:['--input'];
      for(let i=0;i<rest.length;i+=2){if(!allowed.includes(rest[i])||options[rest[i]]!==undefined||rest[i+1]===undefined)throw Error('Invalid AI command options');options[rest[i]]=rest[i+1];}
      let result;
      if(cmd==='ai-plan')result=await ai.propose(options['--goal'],options['--model']);
      else{const {open}=await import('node:fs/promises');const f=await open(options['--input'],'r');let bytes;try{const b=Buffer.alloc(65537);let offset=0;while(offset<b.length){const r=await f.read(b,offset,b.length-offset,null);if(!r.bytesRead)break;offset+=r.bytesRead;}if(offset>65536)throw Error('Plan exceeds 64 KiB');bytes=b.subarray(0,offset);}finally{await f.close();}result=await ai.evaluate(JSON.parse(bytes));}
      console.log(JSON.stringify(result,null,2));return 0;
    }
    case 'version': case '--version': return version();
    case 'help': case '--help':
      console.log('Cognitum: ai-plan --goal TEXT --model MODEL_ID | ai-evaluate --input PLAN.json | ai-harness');
      console.log('Observer laboratory: ruqu observer-lab [--count N --seed N] | --benchmark');
      console.log('Research API: ruqu research [--input file.json | --json JSON] (or JSON stdin)');
      console.log('Research memory: ruqu research-memory export|import|query|recommend --input FILE [--output FILE | --target FILE]');
      console.log('Magic state factory: ruqu magic-factory [--input-error P --shots N --seed N --rounds N] | --research | --benchmark');
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
  run(argv.slice(2)).then((code) => { process.exitCode = code; }).catch((e) => { console.error(e); process.exitCode = 1; });
}
