# ruqu — State-of-the-Art Landscape & Positioning

*Research synthesis. Multi-source, web-researched, with load-bearing claims
adversarially verified. Vendor-reported vs peer-reviewed flags are noted
inline. Date: 2026-06.*

## Bottom line

ruqu is two projects under one roof, and they sit very differently against the
field.

- **The quantum engine** is a competent re-implementation of well-trodden
  techniques (gate fusion, SIMD, multithreading) whose real differentiator is
  *pure-Rust + in-browser WASM*, where 25 qubits is the correct ceiling and the
  competition is thin.
- **The "structural possibility runtime"** is the more interesting bet, but its
  core mechanic — amplitude+phase interference over evidence — is two decades of
  established prior art (van Rijsbergen 2004; Quantum Probability Ranking
  Principle 2009/2010; quantum cognition). ruqu's *defensible* novelty is the
  **systems integration**: interference + a coherence/entropy gate +
  tamper-evident collapse receipts, mapped to concrete governance hooks.

The biggest credibility risks: benchmarking against weak baselines (cosine,
single-agent) instead of the real SOTA (cross-encoders/ColBERT; self-consistency),
and presenting a coherence-gate latency as if it were a QEC decoder number.

---

## Half A — Quantum engine + QEC

### Simulators

Everyone uses the same playbook ruqu uses (gate fusion + SIMD/AVX + OpenMP,
optionally GPU/MPI). State-vector hits a hard wall at `2^n × 16 bytes`.

| Simulator | Type | Practical ceiling | Evidence |
|---|---|---|---|
| Qiskit Aer + cuStateVec | state-vector (GPU) | ~33 qubits/GPU, 50–90× vs CPU | vendor |
| NVIDIA cuQuantum | SV (multi-GPU) | 36 (1 node) / 40 (64×A100) | vendor |
| Google qsim | SV (CPU/GPU) | ~30 CPU / 40 on 90-core | vendor/docs |
| PennyLane Lightning | SV | ~30 single / 41 multi-node | arXiv:2403.02512 |
| QuEST | SV (MPI/GPU) | up to 44 on 4096 nodes | Nature 2019 (peer-reviewed) |
| **Stim** | **Clifford** | **~10⁴ qubits, ~1 kHz shots** | arXiv:2103.02202 (peer-reviewed) |
| quimb / cuTensorNet | tensor-network | 50–1000+ (low entanglement) | Quantum 2021 (peer-reviewed) |
| **Spinoza** | SV, pure Rust | laptop-scale | arXiv:2303.01493 |
| **ruqu** | SV+Clifford+TN, Rust/WASM | ~25 (WASM mem wall) | self-reported |

**Verdict:** On raw scale, a 25-qubit WASM state-vector sim is not competitive
with HPC engines — and shouldn't try to be. 25 qubits (~1 GB) is the genuine
browser ceiling (WASM 32-bit ~2 GB address space; Memory64 could lift it to ~28).
The real comparison set is other *browser* simulators (qulacs-wasm, qukit, Q.js),
which are thin and unmaintained — not Aer/cuQuantum.

### QEC decoders + the coherence-gate framing

SOTA real-time surface-code decoding is defined by the ~1 µs/round cycle
constraint (Google Willow: 1.1 µs) and the "backlog problem":

- **Software MWPM:** PyMatching v2 / sparse blossom — **<1 µs/round at d=17** —
  arXiv:2303.15933 (peer-reviewed).
- **Parallel MWPM:** Fusion Blossom — **1M rounds/s to d=33** — arXiv:2305.08307.
- **FPGA exact MWPM:** Micro Blossom — **~0.8 µs at d=13** — arXiv:2502.14787.
- **FPGA Union-Find/clustering:** Riverlane Collision Clustering — **<1 µs/round**
  — arXiv:2410.05202.
- **Accuracy ceiling:** AlphaQubit (Nature 2024) — ~6% fewer errors than
  tensor-network decoding, but **explicitly "still too slow" for real time**.
- **Min-cut is genuine QEC prior art:** Dennis–Kitaev–Landahl–Preskill (2002,
  quant-ph/0110143) introduced min-cut/max-flow recovery alongside MWPM.

> ⚠️ **Honesty issue.** ruqu's headline numbers (~468 ns p99 "tick," ~1,026 ns
> min-cut query, ~3.8M decisions/s) are a **coherence/health gate** ("is it safe
> to act?"), **not** per-round syndrome decoding. They are plausible as classical
> CPU graph-algorithm latencies, but carry no code distance, noise model, or
> logical error rate, and a fixed ~468 ns implies a tiny fixed graph that does
> not scale with `d`. They must **not** be presented as competitive decoder
> latencies. Frame them as *structural-health gating*; any decoder claim needs
> (distance, noise, logical error rate, throughput-vs-latency) benchmarked
> against sparse blossom / Fusion Blossom.

### Opportunities
1. **A Stim-style Clifford simulator compiled to WASM** breaks the memory wall
   *in-browser* (polynomial memory → thousands of qubits client-side) — a
   genuinely novel browser capability. (ruqu-core already has a stabilizer
   engine; it just needs WASM exposure.)
2. **Multi-paradigm in one Rust crate** (SV + Clifford + tensor-network) is an
   unoccupied niche (Spinoza is SV-only; qoqo wraps QuEST; Stim is C++).
3. **Reproducible independent Rust/WASM benchmarks** vs Spinoza/Stim/quimb are
   themselves a contribution (the field is heavily vendor-benchmarked).

---

## Half B — Structural possibility runtime

### Novelty verdict

**The core mechanic is established prior art, not novel.** Amplitude+phase
interference over evidence is the defining idea of:

- **van Rijsbergen, *The Geometry of IR* (2004)** — relevance as a Hilbert-space
  observable.
- **Quantum Probability Ranking Principle (Zuccon & Azzopardi, ECIR 2009; SIGIR
  2010)** — *the knockout citation*: interference ranks documents so that
  diverse/contradictory items interfere destructively (suppressed) — i.e.
  "interference RAG contradiction suppression," 15+ years early, with an explicit
  `√P·cos(θ)` cross-term, beating MMR/Portfolio Theory with no tuning. *(verified)*
- **Busemeyer & Bruza, *Quantum Models of Cognition and Decision* (2012/2025)** +
  **QuLBIT (2020)** — superposition of contradictory beliefs that collapse to a
  decision on measurement = ruqu's "collapse."
- **CNM (Li–Wang–Melucci, NAACL 2019)** — words as complex vectors (length=weight,
  phase=superposition) with a Born-rule measurement layer.
- **Kang, "Interaction as Interference" (2025, arXiv:2511.10018)** — Born-rule
  aggregation with a formal "Coherent Gain"/"Interference Information" metric =
  ruqu's phase-coherence metric, already named — *but on tabular ML, not
  LLMs/RAG*. *(verified)*

**Where novelty plausibly survives:** the **systems combination** — a
phase-coherence + entropy gate that decides admissibility, a deterministic
collapse, and auditable, hash-chained collapse receipts — because the
auditability literature (clinical-RAG deterministic gates, VeriGraph, C2PA) sits
*entirely separately* from the interference literature, and no source unites
them. Two findings strengthen this:

- The one clean "interference-RAG" precedent the field would cite, **Quantum-RAG
  (2025, arXiv:2508.01918), turned out to be kernel-based, not interference-based**
  *(verified — adversarial check overturned the initial claim)*, so ruqu's
  RAG-specific interference reranker is *less* anticipated than feared.
- The only direct "interference instead of voting for LLM ensembles" prior art
  ("Resonant Intelligence," 2026) is an **unvalidated preprint with zero numbers**
  — novelty opportunity *and* evidentiary vacuum.

> **Net:** the math and each metric are prior art; the integrated, auditable,
> deployed runtime is a defensible engineering/IP combination, not a new
> scientific principle. Position it that way and cite the prior art openly.

### Competitive positioning vs the real SOTA

**Reranking (`ruqu-rag`).** "+10% faithfulness over cosine" is **not meaningful**
— cosine is the universally-beaten weak floor. The real bar:
- Cross-encoders: BGE-reranker-v2-m3 ~0.66 avg nDCG@10 (BEIR); MiniLM ~1,800 docs/s.
- ColBERTv2: ~39.7% MRR@10 (MS MARCO), ~50 nDCG@10 (BEIR).
- LLM listwise (RankGPT-4 ~75.6 nDCG@10 DL19) but 100–1000× latency, and **loses
  5–15% on novel queries**.
- Test contradiction-suppression where it matters: **RGB-counterfactual,
  FaithEval, RAMDocs**, with RAGAS faithfulness/context-precision. MADAM-RAG sets
  a bar of **+15.8% on FaithEval**.

**Multi-agent (`ruqu-agent`).** The field aggregates *discretely* (self-consistency
+17.9% on GSM8K; debate; mixture-of-agents +7.6% on AlpacaEval). The hard finding:
**debate frequently fails to beat self-consistency at much higher cost**, and
**LLMs can't reliably self-verify** (Huang ICLR 2024; Stechly/Kambhampati 2024).
So interference-consensus must be benchmarked against **self-consistency** (not
single-agent), and reasoning-QEC must use **external/cross-agent** verification.

**Governance (`ruqu-receipts` + DEFER gate).** The strongest fit. **DEFER ≈ EU AI
Act Art. 14** (human oversight / "decide not to use"); **receipts ≈ Art. 12**
(automatic lifetime logging) — hash-chaining a value-add beyond what's required;
aligns with **NIST AI RMF MEASURE/MANAGE** and **C2PA**-style tamper-evident
manifests. Selective prediction / conformal abstention (−70–85% calibration
error; bounded hallucination risk) is the academic grounding for
PERMIT/DEFER/DENY.

### Metrics ruqu must report to be credible
- **Reranking:** nDCG@10 + Recall@k on BEIR/MS MARCO; RAGAS faithfulness +
  context-precision; latency for top-100/200 (near cross-encoder ~5–10 ms).
  Baselines: BM25+RRF, a cross-encoder, ColBERTv2 — *not cosine*.
- **Agents:** accuracy + token/latency cost vs **self-consistency** on
  GSM8K/MMLU; contradiction-handling on RAMDocs/FaithEval.
- **Gate:** risk-coverage curves / selective accuracy vs conformal-abstention.

---

## Prioritized recommendations

1. **Reframe the QEC/coherence numbers honestly** — call ~468 ns a
   *structural-health gate*, not a decoder. Cheap, high-integrity.
2. **Re-baseline `ruqu-rag`** with nDCG@10 / Recall@k and a *strong* baseline
   (cross-encoder/ColBERT-style), and prove contradiction-suppression on
   conflicting-evidence tasks with faithfulness/context-precision.
3. **Benchmark `ruqu-agent` interference-consensus vs self-consistency** at equal
   sample budget — the make-or-break for the headline novelty.
4. **Lead Half-B positioning with governance** (Art. 12 receipts + Art. 14 DEFER),
   not physics.
5. **Ship a WASM Clifford (Stim-style) simulator** — the one place the quantum
   engine can claim a genuinely novel *browser* capability beyond the 25-qubit
   wall.
6. **Cite the prior art openly** (QPRP, quantum cognition, Kang 2025) and frame
   ruqu as a deployable systems integration.

## Caveats
- Most simulator scale/speed numbers (cuQuantum, Aer, Lightning, vendor rerankers)
  are **vendor-reported**; Stim, QuEST, AlphaQubit, the decoder papers, QPRP, and
  self-consistency/debate results are **peer-reviewed**.
- A few primary PDFs were read via abstract/HTML, not full text — verify exact
  formulae before any publication/IP filing.
- Adversarial verification overturned the "Quantum-RAG = interference RAG" claim;
  treat single-source preprints (esp. "Resonant Intelligence") as unverified.

## Key sources
Simulators: qsim docs; NVIDIA cuQuantum blogs; Nature 2019 (QuEST); arXiv:2103.02202
(Stim); arXiv:2303.01493 (Spinoza). QEC: arXiv:2303.15933, 2305.08307, 2502.14787,
2410.05202; Nature 2024 (AlphaQubit; below-threshold); quant-ph/0110143. Prior art:
van Rijsbergen 2004; QPRP (ECIR 2009 / SIGIR 2010); Busemeyer & Bruza 2012/2025;
NAACL 2019 (CNM); arXiv:2511.10018. Reranking: arXiv:2112.01488 (ColBERTv2),
2309.15217 (RAGAS), 2309.01431 (RGB), 2504.13079 (RAMDocs), 2508.16757. Agents/
governance: arXiv:2203.11171, 2305.14325, 2406.04692, 2305.20050, 2310.01798,
2405.01563; EU AI Act Art. 12 & 14; NIST AI 100-1; C2PA 2.4.
