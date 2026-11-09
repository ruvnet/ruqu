'use strict';
const {WasmQuantumCircuit,simulate}=require('./node_modules/@ruvector/ruqu/wasm/ruqu_wasm.js');
const clamp=(x,a,b)=>Math.min(b,Math.max(a,x));
const sigmoid=x=>1/(1+Math.exp(-clamp(x,-40,40)));
function validate(e,last) {
 if(!Number.isFinite(e.t)||e.t<=last||!Number.isInteger(e.sensor)||e.sensor<0||e.sensor>=4||!(e.x===null||Number.isFinite(e.x))) throw Error('invalid or stale event');
}
class Field {
 constructor(diffusion=.12){this.diffusion=diffusion;this.reset();}
 reset(){this.state=[0,0,0,0];this.last=0;}
 update(e){
  validate(e,this.last); const dt=e.t-this.last;
  // Convex neighbor mixing and exponential leakage preserve bounded state.
  const d=1-Math.exp(-this.diffusion*dt), leak=Math.exp(-.02*dt), old=this.state;
  this.state=old.map((x,i)=>leak*((1-d)*x+d*(old[(i+1)%4]+old[(i+3)%4])/2));
  if(e.x!==null)this.state[e.sensor]=clamp(.5*this.state[e.sensor]+.5*clamp(e.x,-4,4),-4,4);
  this.last=e.t;
  return sigmoid(2*this.state.reduce((a,b)=>a+b,0)/4/(1.2**2));
 }
}
class EMA {
 constructor(){this.state=0;this.last=0;}
 update(e){validate(e,this.last);this.last=e.t;if(e.x!==null)this.state=.75*this.state+.25*e.x;return sigmoid(2*this.state/(1.2**2));}
}
class Bayes {
 constructor(){this.p=.5;this.last=0;}
 update(e){validate(e,this.last);this.last=e.t;const prior=.03+.94*this.p;this.p=e.x===null?prior:sigmoid(Math.log(prior/(1-prior))+2*e.x/(1.2**2));return this.p;}
}
function angles(p,sensor){return [2*Math.asin(Math.sqrt(clamp(p,0,1))),Math.PI*(sensor+1)/4,Math.PI/8*(sensor-1.5)/1.5];}
function analytic(theta,phase,beta){return clamp((1-Math.cos(theta)*Math.cos(beta)+Math.sin(theta)*Math.cos(phase)*Math.sin(beta))/2,0,1);}
function quantum(theta,phase,beta){
 if(![theta,phase,beta].every(Number.isFinite))throw Error('nonfinite angle');
 const c=new WasmQuantumCircuit(1);
 try{c.ry(0,theta);c.rz(0,phase);c.ry(0,beta);const r=simulate(c);if(!r.probabilities||r.probabilities.length!==2)throw Error('invalid result');return r.probabilities[1];}finally{c.free();}
}
module.exports={Field,EMA,Bayes,angles,analytic,quantum,sigmoid};
