# Independent stabilizer baseline

This isolated Rust scalar implementation is an additive integration experiment. Existing ruqu routing and default backends remain unchanged. A true WebGPU backend is **not delivered** by this package. No GPU execution or performance advantage is claimed.

The implementation is independently authored from the stabilizer formalism and public mathematical references. No QuaSARQ source or documentation was used. Public ruqu circuit and gate declarations were inspected to identify a compatible future adapter boundary; existing stabilizer implementation code was not used. This records the development boundary, not a legal certification.

## Run a Bell circuit

From the repository root:

```sh
printf 'H 0\nCX 0 1\nM 0\nM 1\n' | cargo run --manifest-path integrations/independent-stabilizer/Cargo.toml -- 2 42
cargo test --manifest-path integrations/independent-stabilizer/Cargo.toml
```

The CLI requires exactly two decimal arguments: the qubit count and an unsigned 64-bit seed. It initializes the zero state and reads a UTF-8 program from standard input. Empty lines and `#` comments, including trailing comments, are accepted. Operation names are case-sensitive.

| Instruction | Meaning |
| --- | --- |
| `H q` | Hadamard |
| `S q` | Phase Clifford gate |
| `X q`, `Y q`, `Z q` | Pauli gate |
| `CX c t` | Controlled X; distinct control and target |
| `M q` | Z measurement; append one output event |
| `R q` | Z measurement followed by correction to zero; no output event |

Qubit indices start at zero. Unsupported operations, incorrect operand counts, invalid indices, and equal CX operands reject the entire program before execution. Input is limited to 1 MiB and 100,000 physical lines, including comments and empty lines. The tableau constructor enforces its own qubit and allocation limits. No implicit final measurements are performed.

Output starts with `backend=independent-rust-scalar` and the qubit count. Each explicit measurement produces `measurement[N] qubit=Q value=B` in program order, followed by the total measurement count. Bell measurements agree; the observed bit depends on the seed. Errors go to standard error with a nonzero exit status; output is buffered until execution succeeds.

## Seeded measurement choices

The CLI uses SplitMix64 with wrapping unsigned 64-bit arithmetic. The initial state is exactly the supplied seed, including zero. For each `M` or `R`, increment the state by `0x9e3779b97f4a7c15`; mix with xor/right-shift 30 and multiply by `0xbf58476d1ce4e5b9`, then xor/right-shift 27 and multiply by `0x94d049bb133111eb`, then xor/right-shift 31. The low output bit is the supplied random measurement choice. One bit is consumed even when the measurement is deterministic. This generator is for repeatable simulation, not cryptography. The library accepts the random bit explicitly, allowing another caller-owned generator.

## References and scope

- Scott Aaronson and Daniel Gottesman, [Improved Simulation of Stabilizer Circuits](https://arxiv.org/abs/quant-ph/0406196).
- Daniel Gottesman, [The Heisenberg Representation of Quantum Computers](https://arxiv.org/abs/quant-ph/9807006).
- Existing public [ruqu circuit API](https://github.com/ruvnet/ruqu/blob/eb127994aaf8cdb074ebcc3ff32c1a798b79efa8/crates/ruqu-core/src/circuit.rs) and [gate API](https://github.com/ruvnet/ruqu/blob/eb127994aaf8cdb074ebcc3ff32c1a798b79efa8/crates/ruqu-core/src/gate.rs), inspected at that pinned commit.

This CLI is deliberately a small circuit protocol, not a parser for OpenQASM or a replacement for the existing ruqu simulator. No adapter accepting the existing `ruqu_core::QuantumCircuit` type is delivered. Unsupported gates are rejected. Future GPU work needs a separately validated asynchronous executor, device-limit handling, and CPU/GPU correctness tests before any GPU capability claim.

To roll back the experiment, remove this isolated integration directory and its associated integration-only changes. No default backend selection changes are required.
# Validation and diagnostic timing

Run `cargo test --manifest-path integrations/independent-stabilizer/Cargo.toml --locked --offline`
from the repository root. The tests include an independent dense oracle, separate
development and confirmation seeds, sparse analytic checks across packed-word
boundaries, and the complete circuit CLI. See [ADR.md](ADR.md) for the source
separation protocol and [EVIDENCE.md](EVIDENCE.md) for verified run outcomes.

For process-inclusive diagnostic timing, build with
`cargo build --release --manifest-path integrations/independent-stabilizer/Cargo.toml --locked --offline`
and run `python3 integrations/independent-stabilizer/benchmark.py`.
This script has no competing simulator baseline and establishes no speedup.
