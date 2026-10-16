// All numerical work belongs to the compiled Rust module, never a JS fallback.
let engine;
let busy = false;
function int(value, low, high, label) {
  if (!Number.isInteger(value) || value < low || value > high) throw new Error(`${label} must be an integer between ${low} and ${high}.`);
  return value;
}
function probability(value, max, label) {
  if (!Number.isFinite(value) || value < 0 || value > max) throw new Error(`${label} must be between 0 and ${max}.`);
  return value;
}
function seed(value) {
  if (typeof value !== 'string' || !/^[0-9]{1,18}$/.test(value)) throw new Error('Seed must contain 1 to 18 decimal digits.');
  return value;
}
function validateResearch(request) {
  if (!request || typeof request !== 'object') throw new Error('Missing research request.');
  if (request.op === 'magic-factory') return {op:request.op,input_error:probability(request.input_error,.5,'Input error'),shots:int(request.shots,1,100000,'Attempts'),seed:seed(request.seed),rounds:int(request.rounds,1,3,'Stages')};
  if (request.op === 'cultivation') return {op:request.op,attempts:int(request.attempts,1,32,'Attempts'),seed:seed(request.seed),gate_error:probability(request.gate_error,.05,'Gate error'),preparation_error:probability(request.preparation_error,.05,'Preparation error'),measurement_error:probability(request.measurement_error,.05,'Measurement error'),verification_rounds:2};
  throw new Error('Unsupported explorer research operation.');
}
async function boot() {
  try {
    engine = await import('./wasm/ruqu_wasm.js');
    await engine.default({module_or_path:new URL('./wasm/ruqu_wasm_bg.wasm',import.meta.url)});
    if (typeof engine.research_run_json !== 'function' || typeof engine.simulate !== 'function') throw new Error('This WASM build is missing the explorer APIs. Rebuild the current ruqu-wasm crate.');
    postMessage({type:'ready'});
  } catch (error) { postMessage({type:'init-error',error:String(error?.message || error)}); }
}
self.onmessage = ({data}) => {
  const {id,type}=data || {};
  if (busy) {postMessage({type:'error',id,error:'A simulation is already running. Wait or cancel it first.'});return;}
  if (!engine) {postMessage({type:'error',id,error:'The Rust WASM engine is not ready.'});return;}
  busy = true;
  try {
    const start = performance.now();
    let result;
    if(type==='circuit') {
      const qubits = int(data.qubits,2,8,'Qubits');
      if (!Array.isArray(data.gates) || data.gates.length>64) throw new Error('Circuit supports at most 64 gates.');
      const gates=data.gates.map(g=>{
        if(!g || !['h','x','z','t','s','cnot'].includes(g.name)) throw new Error('Unsupported gate.');
        const qubit=int(g.qubit,0,qubits-1,'Qubit');
        if(g.name==='cnot') {const target=int(g.target,0,qubits-1,'Target');if(target===qubit)throw new Error('CNOT control and target must differ.');return {name:g.name,qubit,target};}
        return {name:g.name,qubit};
      });
      const circuit=new engine.WasmQuantumCircuit(qubits);
      try { for(const g of gates) g.name==='cnot'?circuit.cnot(g.qubit,g.target):circuit[g.name](g.qubit); result=engine.simulate(circuit); }
      finally {circuit.free();}
    } else if(type==='research') result=JSON.parse(engine.research_run_json(JSON.stringify(validateResearch(data.request))));
    else throw new Error('Unsupported worker operation.');
    postMessage({type:'result',id,result,elapsed_ms:performance.now()-start});
  } catch(error) { postMessage({type:'error',id,error:String(error?.message||error)}); }
  finally {busy=false;}
};
boot();
