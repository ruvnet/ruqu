'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const cli=path.resolve(__dirname,'../bin/cli.js');
const api=require('../src/magic-factory.cjs');
const invoke=(args,input)=>spawnSync(process.execPath,[cli,...args],{encoding:'utf8',input});
test('research JSON CLI reads stdin and preserves full unsigned64 seed',()=>{
  const request={op:'magic-sample',input_error:.01,shots:100,seed:'18446744073709551615'};
  const r=invoke(['research'],JSON.stringify(request));
  assert.equal(r.status,0,r.stderr);
  assert.deepEqual(JSON.parse(r.stdout),api.execute(request));
  assert.equal(JSON.parse(r.stdout).seed,'18446744073709551615');
});
test('research command rejects duplicate fields, booleans, unknown keys and unbounded work',()=>{
  for(const request of ['{"op":"magic-sample","shots":1,"shots":2}',JSON.stringify({op:'magic-factory',shots:true}),JSON.stringify({op:'magic-factory',shots:5000001}),JSON.stringify({op:'magic-factory',unexpected:1})]) {
    const r=invoke(['research','--json',request]);assert.equal(r.status,1,request);assert.equal(r.stdout,'');
  }
  assert.equal(invoke(['research','--json','{}','--input','x']).status,1);
});
test('research file CLI routes bounded Pauli propagation',()=>{
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'ruqu-research-'));
  try {
    const request={op:'pauli-evaluate',inputs:['zero','zero'],operations:[{kind:'gate',name:'h',qubit:0},{kind:'gate',name:'cnot',qubit:0,target:1}],observable:'ZZ',max_terms:64,max_work:10000};
    const file=path.join(dir,'request.json');fs.writeFileSync(file,JSON.stringify(request));
    const r=invoke(['research','--input',file]);assert.equal(r.status,0,r.stderr);assert.equal(JSON.parse(r.stdout).conditional_expectation,1);
    fs.writeFileSync(file,' '.repeat(1048577));assert.equal(invoke(['research','--input',file]).status,1);
  } finally {fs.rmSync(dir,{recursive:true,force:true});}
});
test('research memory dispatch is wired without running the agent harness',()=>{
  const r=invoke(['research-memory','import']);assert.equal(r.status,1);assert.match(r.stderr,/research-memory requires --input/);assert.doesNotMatch(r.stderr,/Unknown command/);
});
test('JSON wrapper refuses nonfinite or undefined values before JSON coercion',()=>{
  for(const v of [{op:'magic-factory',input_error:Infinity},{op:'magic-factory',input_error:undefined},{op:'magic-sample',shots:1,seed:42}])assert.throws(()=>api.execute(v));
  assert.throws(()=>api.run({op:'magic-exact'}));
});
