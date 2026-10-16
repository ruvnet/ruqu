#!/usr/bin/env node
'use strict';
// Genuine software measurements of the Rust/WASM ideal 15-to-1 error model.
// This is not a hardware measurement or a physical cultivation reproduction.
const fs = require('node:fs/promises');
const path = require('node:path');
const crypto = require('node:crypto');
const { performance } = require('node:perf_hooks');
const { spawnSync } = require('node:child_process');
const { pathToFileURL } = require('node:url');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..');
const memory = require('../cli/src/research-memory.cjs');
const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');

async function worker(seed) {
  const { run } = await import(pathToFileURL(path.join(root,'cli/bin/cli.js')).href);
  const start = performance.now();
  const code = await run(['magic-factory','--input-error','0.01','--shots','1000','--seed',seed,
    '--rounds','1','--block-qubits','15','--block-rounds','10','--raw-qubits','1','--raw-rounds','1']);
  const elapsedMs = performance.now()-start;
  if (code !== 0) throw Error('Factory CLI returned failure');
  process.stderr.write(JSON.stringify({cliElapsedMs:elapsedMs,maxRssBytes:process.resourceUsage().maxRSS*1024})+'\n');
}

async function main() {
  if (process.argv[2] === '--worker') return worker(process.argv[3]);
  const args = process.argv.slice(2);
  if (args.length && (args.length !== 2 || args[0] !== '--output-dir')) throw Error('Usage: node scripts/measure-research-memory.cjs [--output-dir DIR]');
  const outputDir = args.length ? path.resolve(args[1]) : path.join(root,'docs/research');
  await fs.mkdir(outputDir,{recursive:true});
  const verification = spawnSync(process.execPath,[path.join(root,'scripts/wasm-manifest.cjs')],{cwd:root,encoding:'utf8'});
  if (verification.status !== 0) throw Error('WASM manifest verification failed: '+verification.stderr);
  const manifestBytes = await fs.readFile(path.join(root,'docs/research/wasm-manifest.json'));
  const manifest = JSON.parse(manifestBytes);
  const sourceSha256 = manifest.sha256['crates/ruqu-core/src/magic_states.rs'];
  const wasmSha256 = manifest.sha256['cli/wasm/ruqu_wasm_bg.wasm'];
  const packageJson = JSON.parse(await fs.readFile(path.join(root,'cli/package.json')));
  const model = {model:'ideal_15_to_1_independent_Z',model_version:1,sourceSha256,
    cost_inputs:{rounds:1,block_qubits:15,block_rounds:10,raw_qubits:1,raw_rounds:1}};
  const circuitSha256 = digest(memory.canonical(model));
  const records = [], rawRuns = [];
  for (const seed of ['42','43']) {
    const started = performance.now();
    const child = spawnSync(process.execPath,[__filename,'--worker',seed],{cwd:root,encoding:'utf8',maxBuffer:1024*1024,timeout:30000});
    const processElapsedMs = performance.now()-started;
    if (child.error || child.status !== 0) throw Error('Measurement worker failed: '+(child.error?.message || child.stderr));
    const result = JSON.parse(child.stdout), measured = JSON.parse(child.stderr);
    assert.equal(result.hardware_run,false);
    assert.equal(result.physical_cultivation_reproduction,false);
    assert.equal(result.sample.seed,seed);
    assert.equal(result.sample.attempts,1000);
    assert.equal(result.sample.accepted+result.sample.rejected,1000);
    const measuredAt = new Date().toISOString();
    const record = memory.createEvidence({
      target:{circuitSha256,circuit_identity_type:'software-protocol-model',model,
        noise:{model:'independent_input_Z',p:0.01,clifford_operations:'ideal',syndrome_checks:'ideal'},
        seed,shots:1000,backend:'ruqu-core-wasm',
        environment:{platform:process.platform,architecture:process.arch,node:process.version,
          sourceSha256,wasmSha256,manifestSha256:digest(manifestBytes),toolVersion:packageJson.version},
        features:{qubits:15},
        budget:{maxRuntimeMs:10000,maxMemoryBytes:512*1024*1024,maxOutputError:0.001,minAcceptance:0.75}},
      metrics:{runtimeMs:processElapsedMs,peakMemoryBytes:measured.maxRssBytes,
        outputError:result.exact.conditional_output_error,attempts:result.sample.attempts,
        accepted:result.sample.accepted,rejected:result.sample.rejected,
        cliElapsedMs:measured.cliElapsedMs,
        acceptanceExact:result.exact.acceptance_probability,
        logicalErrorsSampled:result.sample.logical_errors,
        costs:result.resources,
        runtimeDefinition:'Elapsed parent monotonic time for a fresh Node worker invoking the actual CLI run method, including process startup and shutdown',
        memoryDefinition:'Node process.resourceUsage().maxRSS times 1024: cumulative process resident high-water upper bound, including Node, CLI and WASM; not isolated simulator allocation',
        outputErrorDefinition:'Exact ideal independent-input-Z model conditional output error, not a hardware estimate or sampled zero-error bound'},
      provenance:{kind:'native-measured',executionKind:'measured-rust-wasm-software-model',
        runId:'ruqu-wasm-factory-'+seed+'-'+crypto.randomUUID(),toolVersion:packageJson.version,
        sourceSha256,measuredAt,partition:'development',hardware_run:false,
        physical_cultivation_reproduction:false}
    });
    records.push(record);
    rawRuns.push({seed,result,measurement:measured,processElapsedMs});
  }
  const rvf = path.join(outputDir,'ruqu-research.rvf');
  const exportResult = await memory.exportMemory(records,rvf);
  const imported = await memory.importMemory(rvf);
  assert.deepEqual(imported.records,records);
  assert.equal(imported.trustedForPromotion,false);
  const queries = [];
  for (const record of records) {
    assert.equal(memory.lookupExact(imported.records,record.target).digest,record.digest);
    const query = await memory.queryMemory(rvf,record.target);
    assert.equal(query.advisoryOnly,true);
    assert.equal(query.selectedBackend,record.target.backend);
    assert(query.results.some(row=>row.record.digest===record.digest && row.distance===0));
    queries.push({seed:record.target.seed,exactKey:record.key,exactDigest:record.digest,
      matchingDigestRetrieved:record.digest,distance:0});
  }
  const measuredFile = path.join(outputDir,'ruqu-research-measured.json');
  await fs.writeFile(measuredFile,JSON.stringify(records,null,2)+'\n',{flag:'wx'});
  await fs.writeFile(path.join(outputDir,'ruqu-research-measurement-report.json'),JSON.stringify({
    schema:'ruqu-research-software-measurements/v1',model,sourceSha256,wasmSha256,
    scriptSha256:digest(await fs.readFile(__filename)),export:exportResult,rawRuns,queries,
    verdict:'Both measured software model records exported, imported and retrieved with exact matching keys and digests. No backend promotion or comparative performance claim.',
    caveats:['Ideal 15-to-1 independent-Z software model, not the physical cultivation circuit.',
      'Cost defaults are illustrative expected resource estimates, not measured hardware costs.',
      'maxRSS is a cumulative Node process high-water upper bound, not isolated WASM memory.',
      'Seeds 42 and 43 are development examples, not predeclared holdout trials.']
  },null,2)+'\n',{flag:'wx'});
  console.log(JSON.stringify({records:records.length,rvf,rvfBytes:(await fs.stat(rvf)).size,
    accepted:records.map(r=>r.metrics.accepted),runtimesMs:records.map(r=>r.metrics.runtimeMs),
    exactOutputError:records[0].metrics.outputError,roundtripVerified:true}));
}
main().catch(error=>{console.error(error.message);process.exitCode=1;});
