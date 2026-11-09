'use strict';
(async()=>{
 const fs=require('node:fs'),crypto=require('node:crypto');
 const m=await import('./models.mjs'),protocol=JSON.parse(fs.readFileSync('development-protocol.json'));
 const topology=await m.buildTopology();if(!topology.queryCheck.every(Boolean))throw Error('RuVector topology self-query failed');
 const configs=[];for(const diffusion of protocol.candidate_grid.diffusion)for(const gain of protocol.candidate_grid.gain)for(const staleDecay of protocol.candidate_grid.stale_decay)for(const resetThreshold of protocol.candidate_grid.reset_threshold)configs.push({diffusion,gain,staleDecay,resetThreshold});
 const events=protocol.seeds.map(seed=>m.generate(seed,protocol.events_per_seed));
 const mean=a=>a.reduce((x,y)=>x+y,0)/a.length;
 const baselines={ema:mean(events.map(e=>m.score(m.ScalarEMA,e).error)),scalar_bayes:mean(events.map(e=>m.score(m.ScalarBayes,e).error)),independent_bayes:mean(events.map(e=>m.score(m.IndependentBayes,e).error)),original_field:mean(events.map(e=>m.score(m.OriginalField,e,topology.neighbors).error))};
 const ranked=configs.map(config=>({config,error:mean(events.map(e=>m.score(m.ChangePointField,e,topology.neighbors,config).error)),transition_error:mean(events.map(e=>m.score(m.ChangePointField,e,topology.neighbors,config).transitionError))})).sort((a,b)=>a.error-b.error||a.transition_error-b.transition_error);
 const report={development_only:true,protocol_sha256:crypto.createHash('sha256').update(fs.readFileSync('development-protocol.json')).digest('hex'),topology,baselines,candidates:ranked.slice(0,10),selected:ranked[0],evaluated_configs:configs.length};
 fs.writeFileSync('development-results.json',JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
})().catch(e=>{console.error(e);process.exitCode=1;});
