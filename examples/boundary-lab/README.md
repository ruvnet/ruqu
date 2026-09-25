# Boundary Observatory

An offline RF experiment using eight simulated perimeter sensors around a two
metre square. It identifies active frequency channels and estimates emitter
locations. It does not identify arbitrary objects or measure quantum fields.

## Run the whole workflow

From the repository root, with Node 20 or later:

```sh
node examples/boundary-lab/run.cjs
node --test cli/tests/boundary-lab.test.cjs
node examples/boundary-lab/package-smoke.cjs
```

The example runs `plan`, `run`, a separate measurement only `predict` process,
`verify`, and `project`. It fails if the independent prediction differs, replay
fails, or the default simulation does not meet its frozen empirical gates.
Temporary files are removed. To retain all JSON artifacts in a new directory:

```sh
node examples/boundary-lab/run.cjs --output /tmp/boundary-observatory
```

Existing directories and files are not overwritten. A compact summary alone can
be saved with `--summary NEW_FILE`.

## Public CLI

```sh
node cli/bin/cli.js boundary-lab plan
node cli/bin/cli.js boundary-lab run --protocol-hash HASH --seed 20260925 --trials 100
node cli/bin/cli.js boundary-lab predict --file measurements.json
node cli/bin/cli.js boundary-lab verify --file report.json --trusted-root ROOT
node cli/bin/cli.js boundary-lab project --file report.json --trusted-root ROOT
```

Use the actual hash returned by `plan`. Retain the run root through a separately
trusted channel. Extracting a root from a possibly altered report proves only
self consistency. Exact replay additionally binds all calculations to current
source bytes, but neither mechanism proves acquisition or independent review.

`--trials` is the count **per arm**, from 100 through 500. The default produces
400 shuffled trials: 100 signal scenes, 100 empty scenes, 100 isolated injected
traces, and 100 deliberately distorted model mismatch scenes. Signal scenes have
one, two, or three emitters, with at most one emitter per known frequency.

The signal scene gate requires the exact set of active channels, every emitter
within 0.25 metres, and every quadrant correct. At least 80 percent of scenes must
pass. Empty scene false positives must be below 5 percent. Each other control
requires at least 95 percent rejection. Median location error must improve by at
least 20 percent over a calibrated power weighted sensor centroid baseline.
The verdict uses empirical rates. Marginal Wilson intervals are descriptive, not
simultaneous acceptance bounds. Sources share scenes, so source metrics are
reported as counts without invalid independent Bernoulli confidence intervals.

A valid negative hypothesis report exits successfully. Invalid inputs, stale
commitments, and verification failures exit unsuccessfully with no success JSON.

## Measurement contract

The strict schema is enforced by `cli/src/boundary-contract.cjs`.

1. Dataset: `schema`, `evidenceClass`, committed `room`, committed `sensors`, `trials`.
2. Trial: unique `id` and three `channels`, with no seed, arm, labels, or source positions.
3. Channel: fixed `frequencyHz` and eight `rssiDbm` entries in sensor order.
4. RSS values: finite numbers between negative 160 and positive 20 dBm, or `null` for missing samples.
5. Sensors: perimeter position in metres, known synthetic offset in dB, and declared calibration uncertainty.

The predictor subtracts calibration offsets, requires six available sensors,
requires six above threshold for detection, fits location and unknown power,
and abstains when residual or power bounds fail. Missing data is distinct from
no detected signal. Spatial confidence is not inferred from fit residuals.

`project` verifies before emitting local field events and an evidence linked
belief graph. These are **ruqu export contracts**, not native RuField or
WorldGraph objects. They contain raw readings, calibration descriptors, inferred
emitters, synthetic flags, report roots, and raw trial digests. No labels are
exported as graph facts. No external database is contacted. The existing native
observer adapter accepts a different protocol and is unchanged.

## Numerical reference

```sh
python -m pip install -r examples/boundary-lab/requirements-reference.txt
python examples/boundary-lab/reference.py
```

NumPy independently evaluates every grid candidate for all 299 detectable
channels. SciPy also solves continuous location and power for accepted emitters
without using their JavaScript estimates as starting points. This checks the
implementation, not whether the physical model describes an actual room.

## Limits and next physical gate

All measurements are simulated power summaries, not waveforms. The model uses
inverse square propagation with added measurement noise, calibration drift,
occasional missing sensors, and explicit interventions. It omits real indoor
multipath, polarization, antenna response, attenuation by walls, coherent
interference, and cochannel source separation. Simulator and estimator share the
same propagation assumption. Strong synthetic results cannot establish physical
accuracy. Rejecting an isolated injected sensor does not authenticate a network
or defend against coordinated spoofing.

Blinding is an API separation. All predictions are committed before scoring;
the final report reveals both seed and labels for replay. It is not secure
blinding against a researcher who controls the source. Fresh physical trials,
calibration captures, blinded operator labels, surveyed geometry, consent, and
authenticated acquisition require a new protocol. Physical sensor inputs are
deliberately rejected by this version.

No QPU, sensor purchase, RF transmission, laser, external service, publication,
or research schedule change is performed. A QPU would not validate the RF
propagation model. See [the architecture](../../docs/adr/ADR-004-boundary-observatory.md).
