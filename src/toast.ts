import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { buildGlass, fetchBackdrop, type Backdrop } from "./glass";
import { initLang, t } from "./i18n";
import { afterPaint, glyphs } from "./glyphs";

type ToastPayload = {
  kind: "info" | "error" | "progress" | "done";
  title: string;
  detail: string;
  percent: number | null;
  outputs: string[];
};

const AUTO_HIDE_MS: Record<ToastPayload["kind"], number> = { info: 5000, done: 5500, error: 9000, progress: 0 };

const card = document.getElementById("card")!;
const title = card.querySelector<HTMLElement>(".title")!;
const detail = card.querySelector<HTMLElement>(".detail")!;
const reveal = card.querySelector<HTMLButtonElement>(".reveal")!;
const dismiss = card.querySelector<HTMLButtonElement>(".dismiss")!;
const badgeGlyph = card.querySelector<HTMLElement>(".badge-glyph")!;
const meter = card.querySelector<SVGCircleElement>(".meter-fill")!;

dismiss.innerHTML = glyphs.close;
initLang(() => {
  reveal.innerHTML = `${glyphs.folder}<span>${t("toast.reveal")}</span>`;
  dismiss.setAttribute("aria-label", t("toast.dismiss"));
  dismiss.title = t("toast.dismiss");
});

let current: ToastPayload | null = null;
let visible = false;
let hideTimer: number | undefined;
let hovering = false;

// ---- Glass ---------------------------------------------------------------------------------------

const CARD_RADIUS = 18;
let backdrop: Backdrop | null = null;
let glassUrl = "";
let glassHeight = 0;
let glassJob = 0;

/** Rebuilds the card's glass for its current size (the card grows when a button appears). */
async function refreshGlass() {
  if (!backdrop) return;
  const height = card.offsetHeight;
  if (height === glassHeight && glassUrl) return;
  glassHeight = height;
  const job = ++glassJob;
  const left = card.offsetLeft;
  const top = card.offsetTop;
  const desc = { kind: "rect", left, top, width: card.offsetWidth, height, radius: CARD_RADIUS } as const;
  const url = await buildGlass(backdrop, window.innerWidth, desc, {
    blur: 18,
    bezel: 14,
    refraction: 12,
  });
  if (job !== glassJob) {
    URL.revokeObjectURL(url);
    return;
  }
  if (glassUrl) URL.revokeObjectURL(glassUrl);
  glassUrl = url;
  card.style.setProperty("--glass", `url(${url})`);
  card.style.setProperty("--glass-x", `${-left}px`);
  card.style.setProperty("--glass-y", `${-top}px`);
  card.classList.add("has-glass");
}

new ResizeObserver(() => refreshGlass()).observe(card);

function glyphFor(kind: ToastPayload["kind"]): string {
  return { info: glyphs.info, error: glyphs.alert, progress: glyphs.convert, done: glyphs.check }[kind];
}

function scheduleHide() {
  clearTimeout(hideTimer);
  const ms = current ? AUTO_HIDE_MS[current.kind] : 0;
  if (ms > 0 && !hovering) hideTimer = window.setTimeout(hide, ms);
}

async function hide() {
  clearTimeout(hideTimer);
  if (!visible) return;
  visible = false;
  card.classList.remove("in");
  card.classList.add("out");
  await new Promise((r) => setTimeout(r, 220));
  if (!visible) await invoke("hide_toast");
}

function update(p: ToastPayload) {
  const kindChanged = current?.kind !== p.kind;
  current = p;
  card.dataset.kind = p.kind;
  title.textContent = p.title;
  detail.textContent = p.detail;
  detail.title = p.detail;
  reveal.hidden = p.outputs.length === 0;
  if (kindChanged) {
    badgeGlyph.innerHTML = glyphFor(p.kind);
    badgeGlyph.classList.remove("pop");
    void badgeGlyph.offsetWidth; // restart the pop animation
    badgeGlyph.classList.add("pop");
  }
  meter.style.strokeDashoffset = String(100 - (p.kind === "info" || p.kind === "error" ? 100 : (p.percent ?? 0)));

  scheduleHide();
}

async function show(p: ToastPayload) {
  if (visible) {
    update(p);
    return;
  }
  visible = true;
  // The backend captured the desktop behind us just before showing the window.
  backdrop = await fetchBackdrop("toast_backdrop");
  glassHeight = 0;
  update(p);
  await refreshGlass();
  card.classList.remove("out");
  afterPaint(() => {
    if (visible) card.classList.add("in");
  });
}

listen<ToastPayload>("toast:update", (e) => show(e.payload));

reveal.addEventListener("click", (e) => {
  e.stopPropagation();
  if (current?.outputs[0]) invoke("reveal_in_explorer", { path: current.outputs[0] });
  hide();
});

dismiss.addEventListener("click", (e) => {
  e.stopPropagation();
  hide();
});

card.addEventListener("mouseenter", () => {
  hovering = true;
  clearTimeout(hideTimer);
});

card.addEventListener("mouseleave", () => {
  hovering = false;
  scheduleHide();
});

window.addEventListener("contextmenu", (e) => e.preventDefault());
