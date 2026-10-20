import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { buildGlass, demoBackdrop, demoWallpaper, fetchBackdrop, type Backdrop } from "./glass";
import { initLang, setLangLocally, t } from "./i18n";
import { afterPaint, glyphs } from "./glyphs";

type ToastPayload = {
  kind: "info" | "error" | "progress" | "done";
  title: string;
  detail: string;
  percent: number | null;
  outputs: string[];
  note?: string;
  lang?: string;
};

const AUTO_HIDE_MS: Record<ToastPayload["kind"], number> = { info: 5000, done: 5500, error: 9000, progress: 0 };

const card = document.getElementById("card")!;
const title = card.querySelector<HTMLElement>(".title")!;
const detail = card.querySelector<HTMLElement>(".detail")!;
const pct = card.querySelector<HTMLElement>(".pct")!;
const note = card.querySelector<HTMLElement>(".note")!;
const noteText = card.querySelector<HTMLElement>(".note-text")!;
const actions = card.querySelector<HTMLElement>(".actions")!;
const openBtn = card.querySelector<HTMLButtonElement>(".open")!;
const reveal = card.querySelector<HTMLButtonElement>(".reveal")!;
const dismiss = card.querySelector<HTMLButtonElement>(".dismiss")!;
const badgeGlyph = card.querySelector<HTMLElement>(".badge-glyph")!;
const meter = card.querySelector<SVGCircleElement>(".meter-fill")!;

dismiss.innerHTML = glyphs.close;
card.querySelector(".note-icon")!.innerHTML = glyphs.trash;

function renderLabels() {
  openBtn.innerHTML = `${glyphs.file}<span>${t("toast.open")}</span>`;
  reveal.innerHTML = `${glyphs.folder}<span>${t("toast.reveal")}</span>`;
  dismiss.setAttribute("aria-label", t("toast.dismiss"));
  dismiss.title = t("toast.dismiss");
}
initLang(renderLabels);

// ---- File names: cut in the middle so the new extension stays visible ---------------------------

const measure = document.createElement("canvas").getContext("2d")!;

function fitMiddle(el: HTMLElement, text: string) {
  el.textContent = text;
  const max = el.clientWidth;
  measure.font = getComputedStyle(el).font;
  const fits = (s: string) => measure.measureText(s).width <= max;
  if (!max || fits(text)) return;
  // Keep the extension plus a few characters before it, then as much of the start as fits.
  const dot = text.lastIndexOf(".");
  const tail = text.slice(dot > 0 ? Math.max(0, dot - 6) : -8);
  let lo = 0;
  let hi = text.length - tail.length;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (fits(text.slice(0, mid).trimEnd() + "…" + tail)) lo = mid;
    else hi = mid - 1;
  }
  if (lo > 0) el.textContent = text.slice(0, lo).trimEnd() + "…" + tail;
}

let current: ToastPayload | null = null;
let visible = false;
let hideTimer: number | undefined;
let hovering = false;

// ---- Glass ---------------------------------------------------------------------------------------

const CARD_RADIUS = 18;
let backdrop: Backdrop | null = null;
let glassUrl = "";
let glassKey = "";
let glassJob = 0;

/** Rebuilds the card's glass for its current size (the card grows when a button appears). */
async function refreshGlass() {
  if (!backdrop) return;
  const height = card.offsetHeight;
  // Rebuild when the card or the window changes size.
  const key = `${innerWidth}x${innerHeight}:${height}`;
  if (key === glassKey && glassUrl) return;
  glassKey = key;
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
  if (p.lang) {
    setLangLocally(p.lang);
    renderLabels();
  }
  card.dataset.kind = p.kind;
  title.textContent = p.title;
  pct.textContent = p.kind === "progress" && p.percent != null ? `${Math.round(p.percent)}%` : "";
  // One line for a file name or a count; error lists may wrap.
  const single = p.kind !== "error" && !p.detail.includes("\n");
  detail.classList.toggle("single", single);
  if (single) fitMiddle(detail, p.detail);
  else detail.textContent = p.detail;
  detail.title = p.detail;
  note.hidden = !p.note;
  noteText.textContent = p.note ?? "";
  actions.hidden = p.outputs.length === 0;
  openBtn.hidden = p.outputs.length !== 1;
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
  backdrop = inTauri ? await fetchBackdrop("toast_backdrop") : demoToastBackdrop();
  glassKey = "";
  update(p);
  // Show right away on the plain card colour; the glass replaces it as soon as it is built.
  card.classList.remove("has-glass");
  refreshGlass();
  card.classList.remove("out");
  afterPaint(() => {
    if (visible) card.classList.add("in");
  });
}

// ---- Browser demo (npm run dev, then /toast.html?lang=vi) ---------------------------------------

const inTauri = "__TAURI_INTERNALS__" in window;
let demoPaper: HTMLCanvasElement | null = null;

function demoToastBackdrop(): Backdrop {
  const dpr = devicePixelRatio;
  demoPaper ??= demoWallpaper(Math.round(innerWidth * dpr), Math.round(innerHeight * dpr));
  return demoBackdrop(Math.round(innerWidth * dpr), Math.round(innerHeight * dpr), demoPaper);
}

if (inTauri) {
  listen<ToastPayload>("toast:update", (e) => show(e.payload));
} else {
  const params = new URLSearchParams(location.search);
  if (params.get("lang")) setLangLocally(params.get("lang")!);
  renderLabels();
  hovering = true; // keep the card on screen
  card.style.transition = "none"; // show the settled card straight away (also for screenshots)
  const vi = params.get("lang") === "vi";
  const sample: ToastPayload =
    params.get("demo") === "progress"
      ? {
          kind: "progress",
          title: vi ? "Đang chuyển sang MP4 · 2/3" : "Converting to MP4 · 2/3",
          detail: "Screen Recording 2026-10-18 at 21.44.10.mov",
          percent: 58,
          outputs: [],
        }
      : {
          kind: "done",
          title: vi ? "Đã chuyển sang PNG" : "Converted to PNG",
          detail: "WhatsApp Image 2026-07-16 at 11.07.02.png",
          percent: 100,
          outputs: ["C:\\demo\\photo.png"],
          note: vi ? "File gốc đã vào Thùng rác" : "Original moved to Recycle Bin",
        };
  const paint = () => {
    demoPaper = demoWallpaper(Math.round(innerWidth * devicePixelRatio), Math.round(innerHeight * devicePixelRatio));
    document.body.style.background = `url(${demoPaper.toDataURL()}) 0 0 / 100% 100% no-repeat`;
    backdrop = demoToastBackdrop();
  };
  paint();
  show(sample);
  // Headless browsers grow the viewport after load: redraw the fake desktop and its glass.
  window.addEventListener("resize", () => {
    paint();
    refreshGlass();
  });
}

openBtn.addEventListener("click", (e) => {
  e.stopPropagation();
  if (current?.outputs[0] && inTauri) invoke("open_file", { path: current.outputs[0] });
  hide();
});

reveal.addEventListener("click", (e) => {
  e.stopPropagation();
  if (current?.outputs[0] && inTauri) invoke("reveal_in_explorer", { path: current.outputs[0] });
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
