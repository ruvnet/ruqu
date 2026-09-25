# ADR 004: Boundary Observatory

Status: implemented as an offline experiment. Physical acquisition and native service integration are excluded.

Date: 2026-09-25

## Specification

The Boundary Observatory uses RF emitter localization as its first observable.
Actors are the researcher, simulator, measurement only predictor, scorer and
evidence reviewer. Inputs are a committed protocol, bounded seed and trial count.
Outputs are raw synthetic measurements, revealed scoring labels, committed
predictions, metrics, replayable evidence, and local field and belief graph
contracts. This is a classical inverse problem, not a test of holography or QMM.

Acceptance requires at least 80 percent exact signal scenes with the correct
frequency set, every emitter within 0.25 metres and every quadrant correct.
Empty scene false positives must be below 5 percent. Isolated injected traces and
deliberate model mismatch each require 95 percent rejection. Median location error
must improve at least 20 percent over a power weighted centroid baseline. Criteria
are frozen before each run and use empirical rates, not physical confidence bounds.

## Physical approximation

Each source contributes linear power `10^(P1/10) / ||x-s||^2` at sensor position
s, where P1 is received dBm at one metre. Add background power before converting
to dBm. Frequency and antenna factors are absorbed into unknown reference power;
sources occupy distinct frequency channels. This uses the free space inverse
square dependence, not a full electromagnetic or indoor channel model.
See [ITU P.525](https://www.itu.int/rec/R-REC-P.525).

Known sensor offsets, bounded random calibration drift and Gaussian measurement
error are added separately. Ten percent of signal scenes lose one sensor. The
injection control places a strong reading at one sensor. The mismatch control
alternates positive and negative 12 dB distortions around a simulated source.
These interventions do not model an empirically calibrated multipath distribution.

Inference subtracts offsets and searches a 33 by 33 grid. At each candidate it
profiles out source power using the mean of RSS plus geometric attenuation, then
minimizes RMS residual. Detection requires six sensors above threshold, bounded
power and acceptable residual. Missing readings and absent signals are distinct.
No spatial confidence interval is manufactured from a fit residual.

## Control flow

```text
plan: freeze protocol and fingerprint all model/evaluator/helper source
run: validate commitment and parameters, generate all shuffled arms
     commit labels, predict measurements only, commit predictions
     score every scene, miss, control and baseline, emit report and root
verify: bound regular-file input, require separately trusted root
        compare full digest, replay current source and require exact equality
project: verify first, export raw field events and inferred graph with lineage
```

Success: a separate predictor process reproduces all committed predictions using
only measurements. Failure: an attacker alters a verdict and recomputes its root;
full replay still rejects it. A valid negative hypothesis report is not a software
error. An attacker controlling code, verifier and trusted root is outside this model.

## Components and contracts

1. `boundary-contract.cjs`: immutable configuration and strict measurement validation.
2. `boundary-simulator.cjs`: synthetic acquisition and labels, without inference imports.
3. `boundary-inference.cjs`: grid estimator and centroid baseline, without simulator or scorer imports.
4. `boundary-lab.cjs`: provenance, orchestration, scoring, bounded reads, replay and projection.
5. Public CLI: plan, run, predict, verify and project without harness dependencies.
6. Independent Python reference: NumPy grid and SciPy continuous optimization.

The predictor never receives seed, arm, source position or labels. Unknown fields
are rejected at every measurement level. Final reports reveal labels and seed
for replay. This is logical API separation, not secure blinding or independent
acquisition. The default seed is development data.

Field and graph schemas are versioned ruqu export contracts. They preserve raw
values, units, sensor order, synthetic calibration, sequence, report root and
measurement digest. Only predictions become inferred emitters. All graph edges
resolve to exported nodes, and labels never become facts. Spatial confidence
remains null. These are not native RuField or WorldGraph types; the existing
observer adapter accepts a different protocol and remains unchanged.

## Alternatives and deployment

An offline CLI matches the existing lab, requires no new production dependencies
and can be tested without equipment. Hardware first would require instruments,
drivers and authenticated calibration absent from this task. Native adapters
must receive separate compiled round trip tests before integration is claimed.
A learned model or custom field solver adds complexity before the experimental
contract is established. The bounded estimator is independently checkable.

Deployment is the existing CLI package. No database, network service, account,
hardware or acquisition state is modified. Rollback removes the command and its
modules without a migration. Existing Rust and WASM code is unchanged.

## Statistics and security

Four balanced arms contain 100 trials each by default and at most 500 each.
All arms are reported. Scene success includes quadrant boundary failures. Median
location error is conditional on detection; missed sources remain explicit and
count as accuracy failures. Source counts share scenes and receive no independent
Bernoulli intervals. Wilson intervals on scene proportions are marginal descriptive
summaries, not simultaneous confidence statements. No adaptive stopping or seed
search occurs in a run.

Entry points are CLI options and local JSON. Reads require regular files, use
nonblocking open to reject FIFOs, and cap input at 16 MiB. Work is bounded by 2000
trials, three channels, eight sensors and the fixed grid. Unknown fields, physical
evidence labels, changed geometry, nonfinite numbers and duplicate trial IDs fail
closed. No production subprocesses, network calls, credentials or writes occur.
Examples use local child processes and temporary files with cleanup.

Hashing proves neither acquisition nor authentication. Separately retained roots
and exact replay detect changes to the stated experiment. Coordinated spoofing
requires independent sensor authentication and acquisition provenance. Physical
validation, privacy controls and native adapter review are future maintainer gates.

## Acceptance trace

1. Analytic distance, linear power addition and noiseless position tests.
2. Strict label rejection and separate process prediction equality.
3. Noisy scene, location, zone and baseline gates with all arms reported.
4. Empty, injection, mismatch and missing sample controls.
5. Wrong roots, changed inputs, forged verdicts and rehashed forgery rejection.
6. Size, count, malformed input, regular file and FIFO bounds.
7. Synthetic graph lineage, endpoint integrity and no truth leakage.
8. Fresh predefined seeds and maximum admitted run replay.
9. Independent NumPy comparison for all detectable channels and SciPy for accepted positions.
10. Full JavaScript regression suite, build, package smoke and CI commands.
