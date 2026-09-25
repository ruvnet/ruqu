'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const {mkdtempSync, writeFileSync, rmSync, readFileSync} = require('node:fs');
const {tmpdir} = require('node:os');
const path = require('node:path');
const lab = require('../src/boundary-lab.cjs');
const {PROTOCOL, SENSORS, CHANNELS, digest, validateDataset} = require('../src/boundary-contract.cjs');
const {generate, receivedPowerDbm} = require('../src/boundary-simulator.cjs');
const {predict} = require('../src/boundary-inference.cjs');
const protocolHash = lab.plan().protocolHash;
const run = options => lab.run({protocolHash, ...options});
const defaultBundle = run();
const clone = value => structuredClone(value);
const near = (a, b, epsilon = 1e-9) => assert.ok(Math.abs(a - b) < epsilon, `${a} != ${b}`);
function oneTrial() {
  const dataset = clone(defaultBundle.report.dataset);
  dataset.trials = [dataset.trials[0]];
  dataset.trials[0].channels.forEach(channel => { channel.rssiDbm.fill(-95); });
  return dataset;
}
test('all sensors lie on the two metre boundary and committed objects are frozen', () => {
  assert.equal(SENSORS.length, 8);
  for (const sensor of SENSORS) {
    assert.ok(sensor.positionM.some(value => value === 0 || value === 2));
    assert.ok(Object.isFrozen(sensor.positionM));
  }
  assert.ok(Object.isFrozen(PROTOCOL.acceptance));
  assert.equal(lab.plan().protocolHash, protocolHash);
});
test('inverse square simulator matches analytical distance and power addition controls', () => {
  const source = {positionM: [0, 0], powerAtOneMetreDbm: -40};
  near(receivedPowerDbm([1, 0], [source], -160), -40, 1e-10);
  near(receivedPowerDbm([2, 0], [source], -160), -40 - 10 * Math.log10(4), 1e-10);
  near(receivedPowerDbm([1, 0], [source, source], -160), -40 + 10 * Math.log10(2), 1e-10);
  near(receivedPowerDbm([1, 0], [], -95), -95);
  assert.throws(() => receivedPowerDbm([0, 0], [source]));
});
test('generator balances and shuffles every arm without labels or seeds in measurements', () => {
  const {dataset, truth} = generate();
  for (const arm of PROTOCOL.arms) assert.equal(truth.filter(row => row.arm === arm).length, 100);
  assert.deepEqual(dataset, defaultBundle.report.dataset);
  assert.ok(truth.some((row, i) => i > 0 && row.arm !== truth[i - 1].arm));
  assert.doesNotMatch(JSON.stringify(dataset), /"(seed|arm|sources|truth|injectedSensor)"/);
  assert.equal(new Set(dataset.trials.map(trial => trial.id)).size, 400);
  assert.ok(dataset.trials.some(trial => trial.channels.some(channel => channel.rssiDbm.includes(null))));
  const sourceCode = readFileSync(path.join(__dirname, '../src/boundary-inference.cjs'), 'utf8');
  assert.doesNotMatch(sourceCode, /require\(['"]\.\/boundary-(simulator|lab)/);
});
test('independently constructed noiseless measurements recover grid points and unknown power', () => {
  for (const position of [[0.4, 0.7], [1.4, 0.5], [1, 1], [0.8, 1.6]]) {
    const dataset = oneTrial(), power = -43;
    dataset.trials[0].channels[0].rssiDbm = SENSORS.map(sensor => power -
      20 * Math.log10(Math.hypot(position[0] - sensor.positionM[0], position[1] - sensor.positionM[1])) + sensor.offsetDb);
    const channel = predict(dataset).predictions[0].channels[0];
    assert.equal(channel.status, 'detected');
    near(channel.estimate.positionM[0], position[0]); near(channel.estimate.positionM[1], position[1]);
    near(channel.estimate.powerAtOneMetreDbm, power); near(channel.bestFitResidualDb, 0);
  }
});
test('missing data is distinct from absent signals and isolated injection never becomes an emitter', () => {
  const dataset = oneTrial();
  dataset.trials[0].channels[0].rssiDbm.fill(null);
  dataset.trials[0].channels[1].rssiDbm[3] = -30;
  const channels = predict(dataset).predictions[0].channels;
  assert.deepEqual(channels.map(channel => channel.status), ['insufficient-data', 'insufficient-consensus', 'absent']);
  assert.ok(channels.every(channel => channel.estimate === null));
});
test('calibration, geometry, units, count bounds, missing channels, and label leakage fail closed', () => {
  const mutations = [
    dataset => { dataset.truth = []; }, dataset => { dataset.seed = 123; },
    dataset => { dataset.evidenceClass = 'physical-sensor'; }, dataset => { dataset.schema = 'v2'; },
    dataset => { dataset.sensors[0].offsetDb = 40; }, dataset => { dataset.sensors[0].positionM = [1, 1]; },
    dataset => { dataset.trials[0].truth = {}; }, dataset => { dataset.trials[0].id = '../../private'; },
    dataset => { dataset.trials[0].channels.pop(); }, dataset => { dataset.trials[0].channels[0].unit = 'mW'; },
    dataset => { dataset.trials[0].channels[0].frequencyHz = CHANNELS[1]; },
    dataset => { dataset.trials[0].channels[0].rssiDbm.pop(); },
    ...[NaN, Infinity, -161, 21, '0', undefined].map(value => dataset => { dataset.trials[0].channels[0].rssiDbm[0] = value; }),
    dataset => { dataset.trials.push(dataset.trials[0]); }, dataset => { dataset.trials = []; },
    dataset => { dataset.trials = Array(2001).fill(dataset.trials[0]); }
  ];
  for (const mutate of mutations) { const dataset = oneTrial(); mutate(dataset); assert.throws(() => validateDataset(dataset)); }
});
test('full experiment meets frozen gates and reports every error and control', () => {
  const {report} = defaultBundle;
  assert.equal(report.metrics.verdict, 'supported-in-model');
  assert.ok(Object.values(report.metrics.gates).every(Boolean));
  assert.equal(report.metrics.rows.length, 400);
  assert.equal(report.metrics.sourceLocalization.total, 199);
  assert.ok(report.metrics.sceneAccuracy.estimate >= 0.8);
  assert.ok(report.metrics.medianErrorReduction >= 0.2);
  assert.ok(report.metrics.sceneAccuracy.lower <= report.metrics.sceneAccuracy.estimate);
  for (const control of Object.values(report.metrics.controls)) {
    assert.equal(control.falsePositiveRate.total, 100);
    assert.equal(control.falsePositiveRate.successes, 0);
    assert.ok(control.falsePositiveRate.upper < 0.05);
    assert.equal(control.falsePositiveRate.lower, 0);
  }
  assert.match(report.limits.join(' '), /zero quantum hardware/);
  assert.equal(report.predictionBundle.predictionDigest, digest(report.predictionBundle.predictions));
  assert.equal(report.truthDigest, digest(report.truth));
});
test('fresh predefined seeds replay without retries and the maximum run fits the input limit', () => {
  for (const seed of [1, 47, 20260926]) {
    const bundle = run({seed});
    assert.ok(lab.verify(bundle, bundle.root));
    assert.notEqual(bundle.root, defaultBundle.root);
    assert.equal(bundle.report.dataset.trials.length, 400);
  }
  const maximum = run({trials: 500});
  assert.equal(maximum.report.dataset.trials.length, 2000);
  assert.ok(Buffer.byteLength(JSON.stringify(maximum, null, 2)) < 16 * 1024 * 1024);
  assert.ok(lab.verify(maximum, maximum.root));
});
test('failed reconstruction yields a negative result rather than inventing locations or passing empty medians', () => {
  const dataset = clone(defaultBundle.report.dataset);
  for (const trial of dataset.trials) for (const channel of trial.channels) channel.rssiDbm.fill(null);
  const predictions = predict(dataset).predictions;
  const metrics = lab.score(predictions, defaultBundle.report.truth);
  assert.equal(metrics.verdict, 'not-supported-in-model');
  assert.equal(metrics.sceneAccuracy.estimate, 0);
  assert.equal(metrics.missedSources, 199);
  assert.equal(metrics.medianErrorM, null);
  assert.equal(metrics.medianErrorReduction, null);
  assert.equal(metrics.gates.baselineImprovement, false);
  assert.throws(() => lab.score(predictions.slice(1), defaultBundle.report.truth));
  assert.throws(() => lab.score(predictions, [...defaultBundle.report.truth].reverse()));
});
test('run rejects invalid commitments and unbounded or unknown parameters', () => {
  assert.throws(() => lab.run());
  assert.throws(() => run({protocolHash: '0'.repeat(64)}));
  for (const seed of [0, -1, 2 ** 32, 1.1, '1', NaN]) assert.throws(() => run({seed}));
  for (const trials of [0, 99, 501, 100.1, '100', NaN]) assert.throws(() => run({trials}));
  assert.throws(() => run({backend: 'qpu'})); assert.throws(() => lab.run(null));
});
test('raw measurements, source fingerprints, labels, predictions, and forged verdicts are bound by replay', () => {
  assert.deepEqual(run(), defaultBundle);
  assert.ok(lab.verify(defaultBundle, defaultBundle.root));
  const mutations = [
    bundle => { bundle.report.dataset.trials[0].channels[0].rssiDbm[0] = -30; },
    bundle => { bundle.report.truth.reverse(); },
    bundle => { bundle.report.provenance['boundary-inference.cjs'] = '0'.repeat(64); },
    bundle => { bundle.report.predictionBundle.predictions.pop(); },
    bundle => { bundle.report.metrics.sceneAccuracy.estimate = 1; },
    bundle => { bundle.report.metrics.verdict = 'physical-proof'; },
    bundle => { bundle.report.parameters.trials = 500000; },
    bundle => { bundle.report.extra = 'uncommitted'; }
  ];
  for (const mutate of mutations) {
    const altered = clone(defaultBundle); mutate(altered);
    assert.equal(lab.verify(altered, defaultBundle.root), false);
    altered.root = digest(altered.report);
    assert.equal(lab.verify(altered, altered.root), false);
  }
  assert.equal(lab.verify(defaultBundle, '0'.repeat(64)), false);
});
test('projection preserves raw values and lineage and never exports truth as inferred facts', () => {
  const projection = lab.project(defaultBundle, defaultBundle.root);
  assert.equal(projection.fieldEvents.length, 1200);
  assert.match(projection.integrationStatus, /not native/);
  const graph = projection.worldGraph, ids = new Set(graph.nodes.map(node => node.id));
  assert.equal(ids.size, graph.nodes.length);
  assert.equal(graph.nodes.filter(node => node.kind === 'inferred-emitter').length, 199);
  for (const edge of graph.edges) assert.ok(ids.has(edge.from) && ids.has(edge.to));
  for (const event of projection.fieldEvents) {
    assert.equal(event.synthetic, true); assert.equal(event.reportRoot, defaultBundle.root);
    const trial = defaultBundle.report.dataset.trials[event.sequence];
    assert.equal(event.inputDigest, digest(trial));
    assert.deepEqual(event.values, trial.channels.find(channel => channel.frequencyHz === event.frequencyHz).rssiDbm);
  }
  assert.doesNotMatch(JSON.stringify(projection), /"(truth|arm|injectedSensor)"/);
  assert.throws(() => lab.project(defaultBundle, '0'.repeat(64)));
});
test('CLI E2E uses a separate predictor process and rejects tampering, FIFOs, and malformed options', () => {
  const command = path.resolve(__dirname, '../bin/cli.js');
  const invoke = args => {
    const result = spawnSync(process.execPath, [command, 'boundary-lab', ...args],
      {encoding: 'utf8', timeout: 15000, maxBuffer: 16 * 1024 * 1024});
    assert.ifError(result.error);
    return result;
  };
  const plan = invoke(['plan']); assert.equal(plan.status, 0, plan.stderr);
  const hash = JSON.parse(plan.stdout).protocolHash;
  const result = invoke(['run', '--protocol-hash', hash]); assert.equal(result.status, 0, result.stderr);
  const bundle = JSON.parse(result.stdout), dir = mkdtempSync(path.join(tmpdir(), 'ruqu-boundary-'));
  try {
    const report = path.join(dir, 'report.json'), data = path.join(dir, 'measurements.json');
    writeFileSync(report, result.stdout); writeFileSync(data, JSON.stringify(bundle.report.dataset));
    const separate = invoke(['predict', '--file', data]); assert.equal(separate.status, 0, separate.stderr);
    assert.deepEqual(JSON.parse(separate.stdout), bundle.report.predictionBundle);
    for (const command of ['verify', 'project']) {
      const valid = invoke([command, '--file', report, '--trusted-root', bundle.root]);
      assert.equal(valid.status, 0, valid.stderr);
      assert.equal(JSON.parse(valid.stdout).evidenceClass, 'classical-simulation');
    }
    bundle.report.metrics.verdict = 'proven'; writeFileSync(report, JSON.stringify(bundle));
    for (const command of ['verify', 'project']) {
      const result = invoke([command, '--file', report, '--trusted-root', bundle.root]);
      assert.notEqual(result.status, 0); assert.equal(result.stdout, '');
    }
    writeFileSync(data, ' '.repeat(16 * 1024 * 1024 + 1));
    assert.match(invoke(['predict', '--file', data]).stderr, /exceeds 16 MiB/);
    writeFileSync(data, '{invalid'); assert.notEqual(invoke(['predict', '--file', data]).status, 0);
    if (process.platform !== 'win32') {
      const fifo = path.join(dir, 'fifo');
      assert.equal(spawnSync('mkfifo', [fifo], {timeout: 2000}).status, 0);
      const rejected = invoke(['predict', '--file', fifo]);
      assert.equal(rejected.signal, null); assert.match(rejected.stderr, /regular file/);
    }
    for (const args of [[], ['plan', '--seed', '1'], ['run'], ['run', '--protocol-hash', hash, '--seed', 'NaN'],
      ['run', '--protocol-hash', hash, '--trials', '100', '--trials', '100'], ['run', '--backend', 'hardware'],
      ['verify', '--file', report], ['predict', '--file'], ['predict', '--file', dir], ['predict', '--file', '/missing']]) {
      const invalid = invoke(args); assert.notEqual(invalid.status, 0); assert.equal(invalid.stdout, '');
    }
  } finally { rmSync(dir, {recursive: true, force: true}); }
});
