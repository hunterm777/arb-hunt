import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "./styles/tokens.css";
import "./styles/app.css";

import { h } from "./dom";
import { chartView } from "./components/chart";
import { legInputs } from "./components/legs";
import { resultView } from "./components/result";
import { meta, store } from "./state";

function header(): HTMLElement {
  return h(
    "header",
    { class: "top" },
    h("div", { class: "brand" }, h("span", { class: "logo", "aria-hidden": "true" }, "⇄"), h("h1", {}, "arbcalc")),
    h("p", { class: "tagline" }, "Most arbs die after fees. Check yours."),
    h("a", { class: "gh", href: "https://github.com/hunterm777/arb-hunt", rel: "noopener" }, "GitHub"),
  );
}

function footer(): HTMLElement {
  return h(
    "footer",
    {},
    h("p", {}, "Not financial advice. You enter the prices and sizes; nothing here is market data."),
    h("p", { class: "muted" }, `Fee and rounding rules: venue-specs ${meta.venue_specs}. Nothing is fetched after the page loads.`),
  );
}

const app = document.getElementById("app");
if (app) {
  app.replaceChildren(header(), h("main", {}, legInputs(), resultView(), chartView()), footer());
  store.run(true);
}
