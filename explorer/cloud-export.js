export function cloudCircuit(qubits,gates){
  if(!Number.isInteger(qubits)||qubits<2||qubits>8||!Array.isArray(gates)||gates.length>64)throw new Error('Invalid circuit');
  const clean=gates.map(g=>{if(!['h','x','z','s','t','cnot'].includes(g.name)||!Number.isInteger(g.qubit)||g.qubit<0||g.qubit>=qubits)throw new Error('Invalid gate');const out={name:g.name,qubit:g.qubit};if(g.name==='cnot'){if(!Number.isInteger(g.target)||g.target<0||g.target>=qubits||g.target===g.qubit)throw new Error('Invalid target');out.target=g.target;}return out;});
  return {schema_version:1,qubits,gates:clean};
}
export function saveCloudCircuit(qubits,gates){const data=cloudCircuit(qubits,gates);const url=URL.createObjectURL(new Blob([JSON.stringify(data,null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='ruqu-cloud-circuit.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);}
