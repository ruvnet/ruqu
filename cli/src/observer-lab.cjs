'use strict';
const {createHash} = require('node:crypto');
const {readFileSync} = require('node:fs');
const path = require('node:path');
const wasmPath = path.resolve(__dirname, '../wasm/ruqu_wasm.js');
const {WasmQuantumCircuit, simulate} = require(wasmPath);
const {performance} = require('node:perf_hooks');
const PROTOCOL = 'ruqu.observer.v1';
const MAX_EVENTS = 10000;
const engineSha256 = createHash('sha256').update(readFileSync(path.join(path.dirname(wasmPath),'ruqu_wasm_bg.wasm'))).digest('hex');
function canonical(value) {
  if (Array.isArray(value)) {
    for (let i=0;i<value.length;i++) if (!Object.hasOwn(value,i)) throw Error('Sparse evidence array');
    return '[' + value.map(canonical).join(',') + ']';
  }
  if (value !== null && typeof value === 'object') {
    if (![Object.prototype,null].includes(Object.getPrototypeOf(value))) throw Error('Evidence must be plain JSON');
    return '{' + Object.keys(value).sort().map(k => JSON.stringify(k)+':'+canonical(value[k])).join(',') + '}';
  }
  if (typeof value === 'number' && !Number.isFinite(value)) throw Error('Non-finite evidence');
  const encoded = JSON.stringify(value);
  if (encoded === undefined) throw Error('Unsupported evidence value');
  return encoded;
}
const digest = value => createHash('sha256').update(canonical(value)).digest('hex');
function angle(value) {
  if (!Number.isFinite(value) || Math.abs(value) > 2*Math.PI) throw Error('Angle must be finite and within two pi');
}

function probabilities(build) {
  const circuit = new WasmQuantumCircuit(2);
  try {
    build(circuit);
    const p = simulate(circuit).probabilities;
    if (p.length !== 4 || p.some(x => !Number.isFinite(x) || x < 0) ||
        Math.abs(p.reduce((a,b) => a+b, 0)-1) > 1e-10) throw Error('Invalid probabilities');
    return p;
  } finally { circuit.free(); }
}
function bell(a, b, phaseFlip = false) {
  angle(a); angle(b);
  if (typeof phaseFlip !== 'boolean') throw Error('Invalid phase flag');
  return probabilities(c => {
    c.h(0); c.cnot(0,1);
    if (phaseFlip) c.z(1);
    c.ry(0,-a); c.ry(1,-b);
  });
}
const correlation = p => p[0] + p[3] - p[1] - p[2];
const settings = [[0,Math.PI/4],[0,-Math.PI/4],[Math.PI/2,Math.PI/4],[Math.PI/2,-Math.PI/4]];
function chsh(dephased = false) {
  if (typeof dephased !== 'boolean') throw Error('Invalid dephasing flag');
  const distributions = settings.map(([a,b]) => {
    const p = bell(a,b);
    if (!dephased) return p;
    const flipped = bell(a,b,true);
    return p.map((v,i) => (v+flipped[i])/2);
  });
  const e = distributions.map(correlation);
  return {distributions, correlations:e, S:e[0]+e[1]+e[2]-e[3]};
}
function localBound() {
  const values = [];
  for (const a0 of [-1,1]) for (const a1 of [-1,1])
    for (const b0 of [-1,1]) for (const b1 of [-1,1])
      values.push(a0*b0+a0*b1+a1*b0-a1*b1);
  return Math.max(...values.map(Math.abs));
}
function observer(undo) {
  if (typeof undo !== 'boolean') throw Error('Invalid undo flag');
  // q0 = system; q1 = coherent one-qubit observer, not a conscious agent.
  const p = probabilities(c => {
    c.h(0); c.cnot(0,1);
    if (undo) c.cnot(0,1);
    c.h(0);
  });
  return {probabilities:p, systemZero:p[0]+p[2]};
}
function rng(seed) {
  if (!Number.isInteger(seed) || seed < 1 || seed > 0xffffffff) throw Error('Invalid seed');
  let state = seed >>> 0;
  return () => {
    state ^= state << 13; state ^= state >>> 17; state ^= state << 5;
    return (state >>> 0)/4294967296;
  };
}
function sample(p, random) {
  const u = random(); let cumulative = 0;
  for (let i=0;i<p.length;i++) { cumulative += p[i]; if(u<cumulative) return i; }
  return p.length-1;
}
function makeLedger(count = 128, seed = 20260920) {
  if (!Number.isInteger(count) || count < 1 || count > MAX_EVENTS) throw Error('Invalid count');
  const random = rng(seed); const p = bell(0,0); const rows = [];
  let previous = '0'.repeat(64);
  for(let i=0;i<count;i++) {
    const outcome = sample(p,random);
    const record = {sequence:i, source:'ruqu-wasm-simulation', seed,
      engineSha256, protocol:PROTOCOL,
      setting:[0,0], outcome, alice:outcome & 1, bob:(outcome >> 1) & 1};
    const hash = digest({previous,record});
    rows.push({previous,record,hash}); previous = hash;
  }
  return {rows,root:previous};
}
function verify(rows, trustedRoot) {
  try {
    let previous = '0'.repeat(64);
    if (!Array.isArray(rows) || !rows.length || rows.length > MAX_EVENTS ||
        typeof trustedRoot !== 'string' || !/^[a-f0-9]{64}$/.test(trustedRoot)) return false;
    let seed, engine;
    for(let i=0;i<rows.length;i++) {
      const row=rows[i], r=row.record;
      if (Object.keys(row).sort().join(',') !== 'hash,previous,record' ||
          Object.keys(r).sort().join(',') !== 'alice,bob,engineSha256,outcome,protocol,seed,sequence,setting,source' ||
          r.sequence!==i || r.source!=='ruqu-wasm-simulation' || r.protocol!==PROTOCOL ||
          typeof r.engineSha256!=='string' || !/^[a-f0-9]{64}$/.test(r.engineSha256) ||
          !Number.isInteger(r.seed) || r.seed<1 || r.seed>0xffffffff ||
          !Array.isArray(r.setting) || r.setting.length!==2 || r.setting[0]!==0 || r.setting[1]!==0 ||
          ![0,3].includes(r.outcome) || r.alice!==(r.outcome&1) || r.bob!==((r.outcome>>1)&1)) return false;
      if (i===0) { seed=r.seed; engine=r.engineSha256; }
      if(r.seed!==seed || r.engineSha256!==engine || row.previous!==previous ||
         digest({previous,record:r})!==row.hash) return false;
      previous=row.hash;
    }
    return previous===trustedRoot;
  } catch { return false; }
}
function run({count=128, seed=20260920}={}) {
  const ledger=makeLedger(count, seed);
  const memories=ledger.rows.map(({record}) => ({sequence:record.sequence,
    aliceRecall:record.alice, bobRecall:record.bob ^ Number(record.sequence%4===0),
    intervention:record.sequence%4===0 ? 'deliberate-bit-flip' : 'none'}));
  return {
    schema:1, backend:'classical state-vector simulation; no QPU',
    protocol:PROTOCOL,
    wasmSha256:engineSha256,
    coherent:chsh(), dephased:chsh(true), localDeterministicBound:localBound(),
    observerRecorded:observer(false), observerUncomputed:observer(true),
    ledger, memories,
    disagreementFraction:memories.filter(m=>m.aliceRecall!==m.bobRecall).length/memories.length,
    claims:['Simulated predictions only','Memory disagreement is deliberately injected',
      'No Bell hardware test, quantum advantage, alternate realities, or CERN inference',
      'Hash chain is tamper evident only against a separately trusted root',
      'RuField and WorldGraph projection is a separate validated native adapter',
      'No RuVector database integration']
  };
}
function benchmark(iterations=100) {
  if (!Number.isInteger(iterations) || iterations<1 || iterations>1000) throw Error('Invalid iterations');
  function baseline() {
    return settings.map(([a,b]) => bell(a,b).map((v,i)=>(v+bell(a,b,true)[i])/2));
  }
  for(let i=0;i<10;i++) { baseline(); chsh(true); }
  const original=[], optimized=[];
  for(let i=0;i<iterations;i++) {
    const measure=fn=>{const t=performance.now(); fn(); return performance.now()-t;};
    if(i%2) {optimized.push(measure(()=>chsh(true))); original.push(measure(baseline));}
    else {original.push(measure(baseline)); optimized.push(measure(()=>chsh(true)));}
  }
  const stats=xs=>{xs.sort((a,b)=>a-b);return {medianMs:xs[Math.floor(xs.length/2)],p95Ms:xs[Math.min(xs.length-1,Math.floor(xs.length*.95))]};};
  const before=stats(original), after=stats(optimized);
  return {backend:'classical state-vector simulation; no QPU',iterations,node:process.version,
    platform:process.platform,arch:process.arch,wasmSha256:engineSha256,
    original:{simulationsPerEvaluation:20,...before}, optimized:{simulationsPerEvaluation:8,...after},
    medianSpeedup:before.medianMs/after.medianMs,
    equivalent:canonical(baseline())===canonical(chsh(true).distributions),
    interpretation:'Local microbenchmark; no quantum advantage or deployment SLA'};
}
module.exports={bell,chsh,localBound,observer,makeLedger,verify,run,rng,canonical,benchmark};
if(require.main===module) console.log(JSON.stringify(run(),null,2));
