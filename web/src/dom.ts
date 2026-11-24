// Tiny DOM helpers. No styles are set through attributes, so the page runs under a CSP
// without 'unsafe-inline'.

type Attrs = Record<string, string | number | boolean | undefined | EventListener>;
type Child = Node | string | null | undefined | false;

export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Attrs = {},
  ...children: (Child | Child[])[]
): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === false) continue;
    if (typeof v === "function") el.addEventListener(k.replace(/^on/, ""), v);
    else if (k === "class") el.className = String(v);
    else if (v === true) el.setAttribute(k, "");
    else el.setAttribute(k, String(v));
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    el.append(typeof c === "string" ? document.createTextNode(c) : c);
  }
  return el;
}

let uid = 0;
export const nextId = (p: string): string => `${p}-${++uid}`;

export interface TextOpts {
  placeholder?: string;
  num?: boolean;
  hint?: string;
}

/** A text field. Numbers stay text: the core reads their exact digits. */
export function textField(label: string, value: string, onValue: (v: string) => void, o: TextOpts = {}): HTMLLabelElement {
  const id = nextId("f");
  const input = h("input", {
    id,
    type: "text",
    value,
    placeholder: o.placeholder,
    autocomplete: "off",
    spellcheck: "false",
    inputmode: o.num ? "decimal" : undefined,
    class: o.num ? "num" : undefined,
  });
  input.addEventListener("input", () => onValue(input.value));
  return h(
    "label",
    { class: "field", for: id },
    h("span", { class: "flabel" }, label),
    input,
    o.hint ? h("span", { class: "hint" }, o.hint) : null,
  );
}

export function areaField(label: string, value: string, onValue: (v: string) => void, placeholder: string): HTMLLabelElement {
  const id = nextId("f");
  const area = h("textarea", { id, rows: 5, spellcheck: "false", placeholder, class: "num" });
  area.value = value;
  area.addEventListener("input", () => onValue(area.value));
  return h("label", { class: "field wide", for: id }, h("span", { class: "flabel" }, label), area);
}

export function selectField<T extends string>(
  label: string,
  value: T,
  options: readonly (readonly [T, string])[],
  onValue: (v: T) => void,
): HTMLLabelElement {
  const id = nextId("f");
  const sel = h("select", { id }, ...options.map(([v, t]) => h("option", { value: v, selected: v === value }, t)));
  sel.addEventListener("change", () => onValue(sel.value as T));
  return h("label", { class: "field", for: id }, h("span", { class: "flabel" }, label), sel);
}

export function dateField(label: string, value: string, onValue: (v: string) => void): HTMLLabelElement {
  const id = nextId("f");
  const input = h("input", { id, type: "date", value });
  input.addEventListener("change", () => onValue(input.value));
  return h("label", { class: "field", for: id }, h("span", { class: "flabel" }, label), input);
}

export function segmented<T extends string>(
  name: string,
  value: T,
  options: readonly (readonly [T, string])[],
  onValue: (v: T) => void,
): HTMLDivElement {
  const group = h("div", { class: "segmented", role: "radiogroup", "aria-label": name });
  for (const [v, t] of options) {
    const on = v === value;
    const b = h("button", { type: "button", role: "radio", "aria-checked": String(on), class: on ? "on" : "" }, t);
    b.addEventListener("click", () => onValue(v));
    group.append(b);
  }
  return group;
}
