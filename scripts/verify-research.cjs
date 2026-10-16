// SPDX-License-Identifier: MIT
'use strict';
// Runs actual Rust/Node entrypoints and checks shared contracts. No QPU or keys.
const {spawnSync}=require('node:child_process');
const fs=require('node:fs');const path=require('node:path');const crypto=require('node:crypto');const assert=require('node:assert/strict');
const root=path.resolve(__dirname,'..');const factory=require('../cli/src/magic-factory.cjs');
function command(cmd,args){const r=spawnSync(cmd,args,{cwd:root,encoding:'utf8',timeout:120000,maxBuffer:8*1024*1024});if(r.error||r.status!==0)throw Error(`${cmd} ${args.join(' ')} failed: ${r.error?.message??r.stderr}`);return r.stdout;}
function native(example,args=[]){return JSON.parse(command('cargo',['run','--locked','--quiet','-p','ruqu-core','--example',example,'--',...args]));}
function close(a,b){assert.ok(Number.isFinite(a)&&Number.isFinite(b));assert.ok(Math.abs(a-b)<=1e-12*Math.max(Math.abs(a),Math.abs(b),Number.MIN_VALUE)+Number.MIN_VALUE,`${a} != ${b}`);}
const grid=[];
for(const input of ['0','0.001','0.01','0.1','0.5','1']){
  const r=native('magic_states',['--input-error',input,'--shots','1000','--seed','42','--rounds','2']);
  const j=factory.run({input_error:Number(input),shots:1000,seed:'42',rounds:2,transfer_acceptance:.9,transfer_error:1e-6,transfer_qubits:20,transfer_rounds:5});
  close(r.acceptance,j.exact.acceptance_probability);close(r.conditional_output_error,j.exact.conditional_output_error);
  for(const field of ['attempts','accepted','rejected','logical_errors'])assert.equal(r[field],j.sample[field]);
  for(const field of ['expected_raw_states','expected_stage_attempts','expected_qubit_rounds','output_error_upper_bound'])close(r.factory_transfer[field],j.resources[field]);
  grid.push({input_error:Number(input),acceptance:r.acceptance,conditional_output_error:r.conditional_output_error,accepted:r.accepted,rejected:r.rejected,logical_errors:r.logical_errors,rust_node_agreement:true});
}
const encoded=native('pauli_propagation');assert.equal(encoded.qubits,128);assert.equal(encoded.peak_terms,1);close(encoded.expectation,Math.SQRT1_2);
const decoder=native('hypergraph_decoder');assert.equal(decoder.selected_mechanisms,1);assert.equal(decoder.logical_mask,1);
const benchmark=native('magic_states',['--benchmark']);benchmark.build_profile='debug';assert.equal(benchmark.matching_spectra,true);
const fileNames=[...command('git',['diff','--name-only']).trim().split('\n'),...command('git',['ls-files','--others','--exclude-standard']).trim().split('\n')].filter(Boolean).filter(p=>!p.startsWith('docs/validation/'));
const hashes=Object.fromEntries([...new Set(fileNames)].sort().map(p=>[p,crypto.createHash('sha256').update(fs.readFileSync(path.join(root,p))).digest('hex')]));
const report={schema_version:1,kind:'local_classical_model_acceptance',generated_at:new Date().toISOString(),base_commit:command('git',['rev-parse','HEAD']).trim(),node:process.version,rust:command('rustc',['--version']).trim(),platform:`${process.platform}/${process.arch}`,hardware_run:false,pass:true,grid,encoded_t:encoded,correlated_decoder:decoder,enumerator_benchmark:benchmark,file_sha256:hashes};
if(process.argv.length>2){if(process.argv.length!==4||process.argv[2]!=='--output')throw Error('Usage: node scripts/verify-research.cjs [--output PATH]');fs.writeFileSync(path.resolve(root,process.argv[3]),JSON.stringify(report,null,2)+'\n');}
console.log(JSON.stringify(report,null,2));
