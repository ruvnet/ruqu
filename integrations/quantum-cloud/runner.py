#!/usr/bin/env python3
"""Bounded ruqu circuit bridge. No credentials or executable code in input files."""
import argparse
import hashlib
import importlib.metadata
import json
from pathlib import Path


def validate(data):
    if not isinstance(data, dict) or set(data) != {'schema_version', 'qubits', 'gates'}:
        raise ValueError('Expected schema_version, qubits and gates only')
    n = data['qubits']
    if type(data['schema_version']) is not int or data['schema_version'] != 1:
        raise ValueError('Unsupported schema version')
    if type(n) is not int or not 2 <= n <= 8:
        raise ValueError('Qubits must be 2..8')
    if not isinstance(data['gates'], list) or len(data['gates']) > 64:
        raise ValueError('At most 64 gates')
    for g in data['gates']:
        if not isinstance(g, dict) or g.get('name') not in ('h','x','z','s','t','cnot'):
            raise ValueError('Unsupported gate')
        keys = {'name','qubit','target'} if g['name'] == 'cnot' else {'name','qubit'}
        if set(g) != keys:
            raise ValueError('Unexpected gate fields')
        for k in keys - {'name'}:
            if type(g[k]) is not int or not 0 <= g[k] < n:
                raise ValueError('Invalid qubit index')
        if g['name'] == 'cnot' and g['qubit'] == g['target']:
            raise ValueError('Control and target must differ')
    return data


def qasm(data):
    d = validate(data)
    lines = ['OPENQASM 3.0;', f'qubit[{d["qubits"]}] q;', f'bit[{d["qubits"]}] b;']
    for g in d['gates']:
        lines.append(f'cnot q[{g["qubit"]}], q[{g["target"]}];' if g['name']=='cnot' else f'{g["name"]} q[{g["qubit"]}];')
    # SDK measurement strings are q0 first; normalize to ruqu order in receipts.
    lines += [f'b[{i}] = measure q[{i}];' for i in range(d['qubits'])]
    return '\n'.join(lines) + '\n'


def cirq_circuit(data):
    import cirq
    d = validate(data)
    qubits = cirq.LineQubit.range(d['qubits'])
    c = cirq.Circuit(cirq.I(q) for q in qubits)
    for g in d['gates']:
        if g['name'] == 'cnot':
            c.append(cirq.CNOT(qubits[g['qubit']],qubits[g['target']]))
        else:
            c.append({'h':cirq.H,'x':cirq.X,'z':cirq.Z,'s':cirq.S,'t':cirq.T}[g['name']](qubits[g['qubit']]))
    c.append(cirq.measure(*reversed(qubits), key='result'))
    return c


def execute(data, provider, shots=1000, submit=False, device=None, project=None,
            processor=None, mapping=None, receipt=None):
    d = validate(data)
    if type(shots) is not int or not 1 <= shots <= 10000:
        raise ValueError('Shots must be 1..10000')
    if provider not in ('aws','google'):
        raise ValueError('Unknown provider')
    digest = hashlib.sha256(json.dumps(d,sort_keys=True,separators=(',',':')).encode()).hexdigest()
    result = dict(schema_version=1, circuit_sha256=digest, provider=provider,
                  mode='cloud' if submit else 'local', shots=shots, bit_order='q[n-1]...q[0]',
                  sdk_version=importlib.metadata.version('amazon-braket-sdk' if provider=='aws' else 'cirq-google'))
    if submit and not receipt:
        raise ValueError('Cloud submission requires a new receipt path')
    # Reserve receipt before any network write: do not silently overwrite prior job evidence.
    handle = open(receipt, 'x') if submit else None
    def record():
        if handle:
            handle.seek(0); handle.truncate(); json.dump(result,handle,indent=2); handle.flush()
    try:
        result['status']='preparing';record()
        if provider == 'aws':
            from braket.ir.openqasm import Program
            program=Program(source=qasm(d))
            if submit:
                if not device or not device.startswith('arn:aws:braket:'):
                    raise ValueError('Explicit AWS Braket device ARN required')
                from braket.aws import AwsDevice
                backend=AwsDevice(device)
                task=backend.run(program,shots=shots)
                result.update(status='submitted',job_id=task.id,device=device)
                record()  # persist ID before polling; no automatic submission retries
            else:
                from braket.devices import LocalSimulator
                task=LocalSimulator().run(program,shots=shots)
            counts=task.result().measurement_counts
            result['counts']={k[::-1]:int(v) for k,v in counts.items()}
        else:
            import cirq
            circuit=cirq_circuit(d)
            if submit:
                if not project or not processor or not isinstance(mapping,list) or len(mapping)!=d['qubits']:
                    raise ValueError('Google requires project, processor and explicit physical qubit mapping')
                if any(not isinstance(x,list) or len(x)!=2 or any(type(v)is not int or v<0 for v in x) for x in mapping):
                    raise ValueError('Mapping must contain nonnegative [row,column] pairs')
                physical=[cirq.GridQubit(*x) for x in mapping]
                if len(set(physical))!=len(physical):raise ValueError('Mapping must be unique')
                import cirq_google
                engine=cirq_google.Engine(project_id=project)
                target=engine.get_processor(processor).get_device()
                circuit=circuit.transform_qubits(dict(zip(cirq.LineQubit.range(d['qubits']),physical)))
                circuit=cirq.optimize_for_target_gateset(circuit,gateset=cirq.CZTargetGateset())
                target.validate_circuit(circuit)  # reject invalid topology before submission
                job=engine.run_sweep(program=circuit,params=[cirq.ParamResolver({})],repetitions=shots,processor_id=processor)
                result.update(status='submitted',job_id=job.job_id,program_id=job.program_id,project=project,processor=processor,mapping=mapping)
                record()
                sample=job.results()[0]
            else:
                sample=cirq.Simulator(seed=42).run(circuit,repetitions=shots)
            result['counts']={format(int(k),f'0{d["qubits"]}b'):int(v) for k,v in sample.histogram(key='result').items()}
        if sum(result['counts'].values())!=shots:raise ValueError('Provider returned unexpected shot count')
        result['status']='completed';record()
        return result
    except Exception:
        result['status']='failed_or_unconfirmed';record()
        raise
    finally:
        if handle:handle.close()


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('circuit',type=Path)
    p.add_argument('--provider',choices=['aws','google'],required=True)
    p.add_argument('--shots',type=int,default=1000)
    p.add_argument('--submit',action='store_true',help='Explicitly submit to cloud; AWS usage may incur charges')
    p.add_argument('--device');p.add_argument('--project');p.add_argument('--processor')
    p.add_argument('--mapping',type=Path,help='JSON list of physical [row,column] qubits in logical order')
    p.add_argument('--receipt',help='New file for cloud job IDs and results; never overwritten')
    a=p.parse_args()
    if a.circuit.stat().st_size>65536:p.error('Circuit file exceeds 64 KiB')
    d=json.loads(a.circuit.read_text())
    if a.mapping and a.mapping.stat().st_size>4096:p.error('Mapping file exceeds 4 KiB')
    mapping=json.loads(a.mapping.read_text()) if a.mapping else None
    result=execute(d,a.provider,a.shots,a.submit,a.device,a.project,a.processor,mapping,a.receipt)
    print(json.dumps(result,indent=2))

if __name__=='__main__':main()
