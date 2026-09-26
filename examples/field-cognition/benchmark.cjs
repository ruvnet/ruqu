'use strict';
const fs=require('node:fs'),crypto=require('node:crypto'),os=require('node:os');
const {Field,EMA,Bayes,angles,analytic,quantum,sigmoid}=require('./models.cjs');
const protocol=JSON.parse(fs.readFileSync('protocol.json'));
const sha=x=>crypto.createHash('sha256').update(x).digest('hex');
function rng(seed){let x=seed>>>0;return()=>{x^=x<<13;x^=x>>>17;x^=x<<5;return((x>>>0)+.5)/4294967296;};}
function generate(seed){const r=rng(seed),rows=[];let truth=1,t=0,age=100;for(let i=0;i<protocol.steps;i++){if(r()<.03){truth=-truth;age=0;}else age++;t+=.5+r();const sensor=Math.floor(r()*4),g=Math.sqrt(-2*Math.log(r()))*Math.cos(2*Math.PI*r());const x=r()<.2?null:truth+1.2*g;rows.push({t,sensor,x,y:truth===1?1:0,transition:age<5});}return rows;}
const names=['memoryless','ema','bayes','field_no_diffusion','field','ruqu','phase_zero','analytic'];
const all=[],raw=[];let maxDelta=0;const runStart=process.hrtime.bigint();
for(const seed of protocol.seeds){
 const events=generate(seed),ema=new EMA(),bayes=new Bayes(),field=new Field(),noDiff=new Field(0);
 const data=Object.fromEntries(names.map(n=>[n,{errors:0,brier:0,transitionErrors:0,transitionN:0,latency:[]}]));
 const predictions=[];
 for(const e of events){
  const values={};function timed(n,fn){const t=process.hrtime.bigint();values[n]=fn();data[n].latency.push(Number(process.hrtime.bigint()-t));}
  timed('memoryless',()=>e.x===null?.5:sigmoid(2*e.x/(1.2**2)));
  timed('ema',()=>ema.update(e));timed('bayes',()=>bayes.update(e));timed('field_no_diffusion',()=>noDiff.update(e));timed('field',()=>field.update(e));
  const a=angles(values.field,e.sensor);timed('ruqu',()=>quantum(...a));timed('phase_zero',()=>quantum(a[0],0,a[2]));timed('analytic',()=>analytic(...a));
  maxDelta=Math.max(maxDelta,Math.abs(values.ruqu-values.analytic));
  for(const n of names){const p=values[n];if(!Number.isFinite(p)||p<0||p>1)throw Error('invalid probability');const err=Number(Number(p>=.5)!==e.y);data[n].errors+=err;data[n].brier+=(p-e.y)**2;if(e.transition){data[n].transitionErrors+=err;data[n].transitionN++;}}
  predictions.push(values);raw.push({seed,event:e,predictions:values});
 }
 const result={seed,n:events.length,event_sha256:sha(JSON.stringify(events)),prediction_sha256:sha(JSON.stringify(predictions)),models:{}};
 for(const n of names){const d=data[n],times=d.latency.sort((a,b)=>a-b);result.models[n]={error:d.errors/events.length,brier:d.brier/events.length,transition_error:d.transitionErrors/d.transitionN,transition_n:d.transitionN,p50_ns:times[Math.floor(times.length*.5)],p95_ns:times[Math.floor(times.length*.95)]};}
 all.push(result);
}
const mean=a=>a.reduce((x,y)=>x+y,0)/a.length;
const summary=Object.fromEntries(names.map(n=>[n,Object.fromEntries(['error','brier','transition_error','p50_ns','p95_ns'].map(k=>[k,mean(all.map(r=>r.models[n][k]))]))]));
const strongest=['memoryless','ema','bayes'].sort((a,b)=>summary[a].error-summary[b].error)[0];
const improvements=all.map(r=>(r.models[strongest].error-r.models.field.error)/r.models[strongest].error);
const m=mean(improvements),sd=Math.sqrt(mean(improvements.map(x=>(x-m)**2))*5/4),ci=[m-2.776*sd/Math.sqrt(5),m+2.776*sd/Math.sqrt(5)];
const elapsed=Number(process.hrtime.bigint()-runStart)/1e9;
const report={protocol_sha256:sha(fs.readFileSync('protocol.json')),source_sha256:sha(fs.readFileSync('models.cjs')),node:process.version,platform:process.platform,cpu:os.cpus()[0].model,elapsed_s:elapsed,events_per_second:raw.length/elapsed,peak_process_rss_kib:process.resourceUsage().maxRSS,timing_scope:'Fixed model order, no warmup; diagnostic only. RuQu/analytic latency excludes shared field update. RSS includes raw result storage and all models.',known_new_provider_spend_usd:0,existing_compute_cost:'unknown',strongest_baseline:strongest,field_relative_error_reduction_mean:m,field_relative_error_reduction_95_t_interval:ci,max_ruqu_analytic_delta:maxDelta,acceptance_passed:false,promotion:false,summary,seeds:all};
fs.writeFileSync('raw-results.jsonl',raw.map(x=>JSON.stringify(x)).join('\n')+'\n');fs.writeFileSync('results.json',JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
