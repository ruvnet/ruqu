import {readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {dirname,join} from 'node:path';
import {createRequire} from 'node:module';
import * as ruvector from '@ruvector/wasm';
import {RuvectorWasmAdapter} from '@ruvector/wasm/adapter';

const require=createRequire(import.meta.url);
const {WasmQuantumCircuit,simulate}=require('@ruvector/ruqu/wasm/ruqu_wasm.js');

export const clamp=(x,a,b)=>Math.min(b,Math.max(a,x));
export const sigmoid=x=>1/(1+Math.exp(-clamp(x,-40,40)));
const logit=p=>Math.log(clamp(p,1e-9,1-1e-9)/(1-clamp(p,1e-9,1-1e-9)));

export async function buildTopology(){
  const js=fileURLToPath(import.meta.resolve('@ruvector/wasm'));
  await ruvector.default(readFileSync(join(dirname(js),'ruvector_wasm_bg.wasm')));
  const index=new RuvectorWasmAdapter(new ruvector.VectorDB(2,'euclidean',false),{dimensions:2,metric:'euclidean'});
  const coords=[[1,0],[0,1],[-1,0],[0,-1]];
  coords.forEach((vector,node)=>index.insert({id:`node-${node}`,vector,metadata:{node}}));
  const neighbors=coords.map((vector,node)=>index.search({vector,k:3}).filter(x=>x.metadata.node!==node).slice(0,2).map(x=>x.metadata.node));
  return {indexType:index.indexType,usesHnsw:index.usesHnsw,neighbors,coords,queryCheck:coords.map((vector,node)=>index.search({vector,k:1})[0].metadata.node===node)};
}

export function validate(e,last){
  if(!Number.isFinite(e.t)||e.t<=last||!Number.isInteger(e.sensor)||e.sensor<0||e.sensor>=4||!(e.x===null||Number.isFinite(e.x)))throw Error('invalid or stale event');
}

export class ScalarEMA{
  constructor(){this.reset();}
  reset(){this.s=0;this.last=0;}
  update(e){validate(e,this.last);this.last=e.t;if(e.x!==null)this.s=.75*this.s+.25*e.x;return sigmoid(2*this.s/1.21);}
}
export class ScalarBayes{
  constructor(){this.reset();}
  reset(){this.p=.5;this.last=0;}
  update(e){validate(e,this.last);this.last=e.t;const prior=.015+.97*this.p;this.p=e.x===null?prior:sigmoid(logit(prior)+2*e.x/1.21);return this.p;}
}
export class IndependentBayes{
  constructor(){this.reset();}
  reset(){this.p=[.5,.5,.5,.5];this.last=0;}
  update(e){validate(e,this.last);this.last=e.t;this.p=this.p.map(p=>.015+.97*p);if(e.x!==null)this.p[e.sensor]=sigmoid(logit(this.p[e.sensor])+2*e.x/1.21);return this.p[0];}
}
export class OriginalField{
  constructor(neighbors){this.neighbors=neighbors;this.reset();}
  reset(){this.state=[0,0,0,0];this.last=0;}
  update(e){validate(e,this.last);const dt=e.t-this.last,d=1-Math.exp(-.12*dt),leak=Math.exp(-.02*dt),old=this.state;this.state=old.map((x,i)=>leak*((1-d)*x+d*this.neighbors[i].reduce((a,j)=>a+old[j],0)/this.neighbors[i].length));if(e.x!==null)this.state[e.sensor]=clamp(.5*this.state[e.sensor]+.5*clamp(e.x,-4,4),-4,4);this.last=e.t;return sigmoid(2*this.state[0]/1.21);}
}
export class ChangePointField{
  constructor(neighbors,{diffusion,gain,staleDecay,resetThreshold}){this.neighbors=neighbors;this.config={diffusion,gain,staleDecay,resetThreshold};this.reset();}
  reset(){this.z=[0,0,0,0];this.last=0;}
  update(e){
    validate(e,this.last);const dt=e.t-this.last,d=1-Math.exp(-this.config.diffusion*dt),keep=Math.exp(-this.config.staleDecay*dt),old=this.z;
    this.z=old.map((x,i)=>clamp(keep*((1-d)*x+d*this.neighbors[i].reduce((a,j)=>a+old[j],0)/this.neighbors[i].length),-12,12));
    if(e.x!==null){const evidence=2*clamp(e.x,-5,5)/1.21,i=e.sensor,contradicts=Math.sign(evidence)!==Math.sign(this.z[i])&&Math.abs(evidence)>this.config.resetThreshold&&Math.abs(this.z[i])>this.config.resetThreshold;this.z[i]=clamp(contradicts?this.config.gain*evidence:this.z[i]+this.config.gain*evidence,-12,12);}
    this.last=e.t;return sigmoid(this.z[0]);
  }
}

export function angles(p,sensor){return [2*Math.asin(Math.sqrt(clamp(p,0,1))),Math.PI*(sensor+1)/4,Math.PI/8*(sensor-1.5)/1.5];}
export function analytic(theta,phase,beta){return clamp((1-Math.cos(theta)*Math.cos(beta)+Math.sin(theta)*Math.cos(phase)*Math.sin(beta))/2,0,1);}
export function quantum(theta,phase,beta){
  if(![theta,phase,beta].every(Number.isFinite))throw Error('nonfinite angle');
  const circuit=new WasmQuantumCircuit(1);
  try{circuit.ry(0,theta);circuit.rz(0,phase);circuit.ry(0,beta);const result=simulate(circuit);if(!result.probabilities||result.probabilities.length!==2)throw Error('invalid result');return result.probabilities[1];}
  finally{circuit.free();}
}

export function rng(seed){let x=seed>>>0;return()=>{x^=x<<13;x^=x>>>17;x^=x<<5;return((x>>>0)+.5)/4294967296;};}
export function generate(seed,steps=4000){
  const r=rng(seed),latent=[1,1,1,1],waves=[],rows=[];let t=0,node0Age=999;
  const ringDistance=(a,b)=>Math.min((a-b+4)%4,(b-a+4)%4);
  for(let k=0;k<steps;k++){
    if(r()<.012)waves.push({origin:Math.floor(r()*4),start:k});
    for(const w of waves){for(let n=0;n<4;n++)if(k-w.start===3*ringDistance(w.origin,n)){latent[n]*=-1;if(n===0)node0Age=0;}}
    node0Age++;
    t+=.5+r();const sensor=Math.floor(r()*4),u=Math.max(r(),1e-12),g=Math.sqrt(-2*Math.log(u))*Math.cos(2*Math.PI*r());
    let x=r()<.2?null:latent[sensor]+1.1*g;if(x!==null&&r()<.03)x+=r()<.5?-8:8;
    rows.push({t,sensor,x,y:Number(latent[0]===1),transition:node0Age<=5});
  }
  return rows;
}

export function score(Model,events,...args){const m=new Model(...args),out={error:0,brier:0,transitionError:0,transitionN:0,predictions:[]};for(const e of events){const p=m.update(e);if(!Number.isFinite(p)||p<0||p>1)throw Error('invalid probability');const err=Number(Number(p>=.5)!==e.y);out.error+=err;out.brier+=(p-e.y)**2;if(e.transition){out.transitionError+=err;out.transitionN++;}out.predictions.push(p);}out.error/=events.length;out.brier/=events.length;out.transitionError=out.transitionN?out.transitionError/out.transitionN:null;return out;}
