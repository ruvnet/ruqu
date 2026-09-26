'use strict';
(async()=>{
 const fs=require('node:fs'),crypto=require('node:crypto'),os=require('node:os');
 const m=await import('./models.mjs'),protocol=JSON.parse(fs.readFileSync('protocol.json'));
 const sha=x=>crypto.createHash('sha256').update(x).digest('hex'),mean=a=>a.reduce((x,y)=>x+y,0)/a.length;
 const topology=await m.buildTopology();if(!topology.queryCheck.every(Boolean))throw Error('RuVector topology self-query failed');
 const names=['memoryless','scalar_ema','scalar_bayes','independent_bayes','iteration_01_field','change_point_field','ruqu','ruqu_phase_zero','ruqu_analytic_equivalent'];
 const all=[],raw=[];let maxDelta=0;const started=process.hrtime.bigint();
 for(const seed of protocol.seeds){
  const events=m.generate(seed,protocol.events_per_seed),ema=new m.ScalarEMA(),bayes=new m.ScalarBayes(),independent=new m.IndependentBayes(),oldField=new m.OriginalField(topology.neighbors),field=new m.ChangePointField(topology.neighbors,protocol.selected_without_holdout_access);
  const data=Object.fromEntries(names.map(n=>[n,{errors:0,brier:0,transitionErrors:0,transitionN:0,latency:[]}])) , predictions=[];
  for(const e of events){
   const values={};function timed(n,fn){const t=process.hrtime.bigint();values[n]=fn();data[n].latency.push(Number(process.hrtime.bigint()-t));}
   timed('memoryless',()=>e.x===null?.5:m.sigmoid(2*e.x/1.21));timed('scalar_ema',()=>ema.update(e));timed('scalar_bayes',()=>bayes.update(e));timed('independent_bayes',()=>independent.update(e));timed('iteration_01_field',()=>oldField.update(e));timed('change_point_field',()=>field.update(e));
   const a=m.angles(values.change_point_field,e.sensor);timed('ruqu',()=>m.quantum(...a));timed('ruqu_phase_zero',()=>m.quantum(a[0],0,a[2]));timed('ruqu_analytic_equivalent',()=>m.analytic(...a));maxDelta=Math.max(maxDelta,Math.abs(values.ruqu-values.ruqu_analytic_equivalent));
   for(const n of names){const p=values[n];if(!Number.isFinite(p)||p<0||p>1)throw Error('invalid probability');const err=Number(Number(p>=.5)!==e.y);data[n].errors+=err;data[n].brier+=(p-e.y)**2;if(e.transition){data[n].transitionErrors+=err;data[n].transitionN++;}}
   predictions.push(values);raw.push({seed,event:e,predictions:values});
  }
  const result={seed,n:events.length,event_sha256:sha(JSON.stringify(events)),prediction_sha256:sha(JSON.stringify(predictions)),models:{}};
  for(const n of names){const d=data[n],times=d.latency.sort((a,b)=>a-b);result.models[n]={error:d.errors/events.length,brier:d.brier/events.length,transition_error:d.transitionErrors/d.transitionN,transition_n:d.transitionN,p50_ns:times[Math.floor(times.length*.5)],p95_ns:times[Math.floor(times.length*.95)]};}all.push(result);
 }
 const summary=Object.fromEntries(names.map(n=>[n,Object.fromEntries(['error','brier','transition_error','p50_ns','p95_ns'].map(k=>[k,mean(all.map(r=>r.models[n][k]))]))]));
 const strongest=protocol.comparators.slice().sort((a,b)=>summary[a].error-summary[b].error)[0],improvements=all.map(r=>(r.models[strongest].error-r.models.change_point_field.error)/r.models[strongest].error),avg=mean(improvements),sd=Math.sqrt(mean(improvements.map(x=>(x-avg)**2))*5/4),ci=[avg-2.776*sd/Math.sqrt(5),avg+2.776*sd/Math.sqrt(5)];
 const elapsed=Number(process.hrtime.bigint()-started)/1e9;
 const report={protocol_sha256:sha(fs.readFileSync('protocol.json')),source_sha256:sha(fs.readFileSync('models.mjs')),development_protocol_sha256:sha(fs.readFileSync('development-protocol.json')),node:process.version,platform:process.platform,cpu:os.cpus()[0].model,topology,elapsed_s:elapsed,events_per_second:raw.length/elapsed,peak_process_rss_kib:process.resourceUsage().maxRSS,timing_scope:'Fixed model order, no warmup; diagnostic. RuQu latency excludes field update. RSS includes raw storage and all models.',known_new_provider_spend_usd:0,existing_compute_cost:'unknown',strongest_baseline:strongest,field_relative_error_reduction_mean:avg,field_relative_error_reduction_95_t_interval:ci,max_ruqu_analytic_delta:maxDelta,acceptance_passed:avg>=.10,per_seed:all,summary};
 fs.writeFileSync('raw-results.jsonl',raw.map(x=>JSON.stringify(x)).join('\n')+'\n');fs.writeFileSync('results.json',JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
})().catch(e=>{console.error(e);process.exitCode=1;});
