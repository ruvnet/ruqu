# ruqu Quantum Explorer

A responsive, animated browser laboratory powered by the real ruqu Rust WebAssembly engine. No framework, external CDN, remote simulator or JavaScript numerical fallback.

## Explore

* Circuit lab: compose H, X, Z, T, S and CNOT gates on 2 to 8 qubits, inspect exact statevector probabilities, load Bell, GHZ and interference examples, and remove individual gates.
* Magic factory: exact ideal 15 to 1 estimates, seeded counts, conservative joint confidence intervals and expected resource accounting across up to three stages.
* Cultivation: the published fault distance 3 cultivation stage with 13 data qubits and 3 cat ancillas, two verification rounds and bounded circuit level noise trajectories. Escape and the final decoder are excluded.
* Evidence: export the latest request, model, seed, result and browser timing as JSON. Browser RVF export is not implemented; use the CLI for RVF packaging.

## Build and serve

Build the current `ruqu-wasm` crate with wasm-pack, targeting web and writing output to `explorer/wasm` with the basename `ruqu_wasm`. Serve the repository over HTTP and open `/explorer/`. Do not open `index.html` as a file URL; module workers and WASM loading require HTTP.

The deployment must include `index.html`, `styles.css`, `app.js`, `worker.js`, `assets/`, `wasm/ruqu_wasm.js` and `wasm/ruqu_wasm_bg.wasm`. All links are relative for GitHub Pages at `/ruqu/explorer/`.

## Worker contract

The module worker emits `ready` after the compiled engine initializes. It accepts one circuit or research job at a time and returns `{type:'result',id,result,elapsed_ms}` or `{type:'error',id,error}`. Initialization failures emit `init-error` and disable simulation controls. Cancellation terminates and recreates the worker.

Circuit jobs: `{type:'circuit',id,qubits,gates:[{name:'h',qubit:0},{name:'cnot',qubit:0,target:1}]}`. Caps: 2 to 8 qubits, 64 gates. Research jobs: `{type:'research',id,request:{op:'magic-factory',input_error:0.01,shots:10000,seed:'42',rounds:1}}` or cultivation requests. Browser caps: 100,000 factory attempts, three factory stages, 32 cultivation attempts, noise probability up to 0.05 for cultivation. Inputs are checked before WASM execution.

## Validation

Run `node --check explorer/app.js` and `node --check explorer/worker.js`. In a real browser confirm Bell produces 00/11 at 50%, GHZ produces 000/111 at 50%, interference produces 01 at 100%, and accepted plus rejected equals attempts. Check the 320px viewport, reduced motion, keyboard access, worker cancellation and missing WASM error behavior. No hardware performance is inferred from browser simulation.
