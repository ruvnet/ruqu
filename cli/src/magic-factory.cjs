// SPDX-License-Identifier: MIT
// Exact ideal 15-to-1 independent-Z fault model, not a physical QPU simulation.
// Formula reference: https://arxiv.org/abs/quant-ph/0403025 Sec. VI.
'use strict';
const { performance } = require('node:perf_hooks');
const MASK64 = (1n << 64n) - 1n;
const spectrum = (() => {
  let dp = Array.from({length:16},()=>Array(16).fill(0)); dp[0][0]=1;
  for(let column=1;column<=15;column++) {
    const next=dp.map(row=>row.slice());
    for(let syndrome=0;syndrome<16;syndrome++) for(let w=0;w<column;w++) next[syndrome^column][w+1]+=dp[syndrome][w];
    dp=next;
  }
  return Object.freeze(dp[0]);
})();
function probability(p,label) {if(typeof p!=='number'||!Number.isFinite(p)||p<0||p>1) throw Error(`${label} must be finite in [0,1]`);return p;}
function nonnegative(n,label) {if(typeof n!=='number'||!Number.isFinite(n)||n<0) throw Error(`${label} must be finite and nonnegative`);return n;}
function finite(n) {if(!Number.isFinite(n)) throw Error('Resource arithmetic overflow');return n;}
function classify(mask) {let syndrome=0,weight=0;for(let i=0;i<15;i++) if(mask&(1<<i)){syndrome^=i+1;weight++;}return {accepted:syndrome===0,logical_error:weight%2===1};}
function exact(p) {
  probability(p,'input_error');let accepted=0,harmful=0;
  for(let w=0;w<=15;w++){const mass=spectrum[w]*p**w*(1-p)**(15-w);accepted+=mass;if(w%2) harmful+=mass;}
  if(harmful===0&&p>0)throw Error('Output error underflows f64 precision');
  accepted=Math.min(1,Math.max(0,accepted));if(accepted===0) throw Error('Zero acceptance');
  return {input_error:p,acceptance_probability:accepted,rejected_probability:1-accepted,accepted_logical_error_probability:harmful,conditional_output_error:Math.min(1,Math.max(0,harmful/accepted))};
}
function seedValue(seed) {let n;try {if(!/^[0-9]+$/.test(String(seed))) throw Error();n=BigInt(seed);}catch{throw Error('seed must be a decimal unsigned 64 bit integer');}if(n>MASK64)throw Error('seed exceeds unsigned 64 bit range');return n;}
function sample(p,shots,seed) {
  probability(p,'input_error');if(!Number.isSafeInteger(shots)||shots<1||shots>5_000_000)throw Error('shots must be an integer in [1,5000000]');
  let state=seedValue(seed);const initialSeed=state.toString();
  function uniform(){state=(state+0x9e3779b97f4a7c15n)&MASK64;let z=state;z=((z^(z>>30n))*0xbf58476d1ce4e5b9n)&MASK64;z=((z^(z>>27n))*0x94d049bb133111ebn)&MASK64;z^=z>>31n;return Number(z>>11n)/9007199254740992;}
  let accepted=0,logical_errors=0;
  for(let attempt=0;attempt<shots;attempt++){let syndrome=0,parity=0;for(let i=1;i<=15;i++)if(uniform()<p){syndrome^=i;parity^=1;}if(syndrome===0){accepted++;logical_errors+=parity;}}
  const radius=n=>n===0?1:Math.sqrt(Math.log(4/0.05)/(2*n));
  function interval(k,n){if(n===0)return [0,1];const r=radius(n);return [Math.max(0,k/n-r),Math.min(1,k/n+r)];}
  return {attempts:shots,accepted,rejected:shots-accepted,logical_errors,seed:initialSeed,acceptance_estimate:accepted/shots,conditional_output_error_estimate:accepted?logical_errors/accepted:null,confidence:{method:'Hoeffding with union bound for two intervals',joint_coverage:0.95,acceptance:interval(accepted,shots),conditional_output_error:interval(logical_errors,accepted)}};
}
function resources(p,options={}) {
  const rounds=options.rounds??1;if(!Number.isInteger(rounds)||rounds<1||rounds>16)throw Error('rounds must be an integer in [1,16]');
  const blockQ=nonnegative(options.block_qubits??15,'block_qubits'),blockR=nonnegative(options.block_rounds??10,'block_rounds');
  const rawQ=nonnegative(options.raw_qubits??1,'raw_qubits'),rawR=nonnegative(options.raw_rounds??1,'raw_rounds');
  const transferA=probability(options.transfer_acceptance??1,'transfer_acceptance');if(transferA===0)throw Error('Zero transfer yield');
  const transferE=probability(options.transfer_error??0,'transfer_error');
  const transferQ=nonnegative(options.transfer_qubits??0,'transfer_qubits'),transferR=nonnegative(options.transfer_rounds??0,'transfer_rounds');
  let raw=1,attempts=0,volume=finite(rawQ*rawR),error=probability(p,'input_error');const stages=[];
  for(let i=0;i<rounds;i++){const d=exact(error),a=d.acceptance_probability;raw=finite(15*raw/a);attempts=finite((15*attempts+1)/a);volume=finite((15*volume+finite(blockQ*blockR))/a);error=d.conditional_output_error;stages.push(d);}
  const transferApplied=['transfer_acceptance','transfer_error','transfer_qubits','transfer_rounds'].some(k=>Object.hasOwn(options,k));
  if(transferApplied){raw=finite(raw/transferA);attempts=finite((attempts+1)/transferA);volume=finite((volume+finite(transferQ*transferR))/transferA);error=Math.min(1,error/transferA+transferE);}
  return {cost_basis:'illustrative defaults unless caller supplied; expected volume, not peak qubits or wall clock',block_cost:{qubits:blockQ,rounds:blockR},raw_cost:{qubits:rawQ,rounds:rawR},stages,transfer:transferApplied?{acceptance:transferA,new_conditional_error_bound:transferE,qubits:transferQ,rounds:transferR}:null,expected_raw_states:raw,expected_stage_attempts:attempts,expected_qubit_rounds:volume,output_error_upper_bound:error};
}
function run(options={}) {
  const p=options.input_error??0.01,shots=options.shots??100000,seed=options.seed??'42';
  // Validate resource inputs before running the more expensive sampler.
  const estimate=resources(p,options);
  return {schema_version:1,model:'ideal_15_to_1_independent_Z',hardware_run:false,physical_cultivation_reproduction:false,assumptions:['independent input Z faults','ideal Clifford operations and syndrome checks','independent outputs between stages','no noise or latency inferred for hardware'],exact:exact(p),sample:sample(p,shots,seed),resources:estimate};
}
function research(){return {schema_version:1,kind:'literature_profiles_not_ruqu_measurements',profiles:[
  {name:'Willow cultivation',source:'https://arxiv.org/abs/2512.13908',acceptance:0.08,reported_fidelity:0.9999,reported_fidelity_uncertainty:0.0001,expected_attempts_per_accepted_state:12.5,physical_qubit_rounds:null,caveat:'Reported retained state result before full surface code transfer; fidelity is a point estimate with uncertainty, not a certified error bound for a new device.'},
  {name:'Unfolded distillation',source:'https://www.nature.com/articles/s41534-026-01197-z',physical_qubits:53,qec_rounds:5.5,reported_logical_error:3e-7,noise_bias_min:5e6,phase_flip_rate:0.001,acceptance:null,caveat:'Theoretical result under highly biased noise. Missing acceptance prevents cost per accepted state calculation.'},
  {name:'Clifford stabilizer simulation',source:'https://arxiv.org/abs/2609.16929',status:'research direction; full algorithm not implemented in this release'},
  {name:'Efficient logical magic state preparation simulation',source:'https://arxiv.org/abs/2512.23799',status:'research direction; sparse Pauli propagation in ruqu is a bounded reference, not a reproduction of this algorithm'}
]};}
function benchmark(){const start=performance.now();const grid=[0,1e-10,0.001,0.01,0.05,0.1,0.5,1].map(exact);const counts=sample(0.1,25000,'42');const expected=exact(0.1);const [lo,hi]=counts.confidence.conditional_output_error;const pass=counts.accepted+counts.rejected===counts.attempts&&counts.logical_errors<=counts.accepted&&expected.conditional_output_error>=lo&&expected.conditional_output_error<=hi;return {schema_version:1,kind:'synthetic_model_benchmark',pass,elapsed_ms:performance.now()-start,grid,sample:counts,hardware_run:false};}
function parse(args){const options={},allowed=new Set(['input-error','shots','seed','rounds','block-qubits','block-rounds','raw-qubits','raw-rounds','transfer-acceptance','transfer-error','transfer-qubits','transfer-rounds']);
  if(args.length===1&&args[0]==='--research')return {research:true};if(args.length===1&&args[0]==='--benchmark')return {benchmark:true};
  for(let i=0;i<args.length;i+=2){const flag=args[i];const key=flag?.slice(2);if(!flag?.startsWith('--')||!allowed.has(key)||i+1>=args.length||Object.hasOwn(options,key.replaceAll('-','_')))throw Error('Usage: magic-factory [--input-error P --shots N --seed N --rounds N] | --research | --benchmark');const v=args[i+1];if(key==='seed'){seedValue(v);options.seed=v;}else {if(!/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(v))throw Error(`Invalid numeric value for ${flag}`);options[key.replaceAll('-','_')]=Number(v);}}
  return options;
}
module.exports={exact,sample,resources,run,research,benchmark,parse,classify,spectrum};
