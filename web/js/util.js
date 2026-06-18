// util.js — small, dependency-free helpers shared by panels: DOM creation,
// number formatting, gate styling, and hand-rolled SVG/canvas charts.

// ---------------------------------------------------------------------------
// DOM
// ---------------------------------------------------------------------------

/** Create an element with attributes/props and children. */
export function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v == null || v === false) continue;
    if (k === "class") node.className = v;
    else if (k === "style") node.setAttribute("style", v);
    else if (k === "html") node.innerHTML = v;
    else if (k.startsWith("on") && typeof v === "function") {
      node.addEventListener(k.slice(2).toLowerCase(), v);
    } else if (k === "for") node.htmlFor = v;
    else if (k in node && k !== "list") {
      try {
        node[k] = v;
      } catch {
        node.setAttribute(k, v);
      }
    } else node.setAttribute(k, v);
  }
  for (const c of children.flat()) {
    if (c == null || c === false) continue;
    node.append(c.nodeType ? c : document.createTextNode(String(c)));
  }
  return node;
}

/** Clear all children of a node. */
export function clear(node) {
  while (node.firstChild) node.removeChild(node.firstChild);
  return node;
}

/** Namespaced SVG element. */
export function svgEl(tag, attrs = {}, ...children) {
  const node = document.createElementNS("http://www.w3.org/2000/svg", tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v == null || v === false) continue;
    node.setAttribute(k, String(v));
  }
  for (const c of children.flat()) {
    if (c == null || c === false) continue;
    node.append(c.nodeType ? c : document.createTextNode(String(c)));
  }
  return node;
}

// ---------------------------------------------------------------------------
// Formatting / gates
// ---------------------------------------------------------------------------

export function fmt(n, digits = 3) {
  if (n == null || Number.isNaN(n)) return "—";
  if (typeof n !== "number") return String(n);
  return n.toFixed(digits);
}

export function shortHash(h, n = 10) {
  if (!h) return "—";
  return String(h).slice(0, n);
}

/** Normalize a gate string ("Permit"/"PERMIT") to a canonical upper token. */
export function gateToken(gate) {
  const g = String(gate || "").toUpperCase();
  if (g.includes("PERMIT")) return "PERMIT";
  if (g.includes("DEFER")) return "DEFER";
  if (g.includes("DENY")) return "DENY";
  return g || "—";
}

/** CSS class suffix for a gate token: permit|defer|deny. */
export function gateClass(gate) {
  const t = gateToken(gate).toLowerCase();
  if (t === "permit" || t === "defer" || t === "deny") return t;
  return "unknown";
}

/** Build a color-coded gate badge element. */
export function gateBadge(gate, { big = false } = {}) {
  const t = gateToken(gate);
  return el("span", { class: `gate-badge gate-${gateClass(gate)}${big ? " gate-big" : ""}` }, t);
}

// ---------------------------------------------------------------------------
// Charts (hand-rolled SVG / canvas, no libraries)
// ---------------------------------------------------------------------------

/**
 * Horizontal coherence/value gauge as an SVG.
 * value in 0..1.
 */
export function gauge(value, { width = 220, label = "" } = {}) {
  const v = Math.max(0, Math.min(1, Number(value) || 0));
  const h = 16;
  const cls = v >= 0.66 ? "permit" : v >= 0.33 ? "defer" : "deny";
  const wrap = el("div", { class: "gauge" });
  const svg = svgEl("svg", { viewBox: `0 0 ${width} ${h}`, width: "100%", height: h, class: "gauge-svg" });
  svg.append(svgEl("rect", { x: 0, y: 0, width, height: h, rx: 4, class: "gauge-track" }));
  svg.append(svgEl("rect", { x: 0, y: 0, width: width * v, height: h, rx: 4, class: `gauge-fill gauge-${cls}` }));
  wrap.append(svg);
  if (label) wrap.append(el("div", { class: "gauge-label" }, `${label}: ${fmt(v)}`));
  return wrap;
}

/**
 * Vertical bar chart from {labels:[], values:[]} as an SVG. Highlights an
 * optional index (e.g. the selected / argmax bar).
 */
export function barChart(labels, values, { width = 360, height = 120, highlight = -1 } = {}) {
  const n = values.length || 1;
  const pad = 4;
  const gap = 4;
  const barW = Math.max(2, (width - pad * 2 - gap * (n - 1)) / n);
  const max = Math.max(0.0001, ...values.map((v) => Number(v) || 0));
  const baseY = height - 18;
  const svg = svgEl("svg", { viewBox: `0 0 ${width} ${height}`, width: "100%", class: "bar-chart" });
  values.forEach((raw, i) => {
    const v = Number(raw) || 0;
    const bh = (v / max) * (baseY - 6);
    const x = pad + i * (barW + gap);
    const y = baseY - bh;
    svg.append(
      svgEl("rect", {
        x,
        y,
        width: barW,
        height: Math.max(0, bh),
        rx: 2,
        class: i === highlight ? "bar bar-hl" : "bar",
      })
    );
    svg.append(
      svgEl("text", { x: x + barW / 2, y: baseY + 12, "text-anchor": "middle", class: "bar-label" }, labels[i] != null ? String(labels[i]) : "")
    );
    svg.append(
      svgEl("text", { x: x + barW / 2, y: y - 3, "text-anchor": "middle", class: "bar-value" }, fmt(v, 2))
    );
  });
  return svg;
}

/**
 * Rolling sparkline over a numeric series (0..1). Returns an SVG element.
 */
export function sparkline(series, { width = 320, height = 60, gateColors = null } = {}) {
  const svg = svgEl("svg", { viewBox: `0 0 ${width} ${height}`, width: "100%", height, class: "sparkline" });
  svg.append(svgEl("rect", { x: 0, y: 0, width, height, class: "spark-bg" }));
  if (!series.length) return svg;
  const n = series.length;
  const step = n > 1 ? width / (n - 1) : width;
  const pts = series.map((v, i) => {
    const cv = Math.max(0, Math.min(1, Number(v) || 0));
    const x = i * step;
    const y = height - 4 - cv * (height - 8);
    return [x, y];
  });
  // threshold guides for gate bands
  [0.33, 0.66].forEach((t) => {
    const y = height - 4 - t * (height - 8);
    svg.append(svgEl("line", { x1: 0, y1: y, x2: width, y2: y, class: "spark-guide" }));
  });
  const d = pts.map((p, i) => `${i === 0 ? "M" : "L"}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" ");
  svg.append(svgEl("path", { d, class: "spark-line", fill: "none" }));
  // dots colored by gate, if provided
  if (gateColors) {
    pts.forEach((p, i) => {
      svg.append(svgEl("circle", { cx: p[0], cy: p[1], r: 2.4, class: `spark-dot dot-${gateColors[i] || "unknown"}` }));
    });
  }
  return svg;
}

/** Pretty-print a JSON object into a <pre>. */
export function jsonBlock(obj) {
  return el("pre", { class: "json-block" }, JSON.stringify(obj, null, 2));
}

/** Collapsible receipt viewer with an append-to-audit button hook. */
export function receiptViewer(receipt, { onAppend = null, label = "Collapse receipt" } = {}) {
  const wrap = el("details", { class: "receipt" });
  const sum = el("summary", {}, label);
  wrap.append(sum);
  wrap.append(jsonBlock(receipt));
  if (onAppend) {
    wrap.append(
      el(
        "button",
        { class: "btn btn-small", onclick: () => onAppend(receipt) },
        "Append to audit log"
      )
    );
  }
  return wrap;
}
