'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const {createHash} = require('node:crypto');
const {readFileSync} = require('node:fs');
const engine = require('../src/engine.cjs');

function bellMeasured(q, seed) {
  const c = new q.WasmQuantumCircuit(2);
  try { c.h(0); c.cnot(0, 1); c.measure_all(); return q.simulate(c, seed).measurements; }
  finally { c.free(); }
}

test('engine comes from the @ruvector/ruqu-wasm dependency and reports its bytes', () => {
  const e = engine.load();
  assert.equal(e.wasmPath, require.resolve('@ruvector/ruqu-wasm/ruqu_wasm_bg.wasm'));
  assert.equal(e.sha256, createHash('sha256').update(readFileSync(e.wasmPath)).digest('hex'));
  assert.match(e.version, /^2\./);
  assert.equal(engine.load(), e);
});

test('seeded simulate is reproducible', () => {
  const q = engine.quantum();
  for (const seed of [0, 7, 42]) assert.deepEqual(bellMeasured(q, seed), bellMeasured(q, seed));
});

test('non-finite rotation angles are rejected', () => {
  const q = engine.quantum();
  for (const gate of ['rx', 'ry', 'rz']) for (const angle of [NaN, Infinity, -Infinity]) {
    const c = new q.WasmQuantumCircuit(1);
    try { assert.throws(() => c[gate](0, angle), /finite/); } finally { c.free(); }
  }
});
