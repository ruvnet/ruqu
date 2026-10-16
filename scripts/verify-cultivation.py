#!/usr/bin/env python3
"""Independent NumPy executor for ruqu's exported physical cultivation trace.

Checks circuit execution and encoded-state metrics against native Rust. It does
not independently establish that the trace transcribes the cited paper correctly,
reproduce noisy Monte Carlo RNG, or prove distance three.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import platform
import subprocess
import sys
import time

import numpy as np

TOLERANCE = 2e-11
QUBITS = 16
INDICES = np.arange(1 << QUBITS, dtype=np.uint32)
X_SUPPORTS = [(0, 1, 9), (1, 2, 10), (3, 9, 4, 11), (4, 10, 5, 12), (6, 11, 7), (7, 12, 8)]


def codewords():
    """CSS row-space construction, independent of the injection gate execution."""
    masks = [sum(1 << q for q in row) for row in X_SUPPORTS]
    zero = np.zeros(1 << 13, dtype=np.complex128)
    for word in range(64):
        index = 0
        for bit, mask in enumerate(masks):
            if word & (1 << bit):
                index ^= mask
        zero[index] = 1 / 8
    one = zero[np.arange(1 << 13) ^ ((1 << 0) | (1 << 3) | (1 << 6))]
    assert np.isclose(np.vdot(zero, zero), 1)
    assert np.isclose(np.vdot(one, one), 1)
    assert np.isclose(np.vdot(zero, one), 0)
    return zero, one


ZERO, ONE = codewords()


def metrics(state):
    blocks = state.reshape(-1, 1 << 13)
    a = blocks @ ZERO.conj()
    b = blocks @ ONE.conj()
    fidelity = np.sum(np.abs((a + np.exp(-1j * np.pi / 4) * b) / np.sqrt(2)) ** 2).real
    codespace = np.sum(np.abs(a) ** 2 + np.abs(b) ** 2).real
    return [float(np.clip(fidelity, 0, 1)), float(np.clip(codespace, 0, 1)), float(np.clip(codespace-fidelity, 0, 1))]


def pauli(state, q, code):
    bit = (INDICES >> q) & 1
    if code in (2, 3):
        state *= 1 - 2 * bit.astype(np.int8)
    if code in (1, 2):
        state = state[INDICES ^ (1 << q)]
    if code == 2:
        state *= 1j
    return state


def execute(trace, fault=None):
    state = np.zeros(1 << QUBITS, dtype=np.complex128)
    state[0] = 1
    probability = 1.0
    projections = 0
    for location, (label, qs) in enumerate(trace):
        name = label.split('(')[0]
        if name == 'Prepare':
            # All-zero postselection implies every reused ancilla is already |0>.
            # Verify this invariant instead of introducing a random reset oracle.
            if np.sum(np.abs(state[(INDICES >> qs[0]) & 1 == 1]) ** 2) > 1e-20:
                raise AssertionError('Nonzero reset branch requires an expanded oracle')
        elif name == 'H':
            q = qs[0]
            low = INDICES[(INDICES & (1 << q)) == 0]
            high = low | (1 << q)
            a, b = state[low].copy(), state[high].copy()
            state[low], state[high] = (a+b)/math.sqrt(2), (a-b)/math.sqrt(2)
        elif name in ('T', 'Tdg'):
            mask = (INDICES & (1 << qs[0])) != 0
            state[mask] *= np.exp((1 if name == 'T' else -1) * 1j * np.pi / 4)
        elif name == 'CNOT':
            state = state[INDICES ^ (((INDICES >> qs[0]) & 1) << qs[1])]
        elif name == 'CCZ':
            mask = np.ones(len(state), dtype=bool)
            for q in qs:
                mask &= (INDICES & (1 << q)) != 0
            state[mask] *= -1
        elif name != 'Measure':
            raise ValueError(f'Unsupported trace gate {label}')
        # The trace's fault location is after every unitary/preparation and
        # immediately BEFORE each measurement. Digits correspond to listed qs.
        if fault is not None and location == fault[0]:
            code = fault[1]
            for q in qs:
                state = pauli(state, q, code & 3)
                code >>= 2
        if name == 'Measure':
            zero_mask = (INDICES & (1 << qs[0])) == 0
            p0 = float(np.sum(np.abs(state[zero_mask]) ** 2))
            p0 = float(np.clip(p0, 0, 1))
            probability *= p0
            projections += 1
            if p0 < 1e-25:
                return [0.0, [0.0, 0.0, 0.0]], projections
            state[~zero_mask] = 0
            state /= math.sqrt(p0)
    return [probability, metrics(state)], projections


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/release/examples/cultivation')
    parser.add_argument('--trace', help='Optional previously exported trace JSON; otherwise query the binary')
    parser.add_argument('--output', default='docs/research/cultivation-independent-evidence.json')
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    trace_bytes = Path(args.trace).read_bytes() if args.trace else subprocess.check_output([str(binary), '--trace'])
    trace = json.loads(trace_bytes)
    if len(trace) > 10000 or any(len(qs) > 3 or any(not 0 <= q < 16 for q in qs) for _, qs in trace):
        raise ValueError('Trace bounds exceeded')
    groups = {kind: [i for i, (name, _) in enumerate(trace) if name.split('(')[0] == kind] for kind in ['Prepare', 'Measure', 'CNOT', 'CCZ']}
    cases = [('ideal', None)]
    for label in ['Prepare', 'Measure']:
        for i in sorted(set([groups[label][0], groups[label][len(groups[label])//2], groups[label][-1]])):
            cases.append((label, (i, 1)))
    for label, codes in [('CNOT', [1, 2, 5, 6, 15]), ('CCZ', [1, 2, 3, 21, 42, 63])]:
        for i in [groups[label][0], groups[label][-1]]:
            for code in codes:
                cases.append((label, (i, code)))
    rows = []
    started = time.perf_counter()
    for label, fault in cases:
        command = [str(binary), '--ideal'] if fault is None else [str(binary), '--fault', str(fault[0]), str(fault[1])]
        rust = json.loads(subprocess.check_output(command))
        numpy_result, projections = execute(trace, fault)
        rust_flat = np.array([rust[0], *rust[1]])
        numpy_flat = np.array([numpy_result[0], *numpy_result[1]])
        if not np.all(np.isfinite(rust_flat)) or not np.all(np.isfinite(numpy_flat)):
            raise AssertionError('Nonfinite native or oracle result')
        difference = float(np.max(np.abs(rust_flat-numpy_flat)))
        if fault is None and np.max(np.abs(numpy_flat-np.array([1.0, 1.0, 1.0, 0.0]))) > TOLERANCE:
            raise AssertionError('Ideal protocol must accept the target encoded state with unit probability')
        if difference > TOLERANCE:
            raise AssertionError(f'{label} {fault}: Rust={rust}, NumPy={numpy_result}, difference={difference}')
        rows.append({'kind': label, 'fault': list(fault) if fault else None,
                     'gate': trace[fault[0]][0] if fault else None,
                     'rust': rust, 'numpy': numpy_result, 'max_abs_difference': difference,
                     'postselections_executed': projections})
    report = {
        'schema': 'ruqu-cultivation-independent-validation/v1',
        'protocol_id': 'fold-transversal-surface-f3-2609.16929v1',
        'trace_sha256': hashlib.sha256(trace_bytes).hexdigest(),
        'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'oracle_source_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'python': platform.python_version(), 'numpy': np.__version__,
        'trace_locations': len(trace), 'physical_qubits': QUBITS,
        'absolute_tolerance': TOLERANCE, 'comparisons': len(rows),
        'max_abs_difference': max(row['max_abs_difference'] for row in rows),
        'wall_seconds': time.perf_counter()-started, 'cases': rows,
        'scope': 'Independent NumPy dense-state gate execution and CSS metric oracle on the Rust-exported trace; selected deterministic all-zero branches only. No paper transcription, noisy RNG, recovery, escape stage or distance-three proof.'
    }
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({k: report[k] for k in ['comparisons','trace_locations','max_abs_difference','absolute_tolerance','wall_seconds']}))


if __name__ == '__main__':
    main()
