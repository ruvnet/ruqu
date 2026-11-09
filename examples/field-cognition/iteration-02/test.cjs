'use strict';
(async()=>{
 const assert=require('node:assert/strict'),m=await import('./models.mjs');let tests=0,maxDelta=0;
 function check(name,fn){fn();tests++;console.log('PASS '+name);}
 const topology=await m.buildTopology();
 check('actual RuVector topology self-query',()=>{assert(topology.queryCheck.every(Boolean));assert.deepEqual(topology.neighbors,[[1,3],[0,2],[1,3],[0,2]]);});
 check('published RuVector adapter reports flat index',()=>{assert.equal(topology.indexType,'flat');assert.equal(topology.usesHnsw,false);});
 check('RuQu analytic equivalence across 205 circuits',()=>{for(let i=0;i<=40;i++)for(const p of [0,.3,1.2,Math.PI,5]){const a=i*Math.PI/40,b=(i-20)*Math.PI/40,d=Math.abs(m.quantum(a,p,b)-m.analytic(a,p,b));maxDelta=Math.max(d,maxDelta);assert(d<1e-12);}});
 check('phase is observable only with a mixing rotation',()=>{assert(Math.abs(m.quantum(1,0,.5)-m.quantum(1,Math.PI,.5))>.1);assert(Math.abs(m.quantum(1,0,0)-m.quantum(1,Math.PI,0))<1e-12);});
 check('invalid and replay events rejected without mutation',()=>{for(const make of [()=>new m.ScalarEMA(),()=>new m.ScalarBayes(),()=>new m.IndependentBayes(),()=>new m.OriginalField(topology.neighbors),()=>new m.ChangePointField(topology.neighbors,{diffusion:.24,gain:.7,staleDecay:.04,resetThreshold:1.5})]){const x=make();x.update({t:1,sensor:0,x:1});const before=JSON.stringify(x);for(const e of [{t:1,sensor:0,x:0},{t:2,sensor:4,x:0},{t:2,sensor:0,x:NaN},{t:Infinity,sensor:0,x:0}])assert.throws(()=>x.update(e));assert.equal(JSON.stringify(x),before);}});
 check('field bounded under extremes and missing sensors',()=>{const x=new m.ChangePointField(topology.neighbors,{diffusion:.24,gain:.7,staleDecay:.04,resetThreshold:1.5});for(let t=1;t<1000;t++)x.update({t,sensor:t%4,x:t%3?1e300:null});assert(x.z.every(v=>Number.isFinite(v)&&Math.abs(v)<=12));});
 check('reset reproduces initial trajectory',()=>{const x=new m.ChangePointField(topology.neighbors,{diffusion:.24,gain:.7,staleDecay:.04,resetThreshold:1.5}),e={t:1,sensor:0,x:1},a=x.update(e);x.reset();assert.equal(a,x.update(e));});
 check('deterministic replay',()=>{const a=new m.ChangePointField(topology.neighbors,{diffusion:.24,gain:.7,staleDecay:.04,resetThreshold:1.5}),b=new m.ChangePointField(topology.neighbors,{diffusion:.24,gain:.7,staleDecay:.04,resetThreshold:1.5});for(let t=1;t<100;t++){const e={t,sensor:t%4,x:Math.sin(t)};assert.equal(a.update(e),b.update(e));}});
 console.log(JSON.stringify({tests,max_analytic_delta:maxDelta,passed:true,topology_index:topology.indexType}));
})().catch(e=>{console.error(e);process.exitCode=1;});
