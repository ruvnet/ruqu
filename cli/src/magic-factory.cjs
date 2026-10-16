// SPDX-License-Identifier: MIT
'use strict';
// Numerical execution is exclusively the shared ruqu-core Rust implementation.
const { performance } = require('node:perf_hooks');
let wasm;
function jsonValue(v) {
  if (v === null || typeof v === 'string' || typeof v === 'boolean') return;
  if (typeof v === 'number' && Number.isFinite(v)) return;
  if (Array.isArray(v)) { for (const item of v) jsonValue(item); return; }
  if (v && Object.getPrototypeOf(v) === Object.prototype) { for (const value of Object.values(v)) jsonValue(value); return; }
  throw Error('Research requests require finite JSON values');
}
function execute(request) {
  jsonValue(request);
  const encoded=JSON.stringify(request);
  if (Buffer.byteLength(encoded)>1048576) throw Error('Research request exceeds 1 MiB');
  if (!wasm) wasm=require('../wasm/ruqu_wasm.js');
  if (typeof wasm.research_run_json !== 'function') throw Error('Bundled WASM lacks the shared research API; reinstall a current package');
  try { return JSON.parse(wasm.research_run_json(encoded)); }
  catch (error) { throw Error(error instanceof Error ? error.message : String(error)); }
}
function optionsRequest(op,options) {
  if (!options || Object.getPrototypeOf(options)!==Object.prototype) throw Error('Options must be an object');
  if (Object.hasOwn(options,'op')) throw Error('Options cannot override research operation');
  return execute({op,...options});
}
const exact=input_error=>execute({op:'magic-exact',input_error});
const sample=(input_error,shots,seed)=>execute({op:'magic-sample',input_error,shots,seed});
const resources=(input_error,options={})=>optionsRequest('magic-resources',{...options,input_error});
const run=(options={})=>optionsRequest('magic-factory',options);
const classify=mask=>execute({op:'magic-classify',mask});
const spectrum=Object.freeze(execute({op:'magic-spectrum'}));
// CLI parsing validates integer seed text without converting it to a JS Number.
function seedValue(value) {
  if (typeof value!=='string' || !/^[0-9]+$/.test(value) || BigInt(value)>18446744073709551615n) throw Error('seed must be a decimal unsigned 64 bit integer');
  return value;
}
function research(){return {schema_version:1,kind:'literature_profiles_not_ruqu_measurements',profiles:[
  {name:'Willow cultivation',source:'https://arxiv.org/abs/2512.13908',acceptance:0.08,reported_fidelity:0.9999,reported_fidelity_uncertainty:0.0001,expected_attempts_per_accepted_state:12.5,physical_qubit_rounds:null,caveat:'Reported retained state result before full surface code transfer; fidelity is a point estimate with uncertainty, not a certified error bound for a new device.'},
  {name:'Unfolded distillation',source:'https://www.nature.com/articles/s41534-026-01197-z',physical_qubits:53,qec_rounds:5.5,reported_logical_error:3e-7,noise_bias_min:5e6,phase_flip_rate:0.001,acceptance:null,caveat:'Theoretical result under highly biased noise. Missing acceptance prevents cost per accepted state calculation.'},
  {name:'Clifford stabilizer simulation',source:'https://arxiv.org/abs/2609.16929',status:'published f3 cultivation circuit exposed via research JSON; general Clifford stabilizer simulation algorithm not implemented'},
  {name:'Efficient logical magic state preparation simulation',source:'https://arxiv.org/abs/2512.23799',status:'research direction; sparse Pauli propagation in ruqu is a bounded reference, not a reproduction of this algorithm'}
]};}
function benchmark(){const start=performance.now();const grid=[0,1e-10,0.001,0.01,0.05,0.1,0.5,1].map(exact);const counts=sample(0.1,25000,'42');const expected=exact(0.1);const [lo,hi]=counts.confidence.conditional_output_error;const pass=counts.accepted+counts.rejected===counts.attempts&&counts.logical_errors<=counts.accepted&&expected.conditional_output_error>=lo&&expected.conditional_output_error<=hi;return {schema_version:1,kind:'synthetic_model_benchmark',backend:'ruqu-core-wasm',pass,elapsed_ms:performance.now()-start,grid,sample:counts,hardware_run:false};}
function parse(args){const options={},allowed=new Set(['input-error','shots','seed','rounds','block-qubits','block-rounds','raw-qubits','raw-rounds','transfer-acceptance','transfer-error','transfer-qubits','transfer-rounds']);
  if(args.length===1&&args[0]==='--research')return {research:true};if(args.length===1&&args[0]==='--benchmark')return {benchmark:true};
  for(let i=0;i<args.length;i+=2){const flag=args[i];const key=flag?.slice(2);if(!flag?.startsWith('--')||!allowed.has(key)||i+1>=args.length||Object.hasOwn(options,key.replaceAll('-','_')))throw Error('Usage: magic-factory [--input-error P --shots N --seed N --rounds N] | --research | --benchmark');const v=args[i+1];if(key==='seed'){seedValue(v);options.seed=v;}else {if(!/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(v))throw Error(`Invalid numeric value for ${flag}`);options[key.replaceAll('-','_')]=Number(v);}}
  return options;
}
module.exports={execute,exact,sample,resources,run,research,benchmark,parse,classify,spectrum};
