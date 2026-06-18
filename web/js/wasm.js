// wasm.js — load and init the ruQu console WASM module exactly once,
// re-export the API, and surface a visible banner on failure.
//
// Usage:
//   import { getRuntime } from "./wasm.js";
//   const rt = await getRuntime();
//   rt.version();

import init, {
  version,
  analyze_field,
  rag_search,
  swarm_consensus,
  sensing_diagnose,
  gate_from_syndromes,
  syndromes_from_samples,
  quantum_ghz,
  quantum_grover,
  WasmReceiptLog,
} from "../pkg/ruqu_console.js";

// The full runtime surface, re-exported so panels never import pkg directly.
const API = {
  version,
  analyze_field,
  rag_search,
  swarm_consensus,
  sensing_diagnose,
  gate_from_syndromes,
  syndromes_from_samples,
  quantum_ghz,
  quantum_grover,
  WasmReceiptLog,
};

let _runtimePromise = null;
let _ready = false;

/** Whether the WASM runtime finished initializing successfully. */
export function isReady() {
  return _ready;
}

/**
 * Show a blocking, visible error banner if WASM cannot load.
 * Pure DOM, no dependency on panels.
 */
function showInitError(err) {
  const msg =
    "Failed to initialize the ruQu WASM runtime. The console cannot run.\n" +
    String(err && err.message ? err.message : err);
  // Try a dedicated banner element first, then fall back to body-prepend.
  let banner = document.getElementById("wasm-error-banner");
  if (!banner) {
    banner = document.createElement("div");
    banner.id = "wasm-error-banner";
    banner.className = "wasm-error-banner";
    if (document.body) {
      document.body.prepend(banner);
    }
  }
  banner.hidden = false;
  banner.textContent = msg;
  // eslint-disable-next-line no-console
  console.error("[ruqu] WASM init failed:", err);
}

/**
 * Initialize the WASM module once and resolve to the API object.
 * Subsequent calls return the same promise.
 */
export async function getRuntime() {
  if (_runtimePromise) return _runtimePromise;

  _runtimePromise = (async () => {
    try {
      // Relative URL so it works under a GitHub Pages subpath (/ruqu/).
      const wasmUrl = new URL("../pkg/ruqu_console_bg.wasm", import.meta.url);
      await init({ module_or_path: wasmUrl });
      _ready = true;
      return API;
    } catch (err) {
      _ready = false;
      showInitError(err);
      throw err;
    }
  })();

  return _runtimePromise;
}
