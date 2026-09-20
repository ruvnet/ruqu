'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { execFileSync, spawnSync } = require('node:child_process');
const path = require('node:path');
const {
  bell, chsh, localBound, observer, makeLedger, verify, run, rng, canonical, benchmark,
} = require('../src/observer-lab.cjs');

const near = (actual, expected, tolerance = 1e-10) =>
  assert.ok(Math.abs(actual - expected) < tolerance, `${actual} != ${expected}`);
const correlation = p => p[0] + p[3] - p[1] - p[2];
const copy = value => JSON.parse(JSON.stringify(value));
const digest = value => createHash('sha256').update(canonical(value)).digest('hex');
const cli = path.resolve(__dirname, '../bin/cli.js');
const wasmDigest = createHash('sha256')
  .update(readFileSync(path.resolve(__dirname, '../wasm/ruqu_wasm_bg.wasm'))).digest('hex');

test('Bell Z probabilities match the analytical state', () => {
  bell(0, 0).forEach((value, index) => near(value, [0.5, 0, 0, 0.5][index]));
});

test('Bell correlations agree with independent angular identities', () => {
  const angles = [-Math.PI, -0.81, 0, 0.37, Math.PI / 2, Math.PI];
  for (const a of angles) for (const b of angles) {
    near(correlation(bell(a, b)), Math.cos(a - b));
    near(correlation(bell(a, b, true)), Math.cos(a + b));
  }
});

test('all Bell distributions are normalized, nonnegative and no-signalling', () => {
  for (const phase of [false, true]) {
    for (const a of [-1.1, 0, 0.31, Math.PI / 2]) {
      for (const b of [-2.2, 0, 0.76, Math.PI]) {
        const p = bell(a, b, phase);
        assert.equal(p.length, 4);
        assert.ok(p.every(v => Number.isFinite(v) && v >= 0 && v <= 1 + 1e-10));
        near(p.reduce((sum, value) => sum + value, 0), 1);
        near(p[0] + p[2], 0.5);
        near(p[0] + p[1], 0.5);
      }
    }
  }
});

test('CHSH exact expectations distinguish coherent and dephased controls', () => {
  near(chsh().S, 2 * Math.sqrt(2));
  near(chsh(true).S, Math.sqrt(2));
  assert.equal(localBound(), 2);
});

test('coherent observer uncomputation restores interference', () => {
  near(observer(false).systemZero, 0.5);
  near(observer(true).systemZero, 1);
  assert.deepEqual(observer(true), observer(true));
});

test('angles reject nonnumeric and nonfinite values without coercion', () => {
  for (const value of [NaN, Infinity, -Infinity, undefined, null, '0', true, {}, []]) {
    assert.throws(() => bell(value, 0), `first angle ${String(value)}`);
    assert.throws(() => bell(0, value), `second angle ${String(value)}`);
  }
});

test('control flags reject truthy strings and numeric aliases', () => {
  for (const value of ['false', 'true', 0, 1, null, {}, []]) {
    assert.throws(() => bell(0, 0, value));
    assert.throws(() => chsh(value));
    assert.throws(() => observer(value));
  }
});

test('xorshift output is deterministic, bounded and seed dependent', () => {
  const a = rng(1), b = rng(1), c = rng(2);
  const first = Array.from({ length: 64 }, () => a());
  assert.deepEqual(first, Array.from({ length: 64 }, () => b()));
  assert.notDeepEqual(first, Array.from({ length: 64 }, () => c()));
  assert.ok(first.every(v => Number.isFinite(v) && v >= 0 && v < 1));
});

test('seed validation rejects unsafe, coerced or out of range values', () => {
  for (const seed of [0, -1, NaN, Infinity, -Infinity, 0.5, 0x100000000, '1', null, true]) {
    assert.throws(() => rng(seed));
    assert.throws(() => makeLedger(1, seed));
  }
  assert.ok(makeLedger(1, 1).rows.length === 1);
  assert.ok(makeLedger(1, 0xffffffff).rows.length === 1);
});

test('ledger count is an integer bounded at 10000', () => {
  for (const count of [0, -1, 1.5, NaN, Infinity, 10001, '2', null, true]) {
    assert.throws(() => makeLedger(count));
  }
  const ledger = makeLedger(10000);
  assert.equal(ledger.rows.length, 10000);
  assert.ok(verify(ledger.rows, ledger.root));
});

test('identical configuration produces identical evidence and a nonzero digest', () => {
  const a = makeLedger(128, 73), b = makeLedger(128, 73);
  assert.deepEqual(a, b);
  assert.notEqual(a.root, '0'.repeat(64));
  assert.match(a.root, /^[0-9a-f]{64}$/);
  assert.notEqual(a.root, makeLedger(128, 74).root);
});

test('every evidence record binds the protocol and actual WASM binary', () => {
  const ledger = makeLedger(7);
  for (const { record } of ledger.rows) {
    assert.equal(record.protocol, 'ruqu.observer.v1');
    assert.equal(record.engineSha256, wasmDigest);
    assert.equal(record.source, 'ruqu-wasm-simulation');
    assert.equal(record.alice, record.outcome & 1);
    assert.equal(record.bob, (record.outcome >> 1) & 1);
    assert.ok(record.outcome === 0 || record.outcome === 3);
  }
  assert.ok(verify(ledger.rows, ledger.root));
});

test('verification safely rejects malformed inputs and roots', () => {
  const valid = makeLedger(2);
  for (const rows of [undefined, null, false, 7, '', {}, [], [null], [{}], [{ record: {} }]]) {
    assert.equal(verify(rows, valid.root), false);
  }
  for (const root of [undefined, null, false, 7, {}, '', 'g'.repeat(64), '0'.repeat(63)]) {
    assert.equal(verify(valid.rows, root), false);
  }
});

test('any altered evidence field is rejected against its trusted terminal root', () => {
  const ledger = makeLedger(3);
  const mutations = [
    record => { record.sequence += 1; },
    record => { record.seed += 1; },
    record => { record.setting[0] = 0.1; },
    record => { record.outcome ^= 3; },
    record => { record.alice ^= 1; },
    record => { record.bob ^= 1; },
    record => { record.source = 'quantum-hardware'; },
    record => { record.protocol = 'invented-protocol'; },
    record => { record.engineSha256 = '0'.repeat(64); },
  ];
  for (const mutate of mutations) {
    const changed = copy(ledger);
    mutate(changed.rows[1].record);
    assert.equal(verify(changed.rows, ledger.root), false);
  }
});

test('truncation, reordering, duplication and a replacement root are rejected', () => {
  const ledger = makeLedger(4);
  assert.equal(verify(ledger.rows.slice(0, -1), ledger.root), false);
  assert.equal(verify(ledger.rows.slice(1), ledger.root), false);
  assert.equal(verify([...ledger.rows].reverse(), ledger.root), false);
  assert.equal(verify([...ledger.rows, ledger.rows[3]], ledger.root), false);
  assert.equal(verify(ledger.rows, makeLedger(4, 1).root), false);
});

test('self-consistent hashes do not legitimize invalid record schemas', () => {
  const changes = [
    r => { r.unrecognized = true; },
    r => { delete r.alice; },
    r => { r.alice = 17; },
    r => { r.outcome = 99; },
    r => { r.bob ^= 1; },
    r => { r.protocol = 'unknown'; },
    r => { r.source = 'quantum-hardware'; },
    r => { r.engineSha256 = 'invalid'; },
  ];
  for (const change of changes) {
    const ledger = makeLedger(1);
    const row = ledger.rows[0];
    change(row.record);
    row.hash = digest({ previous: row.previous, record: row.record });
    assert.equal(verify(ledger.rows, row.hash), false);
  }
});

test('canonical encoding ignores object key order but preserves array order', () => {
  assert.equal(canonical({ z: [3, 2, 1], a: { b: 2, a: 1 } }),
    canonical({ a: { a: 1, b: 2 }, z: [3, 2, 1] }));
  assert.notEqual(canonical([1, 2]), canonical([2, 1]));
  const ledger = makeLedger(3);
  for (const row of ledger.rows) {
    row.record = Object.fromEntries(Object.entries(row.record).reverse());
  }
  assert.ok(verify(ledger.rows, ledger.root));
});

test('canonical evidence rejects sparse arrays rather than producing invalid JSON', () => {
  assert.throws(() => canonical(new Array(2)));
  assert.throws(() => canonical({ setting: [0, , 0] }));
});

test('run respects configuration and classical interventions preserve evidence', () => {
  const result = run({ count: 20, seed: 91 });
  assert.equal(result.ledger.rows.length, 20);
  assert.equal(result.memories.length, 20);
  near(result.disagreementFraction, 0.25);
  assert.deepEqual(result.ledger, makeLedger(20, 91));
  assert.ok(verify(result.ledger.rows, result.ledger.root));
  assert.deepEqual(result, run({ count: 20, seed: 91 }));
});

test('partial blocks report the actual intervention fraction', () => {
  const result = run({ count: 5, seed: 1 });
  near(result.disagreementFraction, 2 / 5);
  assert.ok(verify(result.ledger.rows, result.ledger.root));
});

test('run identifies simulation and does not claim physical discovery', () => {
  const result = run({ count: 4 });
  assert.match(result.backend, /classical|simulat/i);
  assert.equal(result.wasmSha256, wasmDigest);
  assert.ok(Array.isArray(result.claims) && result.claims.length > 0);
  assert.match(result.claims.join(' '), /simulat/i);
  assert.match(result.claims.join(' '), /No.*(hardware|QPU|quantum advantage|alternate)/i);
});

test('benchmark is an explicit exported operation', () => {
  assert.equal(typeof benchmark, 'function');
});

test('CLI emits one parseable reproducible JSON run with selected configuration', () => {
  const output = execFileSync(process.execPath, [cli, 'observer-lab', '--count', '8', '--seed', '77'],
    { encoding: 'utf8', timeout: 10000 });
  const result = JSON.parse(output);
  assert.equal(result.ledger.rows.length, 8);
  assert.deepEqual(result.ledger, makeLedger(8, 77));
  assert.ok(verify(result.ledger.rows, result.ledger.root));
});

test('CLI flushes the complete maximum-size report before exiting', () => {
  const output = execFileSync(process.execPath, [cli, 'observer-lab', '--count', '10000'],
    { encoding: 'utf8', timeout: 10000, maxBuffer: 32 * 1024 * 1024 });
  const result = JSON.parse(output);
  assert.equal(result.ledger.rows.length, 10000);
  assert.equal(result.memories.length, 10000);
  assert.ok(verify(result.ledger.rows, result.ledger.root));
});

test('CLI help makes the observer-lab command discoverable', () => {
  const output = execFileSync(process.execPath, [cli, '--help'],
    { encoding: 'utf8', timeout: 10000 });
  assert.match(output, /observer-lab/);
});

test('CLI rejects malformed, unsupported and missing arguments', () => {
  const invalid = [
    ['--count'], ['--seed'], ['--unknown'], ['unexpected'],
    ['--count', '2x'], ['--count', '0'], ['--count', '10001'], ['--count', '1.5'],
    ['--seed', '0'], ['--seed', '-1'], ['--seed', '4294967296'], ['--seed', '1x'],
  ];
  for (const args of invalid) {
    const result = spawnSync(process.execPath, [cli, 'observer-lab', ...args],
      { encoding: 'utf8', timeout: 10000 });
    assert.equal(result.error, undefined);
    assert.notEqual(result.status, 0, JSON.stringify(args));
    assert.ok(result.stderr.length > 0, JSON.stringify(args));
  }
});

test('CLI benchmark emits JSON without pretending to measure quantum advantage', () => {
  const output = execFileSync(process.execPath, [cli, 'observer-lab', '--benchmark'],
    { encoding: 'utf8', timeout: 30000 });
  const result = JSON.parse(output);
  assert.equal(result.equivalent, true);
  assert.equal(result.original.simulationsPerEvaluation, 20);
  assert.equal(result.optimized.simulationsPerEvaluation, 8);
  assert.ok(result.original.medianMs >= 0 && result.optimized.medianMs >= 0);
  assert.ok(result.original.p95Ms >= result.original.medianMs);
  assert.ok(result.optimized.p95Ms >= result.optimized.medianMs);
  assert.equal(result.wasmSha256, wasmDigest);
  assert.match(result.backend, /classical|simulat/i);
  assert.match(result.interpretation, /no quantum advantage/i);
});
