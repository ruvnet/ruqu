// SPDX-License-Identifier: MIT
'use strict';
// Exercise the native executable and both actual generated WASM interfaces.
// The web target is initialized from its bytes under Node; this checks its
// module/API contract, not browser rendering or HTTP MIME configuration.
const fs=require('node:fs');
const path=require('node:path');
const assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const root=path.resolve(__dirname,'..');
const nodeWasm=require('../cli/wasm/ruqu_wasm.js');
const cargo=process.env.RUQU_CARGO || 'cargo';
function native(text) {
  const r=spawnSync(cargo,['run','--locked','--quiet','--release','-p','ruqu-core','--example','research_api'],{cwd:root,input:text,encoding:'utf8',timeout:120000,maxBuffer:4*1024*1024});
  if(r.error)throw r.error;
  return r.status===0 ? {ok:true,value:JSON.parse(r.stdout)} : {ok:false,error:r.stderr.trim()};
}
function wasm(api,text) {try{return {ok:true,value:JSON.parse(api.research_run_json(text))};}catch(e){return {ok:false,error:String(e)};}}
function bits(x){const a=new DataView(new ArrayBuffer(8));a.setFloat64(0,x);const b=a.getBigUint64(0);return b>>63n ? (~b)&0xffffffffffffffffn : b|0x8000000000000000n;}
let floatComparisons=0,maxObservedUlp=0n;
function compare(a,b,label) {
  if(typeof a==='number' && typeof b==='number') {
    if(Number.isInteger(a) && Number.isInteger(b))return assert.equal(a,b,label);
    assert.ok(Number.isFinite(a)&&Number.isFinite(b),label+' nonfinite number');
    const distance=bits(a)>bits(b)?bits(a)-bits(b):bits(b)-bits(a);
    floatComparisons++;if(distance>maxObservedUlp)maxObservedUlp=distance;
    assert.ok(distance<=1n,`${label}: ${a} != ${b} (${distance} ULP)`);return;
  }
  if(Array.isArray(a)){assert.ok(Array.isArray(b),label);assert.equal(a.length,b.length,label);a.forEach((v,i)=>compare(v,b[i],`${label}[${i}]`));return;}
  if(a!==null && typeof a==='object'){assert.ok(b!==null&&typeof b==='object',label);assert.deepEqual(Object.keys(a).sort(),Object.keys(b).sort(),label);for(const k of Object.keys(a))compare(a[k],b[k],`${label}.${k}`);return;}
  assert.equal(a,b,label);
}
async function main(){
  const source=fs.readFileSync(path.join(root,'explorer/wasm/ruqu_wasm.js'),'utf8');
  const web=await import('data:text/javascript;base64,'+Buffer.from(source).toString('base64'));
  await web.default({module_or_path:fs.readFileSync(path.join(root,'explorer/wasm/ruqu_wasm_bg.wasm'))});
  const cases=[
    {op:'magic-factory',input_error:.01,shots:1000,seed:'42',rounds:2,transfer_acceptance:.9,transfer_error:1e-6,transfer_qubits:20,transfer_rounds:5},
    {op:'magic-exact',input_error:1e-10},
    {op:'magic-sample',input_error:.1,shots:1000,seed:'18446744073709551615'},
    {op:'magic-resources',input_error:.01,rounds:2},
    {op:'magic-classify',mask:7},
    {op:'magic-spectrum'},
    {op:'pauli-evaluate',inputs:['zero','zero'],operations:[{kind:'gate',name:'h',qubit:0},{kind:'gate',name:'cnot',qubit:0,target:1}],observable:'XX',max_terms:64,max_work:10000},
    {op:'pauli-evaluate',inputs:['t'],operations:[{kind:'pauli-fault',pauli:'Z',probability:.1}],observable:'X',max_terms:64,max_work:10000},
    {op:'hypergraph-decode',detector_count:128,logical_count:128,mechanisms:[{detectors:'170141183460469231731687303715884105728',logicals:'170141183460469231731687303715884105728',probability:.1}],syndrome:'170141183460469231731687303715884105728',max_nodes:100},
    {op:'cultivation',attempts:2,seed:'42'},
    {op:'cultivation',attempts:2,seed:'42',gate_error:.001,preparation_error:.001,measurement_error:.001}
  ];
  const passed=[];
  for(const [i,request] of cases.entries()){
    const text=JSON.stringify(request),n=native(text),v=wasm(nodeWasm,text),w=wasm(web,text);
    assert.equal(n.ok,true,`native ${request.op}: ${n.error}`);assert.equal(v.ok,true,`Node ${request.op}: ${v.error}`);assert.equal(w.ok,true,`web ${request.op}: ${w.error}`);
    compare(n.value,v.value,`${request.op}[${i}] native/Node`);compare(n.value,w.value,`${request.op}[${i}] native/web`);
    // Repeat the same seeded request against each initialized module; this
    // catches accidental unseeded RNG use and leftover state between calls.
    if(request.op==='cultivation') {compare(v.value,wasm(nodeWasm,text).value,'repeat Node cultivation');compare(w.value,wasm(web,text).value,'repeat web cultivation');}
    passed.push(request.op);
  }
  const invalid=[
    '{"op":"magic-sample","shots":1,"shots":2}',
    JSON.stringify({op:'magic-sample',shots:5000001}),
    JSON.stringify({op:'magic-factory',seed:'18446744073709551616'}),
    JSON.stringify({op:'magic-factory',shots:true}),
    JSON.stringify({op:'magic-factory',transfer_acceptance:null}),
    JSON.stringify({op:'magic-exact',unexpected:1}),
    JSON.stringify({op:'cultivation',attempts:257}),
    JSON.stringify({op:'pauli-evaluate',inputs:['t'],operations:[],observable:'X',max_terms:1,max_work:1}),
    JSON.stringify({op:'hypergraph-decode',detector_count:2,logical_count:1,mechanisms:[{detectors:'3',logicals:'1',probability:.1}],syndrome:'3',max_nodes:1}),
    JSON.stringify({op:'hypergraph-decode',detector_count:128,logical_count:1,mechanisms:[],syndrome:'340282366920938463463374607431768211456',max_nodes:100}),
    JSON.stringify({op:'pauli-evaluate',inputs:['zero'],operations:[{kind:'postselect',pauli:'Z',positive:1}],observable:'Z',max_terms:4,max_work:100}),
    ' '.repeat(1048577)
  ];
  for(const [i,text] of invalid.entries())for(const [backend,outcome] of [['native',native(text)],['Node',wasm(nodeWasm,text)],['web',wasm(web,text)]]){
    assert.equal(outcome.ok,false,`${backend} invalid case ${i} accepted`);assert.ok(outcome.error.length>0,`${backend} missing failure reason`);
  }
  console.log(JSON.stringify({schema_version:1,pass:true,interfaces:['native-release','nodejs-wasm','web-wasm-initialized-under-node'],successful_requests:passed.length,operations:[...new Set(passed)],rejected_cases:invalid.length,float_comparisons:floatComparisons,max_observed_ulp:Number(maxObservedUlp),float_tolerance_ulp:1,integer_comparison:'exact',hardware_run:false},null,2));
}
main().catch(e=>{console.error(e);process.exitCode=1;});
