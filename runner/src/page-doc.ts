// Doc pages (the pages experiment): a page written as markdown, by an agent
// or by the user in the app's editor, drawn here as one static HTML page.
// Both go through this, so a doc looks the same whoever wrote it, and the
// markdown is kept beside it (source.md) for the next edit.
//
// A small, strict subset: headings, paragraphs, bold, italic, code, fenced
// code, lists, quotes, rules, tables and links. Everything is escaped
// first; a link only keeps an http(s) or mailto address. No HTML passes
// through, so a doc can never carry a script.

export const DOC_SOURCE = 'source.md';

const esc = (s: string): string => s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!);

function safeHref(url: string): string | undefined {
  const u = url.trim();
  return /^(https?:\/\/|mailto:)/i.test(u) ? u : undefined;
}

// Inline marks on already-escaped text, code spans first so their insides
// stay literal.
export function inline(raw: string): string {
  const codes: string[] = [];
  let s = raw.replace(/`([^`]+)`/g, (_, c: string) => {
    codes.push(`<code>${esc(c)}</code>`);
    return `\u0000${codes.length - 1}\u0000`;
  });
  s = esc(s);
  s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_, text: string, url: string) => {
    const href = safeHref(url.replace(/&amp;/g, '&'));
    return href ? `<a href="${esc(href)}">${text}</a>` : text;
  });
  s = s.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  s = s.replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>');
  s = s.replace(/(^|\W)_([^_\s][^_]*)_(?=\W|$)/g, '$1<em>$2</em>');
  return s.replace(/\u0000(\d+)\u0000/g, (_, i: string) => codes[Number(i)]!);
}

const cells = (line: string): string[] =>
  line
    .trim()
    .replace(/^\||\|$/g, '')
    .split('|')
    .map((c) => c.trim());

export function markdownToHtml(md: string): string {
  const lines = md.replace(/\r\n?/g, '\n').split('\n');
  const out: string[] = [];
  let i = 0;
  const para: string[] = [];
  const flush = () => {
    if (para.length) out.push(`<p>${inline(para.join(' '))}</p>`);
    para.length = 0;
  };
  while (i < lines.length) {
    const line = lines[i]!;
    const fence = /^```(.*)$/.exec(line);
    if (fence) {
      flush();
      const body: string[] = [];
      i++;
      while (i < lines.length && !/^```\s*$/.test(lines[i]!)) body.push(lines[i++]!);
      i++;
      out.push(`<pre><code>${esc(body.join('\n'))}</code></pre>`);
      continue;
    }
    const h = /^(#{1,4})\s+(.+)$/.exec(line);
    if (h) {
      flush();
      const level = Math.min(4, h[1]!.length + 1);
      out.push(`<h${level}>${inline(h[2]!.trim())}</h${level}>`);
      i++;
      continue;
    }
    if (/^(\s*[-*_]){3,}\s*$/.test(line)) {
      flush();
      out.push('<hr>');
      i++;
      continue;
    }
    if (/^\s*\|.*\|\s*$/.test(line) && i + 1 < lines.length && /^\s*\|?\s*:?-{3,}/.test(lines[i + 1]!)) {
      flush();
      const head = cells(line);
      i += 2;
      const rows: string[][] = [];
      while (i < lines.length && /^\s*\|.*\|\s*$/.test(lines[i]!)) rows.push(cells(lines[i++]!));
      out.push(`<table><thead><tr>${head.map((c) => `<th>${inline(c)}</th>`).join('')}</tr></thead><tbody>${rows.map((r) => `<tr>${r.map((c) => `<td>${inline(c)}</td>`).join('')}</tr>`).join('')}</tbody></table>`);
      continue;
    }
    if (/^>\s?/.test(line)) {
      flush();
      const body: string[] = [];
      while (i < lines.length && /^>\s?/.test(lines[i]!)) body.push(lines[i++]!.replace(/^>\s?/, ''));
      out.push(`<blockquote>${markdownToHtml(body.join('\n'))}</blockquote>`);
      continue;
    }
    const item = /^\s*([-*+]|\d+[.)])\s+(.+)$/.exec(line);
    if (item) {
      flush();
      const ordered = /\d/.test(item[1]!);
      const items: string[] = [];
      while (i < lines.length) {
        const m = /^\s*([-*+]|\d+[.)])\s+(.+)$/.exec(lines[i]!);
        if (!m || /\d/.test(m[1]!) !== ordered) break;
        const task = /^\[([ xX])\]\s+(.*)$/.exec(m[2]!);
        items.push(task ? `<li class="task${task[1] !== ' ' ? ' done' : ''}"><span class="box"></span>${inline(task[2]!)}</li>` : `<li>${inline(m[2]!)}</li>`);
        i++;
      }
      out.push(ordered ? `<ol>${items.join('')}</ol>` : `<ul>${items.join('')}</ul>`);
      continue;
    }
    if (!line.trim()) {
      flush();
      i++;
      continue;
    }
    para.push(line.trim());
    i++;
  }
  flush();
  return out.join('\n');
}

// The page palette, the same as make-print's Prints look (theme.mjs;
// redesigned 18 Nov 2026): near-black #08090a with hairline lines in dark,
// warm paper #f4f3ef with white cards in light, the system's own type, no
// font to load. Follows the viewer's appearance with no script, and a
// viewer can force either with data-theme="light|dark" on <html> (the
// website's switch). The names stay (--teal is the accent, --slate and
// --amber as before) so every kind keeps working.
const DOC_DARK = 'color-scheme:dark;--bg:#08090a;--card:#0f1011;--raise:#151618;--line:#1d1e21;--line2:#26272b;--text:#f7f8f8;--dim:#8a8f98;--faint:#62666d;--h1a:#ffffff;--h1b:#9a9ca3;--teal:#7b84ff;--slate:#6b7785;--amber:#f5c15a;--amberline:#3d3320';
const DOC_LIGHT = 'color-scheme:light;--bg:#f4f3ef;--card:#ffffff;--raise:#faf9f6;--line:#e7e5df;--line2:#dcd9d1;--text:#111113;--dim:#5e5d58;--faint:#8e8c85;--h1a:#111113;--h1b:#55555c;--teal:#4f5bff;--slate:#8fa0b2;--amber:#b7811a;--amberline:#efdcae';
export const PAGE_THEME_CSS = `:root{${DOC_DARK};--radius:12px;--sans:"Inter var",Inter,-apple-system,BlinkMacSystemFont,"SF Pro Text","Segoe UI",system-ui,sans-serif;--display:"Inter var",Inter,-apple-system,BlinkMacSystemFont,"SF Pro Display","Segoe UI",system-ui,sans-serif;--mono:ui-monospace,"SF Mono",Menlo,Consolas,monospace}
@media (prefers-color-scheme: light){:root{${DOC_LIGHT}}}
:root[data-theme=light]{${DOC_LIGHT}}:root[data-theme=dark]{${DOC_DARK}}
*{box-sizing:border-box}
html{background:var(--bg)}
body{margin:0;background:var(--bg);color:var(--text);font:15px/1.7 var(--sans);font-feature-settings:"cv11","ss01";-webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}
h1,h2,h3{font-family:var(--display);font-weight:600;color:var(--text)}
h1{font-size:40px;line-height:1.08;letter-spacing:-.032em;margin:0 0 12px;background:linear-gradient(180deg,var(--h1a) 30%,var(--h1b));-webkit-background-clip:text;background-clip:text;-webkit-text-fill-color:transparent;text-wrap:balance}
h2{letter-spacing:-.018em}
.meta{color:var(--faint);font-size:12.5px;font-variant-numeric:tabular-nums}
a{color:var(--text);text-decoration:underline;text-decoration-color:var(--line2);text-decoration-thickness:1px;text-underline-offset:3px}
a:hover{text-decoration-color:var(--dim)}
code{font:13px/1.5 var(--mono)}
::selection{background:color-mix(in srgb,var(--teal) 28%,transparent)}`;

const DOC_CSS = `main{max-width:720px;margin:0 auto;padding:64px 28px 80px}
header{border-bottom:1px solid var(--line);padding-bottom:24px;margin-bottom:36px}
h2{font-size:24px;line-height:1.25;margin:48px 0 12px}h3{font-size:17px;line-height:1.35;margin:32px 0 8px}h4{font-size:13px;margin:24px 0 6px;color:var(--dim);text-transform:uppercase;letter-spacing:.06em;font-weight:500}
p{margin:0 0 16px;text-wrap:pretty}
strong{font-weight:600}
code{background:var(--raise);border:1px solid var(--line);border-radius:5px;padding:1px 5px;font-size:12.5px}
pre{background:var(--card);border:1px solid var(--line);border-radius:var(--radius);padding:16px 18px;overflow-x:auto;margin:0 0 20px}
pre code{border:0;padding:0;background:none;font-size:12.5px;line-height:1.65}
ul,ol{margin:0 0 16px;padding-left:22px}li{margin:6px 0}li::marker{color:var(--faint)}
li.task{list-style:none;margin-left:-22px}
.box{display:inline-block;width:14px;height:14px;border:1.5px solid var(--line2);border-radius:4px;margin-right:10px;vertical-align:-2px}
li.task.done .box{background:var(--teal);border-color:var(--teal)}
li.task.done{color:var(--dim)}
blockquote{margin:0 0 18px;padding:14px 18px;border:1px solid var(--line);border-left:2px solid var(--teal);border-radius:0 var(--radius) var(--radius) 0;background:var(--card);color:var(--dim)}
blockquote p:last-child{margin:0}
hr{border:0;border-top:1px solid var(--line);margin:40px 0}
table{width:100%;border-collapse:separate;border-spacing:0;margin:0 0 20px;font-size:14px;background:var(--card);border:1px solid var(--line);border-radius:var(--radius);overflow:hidden}
th,td{text-align:left;padding:10px 14px;border-bottom:1px solid var(--line)}
tbody tr:last-child td{border-bottom:0}
th{color:var(--faint);font-weight:500;font-size:11.5px;letter-spacing:.04em;text-transform:uppercase;background:var(--raise)}
@media (max-width:600px){main{padding:32px 18px 48px}h1{font-size:30px}}`;

export function renderDoc(title: string, md: string, meta: string): string {
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(title)}</title>
<style>${PAGE_THEME_CSS}
${DOC_CSS}</style>
</head>
<body>
<main>
<header><h1>${esc(title)}</h1><div class="meta">${esc(meta)}</div></header>
${markdownToHtml(md)}
</main>
</body>
</html>
`;
}
