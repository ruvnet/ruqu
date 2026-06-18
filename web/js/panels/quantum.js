// Quantum panel — GHZ probability bars + a Grover search runner, showing the
// quantum engine is live in-browser.
import { el, clear, fmt, barChart } from "../util.js";

export function createQuantumPanel(ctx) {
  const root = el("section", { class: "panel" });
  let ghzN = 3;
  let groverN = 4;
  let groverTarget = 9;
  let groverSeed = 12;

  const ghzHost = el("div", {});
  const groverHost = el("div", {});

  function basisLabels(n) {
    const out = [];
    for (let i = 0; i < 1 << n; i++) out.push("|" + i.toString(2).padStart(n, "0") + "⟩");
    return out;
  }

  function runGhz() {
    let res;
    try {
      res = ctx.rt.quantum_ghz(ghzN);
    } catch (err) {
      clear(ghzHost);
      ghzHost.append(el("div", { class: "error-inline" }, "quantum_ghz failed: " + err));
      return;
    }
    clear(ghzHost);
    ghzHost.append(
      el("div", { class: "muted small" }, `${res.num_qubits}-qubit GHZ — only |0…0⟩ and |1…1⟩ carry weight (~0.5 each).`),
      barChart(basisLabels(res.num_qubits), res.probabilities || [], { height: 150 })
    );
  }

  function runGrover() {
    let res;
    try {
      res = ctx.rt.quantum_grover(groverN, groverTarget, groverSeed);
    } catch (err) {
      clear(groverHost);
      groverHost.append(el("div", { class: "error-inline" }, "quantum_grover failed: " + err));
      return;
    }
    clear(groverHost);
    const measured = "|" + Number(res.measured_state).toString(2).padStart(groverN, "0") + "⟩";
    groverHost.append(
      el(
        "div",
        { class: "readout-grid" },
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Measured"), el("div", { class: "metric-val" }, measured)),
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Found target"), el("div", { class: "action-pill " + (res.target_found ? "action-exec" : "action-defer") }, res.target_found ? "YES" : "NO")),
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Success prob"), el("div", { class: "metric-val" }, fmt(res.success_probability, 3))),
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Iterations"), el("div", { class: "metric-val" }, String(res.num_iterations)))
      )
    );
  }

  function render() {
    clear(root);
    root.append(
      el("h1", { class: "panel-title" }, "Quantum"),
      el("p", { class: "lead" }, "The same in-browser quantum engine that powers interference reranking, exposed directly: a GHZ entangled-state distribution and a Grover amplitude-amplification search."),
      el(
        "div",
        { class: "split" },
        el(
          "div",
          { class: "card" },
          el("h2", {}, "GHZ state"),
          el("label", { class: "field-inline" }, "qubits ", el("input", {
            type: "range",
            min: "1",
            max: "8",
            value: String(ghzN),
            oninput: (e) => {
              ghzN = parseInt(e.target.value, 10);
              ghzNOut.textContent = ghzN;
              runGhz();
            },
          }), (ghzNOut = el("span", { class: "slider-out" }, String(ghzN)))),
          ghzHost
        ),
        el(
          "div",
          { class: "card" },
          el("h2", {}, "Grover search"),
          el(
            "div",
            { class: "row gap wrap" },
            el("label", { class: "field-inline" }, "qubits ", el("input", {
              type: "number",
              min: "1",
              max: "12",
              class: "seed-input",
              value: String(groverN),
              oninput: (e) => {
                groverN = Math.max(1, Math.min(12, parseInt(e.target.value, 10) || 1));
                if (groverTarget >= 1 << groverN) groverTarget = (1 << groverN) - 1;
              },
            })),
            el("label", { class: "field-inline" }, "target ", el("input", {
              type: "number",
              min: "0",
              class: "seed-input",
              value: String(groverTarget),
              oninput: (e) => {
                groverTarget = Math.max(0, parseInt(e.target.value, 10) || 0);
              },
            })),
            el("label", { class: "field-inline" }, "seed ", el("input", {
              type: "number",
              class: "seed-input",
              value: String(groverSeed),
              oninput: (e) => {
                groverSeed = parseInt(e.target.value, 10) || 0;
              },
            })),
            el("button", { class: "btn", onclick: runGrover }, "Run Grover")
          ),
          groverHost
        )
      )
    );
    runGhz();
    runGrover();
  }

  let ghzNOut;

  return {
    root,
    render,
    onShow: render,
  };
}
