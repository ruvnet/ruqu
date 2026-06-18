// Sensing / Live Gate panel — WebSocket-fed live coherence/gate gauge plus a
// topology diagnosis sub-action.
import { el, clear, fmt, gauge, gateBadge, barChart, sparkline } from "../util.js";
import { TelemetryClient, COMPONENT_LABELS } from "../ws.js";
import { defaultTopology, normalizeFrameBits, bandForGate } from "../data.js";

export function createSensingPanel(ctx) {
  const root = el("section", { class: "panel" });
  const client = new TelemetryClient();
  const labels = COMPONENT_LABELS.slice();
  const WINDOW = 6; // sliding window of frames fed to gate_from_syndromes

  let frameBuffer = []; // recent SensorSyndrome frames (sliding window)
  let coherenceSeries = [];
  let gateSeries = [];
  let lastGate = null;
  let eventLog = [];
  let seed = 3;

  // --- live readout hosts ---
  const gaugeHost = el("div", {});
  const gateHost = el("div", {});
  const rootCauseHost = el("div", { class: "metric-val" }, "—");
  const sparkHost = el("div", {});
  const probHost = el("div", {});
  const logHost = el("div", { class: "event-log" });
  const statusHost = el("span", { class: "chip chip-muted" }, "idle");

  function pushLog(line, kind) {
    eventLog.unshift({ t: new Date().toLocaleTimeString(), line, kind });
    if (eventLog.length > 60) eventLog.pop();
    renderLog();
  }

  function renderLog() {
    clear(logHost);
    eventLog.forEach((e) => {
      logHost.append(el("div", { class: "event-row event-" + (e.kind || "info") }, el("span", { class: "event-time" }, e.t), el("span", {}, e.line)));
    });
  }

  function renderGauge() {
    const coh = coherenceSeries.length ? coherenceSeries[coherenceSeries.length - 1] : 0;
    clear(gaugeHost);
    gaugeHost.append(gauge(coh, { label: "coherence" }));
    clear(gateHost);
    gateHost.append(gateBadge(lastGate || "—", { big: true }));
    clear(sparkHost);
    sparkHost.append(sparkline(coherenceSeries, { gateColors: gateSeries }));
  }

  function onFrame(frame) {
    // Normalize and append to sliding window.
    const bits = normalizeFrameBits(frame, labels.length);
    const norm = {
      source: frame.source || "sensor",
      detector_bits: bits,
      confidence: typeof frame.confidence === "number" ? frame.confidence : 0.9,
      timestamp_ns: frame.timestamp_ns || Date.now() * 1e6,
    };
    frameBuffer.push(norm);
    if (frameBuffer.length > WINDOW) frameBuffer.shift();

    let res;
    try {
      res = ctx.rt.gate_from_syndromes(JSON.stringify(frameBuffer), JSON.stringify(labels), seed);
    } catch (err) {
      pushLog("gate_from_syndromes error: " + err, "bad");
      return;
    }

    lastGate = res.gate;
    coherenceSeries.push(typeof res.coherence === "number" ? res.coherence : 0);
    gateSeries.push(bandForGate(res.gate));
    if (coherenceSeries.length > 48) {
      coherenceSeries.shift();
      gateSeries.shift();
    }
    rootCauseHost.textContent = res.root_cause || "—";

    clear(probHost);
    probHost.append(barChart(labels, res.probabilities || [], { height: 120, highlight: labels.indexOf(res.root_cause) }));

    renderGauge();

    const tripped = bits.map((b, i) => (b ? labels[i] : null)).filter(Boolean);
    const regime = frame.regime ? ` [${frame.regime}]` : "";
    pushLog(
      `${norm.source}${regime}: bits[${tripped.join(",") || "none"}] → ${res.gate} coh=${fmt(res.coherence, 2)} rc=${res.root_cause || "—"}`,
      bandForGate(res.gate)
    );

    // Append the receipt to the shared audit log.
    if (res.receipt) ctx.appendReceipt(res.receipt);
  }

  client.onStatus((state, detail) => {
    statusHost.textContent = state + (detail ? " · " + detail : "");
    statusHost.className = "chip chip-" + (state === "connected" ? "ok" : state === "simulated" ? "info" : state === "error" ? "bad" : state === "disconnected" || state === "idle" ? "muted" : "warn");
    ctx.setWsState(state);
    if (state !== "idle") pushLog("WebSocket: " + state + (detail ? " (" + detail + ")" : ""), state === "error" ? "bad" : "info");
  });

  // --- topology diagnosis sub-action ---
  const topology = defaultTopology();
  const diagHost = el("div", {});
  function diagnose() {
    let diag;
    try {
      diag = ctx.rt.sensing_diagnose(JSON.stringify(topology), 0.2, 4, 1);
    } catch (err) {
      clear(diagHost);
      diagHost.append(el("div", { class: "error-inline" }, "sensing_diagnose failed: " + err));
      return;
    }
    clear(diagHost);
    const scores = diag.fragility_scores || [];
    const names = scores.map((s) => s[0]);
    const vals = scores.map((s) => s[1]);
    const weakIdx = names.indexOf(diag.weakest_component);
    diagHost.append(
      el(
        "div",
        { class: "readout-grid" },
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Weakest"), el("div", { class: "metric-val" }, diag.weakest_component || "—")),
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Severity"), el("div", { class: "metric-val" }, fmt(diag.severity, 2))),
        el("div", { class: "metric metric-wide" }, el("div", { class: "metric-label" }, "Fault propagators"), el("div", { class: "metric-val small" }, (diag.fault_propagators || []).join(", ") || "—"))
      ),
      el("h3", {}, "Fragility scores"),
      barChart(names, vals, { height: 140, highlight: weakIdx })
    );
  }

  let urlInput;
  function render() {
    clear(root);
    urlInput = el("input", { class: "url-input", placeholder: "ws://localhost:8080/telemetry (blank = simulated feed)", value: "" });

    root.append(
      el("h1", { class: "panel-title" }, "Sensing / Live Gate"),
      el("p", { class: "lead" }, "A live syndrome telemetry feed is windowed and gated in real time. Leave the URL blank to run the built-in simulated source, which alternates between correlated-failure (DEFER) and isolated-fault (PERMIT) regimes so the gauge visibly moves."),
      el(
        "div",
        { class: "card" },
        el(
          "div",
          { class: "row gap wrap conn-bar" },
          urlInput,
          el("button", { class: "btn", onclick: () => { resetSeries(); client.connect(urlInput.value, onFrame); } }, "Connect"),
          el("button", { class: "btn btn-ghost", onclick: () => { resetSeries(); client.useSimulated(onFrame); } }, "Use simulated"),
          el("button", { class: "btn btn-ghost", onclick: () => client.disconnect() }, "Disconnect"),
          statusHost
        ),
        el(
          "div",
          { class: "readout-grid live-grid" },
          el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Gate"), gateHost),
          el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Root cause"), rootCauseHost),
          el("div", { class: "metric metric-wide" }, el("div", { class: "metric-label" }, "Coherence"), gaugeHost)
        ),
        el("div", { class: "chart-card" }, el("h3", {}, "Coherence (rolling)"), sparkHost),
        el("div", { class: "chart-card" }, el("h3", {}, "Root-cause probabilities"), probHost),
        el("h3", {}, "Event log"),
        logHost
      ),
      el(
        "div",
        { class: "card" },
        el("h2", {}, "Topology diagnosis"),
        el("p", { class: "muted small" }, "Components: " + topology.components.join(", ")),
        el("button", { class: "btn", onclick: diagnose }, "Diagnose topology"),
        diagHost
      )
    );
    renderGauge();
    renderLog();
  }

  function resetSeries() {
    frameBuffer = [];
    coherenceSeries = [];
    gateSeries = [];
    lastGate = null;
    renderGauge();
  }

  return {
    root,
    render,
    onShow() {
      render();
      // Auto-start the simulated feed so the panel is alive with zero backend.
      if (client.mode === "idle") {
        resetSeries();
        client.useSimulated(onFrame);
      }
    },
    onHide() {
      client.disconnect();
    },
  };
}
