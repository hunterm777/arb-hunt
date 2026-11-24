// Loads the built page in headless Chrome and checks the PRD's page rules: no
// requests after load, no CSP violations or console errors, the verdict for the
// opening case and for the PRD's depth example as typed into the page, and how long
// a check at the largest size the core walks takes.
// Usage: CHROME=/path/to/chrome node scripts/bench.mjs   (needs puppeteer-core)
import { createServer } from "node:http";
import { readFileSync, existsSync } from "node:fs";
import { extname, join, resolve } from "node:path";

const { default: puppeteer } = await import("puppeteer-core");
const dist = resolve(import.meta.dirname, "../dist");
const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".woff2": "font/woff2", ".woff": "font/woff" };
const server = createServer((req, res) => {
  const path = join(dist, decodeURIComponent((req.url ?? "/").split("?")[0]).replace(/\/$/, "/index.html"));
  if (!path.startsWith(dist) || !existsSync(path)) return res.writeHead(404).end();
  res.writeHead(200, { "content-type": types[extname(path)] ?? "application/octet-stream" }).end(readFileSync(path));
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/`;

const args = process.env.CHROME_ARGS ? JSON.parse(process.env.CHROME_ARGS) : ["--no-sandbox"];
const browser = await puppeteer.launch({ executablePath: process.env.CHROME, args, headless: true });
const page = await browser.newPage();
await page.setViewport({ width: 1280, height: 1000 });
const problems = [];
const requests = [];
let loaded = false;
page.on("console", (m) => m.type() === "error" && problems.push(`console: ${m.text()}`));
page.on("pageerror", (e) => problems.push(`page error: ${e.message}`));
// data: URIs (the date picker's own icon) are not network requests.
page.on("request", (r) => loaded && !r.url().startsWith("data:") && requests.push(r.url()));
await page.evaluateOnNewDocument(() => {
  document.addEventListener("securitypolicyviolation", (e) => console.error(`CSP ${e.violatedDirective} ${e.blockedURI}`));
});
await page.goto(url, { waitUntil: "networkidle0" });
loaded = true;

const verdict = () => page.evaluate(() => document.querySelector(".verdict")?.textContent ?? "");
const settle = () => new Promise((ok) => setTimeout(ok, 400));

// The page opens on the PRD's first case; venue-specs gives Polymarket economics 0.05.
const opening = await verdict();

// Types the PRD's Python example into the page: depth on both legs, $2.00 fixed costs.
async function typeDepth(levels) {
  await page.evaluate((levels) => {
    const cards = document.querySelectorAll(".card");
    levels.forEach((text, i) => {
      // Switching to depth runs a check at once, so switch only when not there yet.
      if (!cards[i].querySelector("textarea")) {
        [...cards[i].querySelectorAll(".segmented button")].find((b) => b.textContent === "Depth").click();
      }
      const area = document.querySelectorAll(".card")[i].querySelector("textarea");
      area.value = text;
      area.dispatchEvent(new Event("input", { bubbles: true }));
    });
  }, levels);
}
await typeDepth(["0.41 300\n0.42 500", "0.55 200\n0.56 800"]);
await page.evaluate(() => {
  const f = [...document.querySelectorAll("label.field")].find((l) => l.textContent?.startsWith("Fixed costs"))?.querySelector("input");
  f.value = "2.00";
  f.dispatchEvent(new Event("input", { bubbles: true }));
});
await settle();
const example = await verdict();
const chart = await page.evaluate(() => !document.querySelector(".chart")?.classList.contains("hidden"));
await page.screenshot({ path: process.env.SHOT ?? "/tmp/arbcalc.png", fullPage: true });

// The largest check: 200,000 pairs on each side, timed from the edit to the verdict.
const LARGEST = "Profitable: $10102.00 guaranteed on 200000 pairs";
const t0 = Date.now();
await typeDepth(["0.41 100000\n0.42 100000", "0.50 100000\n0.51 100000"]);
await page.waitForFunction((v) => document.querySelector(".verdict")?.textContent === v, { timeout: 30000 }, LARGEST);
const largestMs = Date.now() - t0 - 200; // less the typing pause before a check runs
const largest = await verdict();
loaded = false;

// A phone-width view of the opening case, in a page of its own.
const phone = await browser.newPage();
await phone.setViewport({ width: 390, height: 844, isMobile: true });
await phone.goto(url, { waitUntil: "networkidle0" });
await phone.screenshot({ path: process.env.SHOT_MOBILE ?? "/tmp/arbcalc-mobile.png", fullPage: true });

await browser.close();
server.close();
const out = { opening, example, chart, largest, largestMs, requestsAfterLoad: requests, problems };
console.log(JSON.stringify(out, null, 1));
const fail =
  problems.length ||
  requests.length ||
  !chart ||
  opening !== "Profitable: $1.0625 guaranteed on 100 pairs" ||
  example !== "Profitable: $0.213 guaranteed on 300 pairs" ||
  largest !== LARGEST;
process.exit(fail ? 1 : 0);
