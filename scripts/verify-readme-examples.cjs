// SPDX-License-Identifier: MIT
'use strict';
// Keep the @ruvector/ruqu-wasm README honest.
//
//   node scripts/verify-readme-examples.cjs [packageDir]
//
// packageDir defaults to npm/packages/ruqu-wasm. After a publish, point it at a
// freshly installed node_modules/@ruvector/ruqu-wasm and it checks the
// published README against the published files.
//
// Conventions for README.md in packageDir:
//   ```js / ```javascript  executed verbatim as an ES module in Node, with
//                          '@ruvector/ruqu-wasm' resolving to packageDir. A
//                          fetch shim serves file: URLs so the browser-style
//                          `await init()` example runs unchanged.
//   ```text right after a  the exact stdout that block must print.
//   js block
//   ```ts                  every declaration line must appear in ruqu_wasm.d.ts.
//   ```bash                not executed.
// Every block's imports from '@ruvector/ruqu-wasm' must be real exports.
// The documented engine behaviours are also asserted directly below.
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const { pathToFileURL } = require('node:url');

const root = path.resolve(__dirname, '..');
const defaultDir = path.join(root, 'npm/packages/ruqu-wasm');
const pkgDir = path.resolve(process.argv[2] || defaultDir);
const readme = fs.readFileSync(path.join(pkgDir, 'README.md'), 'utf8');
const dts = fs.readFileSync(path.join(pkgDir, 'ruqu_wasm.d.ts'), 'utf8');
if (pkgDir === defaultDir) {
  assert.equal(fs.readFileSync(path.join(root, 'crates/ruqu-wasm/README.md'), 'utf8'), readme,
    'crates/ruqu-wasm/README.md must match npm/packages/ruqu-wasm/README.md');
}

const blocks = [];
const fence = /^```([a-z]*)\n([\s\S]*?)^```$/gm;
for (let m; (m = fence.exec(readme));) blocks.push({ lang: m[1], code: m[2], end: m.index + m[0].length });

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'ruqu-readme-'));
const scoped = path.join(work, 'node_modules/@ruvector');
fs.mkdirSync(scoped, { recursive: true });
fs.symlinkSync(pkgDir, path.join(scoped, 'ruqu-wasm'), 'dir');
const shim = path.join(work, 'fetch-shim.mjs');
fs.writeFileSync(shim, `import { readFile } from 'node:fs/promises';
const realFetch = globalThis.fetch;
globalThis.fetch = async (input, init) => {
  const url = new URL(input instanceof Request ? input.url : String(input));
  if (url.protocol !== 'file:') return realFetch(input, init);
  return new Response(await readFile(url), { headers: { 'content-type': 'application/wasm' } });
};
`);

async function main() {
  const mod = await import(pathToFileURL(path.join(pkgDir, 'ruqu_wasm.js')).href);
  mod.initSync({ module: fs.readFileSync(path.join(pkgDir, 'ruqu_wasm_bg.wasm')) });
  const exportsSet = new Set(Object.keys(mod));
  let executed = 0, outputs = 0, declarations = 0, imports = 0;

  for (const [i, b] of blocks.entries()) {
    for (const m of b.code.matchAll(/import\s+([^;'"]*?)\s+from\s+['"]@ruvector\/ruqu-wasm['"]/g)) {
      const clause = m[1];
      if (/^\w+/.test(clause) && !clause.startsWith('{')) { assert.ok(exportsSet.has('default'), 'default export'); imports++; }
      const named = clause.match(/\{([^}]*)\}/);
      for (const n of named ? named[1].split(',') : []) {
        const name = n.trim().split(/\s+as\s+/)[0];
        if (!name) continue;
        assert.ok(exportsSet.has(name), `README block ${i} imports '${name}', which @ruvector/ruqu-wasm does not export`);
        imports++;
      }
    }
    if (b.lang === 'ts') {
      for (const line of b.code.split('\n').map((l) => l.trim()).filter((l) => l && l !== '}')) {
        assert.ok(dts.includes(line), `API reference line not in ruqu_wasm.d.ts: ${line}`);
        declarations++;
      }
    }
    if (b.lang === 'js' || b.lang === 'javascript') {
      const file = path.join(work, `block-${i}.mjs`);
      fs.writeFileSync(file, b.code);
      const r = spawnSync(process.execPath, ['--import', pathToFileURL(shim).href, file],
        { cwd: work, encoding: 'utf8', timeout: 60000 });
      assert.equal(r.status, 0, `README block ${i} failed:\n${b.code}\n${r.stderr}`);
      executed++;
      const next = blocks[i + 1];
      if (next && next.lang === 'text' && /^\s*$/.test(readme.slice(b.end, readme.indexOf('```text', b.end)))) {
        assert.equal(r.stdout, next.code, `README block ${i} printed something other than the documented output`);
        outputs++;
      }
    }
  }
  assert.ok(executed >= 5, `expected at least 5 runnable examples, found ${executed}`);

  // Documented behaviours, asserted directly against the engine.
  const { WasmQuantumCircuit, simulate } = mod;
  let c = new WasmQuantumCircuit(2); c.x(0);
  assert.deepEqual(simulate(c).probabilities, [0, 1, 0, 0], 'qubit 0 is the least significant bit');
  c = new WasmQuantumCircuit(2); c.x(1);
  assert.deepEqual(simulate(c).probabilities, [0, 0, 1, 0], 'qubit 1 is bit 1');
  for (const gate of ['rx', 'ry', 'rz']) for (const bad of [NaN, Infinity, -Infinity]) {
    c = new WasmQuantumCircuit(1);
    assert.throws(() => c[gate](0, bad), new RegExp(`^${gate}: angle must be a finite number`));
    assert.equal(c.gate_count, 0);
  }
  c = new WasmQuantumCircuit(2);
  assert.throws(() => c.rzz(0, 1, NaN), /^rzz: angle must be a finite number/);
  c = new WasmQuantumCircuit(1); c.h(0); c.measure(0);
  const seeded = (s) => simulate(c, s).measurements[0].result;
  const run = Array.from({ length: 64 }, (_, s) => seeded(s));
  assert.deepEqual(run, Array.from({ length: 64 }, (_, s) => seeded(s)), 'seeded measurement is reproducible');
  assert.ok(run.includes(true) && run.includes(false), 'seeds explore both outcomes');
  const unseeded = new Set(Array.from({ length: 64 }, () => simulate(c).measurements[0].result));
  assert.equal(unseeded.size, 2, 'unseeded measurement is random');
  assert.equal(simulate(c, null).measurements.length, 1);
  for (const bad of [-1, 1.5, NaN, Infinity, 2 ** 53]) assert.throws(() => simulate(c, bad), /^seed must be an integer/);
  assert.throws(() => new WasmQuantumCircuit(26), (e) => typeof e === 'string' && e.startsWith('Qubit limit exceeded'));
  c = new WasmQuantumCircuit(2); c.h(0); c.cnot(0, 1); c.measure(0);
  assert.equal(simulate(c, 1).gate_count, 2, 'measure is not counted in gate_count');
  assert.equal(mod.estimate_memory(25), 512 * 1024 * 1024);

  console.log(JSON.stringify({ pkgDir, executed, outputs, declarations, imports, behaviours: 'pass' }));
}
main().finally(() => fs.rmSync(work, { recursive: true, force: true })).catch((e) => { console.error(e); process.exit(1); });
