'use strict';
const assert=require('node:assert/strict');
const {Field,EMA,Bayes,quantum,analytic}=require('./models.cjs');
let tests=0,maxDelta=0;
function check(name,fn){fn();tests++;console.log('PASS '+name);}
check('RuQu analytic equivalence across 205 circuits',()=>{for(let i=0;i<=40;i++)for(const p of [0,.3,1.2,Math.PI,5]){let a=i*Math.PI/40,b=(i-20)*Math.PI/40;let d=Math.abs(quantum(a,p,b)-analytic(a,p,b));maxDelta=Math.max(d,maxDelta);assert(d<1e-12);}});
check('phase changes observable with mixing',()=>assert(Math.abs(quantum(1,0,.5)-quantum(1,Math.PI,.5))>.1));
check('phase alone changes no probability',()=>assert(Math.abs(quantum(1,0,0)-quantum(1,Math.PI,0))<1e-12));
check('invalid and replay events rejected without state mutation',()=>{for(const M of [Field,EMA,Bayes]){const m=new M();m.update({t:1,sensor:0,x:1});const before=JSON.stringify(m);for(const e of [{t:1,sensor:0,x:0},{t:2,sensor:4,x:0},{t:2,sensor:0,x:NaN},{t:Infinity,sensor:0,x:0}])assert.throws(()=>m.update(e));assert.equal(JSON.stringify(m),before);}});
check('field bounded under extremes and missing sensors',()=>{const f=new Field();for(let t=1;t<1000;t++)f.update({t,sensor:t%4,x:t%3?1e300:null});assert(f.state.every(x=>Number.isFinite(x)&&Math.abs(x)<=4));});
check('reset reproduces initial trajectory',()=>{const f=new Field(),e={t:1,sensor:0,x:1};const a=f.update(e);f.reset();assert.equal(a,f.update(e));});
check('deterministic replay',()=>{const a=new Field(),b=new Field();for(let t=1;t<100;t++){const e={t,sensor:t%4,x:Math.sin(t)};assert.equal(a.update(e),b.update(e));}});
console.log(JSON.stringify({tests,max_analytic_delta:maxDelta,passed:true}));
