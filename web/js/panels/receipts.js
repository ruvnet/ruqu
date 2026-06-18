// Receipts / Audit panel — shared WasmReceiptLog view: entry list, verify
// chain, export JSONL.
import { el, clear, shortHash, gateBadge } from "../util.js";

export function createReceiptsPanel(ctx) {
  const root = el("section", { class: "panel" });
  const listHost = el("div", { class: "receipt-list" });
  const verifyHost = el("span", { class: "chip chip-muted" }, "not verified");
  const tipHost = el("span", { class: "mono small" }, "—");

  // Parsed entries are mirrored in ctx so panels appending receipts can keep a
  // readable view (the WasmReceiptLog stores raw JSONL).
  function renderList() {
    clear(listHost);
    const entries = ctx.receiptEntries || [];
    if (!entries.length) {
      listHost.append(el("div", { class: "muted" }, "No receipts yet. Collapse a field, run RAG/consensus, or watch the live gate to populate the chain."));
    } else {
      const tbody = el("tbody");
      entries.forEach((e, i) => {
        tbody.append(
          el(
            "tr",
            {},
            el("td", {}, String(i)),
            el("td", {}, e.selected_id || "—"),
            el("td", {}, gateBadge(e.gate_decision || e.gate)),
            el("td", { class: "mono small" }, shortHash(e.entry_hash, 16)),
            el("td", { class: "mono small" }, shortHash(e.field_hash, 12))
          )
        );
      });
      listHost.append(el("table", { class: "data-table" }, el("thead", {}, el("tr", {}, el("th", {}, "seq"), el("th", {}, "selected"), el("th", {}, "gate"), el("th", {}, "entry_hash"), el("th", {}, "field_hash"))), tbody));
    }
    try {
      const len = ctx.receiptLog.len();
      const tip = len ? ctx.receiptLog.tip() : "—";
      tipHost.textContent = `${len} entries · tip ${shortHash(tip, 16)}`;
    } catch {
      tipHost.textContent = "—";
    }
  }

  function verify() {
    let ok = false;
    try {
      ok = ctx.receiptLog.verify();
    } catch (err) {
      verifyHost.textContent = "verify error: " + err;
      verifyHost.className = "chip chip-bad";
      return;
    }
    verifyHost.textContent = ok ? "chain valid ✓" : "chain BROKEN ✗";
    verifyHost.className = "chip " + (ok ? "chip-ok" : "chip-bad");
  }

  function exportJsonl() {
    let text = "";
    try {
      text = ctx.receiptLog.to_jsonl();
    } catch (err) {
      verifyHost.textContent = "export error: " + err;
      verifyHost.className = "chip chip-bad";
      return;
    }
    const blob = new Blob([text], { type: "application/x-ndjson" });
    const url = URL.createObjectURL(blob);
    const a = el("a", { href: url, download: "ruqu-receipts.jsonl" });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  function render() {
    clear(root);
    root.append(
      el("h1", { class: "panel-title" }, "Receipts / Audit"),
      el("p", { class: "lead" }, "Every collapse across the console can be appended to a single tamper-evident, hash-chained receipt log. Verify integrity or export the full chain as JSON Lines."),
      el(
        "div",
        { class: "card" },
        el(
          "div",
          { class: "row gap wrap center" },
          el("button", { class: "btn", onclick: verify }, "Verify chain"),
          el("button", { class: "btn btn-ghost", onclick: exportJsonl }, "Export JSONL"),
          verifyHost,
          tipHost
        ),
        listHost
      )
    );
    renderList();
  }

  return {
    root,
    render,
    onShow: render,
    // main calls this whenever a receipt is appended anywhere.
    notifyAppended: renderList,
  };
}
