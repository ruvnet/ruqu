// Swarm Consensus panel — run swarm_consensus over a preset wavefront and
// render the consensus plan, minority reports, coherence, execute/defer
// decision, and the receipt (including the reasoning-QEC verifier entry).
import { el, clear, fmt, gauge, gateBadge, receiptViewer } from "../util.js";
import { defaultWavefront } from "../data.js";

export function createSwarmPanel(ctx) {
  const root = el("section", { class: "panel" });
  let wavefront = defaultWavefront();
  let seed = 5;
  let outcome = null;

  function run() {
    try {
      outcome = ctx.rt.swarm_consensus(JSON.stringify(wavefront), seed);
    } catch (err) {
      outcome = { error: String(err) };
    }
    renderOutcome();
  }

  const outcomeHost = el("div", {});

  function planDesc(id) {
    const p = wavefront.plans.find((x) => x.id === id);
    return p ? p.description : id;
  }

  function renderOutcome() {
    clear(outcomeHost);
    if (!outcome) return;
    if (outcome.error) {
      outcomeHost.append(el("div", { class: "error-inline" }, "swarm_consensus failed: " + outcome.error));
      return;
    }
    const actionExec = String(outcome.action || "").includes("execute");
    const verifiers = (outcome.receipt && outcome.receipt.verifier_results) || [];

    outcomeHost.append(
      el(
        "div",
        { class: "readout-grid" },
        el("div", { class: "metric" }, el("div", { class: "metric-label" }, "Gate"), gateBadge(outcome.gate, { big: true })),
        el(
          "div",
          { class: "metric" },
          el("div", { class: "metric-label" }, "Action"),
          el("div", { class: "action-pill " + (actionExec ? "action-exec" : "action-defer") }, actionExec ? "EXECUTE" : "DEFER FOR HUMAN REVIEW")
        ),
        el("div", { class: "metric metric-wide" }, el("div", { class: "metric-label" }, "Coherence"), gauge(outcome.coherence, { label: "coherence" }))
      ),
      el(
        "div",
        { class: "card subtle" },
        el("h3", {}, "Consensus plan"),
        el("div", { class: "consensus-plan" }, el("span", { class: "rank-id" }, outcome.consensus_plan_id || "—"), el("span", { class: "rank-text" }, planDesc(outcome.consensus_plan_id)))
      ),
      el(
        "div",
        { class: "card subtle" },
        el("h3", {}, "Minority reports"),
        (outcome.minority_reports && outcome.minority_reports.length)
          ? el("ul", { class: "minority-list" }, ...outcome.minority_reports.map((id) => el("li", {}, el("span", { class: "rank-id" }, id), " — ", planDesc(id))))
          : el("div", { class: "muted" }, "None — unanimous.")
      ),
      el(
        "div",
        { class: "card subtle" },
        el("h3", {}, "Verifiers (reasoning QEC)"),
        verifiers.length
          ? el("ul", { class: "verifier-list" }, ...verifiers.map((v) => el(
              "li",
              { class: v.passed ? "ver-pass" : "ver-fail" },
              el("strong", {}, (v.passed ? "✓ " : "✗ ") + v.name),
              el("div", { class: "rank-meta" }, v.detail || "")
            )))
          : el("div", { class: "muted" }, "No verifier entries.")
      ),
      outcome.receipt ? receiptViewer(outcome.receipt, { onAppend: ctx.appendReceipt, label: "Consensus receipt" }) : null
    );
  }

  function renderConfig() {
    const agentRows = wavefront.agents.map((a) => el("tr", {}, el("td", {}, a.id), el("td", {}, a.role), el("td", {}, fmt(a.confidence, 2))));
    const voteRows = wavefront.votes.map((v) => el(
      "tr",
      {},
      el("td", {}, v.agent_id),
      el(
        "td",
        {},
        el("select", {
          class: "select select-small",
          onchange: (e) => {
            v.plan_id = e.target.value;
          },
        }, ...wavefront.plans.map((p) => el("option", { value: p.id, selected: p.id === v.plan_id }, p.id)))
      ),
      el("td", {}, fmt(v.confidence, 2)),
      el("td", {}, v.support ? "support" : "oppose")
    ));

    return el(
      "div",
      {},
      el("h3", {}, "Agents (" + wavefront.agents.length + ")"),
      el("table", { class: "data-table" }, el("thead", {}, el("tr", {}, el("th", {}, "id"), el("th", {}, "role"), el("th", {}, "confidence"))), el("tbody", {}, ...agentRows)),
      el("h3", {}, "Plans (" + wavefront.plans.length + ")"),
      el("div", { class: "corpus-list" }, ...wavefront.plans.map((p) => el(
        "div",
        { class: "corpus-item" },
        el("span", { class: "rank-id" }, p.id),
        el("span", { class: "rank-text" }, p.description),
        el("span", { class: "rank-meta" }, "evidence " + fmt(p.evidence_support, 2) + " · steps: " + p.steps.join(" → "))
      ))),
      el("h3", {}, "Votes (editable plan choice)"),
      el("table", { class: "data-table" }, el("thead", {}, el("tr", {}, el("th", {}, "agent"), el("th", {}, "votes for"), el("th", {}, "confidence"), el("th", {}, "stance"))), el("tbody", {}, ...voteRows))
    );
  }

  function render() {
    clear(root);
    root.append(
      el("h1", { class: "panel-title" }, "Swarm Consensus"),
      el("p", { class: "lead" }, "Multiple agents vote across competing plans; collapse consensus selects the reinforced plan, records minority reports, and runs a reasoning quantum-error-correction verifier before deciding execute vs. defer-for-human-review."),
      el(
        "div",
        { class: "split" },
        el(
          "div",
          { class: "card" },
          el("h2", {}, "Wavefront"),
          renderConfig(),
          el(
            "div",
            { class: "row gap" },
            el("button", { class: "btn", onclick: run }, "Run consensus"),
            el("button", {
              class: "btn btn-ghost",
              onclick: () => {
                wavefront = defaultWavefront();
                render();
                run();
              },
            }, "Reset preset"),
            el("label", { class: "seed-field" }, "seed ", el("input", {
              type: "number",
              class: "seed-input",
              value: String(seed),
              oninput: (e) => {
                seed = parseInt(e.target.value, 10) || 0;
              },
            }))
          )
        ),
        el("div", { class: "card" }, el("h2", {}, "Outcome"), outcomeHost)
      )
    );
    renderOutcome();
  }

  return {
    root,
    render,
    onShow() {
      render();
      run();
    },
  };
}
