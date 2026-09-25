'use strict';
const assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const {mkdtempSync, mkdirSync, writeFileSync, rmSync} = require('node:fs');
const {tmpdir} = require('node:os');
const path = require('node:path');
const command = path.resolve(__dirname, '../../cli/bin/cli.js');
function invoke(args) {
  const result = spawnSync(process.execPath, [command, 'boundary-lab', ...args],
    {encoding: 'utf8', timeout: 30000, maxBuffer: 16 * 1024 * 1024});
  if (result.error || result.status !== 0) throw Error(result.error?.message || result.stderr);
  return JSON.parse(result.stdout);
}
const args = process.argv.slice(2);
if (args.length && (args.length !== 2 || !['--output', '--summary'].includes(args[0])))
  throw Error('Usage: node examples/boundary-lab/run.cjs [--output NEW_DIRECTORY | --summary NEW_FILE]');
const output = args[0] === '--output' ? path.resolve(args[1]) : null;
if (output) mkdirSync(output);
const dir = output || mkdtempSync(path.join(tmpdir(), 'ruqu-boundary-'));
const write = (name, data) => {
  const file = path.join(dir, `${name}.json`);
  writeFileSync(file, JSON.stringify(data, null, 2) + '\n', {flag: 'wx'});
  return file;
};
try {
  const plan = invoke(['plan']);
  const bundle = invoke(['run', '--protocol-hash', plan.protocolHash]);
  const root = bundle.root;
  write('plan', plan);
  const file = write('report', bundle), measurements = write('measurements', bundle.report.dataset);
  const predictions = invoke(['predict', '--file', measurements]);
  assert.deepEqual(predictions, bundle.report.predictionBundle);
  write('predictions', predictions);
  const verification = invoke(['verify', '--file', file, '--trusted-root', root]);
  const projection = invoke(['project', '--file', file, '--trusted-root', root]);
  write('projection', projection);
  assert.equal(bundle.report.metrics.verdict, 'supported-in-model');
  const {rows, ...metrics} = bundle.report.metrics;
  const summary = {schema: 'ruqu.boundary.e2e.v1', evidenceClass: bundle.report.evidenceClass,
    protocolHash: plan.protocolHash, root, parameters: bundle.report.parameters,
    trialCount: rows.length, separatePredictorMatched: true, verification, metrics,
    projection: {fieldEvents: projection.fieldEvents.length, nodes: projection.worldGraph.nodes.length,
      edges: projection.worldGraph.edges.length, integrationStatus: projection.integrationStatus},
    limits: bundle.report.limits};
  if (output) write('summary', summary);
  if (args[0] === '--summary') writeFileSync(path.resolve(args[1]), JSON.stringify(summary, null, 2) + '\n', {flag: 'wx'});
  console.log(JSON.stringify(summary, null, 2));
} finally { if (!output) rmSync(dir, {recursive: true, force: true}); }
