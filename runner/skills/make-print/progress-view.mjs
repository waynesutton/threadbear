// the progress print for a long task (progress.mjs keeps its state): what's
// done, what's under way, questions waiting on the user with what happens
// by default, anything stuck, and the latest deliverables. Panels with
// nothing in them are left out, so each task gets only the panels it needs.
// Every time is the real clock's, said as an age at the moment of building;
// the page is rebuilt after every step, so the ages stay true.

const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

export function ago(iso, now = Date.now()) {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "";
  const s = Math.max(0, Math.round((now - t) / 1000));
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.round(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ${Math.round((s % 3600) / 60)}m ago`;
  return `${Math.floor(s / 86400)}d ago`;
}

export function took(from, to = Date.now()) {
  const s = Math.max(0, Math.round((to - Date.parse(from)) / 1000));
  if (s < 3600) return `${Math.max(1, Math.round(s / 60))}m`;
  return `${Math.floor(s / 3600)}h ${Math.round((s % 3600) / 60)}m`;
}

const clock = (iso) => {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : d.toLocaleString("en-US", { hour: "numeric", minute: "2-digit", month: "short", day: "numeric" });
};

// Each status has a mark, a label and a colour; never colour alone.
const STATUS = {
  done: { mark: "✓", label: "Done", cls: "good" },
  doing: { mark: "●", label: "Doing", cls: "acc" },
  todo: { mark: "○", label: "To do", cls: "dim" },
  stuck: { mark: "!", label: "Stuck", cls: "bad" },
  skipped: { mark: "–", label: "Skipped", cls: "dim" },
};

export function progressBody(s, now = Date.now()) {
  const steps = s.steps ?? [];
  const counted = steps.filter((x) => x.status !== "skipped");
  const done = counted.filter((x) => x.status === "done").length;
  const pct = counted.length ? Math.round((done / counted.length) * 100) : 0;
  const doing = steps.find((x) => x.status === "doing");
  const open = (s.questions ?? []).filter((q) => q.answer === undefined);
  const answered = (s.questions ?? []).filter((q) => q.answer !== undefined);
  const stuck = [...(s.stuck ?? []), ...steps.filter((x) => x.status === "stuck").map((x) => ({ what: x.title, why: x.note ?? "", since: x.at }))];
  const delivered = s.deliverables ?? [];
  const note = (s.notes ?? [])[0];

  const head = `<section class="card pg-head">
  <div class="pg-top"><div><div class="pg-task">${esc(s.task)}</div>${doing ? `<div class="pg-now"><b>Now:</b> ${esc(doing.title)}${doing.note ? ` <span class="dim">· ${esc(doing.note)}</span>` : ""}</div>` : ""}</div>
  <div class="pg-time"><b>${took(s.startedAt, now)}</b><span>running since ${esc(clock(s.startedAt))}</span></div></div>
  <div class="pg-bar" role="img" aria-label="${done} of ${counted.length} steps done"><i style="width:${pct}%"></i></div>
  <div class="pg-count"><span>${done} of ${counted.length} steps done · ${pct}%</span><span>updated ${esc(ago(s.updatedAt, now))}</span></div>
  ${note ? `<div class="pg-note">${esc(note.text)} <span class="dim">· ${esc(ago(note.at, now))}</span></div>` : ""}
</section>`;

  const tiles = `<div class="tiles">
  <div class="card tile"><div class="k">Done</div><div class="v">${done}<span class="of">/${counted.length}</span></div></div>
  <div class="card tile${open.length ? " warn" : ""}"><div class="k">Waiting on you</div><div class="v">${open.length}</div>${open.length ? '<div class="k">it keeps going with the defaults</div>' : ""}</div>
  <div class="card tile${stuck.length ? " badtile" : ""}"><div class="k">Stuck</div><div class="v">${stuck.length}</div></div>
  <div class="card tile"><div class="k">Delivered</div><div class="v">${delivered.length}</div></div>
</div>`;

  const questions = open.length
    ? `<section class="card"><h2>Waiting on you</h2><ul class="pg-list">${open
        .map((q) => `<li class="q"><div><b>${esc(q.question)}</b></div><div class="def"><span class="tag">If no answer</span>${esc(q.default)}</div><div class="dim when">asked ${esc(ago(q.asked, now))}</div></li>`)
        .join("")}</ul><p class="more">Answer in the thread. Anything that can't be undone, like deleting or publishing, waits for you.</p></section>`
    : "";
  const stuckPanel = stuck.length
    ? `<section class="card"><h2>Stuck</h2><ul class="pg-list">${stuck.map((x) => `<li><div><span class="st bad">! Stuck</span> <b>${esc(x.what)}</b></div>${x.why ? `<div class="dim">${esc(x.why)}</div>` : ""}<div class="dim when">since ${esc(ago(x.since, now))}</div></li>`).join("")}</ul></section>`
    : "";
  const stepsPanel = steps.length
    ? `<section class="card"><h2>Steps</h2><ol class="pg-steps">${steps
        .map((x) => {
          const st = STATUS[x.status] ?? STATUS.todo;
          return `<li class="${x.status}"><span class="st ${st.cls}"><i>${st.mark}</i>${st.label}</span><span class="t">${esc(x.title)}${x.note ? `<span class="dim"> · ${esc(x.note)}</span>` : ""}</span><span class="dim when">${x.status === "todo" ? "" : esc(ago(x.at, now))}</span></li>`;
        })
        .join("")}</ol></section>`
    : "";
  const deliverPanel = delivered.length
    ? `<section class="card"><h2>Latest deliverables</h2><ul class="pg-list">${delivered
        .slice(0, 8)
        .map((d) => `<li><div><b>${esc(d.what)}</b></div>${d.where ? `<div class="dim mono">${/^https?:\/\//.test(d.where) ? `<a href="${esc(d.where)}">${esc(d.where)}</a>` : esc(d.where)}</div>` : ""}<div class="dim when">${esc(ago(d.at, now))}</div></li>`)
        .join("")}</ul></section>`
    : "";
  const answeredPanel = answered.length
    ? `<details class="card answered"><summary>Answered (${answered.length})</summary><ul class="pg-list">${answered.map((q) => `<li><div>${esc(q.question)}</div><div><b>${esc(q.answer)}</b></div></li>`).join("")}</ul></details>`
    : "";

  const side = [questions, stuckPanel].filter(Boolean);
  return `${head}\n${tiles}\n${side.length ? `<div class="${side.length > 1 ? "grid2" : ""}">${side.join("")}</div>` : ""}\n<div class="${stepsPanel && deliverPanel ? "grid2 wide" : ""}">${stepsPanel}${deliverPanel}</div>\n${answeredPanel}`;
}

export const PROGRESS_CSS = `
.pg-head{display:grid;gap:10px}
.pg-top{display:flex;justify-content:space-between;gap:16px;align-items:flex-start}
.pg-task{font-size:15px;font-weight:600}
.pg-now{margin-top:4px}
.pg-time{text-align:right;white-space:nowrap}
.pg-time b{display:block;font-size:22px;font-variant-numeric:tabular-nums}
.pg-time span{color:var(--dim);font-size:11px}
.pg-bar{height:8px;border-radius:4px;background:var(--line);overflow:hidden}
.pg-bar i{display:block;height:100%;background:var(--accent);border-radius:4px}
.pg-count{display:flex;justify-content:space-between;color:var(--dim);font-size:11.5px}
.pg-note{border-left:3px solid var(--accent);padding-left:10px}
.tile .of{font-size:14px;color:var(--dim);font-weight:500}
.tile.badtile{border-color:color-mix(in srgb,var(--bad) 45%,var(--line))}.tile.badtile .v{color:var(--bad)}
h2{margin:0 0 10px!important;font-size:13.5px!important}
.pg-list{list-style:none;margin:0;padding:0;display:grid;gap:10px}
.pg-list li{display:grid;gap:2px}
.q .def{display:flex;gap:8px;align-items:baseline}
.tag{font-size:10.5px;color:var(--amber);border:1px solid var(--amberline);border-radius:4px;padding:0 5px;white-space:nowrap}
.when{font-size:11px}
.mono{font:11.5px ui-monospace,"SF Mono",Menlo,monospace;word-break:break-all}
.pg-steps{list-style:none;margin:0;padding:0}
.pg-steps li{display:grid;grid-template-columns:86px 1fr auto;gap:10px;align-items:baseline;padding:6px 0;border-bottom:1px solid var(--line)}
.pg-steps li:last-child{border-bottom:0}
.pg-steps li.done .t{color:var(--dim)}
.pg-steps li.doing .t{font-weight:600}
.st{font-size:11px;white-space:nowrap}
.st i{font-style:normal;display:inline-block;width:14px;text-align:center;margin-right:4px}
.st.good{color:var(--good)}.st.acc{color:var(--accent)}.st.bad{color:var(--bad);font-weight:600}.st.dim{color:var(--dim)}
.answered summary{cursor:pointer;color:var(--dim)}
.answered ul{margin-top:10px}
.grid2.wide{grid-template-columns:1.4fr 1fr}
main>*+*{margin-top:var(--gap)}main>header+*{margin-top:0}
@media (max-width:700px){.grid2.wide{grid-template-columns:1fr}.pg-steps li{grid-template-columns:76px 1fr}.pg-steps li .when{display:none}}
`;
