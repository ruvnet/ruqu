# ruqu cloud quantum bridge

Take the Explorer circuit to Amazon Braket or Google Cirq without putting credentials in a public website. This integration supports H, X, Z, S, T and CNOT, 2 to 8 qubits and up to 64 gates. The existing factory and cultivation research protocols are not hardware exports.

## Start locally

In the [Explorer](https://ruvnet.github.io/ruqu/explorer/), compose a circuit and select **Download circuit for AWS / Google**. Save the JSON in your checkout, then:

```sh
python3 -m venv .venv
. .venv/bin/activate
pip install -r integrations/quantum-cloud/requirements.lock
python integrations/quantum-cloud/runner.py ruqu-cloud-circuit.json --provider aws
python integrations/quantum-cloud/runner.py ruqu-cloud-circuit.json --provider google
```

Both commands use real provider SDK local simulators. They need no cloud credentials and make no cloud task submissions. SDK versions and all transitive packages are pinned in the tested Python 3.12 environment. This lock is version pinned, not hash locked. Review dependencies before deployment.

Counts use the same bit order as ruqu: the rightmost bit is qubit 0. For the Bell example expect only 00 and 11, approximately half each. Repeated sampling fluctuates; it does not guarantee an exact 50/50 split. Local Google sampling uses seed 42. AWS local sampling is not seeded by this runner.

## AWS Braket

Use your host's AWS credential chain, such as a configured SSO profile or workload role. The browser never requests credentials. Choose a currently available gate based device ARN from Braket. Do not assume every device supports every circuit.

```sh
AWS_PROFILE=quantum python integrations/quantum-cloud/runner.py ruqu-cloud-circuit.json \
  --provider aws --device "$BRAKET_DEVICE_ARN" --shots 1000 \
  --submit --receipt aws-job.json
```

`--submit` is the explicit cloud action. AWS tasks may incur device, shot, simulator runtime and storage charges. The 10,000 shot cap is not a dollar budget. Check current pricing and account permissions before submitting. Default SDK S3 handling applies. OpenQASM 3 is generated from validated gate records, never accepted as executable user input. A device may reject unsupported operations.

## Google Cloud / Quantum Engine

A normal GCP account does **not** grant quantum hardware access. Google currently requires approved project and user access to the Quantum Computing Service. Authenticate on the host through Application Default Credentials. Consult your project administrator for IAM and service access.

Choose a processor and create `mapping.json` with one physical grid coordinate per logical qubit, for example `[[5,2],[5,3]]`. These coordinates are only an example, not an assertion that a processor has these qubits. The runner compiles to CZ operations and validates against the selected processor. It does not automatically route nonadjacent qubits; invalid maps fail before submission.

```sh
gcloud auth application-default login
python integrations/quantum-cloud/runner.py ruqu-cloud-circuit.json \
  --provider google --project "$GOOGLE_CLOUD_PROJECT" --processor "$QUANTUM_PROCESSOR" \
  --mapping mapping.json --shots 1000 --submit --receipt google-job.json
```

Hardware routing, calibration aware optimization, error mitigation and advantage claims are outside this release. These are provider integrations, not evidence of quantum advantage.

## Evidence and recovery

A receipt records the canonical circuit SHA256, SDK version, provider, mode, shots, bit order and results. Cloud job identifiers are flushed before waiting for results. Receipt creation is exclusive: reuse of a path is rejected before submission. A failed wait retains the job ID where available. Do not resubmit blindly: inspect the AWS task or Google job using its ID first. A network failure during submission can still leave an unknown job state; the adapter does not provide distributed exactly once execution or automatic resubmission.

## Validation

```sh
python -m unittest discover -s integrations/quantum-cloud -v
```

Nine circuits run on both actual SDK simulators: empty, asymmetric X operations, Bell, GHZ, Z interference, S phase, T phase and reversed CNOT. Tests also cover rejected inputs, shot bounds, receipt reuse, retained job IDs and Google topology failure before submission. Cloud lifecycle checks use mocks; no live hardware or tenant access is claimed.

## Official references (checked 2026-10-16)

* [AWS OpenQASM task submission](https://docs.aws.amazon.com/braket/latest/developerguide/braket-openqasm-create-submit-task.html)
* [Google access and authentication](https://quantumai.google/cirq/google/access)
* [Google Quantum Engine API](https://quantumai.google/cirq/google/engine)
