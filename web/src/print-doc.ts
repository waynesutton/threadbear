// What a published Print's HTML is wrapped in before the website draws it
// (print.tsx): the same Content-Security-Policy the Mac puts in front of a
// page in a reply (RichPages.policy), so nothing loads from anywhere and
// only inline styles and data: pictures show, and a <base> that sends every
// link to a new window, which the frame's sandbox then refuses. With the
// frame's empty `sandbox` (no scripts, no forms, no same origin), a print
// is a picture of a page: it can't run, send or go anywhere.
export const PRINT_POLICY = "default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:; media-src data:; form-action 'none'; base-uri 'none'; frame-src 'none'";

// The look a print is drawn in: the viewer's system, or always light or dark.
export type PrintTheme = 'system' | 'light' | 'dark';

// A print keeps both looks in one page: its CSS picks one with
// `prefers-color-scheme`, which a browser answers from the computer's
// appearance. The Mac app has an appearance of its own, so with the app
// dark and the Mac light, a print was dark in the app and light on the web
// (Wayne, 18 Nov). Inside a frame the query follows the frame's
// `color-scheme`, which browsers don't all pass down, so forcing a look
// settles the query in the CSS itself, read the same by every browser and
// every print ever made, and marks the page `data-theme` for newer prints'
// own rules.
export function forceTheme(html: string, theme: PrintTheme): string {
  if (theme === 'system') return html;
  const query = /\(\s*prefers-color-scheme\s*:\s*(light|dark)\s*\)/gi;
  const forced = html.replace(query, (_, scheme: string) => (scheme.toLowerCase() === theme ? '(min-width: 0px)' : '(max-width: -1px)'));
  return /<html\b/i.test(forced) ? forced.replace(/<html\b/i, `<html data-theme="${theme}"`) : forced;
}

export function printDoc(html: string, theme: PrintTheme = 'system'): string {
  const head = `<meta http-equiv="Content-Security-Policy" content="${PRINT_POLICY}"><base target="_blank">`;
  const page = forceTheme(html, theme);
  // In front of everything, so no markup of the print's comes before it.
  return /^\s*<!doctype[^>]*>/i.test(page) ? page.replace(/^(\s*<!doctype[^>]*>)/i, `$1${head}`) : head + page;
}
