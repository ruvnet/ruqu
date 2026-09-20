'use strict';
// Actual JS producer -> compiled Rust verifier -> real upstream graph types.
// The root is trusted here because the test just created the fixture in memory.
// Do not obtain production trust anchors from an untrusted input document.
const assert = require('node:assert/strict');
const {spawnSync} = require('node:child_process');
const path = require('node:path');
const {run} = require('../../cli/src/observer-lab.cjs');
const binary = path.join(__dirname, 'target', 'debug', process.platform === 'win32' ? 'ruqu-observer-world.exe' : 'ruqu-observer-world');
function invoke(value, root, args=['--trusted-root',root]) {
  const result = spawnSync(binary,args,{input:typeof value==='string'?value:JSON.stringify(value),
    encoding:'utf8',maxBuffer:128*1024*1024,timeout:60000});
  if (result.error) throw result.error;
  return result;
}
function reject(value,root,args) {
  const result=invoke(value,root,args);
  assert.notEqual(result.status,0);
  assert.equal(result.stdout,'','failed conversion emitted partial output');
  assert.match(result.stderr,/observer-world:/);
}
let accepted=0,rejected=0;
for (const count of [1,3,128,10000]) {
  const fixture=run({count,seed:20260920});
  const root=fixture.ledger.root;
  const result=invoke(fixture,root);
  assert.equal(result.status,0,result.stderr);
  const output=JSON.parse(result.stdout);
  const changed=Math.ceil(count/4);
  assert.equal(output.schema,'ruqu.observer.world.v1');
  assert.equal(output.synthetic,true);
  assert.equal(output.eventCount,count);
  assert.equal(output.disagreementCount,changed);
  assert.equal(output.trustedRoot,root);
  assert.equal(output.worldgraph.schema_version,2);
  assert.equal(output.worldgraph.nodes.length,1+3*count);
  assert.equal(output.worldgraph.edges.length,3*count+changed);
  const nodes=new Map(output.worldgraph.nodes.map(n=>[n.id,n]));
  assert.equal(nodes.size,output.worldgraph.nodes.length);
  for (const edge of output.worldgraph.edges) {
    assert.ok(nodes.has(edge.from)); assert.ok(nodes.has(edge.to));
  }
  assert.equal(output.worldgraph.edges.filter(e=>e.edge.rel==='derived_from').length,2*count);
  assert.equal(output.worldgraph.edges.filter(e=>e.edge.rel==='contradicts').length,changed);
  output.fieldEvents.forEach((event,i)=>{
    assert.equal(event.provenance.synthetic,true);
    assert.equal(event.sensor.modality,'synthetic_sim');
    assert.equal(event.tensor.privacy_class,'P2');
    assert.equal(event.observation.attributes.evidence_row_hash,fixture.ledger.rows[i].hash);
    assert.equal(event.provenance.firmware_hash,'sha256:'+fixture.wasmSha256);
    assert.ok(!event.sensor.position_m);
    assert.ok(!event.observation.range_m);
  });
  if(count===128) assert.equal(invoke(fixture,root).stdout,result.stdout,'projection replay diverged');
  accepted++;
}
const fixture=run({count:8});
const root=fixture.ledger.root;
for(const mutate of [
  x=>{x.ledger.rows[0].record.alice^=1;},
  x=>{x.ledger.rows.pop();x.memories.pop();},
  x=>{x.memories[0].bobRecall^=1;},
  x=>{x.backend='real QPU';},
  x=>{x.wasmSha256='f'.repeat(64);},
  x=>{x.ledger.rows[0].record.source='physical-sensor';},
  x=>{x.ledger.rows[0].record.protocol='alternate-realities';},
  x=>{x.ledger.rows[0].record.setting=[null,null];},
  x=>{x.ledger.rows[0].record.extra='unexpected';},
]) {
  const altered=structuredClone(fixture); mutate(altered); reject(altered,root); rejected++;
}
reject(fixture,'0'.repeat(64)); rejected++;
reject('{',root); rejected++;
reject(' '.repeat(32*1024*1024+1),root); rejected++;
reject(fixture,root,[]); rejected++;
reject(fixture,root,['--trusted-root',root,'--production']); rejected++;
console.log(JSON.stringify({pass:true,acceptedRuns:accepted,rejectedCases:rejected,
  maxEvents:10000,backend:'actual ruQu WASM + pinned native ruField and WorldGraph; simulation only'},null,2));
