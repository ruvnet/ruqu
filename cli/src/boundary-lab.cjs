'use strict';
const {createHash} = require('node:crypto');
const {readFileSync, openSync, readSync, closeSync, fstatSync, constants} = require('node:fs');
const path = require('node:path');
const {canonical} = require('./observer-lab.cjs');
const {PROTOCOL, digest, validParameters} = require('./boundary-contract.cjs');
const {generate} = require('./boundary-simulator.cjs');
const {predict, zone} = require('./boundary-inference.cjs');
const MAX_BYTES = 16 * 1024 * 1024;
function provenance() {
  return Object.fromEntries(['boundary-contract.cjs', 'boundary-simulator.cjs', 'boundary-inference.cjs',
    'boundary-lab.cjs', 'observer-lab.cjs'].map(file =>
    [file, createHash('sha256').update(readFileSync(path.join(__dirname, file))).digest('hex')]));
}
function plan() {
  const source = provenance();
  return {protocol: PROTOCOL, provenance: source, protocolHash: digest({protocol: PROTOCOL, provenance: source})};
}
function rate(successes, total) {
  if (!total) return {successes, total, estimate: null, lower: null, upper: null};
  const estimate = successes / total, z = 1.959963984540054, z2 = z * z;
  const center = (estimate + z2 / (2 * total)) / (1 + z2 / total);
  const radius = z * Math.sqrt(estimate * (1 - estimate) / total + z2 / (4 * total * total)) / (1 + z2 / total);
  return {successes, total, estimate, lower: successes === 0 ? 0 : Math.max(0, center - radius),
    upper: successes === total ? 1 : Math.min(1, center + radius)};
}
function median(values) {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b), middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}
const distance = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]);
function score(predictions, truth) {
  if (!Array.isArray(predictions) || !Array.isArray(truth) || predictions.length !== truth.length || !truth.length)
    throw Error('Prediction and truth counts differ or are empty');
  const errors = [], baselineErrors = [];
  let sources = 0, localized = 0, zonesCorrect = 0, scenesCorrect = 0, signalCount = 0, missedSources = 0;
  const controls = Object.fromEntries(PROTOCOL.arms.slice(1).map(arm => [arm, {total: 0, falsePositives: 0}]));
  const rows = truth.map((label, index) => {
    const prediction = predictions[index];
    if (prediction.id !== label.id) throw Error('Prediction and truth alignment failed');
    const detected = prediction.channels.filter(channel => channel.status === 'detected');
    if (label.arm !== 'signal') {
      const control = controls[label.arm];
      control.total++; control.falsePositives += Number(detected.length > 0);
      return {id: label.id, arm: label.arm, correctlyRejected: detected.length === 0};
    }
    signalCount++;
    let allCorrect = detected.length === label.sources.length;
    const sourceScores = label.sources.map(source => {
      sources++;
      const channel = prediction.channels.find(channel => channel.frequencyHz === source.frequencyHz);
      const errorM = channel.estimate ? distance(channel.estimate.positionM, source.positionM) : null;
      const baselineErrorM = channel.baseline ? distance(channel.baseline.positionM, source.positionM) : null;
      if (errorM !== null) errors.push(errorM); else missedSources++;
      if (baselineErrorM !== null) baselineErrors.push(baselineErrorM);
      const locationCorrect = errorM !== null && errorM <= PROTOCOL.acceptance.maximumLocationErrorM;
      const zoneCorrect = channel.estimate !== null && channel.estimate.zone === zone(source.positionM);
      localized += Number(locationCorrect); zonesCorrect += Number(zoneCorrect);
      allCorrect &&= locationCorrect && zoneCorrect;
      return {frequencyHz: source.frequencyHz, errorM, baselineErrorM, locationCorrect, zoneCorrect};
    });
    scenesCorrect += Number(allCorrect);
    return {id: label.id, arm: label.arm, correct: allCorrect, sources: sourceScores};
  });
  const controlRates = Object.fromEntries(Object.entries(controls).map(([arm, value]) => [arm, {
    interpretation: arm === 'model-mismatch' ? 'Acceptance of deliberately distorted data, not proof that no emitter exists' : 'Detection without a simulated physical emitter',
    falsePositiveRate: rate(value.falsePositives, value.total),
    rejectionRate: rate(value.total - value.falsePositives, value.total)}]));
  const medianErrorM = median(errors), baselineMedianErrorM = median(baselineErrors);
  const medianErrorReduction = medianErrorM !== null && baselineMedianErrorM > 0 ? 1 - medianErrorM / baselineMedianErrorM : null;
  const sceneAccuracy = rate(scenesCorrect, signalCount);
  const a = PROTOCOL.acceptance;
  const gates = {
    sceneAccuracy: sceneAccuracy.estimate >= a.minimumSceneAccuracy,
    localization: localized / sources >= a.minimumSceneAccuracy,
    emptyFalsePositives: controlRates.empty.falsePositiveRate.estimate < a.maximumFalsePositiveRate,
    injectionRejection: controlRates['single-sensor-injection'].rejectionRate.estimate >= a.minimumControlRejection,
    mismatchRejection: controlRates['model-mismatch'].rejectionRate.estimate >= a.minimumControlRejection,
    baselineImprovement: medianErrorReduction !== null && medianErrorReduction >= a.minimumMedianErrorReduction
  };
  const fraction = successes => ({successes, total: sources, estimate: successes / sources,
    interval: 'Not reported: sources in a scene are clustered'});
  return {sceneAccuracy, sourceLocalization: fraction(localized), zoneAccuracy: fraction(zonesCorrect),
    missedSources, medianErrorM, baselineMedianErrorM, medianErrorReduction,
    errorSummaryScope: 'Median over localized sources; missed sources reported separately and included as failures in accuracy',
    controls: controlRates, gates, verdict: Object.values(gates).every(Boolean) ? 'supported-in-model' : 'not-supported-in-model', rows};
}
function run(options = {}) {
  if (!options || typeof options !== 'object' || Array.isArray(options) ||
      Object.keys(options).some(key => !['protocolHash', 'seed', 'trials'].includes(key))) throw Error('Unknown run option');
  const {protocolHash, seed = PROTOCOL.defaultSeed, trials = PROTOCOL.trialsPerArm.default} = options;
  validParameters(seed, trials);
  const committed = plan();
  if (protocolHash !== committed.protocolHash) throw Error('Missing or changed commitment; run boundary-lab plan first');
  const {dataset, truth} = generate({seed, trials});
  const truthDigest = digest(truth);
  // All measurement-only predictions are committed before the evaluator can inspect labels.
  const predictionBundle = predict(dataset);
  const metrics = score(predictionBundle.predictions, truth);
  const report = {schema: 'ruqu.boundary.result.v1', evidenceClass: 'classical-simulation', protocolHash,
    provenance: committed.provenance, parameters: {seed, trials}, dataset, truth, truthDigest, predictionBundle, metrics,
    limits: ['Synthetic RSS power measurements only; zero quantum hardware or physical sensor calls',
      'One emitter per known frequency; not general object identification or cochannel separation',
      'Free space inverse square model; no measured indoor multipath, walls, or antenna response',
      'Blinding is an API boundary, not process isolation; seed and truth are revealed in the final report',
      'Single sensor injection rejection does not authenticate sensors or reject coordinated spoofing',
      'Marginal sampling intervals exclude systematic error and do not establish physical performance',
      'Development seed is public; independent replication requires fresh externally selected trials',
      'Hash commitments are not signatures, timestamps, or proof of acquisition',
      'No holography, QMM, new physics, quantum advantage, or Nobel level discovery established']};
  return {report, root: digest(report)};
}
function verify(bundle, trustedRoot) {
  try {
    if (!bundle || typeof trustedRoot !== 'string' || !/^[a-f0-9]{64}$/.test(trustedRoot) ||
        Object.keys(bundle).sort().join(',') !== 'report,root' || bundle.root !== trustedRoot || digest(bundle.report) !== trustedRoot) return false;
    return canonical(bundle) === canonical(run({...bundle.report.parameters, protocolHash: bundle.report.protocolHash}));
  } catch { return false; }
}
function project(bundle, trustedRoot) {
  if (!verify(bundle, trustedRoot)) throw Error('Evidence verification failed before projection');
  const {dataset, predictionBundle} = bundle.report;
  const nodes = [{id: 'room', kind: 'synthetic-room', dimensionsM: [2, 2]}];
  const edges = [], fieldEvents = [];
  for (const sensor of dataset.sensors) {
    nodes.push({id: sensor.id, kind: 'synthetic-sensor', positionM: sensor.positionM});
    edges.push({kind: 'located-in', from: sensor.id, to: 'room'});
  }
  for (const [sequence, trial] of dataset.trials.entries()) {
    const prediction = predictionBundle.predictions[sequence];
    const lineage = {reportRoot: trustedRoot, inputDigest: prediction.inputDigest};
    nodes.push({id: trial.id, kind: 'synthetic-observation', ...lineage});
    edges.push({kind: 'located-in', from: trial.id, to: 'room'});
    for (const [index, channel] of trial.channels.entries()) {
      fieldEvents.push({schema: 'ruqu.boundary.field-event.v1', id: `${trial.id}:${channel.frequencyHz}`,
        synthetic: true, modality: 'synthetic-rss', sequence, timeBasis: 'simulation-sequence-not-wall-clock',
        frequencyHz: channel.frequencyHz, unit: 'dBm', sensorIds: dataset.sensors.map(sensor => sensor.id),
        values: channel.rssiDbm, ...lineage});
      const inferred = prediction.channels[index];
      if (inferred.status !== 'detected') continue;
      const id = `${trial.id}:emitter:${channel.frequencyHz}`;
      nodes.push({id, kind: 'inferred-emitter', synthetic: true, frequencyHz: channel.frequencyHz,
        ...inferred.estimate, confidence: null, uncertaintyMeaning: 'Residual is model fit, not a calibrated spatial confidence interval', ...lineage});
      edges.push({kind: 'located-in', from: id, to: 'room'}, {kind: 'derived-from', from: id, to: trial.id});
    }
  }
  return {schema: 'ruqu.boundary.projection.v1', evidenceClass: 'classical-simulation', reportRoot: trustedRoot,
    integrationStatus: 'Local export contract only; not native RuField or WorldGraph serialization and no service writes',
    sensors: dataset.sensors, fieldEvents, worldGraph: {schema: 'ruqu.boundary.graph.v1', nodes, edges}};
}
function readBounded(file) {
  const fd = openSync(file, constants.O_RDONLY | constants.O_NONBLOCK);
  try {
    const stat = fstatSync(fd);
    if (!stat.isFile()) throw Error('Input must be a regular file');
    if (stat.size > MAX_BYTES) throw Error('Input exceeds 16 MiB');
    const buffer = Buffer.alloc(Math.min(stat.size + 1, MAX_BYTES + 1));
    let total = 0, n;
    while (total < buffer.length && (n = readSync(fd, buffer, total, buffer.length - total, null)) > 0) total += n;
    if (total !== stat.size) throw Error('Input changed while reading');
    return JSON.parse(buffer.subarray(0, total).toString('utf8'));
  } finally { closeSync(fd); }
}
function cli(args) {
  const [command, ...rest] = args;
  if (command === 'plan' && !rest.length) return plan();
  const allowed = {run: ['--protocol-hash', '--seed', '--trials'], predict: ['--file'],
    verify: ['--file', '--trusted-root'], project: ['--file', '--trusted-root']}[command];
  if (!allowed || rest.length % 2) throw Error('Usage: boundary-lab plan | run --protocol-hash HASH [--seed N --trials N] | predict --file DATASET | verify|project --file REPORT --trusted-root HASH');
  const parsed = {};
  for (let i = 0; i < rest.length; i += 2) {
    if (!allowed.includes(rest[i]) || Object.hasOwn(parsed, rest[i]) || !rest[i + 1]) throw Error('Unknown, missing, or repeated option');
    parsed[rest[i]] = rest[i + 1];
  }
  if (command === 'run') {
    const options = {protocolHash: parsed['--protocol-hash']};
    for (const key of ['seed', 'trials']) if (parsed[`--${key}`] !== undefined) {
      if (!/^\d+$/.test(parsed[`--${key}`])) throw Error(`Invalid ${key}`);
      options[key] = Number(parsed[`--${key}`]);
    }
    return run(options);
  }
  if (!parsed['--file']) throw Error('Input file required');
  if (command !== 'predict' && !parsed['--trusted-root']) throw Error('Separately trusted root required');
  const input = readBounded(parsed['--file']);
  if (command === 'predict') return predict(input);
  if (command === 'project') return project(input, parsed['--trusted-root']);
  if (!verify(input, parsed['--trusted-root'])) throw Error('Evidence verification failed');
  return {verified: true, root: parsed['--trusted-root'], evidenceClass: 'classical-simulation'};
}
// Scoring is analysis, not receipt verification; only verify and project enforce replay.
module.exports = {plan, run, verify, project, cli, score};
