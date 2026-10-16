'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const m = require('../src/research-memory.cjs');
// These are synthetic test fixtures that exercise the native-measured schema.
// They are not measurements, performance claims, or publishable research artifacts.
function fixture(seed=1, backend='baseline', runtimeMs=100, partition='holdout') {
  return m.createEvidence({ target: { circuitSha256:'a'.repeat(64), noise:{p:0.001}, seed, backend,
    environment:{machine:'synthetic-unit-test',platform:process.platform}, features:{qubits:7,gates:100},
    budget:{maxRuntimeMs:200,maxMemoryBytes:100000,maxOutputError:0.01,minAcceptance:0.5} },
    metrics:{runtimeMs,peakMemoryBytes:1000,outputError:0.001,attempts:100,accepted:80,rejected:20},
    provenance:{kind:'native-measured',runId:`synthetic-unit-test-${seed}-${backend}`,toolVersion:'test-fixture',sourceSha256:'b'.repeat(64),partition,measuredAt:new Date().toISOString()} });
}
test('identity includes full noise, budget, seed, backend and environment',()=>{
  const r=fixture();
  for (const patch of [{seed:2},{noise:{p:0.002}},{budget:{...r.target.budget,maxRuntimeMs:201}},{backend:'new'},{environment:{machine:'other'}}])
    assert.notEqual(m.exactKey({...r.target,...patch}),r.key);
  assert.equal(m.exactKey({...r.target,noise:{p:0.001}}),r.key);
  assert.equal(m.lookupExact([r],r.target).digest,r.digest);
  assert.equal(m.lookupExact([r],{...r.target,seed:2}),null);
});
test('rejects tamper, NaN, stale and future evidence',()=>{
  const r=fixture();
  assert.throws(()=>m.validateEvidence({...r,metrics:{...r.metrics,runtimeMs:1}}),/digest/);
  assert.throws(()=>m.exactKey({...r.target,seed:NaN}),/finite/);
  assert.throws(()=>m.validateEvidence(r,{now:Date.now()+31*86400000}),/Stale/);
  assert.throws(()=>m.createEvidence({...r,provenance:{...r.provenance,measuredAt:'2099-01-01'}}),/time/);
  assert.throws(()=>m.canonical(JSON.parse('{"__proto__":{}}')),/Unsafe/);
});
test('accounts all rejects and enforces budget checks',()=>{
  const r=fixture();
  assert.throws(()=>m.createEvidence({...r,metrics:{...r.metrics,rejected:0}}),/Unaccounted/);
  const slow=fixture(2,'slow',201);
  assert.equal(m.withinBudget(slow),false);
  assert.equal(m.recommend([slow],r.target).candidates.length,0);
});
test('recommendations are advisory, stale and synthetic sources do not choose backend',()=>{
  const r=fixture(1,'candidate',50), t=fixture().target;
  const result=m.recommend([r],t);
  assert.equal(result.advisoryOnly,true); assert.equal(result.selectedBackend,'baseline');
  assert.equal(result.candidates[0].backend,'candidate');
  assert.equal(m.recommend([r],t,{now:Date.now()+31*86400000}).candidates.length,0);
  const synthetic=m.createEvidence({...r,provenance:{...r.provenance,kind:'synthetic-test'}});
  assert.equal(m.recommend([synthetic],t).candidates.length,0);
  assert.throws(()=>m.recommend([r,r],t),/replayed/);
});
test('promotion needs locally validated distinct paired native holdouts',()=>{
  const pairs=[1,2].map(seed=>({baseline:fixture(seed),candidate:fixture(seed,'candidate',70)}));
  assert.equal(m.checkPromotion(pairs).eligible,false);
  const ledger=pairs.flatMap(p=>[p.baseline,p.candidate]).map(r=>({runId:r.provenance.runId,digest:r.digest}));
  assert.equal(m.checkPromotion(pairs,{locallyValidatedEvidence:ledger}).eligible,true);
  assert.equal(m.checkPromotion([pairs[0],pairs[0]],{locallyValidatedEvidence:ledger}).eligible,false);
  const bad=structuredClone(pairs); bad[0].candidate=fixture(1,'candidate',201);
  assert.equal(m.checkPromotion(bad,{locallyValidatedEvidence:ledger}).eligible,false);
  assert.equal(m.checkPromotion(pairs,{locallyValidatedEvidence:ledger,minImprovement:0.4}).eligible,false);
});
test('promotion approves content digests and rejects run ID replay with recomputed hashes',()=>{
  const pairs=[1,2].map(seed=>({baseline:fixture(seed),candidate:fixture(seed,'candidate',120)}));
  const ledger=pairs.flatMap(p=>[p.baseline,p.candidate]).map(r=>({runId:r.provenance.runId,digest:r.digest}));
  assert.equal(m.checkPromotion(pairs,{locallyValidatedEvidence:ledger}).eligible,false);
  const replaced=pairs.map(p=>({...p,candidate:m.createEvidence({...p.candidate,metrics:{...p.candidate.metrics,runtimeMs:1}})}));
  assert.equal(m.checkPromotion(replaced,{locallyValidatedEvidence:ledger}).eligible,false);
  assert.match(m.checkPromotion(replaced,{locallyValidatedEvidence:ledger}).reason,/digest/);
  const ids=ledger.map(r=>r.runId);
  assert.equal(m.checkPromotion(replaced,{locallyValidatedRunIds:ids}).eligible,false);
  assert.equal(m.checkPromotion(replaced,{locallyValidatedEvidence:ids}).eligible,false);
  assert.equal(m.checkPromotion(replaced,{locallyValidatedEvidence:[...ledger,ledger[0]]}).eligible,false);
});
test('seed identity is lossless for u64 inputs',()=>{
  const t=fixture().target;
  for(const seed of [9007199254740992,9007199254740993,-1,0.1,'18446744073709551616','-1','01','1.0','seed'])
    assert.throws(()=>m.exactKey({...t,seed}),/identity/);
  assert.notEqual(m.exactKey({...t,seed:'9007199254740992'}),m.exactKey({...t,seed:'9007199254740993'}));
  assert.doesNotThrow(()=>m.exactKey({...t,seed:'18446744073709551615'}));
  assert.throws(()=>m.exactKey({...t,noise:{p:1e308}}),/probability/);
  assert.throws(()=>m.exactKey({...t,noise:{physicalErrorRate:-0.1}}),/probability/);
  assert.equal(m.checkPromotion([],{minPairs:NaN}).eligible,false);
});
test('real official RVF WASM binary roundtrip, retrieval, byte and manifest tamper rejection',async()=>{
  const dir=await fs.mkdtemp(path.join(os.tmpdir(),'ruqu-memory-test-')), file=path.join(dir,'memory.rvf');
  try {
    const records=[fixture(),fixture(2,'candidate',60)];
    await m.exportMemory(records,file);
    const bytes=await fs.readFile(file);
    assert.notEqual(bytes[0],'{'.charCodeAt(0));
    const memory=await m.importMemory(file);
    assert.deepEqual(memory.records,records); assert.equal(memory.trustedForPromotion,false);
    const query=await m.queryMemory(file,records[0].target);
    assert.equal(query.advisoryOnly,true); assert.equal(query.results.length,2);
    assert.equal(query.results[0].distance,0);
    await assert.rejects(m.exportMemory(records,file),/EEXIST/);
    const tampered=Buffer.from(bytes); tampered[tampered.length-1]^=1;
    await fs.writeFile(file,tampered);
    await assert.rejects(m.importMemory(file),/integrity/);
    await fs.writeFile(file,bytes);
    const sidecar=JSON.parse(await fs.readFile(file+'.json','utf8')); sidecar.records[0].metrics.runtimeMs=0;
    await fs.writeFile(file+'.json',JSON.stringify(sidecar));
    await assert.rejects(m.importMemory(file),/integrity/);
  } finally { await fs.rm(dir,{recursive:true,force:true}); }
});
