'use strict';
const {createHash} = require('node:crypto');
const {canonical} = require('./observer-lab.cjs');
const digest = value => createHash('sha256').update(canonical(value)).digest('hex');
function freeze(value) {
  if (value && typeof value === 'object') { Object.values(value).forEach(freeze); Object.freeze(value); }
  return value;
}
const SENSORS = freeze([[0, 0], [1, 0], [2, 0], [2, 1], [2, 2], [1, 2], [0, 2], [0, 1]]
  .map((positionM, i) => ({id: `boundary-${i}`, positionM,
    offsetDb: [-0.5, 0.3, 0.1, -0.4, 0.2, -0.1, 0.4, 0][i],
    calibration: 'synthetic-known-offset', uncertaintyDb: 0.2})));
const CHANNELS = freeze([2402000000, 2426000000, 2480000000]);
const ROOM = freeze({widthM: 2, heightM: 2, frame: 'local-cartesian-synthetic'});
const PROTOCOL = freeze({
  schema: 'ruqu.boundary.protocol.v1', version: 1, evidenceClass: 'classical-simulation',
  hypothesis: 'Boundary RSS measurements localize one to three spectrally separated emitters in the declared model',
  null: 'Scene accuracy is below 80 percent or control false positives reach 5 percent',
  registration: 'Local source commitment, not independent preregistration or human blinding',
  room: ROOM, sensors: SENSORS, channelsHz: CHANNELS,
  arms: ['signal', 'empty', 'single-sensor-injection', 'model-mismatch'],
  trialsPerArm: {minimum: 100, maximum: 500, default: 100}, defaultSeed: 20260925,
  simulation: {noiseFloorDbm: -95, measurementSigmaDb: 0.35, calibrationDriftDb: 0.2,
    sourceMarginM: 0.25, minimumPowerAtOneMetreDbm: -48, maximumPowerAtOneMetreDbm: -40,
    missingSensorProbability: 0.1, mismatchAlternatingGainDb: 12},
  inference: {thresholdDbm: -75, minimumSensors: 6, gridMinimumM: 0.2, gridMaximumM: 1.8,
    gridStepM: 0.05, maximumResidualDb: 2, minimumPowerAtOneMetreDbm: -55,
    maximumPowerAtOneMetreDbm: -30},
  acceptance: {minimumSceneAccuracy: 0.8, maximumLocationErrorM: 0.25,
    maximumFalsePositiveRate: 0.05, minimumControlRejection: 0.95,
    minimumMedianErrorReduction: 0.2},
  statistics: 'Wilson 95 percent marginal descriptive intervals; not simultaneous bounds or physical uncertainty',
  rules: ['All four arms reported without adaptive stopping', 'Predictor receives measurements only',
    'Commit all predictions before scoring truth', 'No object identity, quantum advantage, or new physics claims',
    'No QPU, RF transmission, hardware acquisition, or external graph writes']
});
function keys(value, expected, name) {
  if (!value || typeof value !== 'object' || Array.isArray(value) ||
      Object.getPrototypeOf(value) !== Object.prototype ||
      Object.keys(value).sort().join(',') !== [...expected].sort().join(',')) throw Error(`Invalid ${name} fields`);
}
function validParameters(seed, trials) {
  if (!Number.isSafeInteger(seed) || seed < 1 || seed > 0xffffffff) throw Error('Seed must be an integer in [1, 4294967295]');
  if (!Number.isSafeInteger(trials) || trials < 100 || trials > 500) throw Error('Trials per arm must be an integer in [100, 500]');
}
function validateDataset(dataset) {
  keys(dataset, ['schema', 'evidenceClass', 'room', 'sensors', 'trials'], 'dataset');
  if (dataset.schema !== 'ruqu.boundary.measurements.v1' || dataset.evidenceClass !== 'classical-simulation')
    throw Error('Only synthetic boundary measurements v1 are accepted');
  if (canonical(dataset.room) !== canonical(ROOM) || canonical(dataset.sensors) !== canonical(SENSORS))
    throw Error('Geometry or calibration differs from the committed sensor contract');
  if (!Array.isArray(dataset.trials) || dataset.trials.length < 1 || dataset.trials.length > 2000)
    throw Error('Expected 1 to 2000 measurement trials');
  const ids = new Set();
  for (const trial of dataset.trials) {
    keys(trial, ['id', 'channels'], 'trial');
    if (typeof trial.id !== 'string' || !/^trial[0-9]{4}$/.test(trial.id) || ids.has(trial.id)) throw Error('Invalid or duplicate trial ID');
    ids.add(trial.id);
    if (!Array.isArray(trial.channels) || trial.channels.length !== CHANNELS.length) throw Error('Missing channels');
    trial.channels.forEach((channel, i) => {
      keys(channel, ['frequencyHz', 'rssiDbm'], 'channel');
      if (channel.frequencyHz !== CHANNELS[i] || !Array.isArray(channel.rssiDbm) || channel.rssiDbm.length !== SENSORS.length)
        throw Error('Invalid frequency or sensor count');
      for (const value of channel.rssiDbm) if (value !== null && (!Number.isFinite(value) || value < -160 || value > 20))
        throw Error('RSS must be null or finite dBm in [-160, 20]');
    });
  }
  return dataset;
}
module.exports = {PROTOCOL, SENSORS, CHANNELS, ROOM, digest, keys, validParameters, validateDataset};
