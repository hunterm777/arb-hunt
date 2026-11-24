// Guaranteed profit as the number of pairs grows, with the profit-maximising size and
// the break-even size marked. The points are the core's curve.
import { h } from "../dom";
import { money, pairs } from "../format";
import { store } from "../state";

const C = { bg: "#13161B", grid: "#22262E", text: "#8B929C", zero: "#4B5260", line: "#3B82F6", profit: "#22C55E", loss: "#EF4444" };

const canvas = h("canvas", { role: "img", "aria-label": "Guaranteed profit vs size" });
const legend = h("ul", { class: "legend" });
const root = h("section", { class: "chart", "aria-label": "Profit vs size" });
let width = 0;

function ticks(lo: number, hi: number, n: number): number[] {
  const span = hi - lo || 1;
  const raw = span / n;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const step = ([1, 2, 2.5, 5, 10].find((m) => m * mag >= raw) ?? 10) * mag;
  const out: number[] = [];
  for (let v = Math.ceil(lo / step) * step; v <= hi + step * 1e-9; v += step) out.push(v);
  return out;
}

function draw(): void {
  const res = store.result;
  if (!res || "err" in res || !res.ok.curve.length) {
    root.classList.add("hidden");
    return;
  }
  root.classList.remove("hidden");
  const r = res.ok;
  const dpr = window.devicePixelRatio || 1;
  const w = Math.max(320, width || 800);
  const hgt = w < 600 ? 240 : 300;
  if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(hgt * dpr)) {
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(hgt * dpr);
    canvas.style.width = `${w}px`;
    canvas.style.height = `${hgt}px`;
  }
  const g = canvas.getContext("2d");
  if (!g) return;
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.fillStyle = C.bg;
  g.fillRect(0, 0, w, hgt);
  const pts = r.curve;
  const xs = pts.map((p) => p[0]);
  const ys = pts.map((p) => p[1]);
  let ylo = Math.min(0, ...ys);
  let yhi = Math.max(0, ...ys);
  if (ylo === yhi) yhi = ylo + 1;
  const pad = (yhi - ylo) * 0.08;
  ylo -= pad;
  yhi += pad;
  const xlo = 0;
  const xhi = Math.max(...xs, 1);
  const m = { l: 76, r: 16, t: 16, b: 34 };
  const X = (x: number): number => m.l + ((x - xlo) / (xhi - xlo || 1)) * (w - m.l - m.r);
  const Y = (y: number): number => m.t + (1 - (y - ylo) / (yhi - ylo)) * (hgt - m.t - m.b);
  g.font = "11px 'JetBrains Mono', ui-monospace, monospace";
  g.lineWidth = 1;
  g.fillStyle = C.text;
  g.textAlign = "right";
  g.textBaseline = "middle";
  for (const v of ticks(ylo, yhi, 5)) {
    g.strokeStyle = C.grid;
    g.beginPath();
    g.moveTo(m.l, Y(v));
    g.lineTo(w - m.r, Y(v));
    g.stroke();
    g.fillText(money(Number(v.toPrecision(12))), m.l - 8, Y(v));
  }
  g.textAlign = "center";
  g.textBaseline = "top";
  for (const v of ticks(xlo, xhi, w < 600 ? 4 : 8)) if (Number.isInteger(v)) g.fillText(v.toLocaleString("en-US"), X(v), hgt - m.b + 10);
  g.strokeStyle = C.zero;
  g.beginPath();
  g.moveTo(m.l, Y(0));
  g.lineTo(w - m.r, Y(0));
  g.stroke();
  const mark = (n: number | null, color: string): void => {
    if (n === null) return;
    g.strokeStyle = color;
    g.setLineDash([4, 4]);
    g.beginPath();
    g.moveTo(X(n), m.t);
    g.lineTo(X(n), hgt - m.b);
    g.stroke();
    g.setLineDash([]);
  };
  mark(r.sizing.break_even_size, C.text);
  mark(r.sizing.optimal_size, C.profit);
  g.lineWidth = 2;
  g.strokeStyle = C.line;
  g.beginPath();
  pts.forEach(([x, y], i) => (i ? g.lineTo(X(x), Y(y)) : g.moveTo(X(x), Y(y))));
  g.stroke();
  const items: [string, string][] = [["Guaranteed profit", C.line]];
  if (r.sizing.optimal_size !== null) items.push([`Profit-maximising size: ${pairs(r.sizing.optimal_size)}`, C.profit]);
  if (r.sizing.break_even_size !== null) items.push([`Break-even size: ${pairs(r.sizing.break_even_size)}`, C.text]);
  legend.replaceChildren(
    ...items.map(([t, c]) => {
      const sw = h("span", { class: "swatch", "aria-hidden": "true" });
      sw.style.backgroundColor = c;
      return h("li", {}, sw, t);
    }),
  );
}

export function chartView(): HTMLElement {
  root.append(h("h2", {}, "Profit vs size"), canvas, legend);
  let queued = false;
  store.subscribe(() => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      draw();
    });
  });
  new ResizeObserver((entries) => {
    const w = Math.round(entries[0]?.contentRect.width ?? 0);
    if (w && w !== width) {
      width = w;
      draw();
    }
  }).observe(root);
  return root;
}
