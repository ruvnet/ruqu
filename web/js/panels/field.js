// Possibility Field panel — editable candidate table, live analyze_field,
// entropy/coherence readout, gate badge, probability bar chart, collapse receipt.
import { el, clear, fmt, gauge, gateBadge, barChart, receiptViewer } from "../util.js";
import { defaultCandidates } from "../data.js";

export function createFieldPanel(ctx) {
  const root = el("section", { class: "panel" });
  let candidates = defaultCandidates();
  let seed = 42;
  let last = null;

  function compute() {
    try {
      last = ctx.rt.analyze_field(JSON.stringify(candidates), seed);
    } catch (err) {
      last = { error: String(err) };
    }
    renderReadout();
  }

  const readoutHost = el("div", { class: "field-readout" });

  function renderReadout() {
    clear(readoutHost);
    if (!last) return;
    if (last.error) {
      readoutHost.append(el("div", { class: "error-inline" }, "analyze_field failed: " + last.error));
      return;
    }
    const selIdx = candidates.findIndex((c) => c.id === last.selected_id);
    readoutHost.append(
      el(
        "div",
        { class: "readout-grid" },
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Gate"), gateBadge(last.gate, { big: true })),
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Selected"), el("div", { class: "metric-val" }, last.selected_id || "—")),
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Entropy"), el("div", { class: "metric-val" }, fmt(last.entropy))),
        el("div", { class: "metric metric-wide" }, el("div", { class: "metric-label" }, "Coherence"), gauge(last.coherence, { label: "coherence" }))
      ),
      el("div", { class: "chart-card" }, el("h3", {}, "Interference probabilities"), barChart(candidates.map((c) => c.id), last.probabilities || [], { highlight: selIdx, height: 140 })),
      el("div", { class: "muted small" }, "field_hash: " + (last.field_hash || "—")),
      last.receipt
        ? receiptViewer(last.receipt, { onAppend: ctx.appendReceipt, label: "Collapse receipt" })
        : el("div", { class: "muted" }, "No receipt (empty field).")
    );
  }

  function renderTable() {
    const tbody = el("tbody");
    candidates.forEach((c, i) => {
      const ampOut = el("span", { class: "slider-out" }, fmt(c.amplitude, 2));
      const phaseOut = el("span", { class: "slider-out" }, fmt(c.phase, 2));
      tbody.append(
        el(
          "tr",
          {},
          el("td", {}, el("input", {
            class: "id-input",
            value: c.id,
            oninput: (e) => {
              c.id = e.target.value;
              compute();
            },
          })),
          el(
            "td",
            {},
            el("input", {
              type: "range",
              min: "0",
              max: "1",
              step: "0.01",
              value: String(c.amplitude),
              oninput: (e) => {
                c.amplitude = parseFloat(e.target.value);
                ampOut.textContent = fmt(c.amplitude, 2);
                compute();
              },
            }),
            ampOut
          ),
          el(
            "td",
            {},
            el("input", {
              type: "range",
              min: "0",
              max: String(2 * Math.PI),
              step: "0.01",
              value: String(c.phase),
              oninput: (e) => {
                c.phase = parseFloat(e.target.value);
                phaseOut.textContent = fmt(c.phase, 2);
                compute();
              },
            }),
            phaseOut
          ),
          el("td", {}, el("button", {
            class: "btn btn-small btn-ghost",
            title: "remove",
            onclick: () => {
              candidates.splice(i, 1);
              render();
              compute();
            },
          }, "✕"))
        )
      );
    });
    return el(
      "table",
      { class: "data-table" },
      el("thead", {}, el("tr", {}, el("th", {}, "id"), el("th", {}, "amplitude (0..1)"), el("th", {}, "phase (0..2π)"), el("th", {}, ""))),
      tbody
    );
  }

  function render() {
    clear(root);
    root.append(
      el("h1", { class: "panel-title" }, "Possibility Field"),
      el("p", { class: "lead" }, "Edit candidate amplitudes and phases; the field is re-analyzed on every change. Candidates in phase reinforce (PERMIT); a candidate near phase π contradicts the field (DENY)."),
      el(
        "div",
        { class: "split" },
        el(
          "div",
          { class: "card" },
          el("h2", {}, "Candidates"),
          renderTable(),
          el(
            "div",
            { class: "row gap" },
            el("button", {
              class: "btn",
              onclick: () => {
                candidates.push({ id: "candidate-" + (candidates.length + 1), amplitude: 0.6, phase: 0.0 });
                render();
                compute();
              },
            }, "+ Add candidate"),
            el("button", {
              class: "btn btn-ghost",
              onclick: () => {
                candidates = defaultCandidates();
                render();
                compute();
              },
            }, "Reset preset"),
            el("label", { class: "seed-field" }, "seed ", el("input", {
              type: "number",
              value: String(seed),
              class: "seed-input",
              oninput: (e) => {
                seed = parseInt(e.target.value, 10) || 0;
                compute();
              },
            }))
          )
        ),
        el("div", { class: "card" }, el("h2", {}, "Collapse"), readoutHost)
      )
    );
    renderReadout();
  }

  return {
    root,
    render,
    onShow() {
      render();
      compute();
    },
  };
}
