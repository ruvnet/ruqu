// Clifford (large-N) panel — drives the Aaronson–Gottesman stabilizer
// simulator exposed via the WASM module. Because a Clifford circuit is stored
// as an O(n^2)-bit tableau (not a 2^n amplitude array), this runs *thousands*
// of qubits client-side, far past the ~25-qubit state-vector memory wall.
import { el, clear, fmt } from "../util.js";

const CALLOUT =
  "State-vector simulation caps at ~25 qubits in the browser (2^n memory); this Clifford engine runs thousands (polynomial memory).";

export function createCliffordPanel(ctx) {
  const root = el("section", { class: "panel" });

  let ghzN = 1024;
  let ghzSeed = 1;
  let randN = 1024;
  let randDepth = 20;
  let randSeed = 7;

  const ghzHost = el("div", {});
  const randHost = el("div", {});

  function bitSample(bits) {
    if (!bits || !bits.length) return "—";
    return bits.map((b) => (b ? "1" : "0")).join("");
  }

  function metric(label, value, cls) {
    return el(
      "div",
      { class: "metric" },
      el("div", { class: "metric-label" }, label),
      el("div", { class: "metric-val" + (cls ? " " + cls : "") }, value)
    );
  }

  function runGhz() {
    let res;
    try {
      res = ctx.rt.clifford_ghz(ghzN, ghzSeed);
    } catch (err) {
      clear(ghzHost);
      ghzHost.append(el("div", { class: "error-inline" }, "clifford_ghz failed: " + err));
      return;
    }
    clear(ghzHost);
    ghzHost.append(
      el(
        "div",
        { class: "readout-grid" },
        metric("Qubits", String(res.num_qubits)),
        metric("Elapsed", fmt(res.elapsed_ms, 1) + " ms"),
        el(
          "div",
          { class: "metric" },
          el("div", { class: "metric-label" }, "All equal"),
          el(
            "div",
            { class: "action-pill " + (res.all_equal ? "action-exec" : "action-defer") },
            res.all_equal ? "YES" : "NO"
          )
        ),
        metric("Ones", String(res.ones) + " / " + res.num_qubits)
      ),
      el("div", { class: "muted small" }, "first " + (res.sample_prefix || []).length + " measured bits:"),
      el("pre", { class: "json-block" }, bitSample(res.sample_prefix)),
      el(
        "div",
        { class: "muted small" },
        res.all_equal
          ? "GHZ collapsed to a single basis state — every measured bit agrees, as it must."
          : "Unexpected: GHZ bits disagree."
      )
    );
  }

  function runRandom() {
    let res;
    try {
      res = ctx.rt.clifford_random(randN, randDepth, randSeed);
    } catch (err) {
      clear(randHost);
      randHost.append(el("div", { class: "error-inline" }, "clifford_random failed: " + err));
      return;
    }
    clear(randHost);
    randHost.append(
      el(
        "div",
        { class: "readout-grid" },
        metric("Qubits", String(res.num_qubits)),
        metric("Depth", String(res.depth)),
        metric("Gates", String(res.gates_applied)),
        metric("Elapsed", fmt(res.elapsed_ms, 1) + " ms"),
        metric("Ones", String(res.ones) + " / " + res.num_qubits)
      ),
      el("div", { class: "muted small" }, "first " + (res.sample_prefix || []).length + " measured bits:"),
      el("pre", { class: "json-block" }, bitSample(res.sample_prefix))
    );
  }

  function numInput(getVal, setVal, { min = 1, max = 16384, step = 1, cls = "seed-input" } = {}) {
    return el("input", {
      type: "number",
      min: String(min),
      max: String(max),
      step: String(step),
      class: cls,
      value: String(getVal()),
      oninput: (e) => {
        const v = parseInt(e.target.value, 10);
        setVal(Number.isNaN(v) ? min : Math.max(min, Math.min(max, v)));
      },
    });
  }

  function render() {
    clear(root);
    root.append(
      el("h1", { class: "panel-title" }, "Clifford (large-N)"),
      el(
        "p",
        { class: "lead" },
        "ruQu's Aaronson–Gottesman stabilizer simulator, exposed in the browser. Clifford circuits are stored as a polynomial-size tableau, so this runs thousands of qubits client-side."
      ),
      el("div", { class: "callout" }, CALLOUT),
      el(
        "div",
        { class: "split" },
        el(
          "div",
          { class: "card" },
          el("h2", {}, "GHZ state (large N)"),
          el(
            "div",
            { class: "row gap wrap" },
            el(
              "label",
              { class: "field-inline" },
              "qubits ",
              numInput(() => ghzN, (v) => (ghzN = v))
            ),
            el(
              "label",
              { class: "field-inline" },
              "seed ",
              numInput(() => ghzSeed, (v) => (ghzSeed = v), { min: 0 })
            ),
            el("button", { class: "btn", onclick: runGhz }, "Run GHZ")
          ),
          ghzHost
        ),
        el(
          "div",
          { class: "card" },
          el("h2", {}, "Random Clifford circuit"),
          el(
            "div",
            { class: "row gap wrap" },
            el(
              "label",
              { class: "field-inline" },
              "qubits ",
              numInput(() => randN, (v) => (randN = v))
            ),
            el(
              "label",
              { class: "field-inline" },
              "depth ",
              numInput(() => randDepth, (v) => (randDepth = v), { min: 1, max: 1000 })
            ),
            el(
              "label",
              { class: "field-inline" },
              "seed ",
              numInput(() => randSeed, (v) => (randSeed = v), { min: 0 })
            ),
            el("button", { class: "btn", onclick: runRandom }, "Run random circuit")
          ),
          randHost
        )
      )
    );
    runGhz();
    runRandom();
  }

  return {
    root,
    render,
    onShow: render,
  };
}
