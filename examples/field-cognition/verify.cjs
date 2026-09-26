'use strict';
const fs=require('node:fs'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const ref=JSON.parse(fs.readFileSync('ruos-summary.json')),r=JSON.parse(fs.readFileSync('results.json'));
assert.equal(sha('protocol.json'),'c735f8e6fb79eaa7ff9ff817772815a794e70bfc945fcb6f869654b51d7a3565');
assert.equal(sha('models.cjs'),'7108cf5ac37ba99d41cf5a848a2740ae5785ec6986bd938ca7504b23a8ba4399');
assert.equal(sha('raw-results.jsonl'),ref.raw_sha256);
for(let i=0;i<5;i++){assert.equal(r.seeds[i].event_sha256,ref.seeds[i].event_sha256);assert.equal(r.seeds[i].prediction_sha256,ref.seeds[i].prediction_sha256);}
assert(r.max_ruqu_analytic_delta<1e-12);assert.equal(r.acceptance_passed,false);assert.equal(r.promotion,false);
console.log(JSON.stringify({replay:'passed',seeds:5,events:10000,raw_sha256:ref.raw_sha256,candidate_promotion:false}));
