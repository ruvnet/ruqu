// Overview panel — runtime version banner, description, status chips, legend.
import { el, clear } from "../util.js";

export function createOverviewPanel(ctx) {
  const root = el("section", { class: "panel" });

  function render() {
    clear(root);
    const rt = ctx.rt;
    let ver = { console: "?", possibility: "?", sensing: "?", receipts: "?" };
    try {
      if (rt) ver = rt.version();
    } catch {
      /* keep placeholders */
    }

    root.append(
      el("h1", { class: "panel-title" }, "ruQu Structural Possibility Runtime"),
      el(
        "p",
        { class: "lead" },
        "A receipt-backed decision runtime that models options as a possibility field, ranks them by quantum-style interference, and gates each collapse PERMIT / DEFER / DENY — running entirely in your browser via WebAssembly (ADR-258)."
      ),

      el(
        "div",
        { class: "card" },
        el("h2", {}, "Runtime versions"),
        el(
          "div",
          { class: "version-grid" },
          verCell("console", ver.console),
          verCell("possibility", ver.possibility),
          verCell("sensing", ver.sensing),
          verCell("receipts", ver.receipts)
        )
      ),

      el(
        "div",
        { class: "card" },
        el("h2", {}, "Status"),
        el(
          "div",
          { class: "chips" },
          chip(ctx.wasmReady ? "WASM loaded ✓" : "WASM loading…", ctx.wasmReady ? "ok" : "warn"),
          (() => {
            const c = chip("WebSocket: " + (ctx.wsState || "idle"), wsChipClass(ctx.wsState));
            c.id = "overview-ws-chip";
            return c;
          })(),
          chip("Receipts: " + (ctx.receiptLog ? ctx.receiptLog.len() : 0), "muted")
        )
      ),

      el(
        "div",
        { class: "card" },
        el("h2", {}, "Gate legend"),
        el(
          "div",
          { class: "legend" },
          legendRow("permit", "PERMIT", "Coherent field, a clear winner — safe to act automatically."),
          legendRow("defer", "DEFER", "Ambiguous or high-entropy field — escalate for human review."),
          legendRow("deny", "DENY", "Contradicted / incoherent field — block the action.")
        )
      )
    );
  }

  function verCell(name, v) {
    return el("div", { class: "version-cell" }, el("span", { class: "version-name" }, name), el("span", { class: "version-num" }, "v" + v));
  }
  function chip(text, kind) {
    return el("span", { class: "chip chip-" + kind }, text);
  }
  function wsChipClass(state) {
    if (state === "connected") return "ok";
    if (state === "simulated") return "info";
    if (state === "error") return "bad";
    if (state === "connecting" || state === "reconnecting") return "warn";
    return "muted";
  }
  function legendRow(cls, label, desc) {
    return el(
      "div",
      { class: "legend-row" },
      el("span", { class: `gate-badge gate-${cls}` }, label),
      el("span", { class: "legend-desc" }, desc)
    );
  }

  return {
    root,
    render,
    onShow: render,
    // Allow main to live-update the WS chip without a full re-render.
    updateWs(state) {
      const c = root.querySelector("#overview-ws-chip");
      if (c) {
        c.textContent = "WebSocket: " + state;
        c.className = "chip chip-" + wsChipClass(state);
      }
    },
  };
}
