// SPDX-License-Identifier: MIT
'use strict';
// Single loader for the quantum engine. The CLI depends on the published
// @ruvector/ruqu-wasm package (a wasm-bindgen `web` build) instead of bundling
// its own copy. Node loads its ESM entry through require(esm), available
// without flags from Node 20.19 and 22.12, then instantiates it synchronously
// from the exact bytes whose SHA-256 is reported as evidence.
const {createHash} = require('node:crypto');
const {readFileSync} = require('node:fs');
const path = require('node:path');

const PACKAGE = '@ruvector/ruqu-wasm';
let engine;

function load() {
  if (engine) return engine;
  let api, packageJsonPath;
  try {
    packageJsonPath = require.resolve(`${PACKAGE}/package.json`);
    api = require(PACKAGE);
  } catch (error) {
    if (error && error.code === 'ERR_REQUIRE_ESM') {
      throw Error(`${PACKAGE} is an ES module; this Node ${process.version} cannot require() it. Use Node >=20.19 or >=22.12.`);
    }
    throw Error(`Cannot load ${PACKAGE}: ${error instanceof Error ? error.message : String(error)}. Reinstall @ruvector/ruqu.`);
  }
  const dir = path.dirname(packageJsonPath);
  const wasmPath = path.join(dir, 'ruqu_wasm_bg.wasm');
  const bytes = readFileSync(wasmPath);
  if (typeof api.initSync !== 'function') throw Error(`${PACKAGE} does not export initSync; install ${PACKAGE} ^2.2.0`);
  api.initSync({module: bytes});
  const {version} = JSON.parse(readFileSync(packageJsonPath, 'utf8'));
  engine = {api, version, wasmPath, sha256: createHash('sha256').update(bytes).digest('hex')};
  return engine;
}

module.exports = {load, quantum: () => load().api};
