// Interference RAG panel — the headline demo. Two ranked columns side by side:
// plain cosine vs interference reranking. The contradicted-but-high-cosine doc
// tops cosine yet is suppressed to the bottom by interference.
import { el, clear, fmt, gateBadge, receiptViewer } from "../util.js";
import { defaultRagCorpus, defaultRagQuery } from "../data.js";

export function createRagPanel(ctx) {
  const root = el("section", { class: "panel" });
  const corpus = defaultRagCorpus();
  let result = null;
  let seed = 7;
  let phaseKickback = true;

  // Named query presets aligned to embedding axes [cache, db, queue, auth].
  const queries = {
    "Cache TTL (cache axis)": defaultRagQuery(),
    "DB pool (db axis)": [0.1, 0.95, 0.06, 0.04],
    "Queue retries (queue axis)": [0.06, 0.1, 0.96, 0.04],
    "Auth tokens (auth axis)": [0.04, 0.06, 0.05, 0.97],
  };
  let queryName = "Cache TTL (cache axis)";

  function textFor(id) {
    const d = corpus.find((c) => c.id === id);
    return d ? d.text : "";
  }
  function metaFor(id) {
    return corpus.find((c) => c.id === id) || {};
  }

  function compute() {
    try {
      const q = queries[queryName];
      result = ctx.rt.rag_search(JSON.stringify(q), JSON.stringify(corpus), 3, 3, phaseKickback, seed);
    } catch (err) {
      result = { error: String(err) };
    }
    renderResult();
  }

  const resultHost = el("div", {});

  function renderResult() {
    clear(resultHost);
    if (!result) return;
    if (result.error) {
      resultHost.append(el("div", { class: "error-inline" }, "rag_search failed: " + result.error));
      return;
    }

    const cosineIds = result.cosine_top_k || [];
    const interSel = result.selected || [];
    // The contradicted doc id, for highlighting in both columns.
    const contradictedId = (corpus.find((c) => (c.contradiction || 0) >= 0.5) || {}).id;

    // Plain cosine column
    const cosineCol = el("div", { class: "rank-col" }, el("h3", {}, "Plain cosine"));
    cosineIds.forEach((id, i) => {
      const isBad = id === contradictedId;
      cosineCol.append(
        el(
          "div",
          { class: "rank-item" + (isBad ? " rank-bad" : "") },
          el("div", { class: "rank-row" }, el("span", { class: "rank-num" }, "#" + (i + 1)), el("span", { class: "rank-id" }, id), isBad ? el("span", { class: "flag flag-bad" }, "contradicted") : null),
          el("div", { class: "rank-text" }, textFor(id))
        )
      );
    });

    // Interference column
    const interCol = el("div", { class: "rank-col" }, el("h3", {}, "Interference rerank"));
    interSel.forEach((s, i) => {
      const isBad = s.id === contradictedId;
      interCol.append(
        el(
          "div",
          { class: "rank-item" + (isBad ? " rank-bad" : " rank-good") },
          el(
            "div",
            { class: "rank-row" },
            el("span", { class: "rank-num" }, "#" + (i + 1)),
            el("span", { class: "rank-id" }, s.id),
            el("span", { class: "rank-score" }, "score " + fmt(s.score, 3)),
            isBad ? el("span", { class: "flag flag-bad" }, "suppressed") : null
          ),
          el("div", { class: "rank-text" }, s.text || textFor(s.id)),
          el("div", { class: "rank-meta" }, "phase " + fmt(s.phase, 2) + " · contradiction " + fmt(metaFor(s.id).contradiction || 0, 2))
        )
      );
    });

    const cosTop = cosineIds[0];
    const interWinner = interSel[0] && interSel[0].id;
    const interLast = interSel.length ? interSel[interSel.length - 1].id : null;
    const demoHit = cosTop === contradictedId && interLast === contradictedId;

    resultHost.append(
      el(
        "div",
        { class: "callout " + (demoHit ? "callout-good" : "callout-info") },
        demoHit
          ? `Cosine ranks the contradicted/stale doc “${contradictedId}” #1, but interference suppresses it to last and promotes “${interWinner}”.`
          : `Cosine top: ${cosTop || "—"} · Interference winner: ${interWinner || "—"}.`
      ),
      el("div", { class: "rank-cols" }, cosineCol, interCol),
      el("div", { class: "row gap center" }, el("span", { class: "metric-label" }, "Gate"), gateBadge(result.gate)),
      result.receipt ? receiptViewer(result.receipt, { onAppend: ctx.appendReceipt, label: "RAG collapse receipt" }) : null
    );
  }

  function render() {
    clear(root);
    root.append(
      el("h1", { class: "panel-title" }, "Interference RAG"),
      el("p", { class: "lead" }, "Retrieval reranked by quantum-style interference: documents that are merely similar (high cosine) but contradicted or stale destructively interfere and sink, while corroborated, trustworthy documents reinforce."),
      el(
        "div",
        { class: "card" },
        el(
          "div",
          { class: "row gap wrap" },
          el("label", { class: "field-inline" }, "Query ", el("select", {
            class: "select",
            onchange: (e) => {
              queryName = e.target.value;
              compute();
            },
          }, ...Object.keys(queries).map((k) => el("option", { value: k, selected: k === queryName }, k)))),
          el("label", { class: "field-inline" }, el("input", {
            type: "checkbox",
            checked: phaseKickback,
            onchange: (e) => {
              phaseKickback = e.target.checked;
              compute();
            },
          }), " phase kickback"),
          el("label", { class: "field-inline" }, "seed ", el("input", {
            type: "number",
            class: "seed-input",
            value: String(seed),
            oninput: (e) => {
              seed = parseInt(e.target.value, 10) || 0;
              compute();
            },
          }))
        ),
        resultHost
      ),
      el(
        "div",
        { class: "card" },
        el("h2", {}, "Corpus (" + corpus.length + " docs)"),
        el("div", { class: "corpus-list" }, ...corpus.map((d) => el(
          "div",
          { class: "corpus-item" + ((d.contradiction || 0) >= 0.5 ? " corpus-bad" : "") },
          el("span", { class: "rank-id" }, d.id),
          el("span", { class: "rank-text" }, d.text),
          el("span", { class: "rank-meta" }, `trust ${fmt(d.source_trust || 0, 2)} · recency ${fmt(d.recency || 0, 2)} · contradiction ${fmt(d.contradiction || 0, 2)}`)
        )))
      )
    );
    renderResult();
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
