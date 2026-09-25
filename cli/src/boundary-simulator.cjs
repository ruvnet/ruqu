'use strict';
const {rng} = require('./observer-lab.cjs');
const {PROTOCOL, SENSORS, CHANNELS, ROOM, validParameters, validateDataset} = require('./boundary-contract.cjs');
const dbmToMilliwatt = dbm => 10 ** (dbm / 10);
function receivedPowerDbm(positionM, sources, noiseFloorDbm = -95) {
  let power = dbmToMilliwatt(noiseFloorDbm);
  for (const source of sources) {
    const distanceSquared = positionM.reduce((sum, value, i) => sum + (value - source.positionM[i]) ** 2, 0);
    if (!(distanceSquared > 0)) throw Error('Source coincides with sensor');
    power += dbmToMilliwatt(source.powerAtOneMetreDbm) / distanceSquared;
  }
  return 10 * Math.log10(power);
}
function generate({seed = PROTOCOL.defaultSeed, trials = PROTOCOL.trialsPerArm.default} = {}) {
  validParameters(seed, trials);
  const random = rng(seed), sim = PROTOCOL.simulation;
  const normal = () => Math.sqrt(-2 * Math.log(Math.max(random(), Number.EPSILON))) * Math.cos(2 * Math.PI * random());
  const shuffle = values => {
    for (let i = values.length - 1; i > 0; i--) { const j = Math.floor(random() * (i + 1)); [values[i], values[j]] = [values[j], values[i]]; }
    return values;
  };
  const schedule = shuffle(PROTOCOL.arms.flatMap(arm => Array.from({length: trials}, (_, index) => ({arm, index}))));
  const dataset = {schema: 'ruqu.boundary.measurements.v1', evidenceClass: 'classical-simulation',
    room: ROOM, sensors: SENSORS, trials: []};
  const truth = [];
  for (const [sequence, {arm, index}] of schedule.entries()) {
    const id = `trial${String(sequence).padStart(4, '0')}`;
    const count = arm === 'signal' ? 1 + index % 3 : Number(arm === 'model-mismatch');
    const frequencies = shuffle([...CHANNELS]);
    const sources = Array.from({length: count}, (_, i) => ({frequencyHz: frequencies[i],
      positionM: [0, 1].map(() => sim.sourceMarginM + random() * (2 - 2 * sim.sourceMarginM)),
      powerAtOneMetreDbm: sim.minimumPowerAtOneMetreDbm + random() *
        (sim.maximumPowerAtOneMetreDbm - sim.minimumPowerAtOneMetreDbm)}));
    const injectedSensor = Math.floor(random() * SENSORS.length);
    const missingSensor = random() < sim.missingSensorProbability ? Math.floor(random() * SENSORS.length) : -1;
    const channels = CHANNELS.map(frequencyHz => ({frequencyHz, rssiDbm: SENSORS.map((sensor, i) => {
      if (i === missingSensor && arm === 'signal') return null;
      let value = receivedPowerDbm(sensor.positionM, sources.filter(source => source.frequencyHz === frequencyHz), sim.noiseFloorDbm);
      if (arm === 'single-sensor-injection' && i === injectedSensor && frequencyHz === frequencies[0]) value = -35;
      if (arm === 'model-mismatch' && frequencyHz === frequencies[0]) value += (i % 2 ? 1 : -1) * sim.mismatchAlternatingGainDb;
      return value + sensor.offsetDb + (2 * random() - 1) * sim.calibrationDriftDb + normal() * sim.measurementSigmaDb;
    })}));
    dataset.trials.push({id, channels});
    truth.push({id, arm, sources, injectedSensor: arm === 'single-sensor-injection' ? injectedSensor : null});
  }
  validateDataset(dataset);
  return {dataset, truth};
}
module.exports = {generate, receivedPowerDbm};
