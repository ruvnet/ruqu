# Exact circuit simulation of the f=3 cultivation stage

`ruqu_core::cultivation::run_surface_cultivation` executes a complete small non-Clifford circuit with sampled physical faults. Its protocol identity is `fold-transversal-surface-f3-2609.16929v1`.

## Source and scope

The circuit follows [Takada, Bartlett and Williamson, arXiv:2609.16929v1](https://arxiv.org/html/2609.16929v1), Appendix A.1, Table 3, Figures 7, 18 and 19(a), and Equations 183–192. Implementation is an exact 16-qubit dense state-vector trajectory engine. It does **not** implement the paper's polynomial Clifford-stabilizer simulation algorithm or reproduce its large-distance results.

The separately reviewed [Surti, Daguerre and Kim, PRX Quantum 7, 020329](https://arxiv.org/abs/2512.23799) motivates future efficient representations. Its simulation algorithm is not implemented by this module.

The complete cultivation-stage sequence is injection, rotated-to-regular code conversion, twelve syndrome measurements, two cat-assisted logical checks and twelve more syndrome measurements. Escape, fictitious error detection/correction, decoding and consuming the state in a logical gate are outside this circuit. Consequently raw full-state infidelity here must not be compared directly with the papers' decoded logical error curves.

## Checkable transcription

Original rotated data retain row-major labels 0 through 8. Added square centers are 9 at upper left, 10 at upper right, 11 at lower left and 12 at lower right. Cat qubits are 13, 14 and 15. A noiseless completed attempt uses 180 physical gates, 30 readouts and 43 preparations.

Figure 18 starts with H on 0, 2, 4, 5 and 6, then T on 4. Its CNOT sequence is `(0,3), (5,8), (6,7), (4,1), (5,4), (1,7), (2,1), (0,4), (5,7), (0,1)`. Disjoint operations in a shared figure column commute.

Conversion prepares 9 and 12 in plus states and 10 and 11 in zero states. Its two interaction layers are `(9,0), (1,10), (3,11), (12,4)` and `(9,1), (4,10), (6,11), (12,5)`. The order between layers matters.

The fold is `[0,9,4,12,8]`. Reflection pairs are `(3,1), (11,10), (7,5)` and `(6,2)`. The three-qubit cat uses the published offset tuple `(0,0,2)`. Controlled Hermitian factors use physical T/T-dagger and CNOT gates; controlled CZ factors use physical CCZ gates. All three cat readout bits must be zero after unencoding.

All X checks precede all Z checks. **Within each check this implementation orders CNOTs by ascending data index.** Appendix A.2 cites another paper for its syndrome gadgets without listing those internal orders in text. This explicit ordering is an implementation choice; agreement with the authors' noisy curves and fault-distance certification remain unestablished. The circuit is a concrete realization of the published f=3 cultivation-stage construction, not a claim of bit-identical author circuit reproduction.

## Noise and accounting

Each one-, two- or three-qubit gate is followed by a uniformly sampled nonidentity Pauli product with total probability `gate_error`, including CCZ's 63 possible faults. Preparations and measurements have separately configurable X faults. There is no idling, coherent, leakage or non-Markovian noise. Setting all three probabilities equal gives Appendix A.2's stated channel family.

The simulator stops a rejected attempt immediately. Rejection stage counts sum to rejected attempts, accepted plus rejected equals requested attempts, and all work already performed on rejected attempts is charged. This differs from the paper's accounting convention of simulating whole failed attempts and must be considered when comparing cost.

## Independent checks and numerical interpretation

The reference codewords are constructed from the six X-check binary row generators, independently of the injection circuit. The logical X boundary is `[0,3,6]` and logical Z boundary `[0,1,2]`. The target is `(zero_L + exp(i*pi/4) one_L)/sqrt(2)`.

Tests compare the injection/conversion output with this algebraic target, independently evaluate all twelve tensor-Pauli checks, verify all 39 single-data-Pauli faults leave the code space, and test that the cat gadget rejects the orthogonal logical magic state. These checks establish circuit and observable consistency, not a proof of fault distance under circuit-level noise.

Reports include acceptance Wilson intervals, accepted full-state fidelity, codespace probability and joint codespace/HXY-minus probability. Full infidelity includes residual correctable physical errors. A separately sampled ideal target-projector oracle supplies Bernoulli output-error events and its Wilson interval. Zero events always retain a nonzero upper bound. No accepted attempts produces null output metrics.

## Run

```sh
cargo test -p ruqu-core --release --test cultivation
cargo test -p ruqu-core --release cultivation::tests
cargo run -p ruqu-core --release --example cultivation
```

The dense state occupies 1 MiB; reference vectors and bookkeeping require additional memory. The native API rejects more than 10,000 attempts and any logical-check count other than two. The JSON API applies its tighter request budget. The seed is explicit; Rust and WASM execute identical mathematics and seeded RNG streams.


## Exhaustive single-fault audit

`cargo run -p ruqu-core --release --example cultivation -- --audit` enumerates all 2,485 single-fault mechanisms at 253 physical locations, with exact postselection branch weights. The audit observed zero mechanisms with accepted joint codespace/HXY-minus probability above 1e-10; the maximum was 4.14e-33 before a numerical probability clamp. Of these, 241 mechanisms had nonzero acceptance, and 121 produced accepted raw infidelity, representing residual physical errors rather than an encoded logical flip. This validates first-order behavior with ideal final error detection; it does not certify fault distance three, which also requires two-fault analysis and/or a proof. It is not a fictitious correction benchmark.

The explicit exhaustive regression is available with `cargo test -p ruqu-core --release --test cultivation -- --ignored`. Use `--trace` on the example to export the complete physical operation list as JSON and `--fault LOCATION CODE` to obtain an exact accepted-branch reference for independent simulators.
