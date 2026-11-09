import {readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {dirname,join} from 'node:path';
import {createRequire} from 'node:module';
import * as ruvector from '@ruvector/wasm';
import {RuvectorWasmAdapter} from '@ruvector/wasm/adapter';
const require=createRequire(import.meta.url);
const {WasmQuantumCircuit,simulate}=require('@ruvector/ruqu/wasm/ruqu_wasm.js');

export const clamp=(x,a,b)=>Math.min(b,Math.max(a,x));
const sigmoid=x=>1/(1+Math.exp(-clamp(x,-40,40)));
const atanh=x=>.5*Math.log((1+clamp(x,-.999999999, .999999999))/(1-clamp(x,-.999999999,.999999999)));
const pdf=(x,mu,sigma)=>Math.exp(-.5*((x-mu)/sigma)**2)/sigma;
export function mixtureLLR(x,sigma=1.1){const f=s=>.97*pdf(x,s,sigma)+.015*pdf(x,s+8,sigma)+.015*pdf(x,s-8,sigma);return Math.log(Math.max(f(1),1e-300)/Math.max(f(-1),1e-300));}
const predict=(m,rate,dt)=>clamp(m*Math.exp(-2*rate*dt),-.999999,.999999);
const observe=(m,x,temp=1)=>x===null?m:Math.tanh(atanh(m)+temp*mixtureLLR(x)/2);
export function validate(e,last){if(!Number.isFinite(e.t)||e.t<=last||!Number.isInteger(e.sensor)||e.sensor<0||e.sensor>=4||!(e.x===null||Number.isFinite(e.x)))throw Error('invalid or stale event');}

export async function buildTopology(){const js=fileURLToPath(import.meta.resolve('@ruvector/wasm'));await ruvector.default(readFileSync(join(dirname(js),'ruvector_wasm_bg.wasm')));const index=new RuvectorWasmAdapter(new ruvector.VectorDB(2,'euclidean',false),{dimensions:2,metric:'euclidean'});const coords=[[1,0],[0,1],[-1,0],[0,-1]];coords.forEach((vector,node)=>index.insert({id:`node-${node}`,vector,metadata:{node}}));const neighbors=coords.map((vector,node)=>index.search({vector,k:3}).filter(x=>x.metadata.node!==node).slice(0,2).map(x=>x.metadata.node));return {indexType:index.indexType,usesHnsw:index.usesHnsw,neighbors,queryCheck:coords.map((vector,node)=>index.search({vector,k:1})[0].metadata.node===node)};}

export class ScalarBayes{
 constructor({hazardRate=.008,observationTemperature=1}={}){this.config={hazardRate,observationTemperature};this.reset();}
 reset(){this.m=0;this.last=0;}
 update(e,horizon=3){validate(e,this.last);this.m=predict(this.m,this.config.hazardRate,e.t-this.last);this.m=observe(this.m,e.x,this.config.observationTemperature);this.last=e.t;return (1+predict(this.m,this.config.hazardRate,horizon))/2;}
}
export class GraphFilter{
 constructor(neighbors,{hazardRate=.008,diffusion=.05}={}){this.neighbors=neighbors;this.config={hazardRate,diffusion};this.reset();}
 reset(){this.m=[0,0,0,0];this.last=0;}
 update(e,horizon=3){validate(e,this.last);const dt=e.t-this.last,old=this.m.map(x=>predict(x,this.config.hazardRate,dt)),a=1-Math.exp(-this.config.diffusion*dt);this.m=old.map((x,i)=>clamp((1-a)*x+a*this.neighbors[i].reduce((s,j)=>s+old[j],0)/2,-.999999,.999999));this.m[e.sensor]=observe(this.m[e.sensor],e.x);this.last=e.t;return (1+predict(this.m[0],this.config.hazardRate,horizon))/2;}
}
export class VelocityField{
 constructor(neighbors,{hazardRate=.008,changeThreshold=.7,secondsPerHop=3,maxPending=32,transportEnabled=true}={}){this.neighbors=neighbors;this.config={hazardRate,changeThreshold,secondsPerHop,maxPending,transportEnabled};this.distanceToZero=neighbors.map((_,start)=>{if(start===0)return 0;const seen=new Set([start]),q=[[start,0]];while(q.length){const [n,d]=q.shift();for(const v of neighbors[n]){if(v===0)return d+1;if(!seen.has(v)){seen.add(v);q.push([v,d+1]);}}}return Infinity;});this.reset();}
 reset(){this.m=[0,0,0,0];this.sign=[0,0,0,0];this.pending=[];this.last=0;this.maxObservedPending=0;}
 update(e,horizon=3){
  validate(e,this.last);const dt=e.t-this.last;this.m=this.m.map(x=>predict(x,this.config.hazardRate,dt));
  const due=this.pending.filter(p=>p.due<=e.t);this.pending=this.pending.filter(p=>p.due>e.t);for(const p of due)this.m[0]=clamp((1-2*p.confidence)*this.m[0],-.999999,.999999);
  const before=this.m[e.sensor],after=observe(before,e.x);this.m[e.sensor]=after;const s=Math.abs(after)>=this.config.changeThreshold?Math.sign(after):0;
  if(this.config.transportEnabled&&s&&this.sign[e.sensor]&&s!==this.sign[e.sensor]&&e.sensor!==0){const d=this.distanceToZero[e.sensor],candidate={due:e.t+d*this.config.secondsPerHop,confidence:clamp(Math.abs(after),.5,.98),source:e.sensor};if(!this.pending.some(p=>Math.abs(p.due-candidate.due)<this.config.secondsPerHop/2))this.pending.push(candidate);}
  if(s)this.sign[e.sensor]=s;this.pending.sort((a,b)=>a.due-b.due);if(this.pending.length>this.config.maxPending)this.pending=this.pending.slice(0,this.config.maxPending);this.maxObservedPending=Math.max(this.maxObservedPending,this.pending.length);this.last=e.t;
  let forecast=predict(this.m[0],this.config.hazardRate,horizon);for(const p of this.pending)if(p.due<=e.t+horizon)forecast=clamp((1-2*p.confidence)*forecast,-.999999,.999999);return (1+forecast)/2;
 }
}

export function quantum(p,sensor,phaseZero=false){const theta=2*Math.asin(Math.sqrt(clamp(p,0,1))),phase=phaseZero?0:Math.PI*(sensor+1)/4,beta=Math.PI/8*(sensor-1.5)/1.5,c=new WasmQuantumCircuit(1);try{c.ry(0,theta);c.rz(0,phase);c.ry(0,beta);return simulate(c).probabilities[1];}finally{c.free();}}
export function analytic(p,sensor,phaseZero=false){const theta=2*Math.asin(Math.sqrt(clamp(p,0,1))),phase=phaseZero?0:Math.PI*(sensor+1)/4,beta=Math.PI/8*(sensor-1.5)/1.5;return clamp((1-Math.cos(theta)*Math.cos(beta)+Math.sin(theta)*Math.cos(phase)*Math.sin(beta))/2,0,1);}
export function rng(seed){let x=seed>>>0;return()=>{x^=x<<13;x^=x>>>17;x^=x<<5;return((x>>>0)+.5)/4294967296;};}
export function generate(seed,steps=5000,cfg={}){
 const c={waveProbability:.012,secondsPerHop:3,horizon:3,missing:.2,sigma:1.1,outlier:.03,...cfg},r=rng(seed),times=[];let t=0;for(let k=0;k<steps;k++){t+=.5+r();times.push(t);}const waves=[];for(let k=0;k<steps;k++)if(r()<c.waveProbability)waves.push({origin:Math.floor(r()*4),born:times[k]});const arrivals=[];for(const w of waves)for(let node=0;node<4;node++){const d=Math.min((node-w.origin+4)%4,(w.origin-node+4)%4);arrivals.push({t:w.born+d*c.secondsPerHop,node});}arrivals.sort((a,b)=>a.t-b.t||a.node-b.node);
 const latent=[1,1,1,1],rows=[];let ai=0,lastNode0Change=-Infinity;for(let k=0;k<steps;k++){t=times[k];while(ai<arrivals.length&&arrivals[ai].t<=t){latent[arrivals[ai].node]*=-1;if(arrivals[ai].node===0)lastNode0Change=arrivals[ai].t;ai++;}const sensor=Math.floor(r()*4),u=Math.max(r(),1e-12),g=Math.sqrt(-2*Math.log(u))*Math.cos(2*Math.PI*r());let x=r()<c.missing?null:latent[sensor]+c.sigma*g;if(x!==null&&r()<c.outlier)x+=r()<.5?-8:8;let future=latent[0];for(let j=ai;j<arrivals.length&&arrivals[j].t<=t+c.horizon;j++)if(arrivals[j].node===0)future*=-1;rows.push({t,sensor,x,y:Number(future===1),current:Number(latent[0]===1),transition:t-lastNode0Change<=c.horizon||future!==latent[0]});}return rows;
}
export function score(factory,events){const model=factory(),out={error:0,brier:0,transitionError:0,transitionN:0,predictions:[]};for(const e of events){const p=model.update(e,3),err=Number(Number(p>=.5)!==e.y);out.error+=err;out.brier+=(p-e.y)**2;if(e.transition){out.transitionError+=err;out.transitionN++;}out.predictions.push(p);}out.error/=events.length;out.brier/=events.length;out.transitionError/=out.transitionN||1;out.maxPending=model.maxObservedPending??0;return out;}
