'use strict';
const assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const {mkdtempSync, writeFileSync, rmSync} = require('node:fs');
const {tmpdir} = require('node:os');
const path = require('node:path');
const root = path.resolve(__dirname, '../..');
const dir = mkdtempSync(path.join(tmpdir(), 'ruqu-boundary-package-'));
function execute(command, args, cwd = root) {
  const result = spawnSync(command, args, {cwd, encoding: 'utf8', timeout: 30000, maxBuffer: 16 * 1024 * 1024});
  if (result.error || result.status !== 0) throw Error(result.error?.message || result.stderr);
  return result.stdout;
}
try {
  const packed = JSON.parse(execute('npm', ['pack', '--ignore-scripts', '--json', '--pack-destination', dir], path.join(root, 'cli')))[0];
  execute('tar', ['-xzf', path.join(dir, packed.filename), '-C', dir]);
  const command = path.join(dir, 'package/bin/cli.js');
  const invoke = args => JSON.parse(execute(process.execPath, [command, 'boundary-lab', ...args], dir));
  const plan = invoke(['plan']);
  const bundle = invoke(['run', '--protocol-hash', plan.protocolHash]);
  assert.equal(plan.protocolHash, require('../../cli/src/boundary-lab.cjs').plan().protocolHash);
  const report = path.join(dir, 'report.json');
  writeFileSync(report, JSON.stringify(bundle), {flag: 'wx'});
  assert.equal(invoke(['verify', '--file', report, '--trusted-root', bundle.root]).verified, true);
  assert.equal(bundle.report.metrics.verdict, 'supported-in-model');
  console.log(JSON.stringify({package: packed.name, version: packed.version, verified: true,
    protocolHash: plan.protocolHash, root: bundle.root,
    scope: 'Extracted package executes offline without installing dependencies; nothing published'}, null, 2));
} finally { rmSync(dir, {recursive: true, force: true}); }
