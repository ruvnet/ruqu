# Independent stabilizer laboratory

Status: proposed, experimental CPU implementation. No production promotion.

## Decision

Add a dependency-free, standalone Rust crate under integrations. Use the published
Aaronson and Gottesman tableau construction and independently derived Pauli
algebra. Keep existing ruqu simulation APIs and routing unchanged. A dense
QuantumState return value cannot represent the intended scalable compact state.

The implementation worker received only a mathematical specification and API
contract. The evaluator received the contract and independently wrote a dense
complex state-vector oracle without reading the implementation. The coordinator
previously read QuaSARQ documentation; this is documented source separation,
not a legal certification of clean-room status. No QuaSARQ source is incorporated
or translated. Source: https://arxiv.org/abs/quant-ph/0406196 .

## Frozen acceptance

Primary outcome: correct independent Clifford simulation, measurement and reset.
The independent dense state-vector oracle is the correctness baseline. Thirty
development seeds and ten separately selected confirmation seeds cover five
small widths and 96 steps each. Every step checks signed stabilizers and
symplectic invariants. Analytic GHZ cases exercise 31/32/33/255/256/257 qubits.
CLI tests cover circuit input through measurement output and rejection behavior.
No correctness failures are acceptable. This run does not optimize performance.
Confirmation data was withheld from the implementation worker before its first
candidate. If confirmation fails, it cannot be reused as untouched evidence.

Scope: H, S, X, Y, Z, CNOT, Z measurement and reset to zero. Explicitly reject
unsupported CLI operations. The library accepts explicit measurement random
bits; the CLI specifies SplitMix64 and consumes one bit per M or R instruction.
Packed tableau storage has a 64 MiB requested-allocation limit. CLI input is
limited to 1 MiB and 100,000 lines. These are memory/input limits, not a execution
time guarantee. Never process untrusted workloads without an external timeout.

## Cost, ownership and promotion

Owner: Codex coordinator, independent implementation/evaluation workers.
Resource: an isolated ruqu branch and temporary ruOS workspace. The user explicitly
authorized proceeding directly after the federation ownership service failed.
No lease was acquired and no exclusive repository-wide ownership is claimed.
No default branch mutation, automatic merge, QPU job or paid API job is authorized.
Active-work ceiling: 90 minutes. Full compute cost is unknown.

This is an implementation candidate, not a speedup result. No numerical safety
score or signed evidence receipt is inferred from tests. Integration-ready status
requires independent review, repository/security gates, a defined safety rubric,
and matched performance and cost measurements. Native GPU and transpiler claims
require actual device execution, exact word agreement, and transfer-inclusive
benchmarking against Stim and the CPU implementation.

## Rollback

Do not enable any new default route. Delete or revert this isolated integration
directory and its dedicated workflow. Existing callers and workspace manifests
are unaffected. A later optional ruqu-core adapter requires a separate decision.
