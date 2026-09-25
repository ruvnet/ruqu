'use strict';
const {PROTOCOL, SENSORS, validateDataset, digest} = require('./boundary-contract.cjs');
const cfg = PROTOCOL.inference;
const mean = values => values.reduce((a, b) => a + b, 0) / values.length;
const zone = ([x, y]) => `${y < 1 ? 'south' : 'north'}-${x < 1 ? 'west' : 'east'}`;
// The search profiles out the unknown source power. No simulator or truth module is imported.
const grid = [];
const steps = Math.round((cfg.gridMaximumM - cfg.gridMinimumM) / cfg.gridStepM);
for (let x = 0; x <= steps; x++) for (let y = 0; y <= steps; y++) {
  const positionM = [cfg.gridMinimumM + x * cfg.gridStepM, cfg.gridMinimumM + y * cfg.gridStepM];
  grid.push({positionM, attenuationDb: SENSORS.map(sensor =>
    20 * Math.log10(Math.hypot(...positionM.map((v, i) => v - sensor.positionM[i]))))});
}
function inferChannel(channel) {
  const readings = channel.rssiDbm.map((value, i) => value === null ? null : value - SENSORS[i].offsetDb);
  const visible = readings.flatMap((value, i) => value !== null && value > cfg.thresholdDbm ? [i] : []);
  const base = {frequencyHz: channel.frequencyHz, sensorsAboveThreshold: visible.length};
  if (readings.filter(value => value !== null).length < cfg.minimumSensors)
    return {...base, status: 'insufficient-data', estimate: null, baseline: null};
  if (!visible.length) return {...base, status: 'absent', estimate: null, baseline: null};
  if (visible.length < cfg.minimumSensors) return {...base, status: 'insufficient-consensus', estimate: null, baseline: null};
  const weights = visible.map(i => 10 ** ((readings[i] - cfg.thresholdDbm) / 10));
  const total = weights.reduce((a, b) => a + b, 0);
  const baseline = [0, 1].map(axis => visible.reduce((sum, i, j) => sum + SENSORS[i].positionM[axis] * weights[j], 0) / total);
  let best;
  for (const candidate of grid) {
    const powers = visible.map(i => readings[i] + candidate.attenuationDb[i]);
    const powerAtOneMetreDbm = mean(powers);
    const residualDb = Math.sqrt(mean(powers.map(value => (value - powerAtOneMetreDbm) ** 2)));
    if (!best || residualDb < best.residualDb) best = {positionM: [...candidate.positionM],
      powerAtOneMetreDbm, residualDb, zone: zone(candidate.positionM)};
  }
  const accepted = best.residualDb <= cfg.maximumResidualDb &&
    best.powerAtOneMetreDbm >= cfg.minimumPowerAtOneMetreDbm && best.powerAtOneMetreDbm <= cfg.maximumPowerAtOneMetreDbm;
  return {...base, status: accepted ? 'detected' : 'model-mismatch', estimate: accepted ? best : null,
    bestFitResidualDb: best.residualDb, baseline: {positionM: baseline, zone: zone(baseline)}};
}
function predict(dataset) {
  validateDataset(dataset);
  const predictions = dataset.trials.map(trial => ({id: trial.id, inputDigest: digest(trial), channels: trial.channels.map(inferChannel)}));
  return {schema: 'ruqu.boundary.predictions.v1', evidenceClass: 'classical-simulation',
    datasetDigest: digest(dataset), predictions, predictionDigest: digest(predictions)};
}
module.exports = {predict, zone};
