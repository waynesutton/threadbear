#!/usr/bin/env node
// A page as a PDF, through headless Chrome's own print, light theme:
//
//   node export-pdf.mjs <index.html> <out.pdf>
//
// Prints are static, so what prints is what the page shows. Uses Google
// Chrome from /Applications; says so if it isn't there.
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const CHROMES = [
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  "/Applications/Chromium.app/Contents/MacOS/Chromium",
  "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
];

const [input, output] = process.argv.slice(2);
if (!input || !output) {
  console.error("usage: node export-pdf.mjs <index.html> <out.pdf>");
  process.exit(2);
}
const chrome = process.env.CHROME_PATH ?? CHROMES.find((p) => existsSync(p));
if (!chrome) {
  console.error("No Chrome found for printing. Install Google Chrome, or set CHROME_PATH. The HTML export works without it.");
  process.exit(3);
}
const profile = mkdtempSync(join(tmpdir(), "page-pdf-"));
try {
  execFileSync(
    chrome,
    [
      "--headless=new",
      "--disable-gpu",
      "--no-first-run",
      "--no-default-browser-check",
      `--user-data-dir=${profile}`,
      "--blink-settings=preferredColorScheme=1",
      "--no-pdf-header-footer",
      `--print-to-pdf=${resolve(output)}`,
      pathToFileURL(resolve(input)).href,
    ],
    { stdio: "pipe", timeout: 60_000 },
  );
} finally {
  rmSync(profile, { recursive: true, force: true });
}
if (!existsSync(output) || statSync(output).size < 500) {
  console.error("Chrome didn't write the PDF.");
  process.exit(1);
}
console.log(`${resolve(output)} (${(statSync(output).size / 1024).toFixed(0)} KB)`);
