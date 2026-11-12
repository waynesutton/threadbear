// Markdown for report sections: the same strict subset as the runner's doc
// pages (runner/src/page-doc.ts). Everything is escaped; links keep only
// http(s) and mailto. No HTML passes through.
const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

function inline(raw) {
  const codes = [];
  let s = raw.replace(/`([^`]+)`/g, (_, c) => {
    codes.push(`<code>${esc(c)}</code>`);
    return `\u0000${codes.length - 1}\u0000`;
  });
  s = esc(s)
    .replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, t, u) => (/^(https?:\/\/|mailto:)/i.test(u.replace(/&amp;/g, "&")) ? `<a href="${u}">${t}</a>` : t))
    .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
    .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>");
  return s.replace(/\u0000(\d+)\u0000/g, (_, i) => codes[Number(i)]);
}

export function markdown(md) {
  const out = [];
  const lines = String(md ?? "").replace(/\r\n?/g, "\n").split("\n");
  let para = [];
  const flush = () => {
    if (para.length) out.push(`<p>${inline(para.join(" "))}</p>`);
    para = [];
  };
  for (let i = 0; i < lines.length; i++) {
    const l = lines[i];
    const h = /^(#{1,4})\s+(.+)$/.exec(l);
    const item = /^\s*([-*+]|\d+[.)])\s+(.+)$/.exec(l);
    if (h) {
      flush();
      out.push(`<h${Math.min(4, h[1].length + 2)}>${inline(h[2])}</h${Math.min(4, h[1].length + 2)}>`);
    } else if (item) {
      flush();
      const ordered = /\d/.test(item[1]);
      const items = [];
      while (i < lines.length) {
        const m = /^\s*([-*+]|\d+[.)])\s+(.+)$/.exec(lines[i]);
        if (!m || /\d/.test(m[1]) !== ordered) break;
        items.push(`<li>${inline(m[2])}</li>`);
        i++;
      }
      i--;
      out.push(ordered ? `<ol>${items.join("")}</ol>` : `<ul>${items.join("")}</ul>`);
    } else if (!l.trim()) flush();
    else para.push(l.trim());
  }
  flush();
  return out.join("\n");
}
