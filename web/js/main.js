// main.js — boot WASM, build nav, mount panels, own shared state (the
// WasmReceiptLog + a parsed mirror for the audit view).
import { getRuntime, isReady } from "./wasm.js";
import { el, clear } from "./util.js";
import { createOverviewPanel } from "./panels/overview.js";
import { createFieldPanel } from "./panels/field.js";
import { createRagPanel } from "./panels/rag.js";
import { createSwarmPanel } from "./panels/swarm.js";
import { createSensingPanel } from "./panels/sensing.js";
import { createReceiptsPanel } from "./panels/receipts.js";
import { createQuantumPanel } from "./panels/quantum.js";
import { createCliffordPanel } from "./panels/clifford.js";

const NAV = [
  { id: "overview", label: "Overview", icon: "◎", make: createOverviewPanel },
  { id: "field", label: "Possibility Field", icon: "≈", make: createFieldPanel },
  { id: "rag", label: "Interference RAG", icon: "⤳", make: createRagPanel },
  { id: "swarm", label: "Swarm Consensus", icon: "⨁", make: createSwarmPanel },
  { id: "sensing", label: "Sensing / Live Gate", icon: "◉", make: createSensingPanel },
  { id: "receipts", label: "Receipts / Audit", icon: "▤", make: createReceiptsPanel },
  { id: "quantum", label: "Quantum", icon: "⚛", make: createQuantumPanel },
  { id: "clifford", label: "Clifford (large-N)", icon: "⊞", make: createCliffordPanel },
];

async function boot() {
  const statusEl = document.getElementById("boot-status");
  let rt;
  try {
    rt = await getRuntime();
  } catch {
    if (statusEl) statusEl.textContent = "WASM failed to load — see banner above.";
    return; // wasm.js already showed the error banner
  }

  // ---- shared context passed to every panel ----
  const ctx = {
    rt,
    wasmReady: isReady(),
    wsState: "idle",
    receiptLog: new rt.WasmReceiptLog(),
    receiptEntries: [], // parsed mirror for the audit table
    appendReceipt,
    setWsState,
  };

  const panels = {};
  let current = null;

  function appendReceipt(receipt) {
    try {
      const entry_hash = ctx.receiptLog.append(JSON.stringify(receipt));
      ctx.receiptEntries.push({
        entry_hash,
        selected_id: receipt.selected_id,
        gate: receipt.gate_decision,
        gate_decision: receipt.gate_decision,
        field_hash: receipt.field_hash,
      });
      if (panels.receipts && panels.receipts.notifyAppended) panels.receipts.notifyAppended();
      // keep the overview receipt count fresh if it's mounted
      if (panels.overview && current === "overview" && panels.overview.render) panels.overview.render();
    } catch (err) {
      // eslint-disable-next-line no-console
      console.error("[ruqu] receipt append failed:", err);
    }
  }

  function setWsState(state) {
    ctx.wsState = state;
    if (panels.overview && panels.overview.updateWs) panels.overview.updateWs(state);
  }

  // ---- build layout ----
  const navEl = document.getElementById("nav");
  const mainEl = document.getElementById("main");
  clear(navEl);

  function select(id) {
    if (current === id) return;
    // hide previous
    if (current && panels[current] && panels[current].onHide) panels[current].onHide();
    current = id;
    // nav active state
    navEl.querySelectorAll(".nav-item").forEach((n) => n.classList.toggle("active", n.dataset.id === id));
    // lazily create the panel
    if (!panels[id]) {
      const def = NAV.find((d) => d.id === id);
      panels[id] = def.make(ctx);
    }
    clear(mainEl);
    mainEl.append(panels[id].root);
    if (panels[id].onShow) panels[id].onShow();
    else if (panels[id].render) panels[id].render();
    // reflect in hash for deep-linking under the Pages subpath
    if (location.hash.slice(1) !== id) {
      history.replaceState(null, "", "#" + id);
    }
  }

  NAV.forEach((def) => {
    const item = el(
      "button",
      { class: "nav-item", "data-id": def.id, onclick: () => select(def.id) },
      el("span", { class: "nav-icon" }, def.icon),
      el("span", { class: "nav-label" }, def.label)
    );
    item.dataset.id = def.id;
    navEl.append(item);
  });

  window.addEventListener("hashchange", () => {
    const id = location.hash.slice(1);
    if (NAV.some((d) => d.id === id)) select(id);
  });

  const initial = NAV.some((d) => d.id === location.hash.slice(1)) ? location.hash.slice(1) : "overview";
  select(initial);

  const statusBar = document.getElementById("boot-status");
  if (statusBar) statusBar.textContent = "runtime ready";
}

boot();
