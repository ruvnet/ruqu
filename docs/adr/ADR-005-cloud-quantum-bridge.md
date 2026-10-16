# ADR 005: Host side quantum provider bridge

Status: implemented, provider local simulators validated; live cloud access unverified.

## Outcome and constraints

Users export an Explorer circuit to AWS Braket or Google Quantum Engine. Static Pages must never collect cloud credentials or submit billable jobs. Preserve ruqu bit order, reject unrecognized instructions, bound input size and shots, retain job identifiers. No hardware advantage or latest algorithm claims are implied.

## Flow and boundaries

Explorer state → schema version 1 JSON → validated Python runner → provider local simulator by default, or explicit `--submit` → normalized receipt. AWS uses OpenQASM 3. Google uses Cirq with a caller specified GridQubit map, CZ compilation and device validation. Unsupported topology fails before submission. Input is data only; no evaluation, dynamic imports from input, arbitrary endpoints or browser secrets.

A managed web proxy was rejected because it adds credential custody, IAM, tenancy and billing infrastructure to a static educational site. Source generation alone was rejected because it lacks reusable validation and consistent receipts. Host side adapters preserve standard AWS credentials and Google ADC boundaries.

## Security review

Assets: host credentials, cloud task authorization, receipts and experiment integrity. Untrusted entry point: downloaded JSON. Mitigations: exact key allowlists, integer index checks, 64 gate / 8 qubit limits, 64 KiB CLI input cap, 4 KiB mapping cap, 10,000 shot bound, explicit cloud flag and device identifiers. Receipt paths are chosen by the host operator and exclusively created. Python code is trusted repository code, not embedded in the input. No secrets are serialized by the runner.

Ruflo `@claude-flow/cli@3.25.6 security scan --target integrations/quantum-cloud --depth deep --type all` reported no findings on 2026-10-16. Manual boundary review confirms no input execution and no browser credentials. This scanner result is not a dependency vulnerability attestation; advisory feed freshness is not reported by the tool. requirements.lock pins the tested environment; it does not provide artifact hashes.

Residual risks: shots do not bound dollar cost, remote queue/runtime is uncontrolled, SDK/device changes may reject circuits, and an interrupted submission can have an unknown outcome. Operators must inspect provider task history rather than blindly retry. Google service approval must be obtained by the project owner. Live hardware acceptance remains pending tenant configuration and a reviewed task budget.

## Acceptance trace

* Portable gate semantics and bit order: nine circuits on both real provider SDK local simulators.
* Invalid input and bounded execution: validation tests.
* Google unsupported physical layout: mocked device rejects before run_sweep.
* Recovery: task ID retained on polling failure; receipt reuse rejected.
* Browser export: Playwright downloads and checks actual Bell JSON.
* Existing UI: full Explorer browser regression suite, including 320px and welcome dialog.
* Cloud hardware: not exercised; adapters are not represented as verified account connections.

Rollback: revert the bridge and Explorer export commits. No deployed backend or credential migrations exist; already submitted cloud jobs remain under provider control.

Tested dependency lock SHA256: `7e2a5e80e9719589ba256e32964734cbfc7b5218197173dae3a03294ae4622ac`.
