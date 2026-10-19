import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { buildGlass, demoBackdrop, fetchBackdrop } from "./glass";
import { afterPaint, escapeHtml, glyphs } from "./glyphs";
import { hintOf, initLang, t } from "./i18n";

type Category = "image" | "animated" | "video" | "audio";
type TargetOption = { id: string; label: string; hint: string; hintVi?: string };
type WheelInfo = {
  category: Category;
  categoryLabel: string;
  title: string;
  files: string[];
  skipped: number;
  options: TargetOption[];
};
type WheelOpen = { mode: "hotkey" | "drag"; info: WheelInfo | null; deleteOriginal?: boolean };

// Geometry in CSS pixels; the window is 330x330 logical pixels (keep in sync with wheel.css
// and WHEEL_SIZE in lib.rs).
const SIZE = 330;
const C = SIZE / 2;
const R_OUT = 140;
const R_IN = 62;
const HUB_R = 54;
const GAP = 8;
const CORNER_OUT = 12;
const CORNER_IN = 8;
const HOT_LIFT = 5;

const HUB = -2;
const OUTSIDE = -1;

const stage = document.getElementById("stage")!;

let mode: WheelOpen["mode"] = "hotkey";
let info: WheelInfo | null = null;
let unsupported = false;
let hot = OUTSIDE;
let phase: "hidden" | "opening" | "open" | "closing" = "hidden";
let releaseTimer: number | undefined;
let glassUrl = "";
let openToken = 0;
let pendingDragPaths: string[] = [];
let deleteOriginal = false;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Outside Tauri (plain `npm run dev` in a browser) the wheel runs as a self-replaying demo. */
const inTauri = "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  if (inTauri) return invoke<T>(cmd, args);
  console.log("[demo]", cmd, args ?? "");
  if (cmd === "hide_wheel") setTimeout(openDemo, 700);
  return null;
}

// ---- Geometry ----------------------------------------------------------------------------------

type Pt = [number, number];
const f = (p: Pt) => `${p[0].toFixed(2)} ${p[1].toFixed(2)}`;
const at = (len: number, u: Pt, off: number, v: Pt): Pt => [C + len * u[0] + off * v[0], C + len * u[1] + off * v[1]];
const scaleFromCentre = (p: Pt, k: number): Pt => [C + (p[0] - C) * k, C + (p[1] - C) * k];

/**
 * Donut slice i of n with a constant pixel gap and rounded corners, starting at 12 o'clock.
 * Each corner is a small circle tangent to the straight side and to the outer or inner arc.
 */
function slicePath(i: number, n: number): string {
  const step = (2 * Math.PI) / n;
  const a0 = -Math.PI / 2 + i * step;
  const a1 = a0 + step;
  const g = GAP / 2;
  const u0: Pt = [Math.cos(a0), Math.sin(a0)];
  const v0: Pt = [-Math.sin(a0), Math.cos(a0)]; // into the slice from side 0
  const u1: Pt = [Math.cos(a1), Math.sin(a1)];
  const v1: Pt = [Math.sin(a1), -Math.cos(a1)]; // into the slice from side 1

  const ro = CORNER_OUT;
  const ri = CORNER_IN;
  const to = Math.sqrt((R_OUT - ro) ** 2 - (g + ro) ** 2);
  const ti = Math.sqrt((R_IN + ri) ** 2 - (g + ri) ** 2);

  // Corner circle centres, then their tangent points on the sides and the arcs.
  const c0o = at(to, u0, g + ro, v0);
  const c1o = at(to, u1, g + ro, v1);
  const c1i = at(ti, u1, g + ri, v1);
  const c0i = at(ti, u0, g + ri, v0);

  const s0o = at(to, u0, g, v0);
  const o0 = scaleFromCentre(c0o, R_OUT / (R_OUT - ro));
  const o1 = scaleFromCentre(c1o, R_OUT / (R_OUT - ro));
  const s1o = at(to, u1, g, v1);
  const s1i = at(ti, u1, g, v1);
  const i1 = scaleFromCentre(c1i, R_IN / (R_IN + ri));
  const i0 = scaleFromCentre(c0i, R_IN / (R_IN + ri));
  const s0i = at(ti, u0, g, v0);

  const large = step > Math.PI ? 1 : 0;
  return [
    `M${f(s0o)}`,
    `A${ro} ${ro} 0 0 1 ${f(o0)}`,
    `A${R_OUT} ${R_OUT} 0 ${large} 1 ${f(o1)}`,
    `A${ro} ${ro} 0 0 1 ${f(s1o)}`,
    `L${f(s1i)}`,
    `A${ri} ${ri} 0 0 1 ${f(i1)}`,
    `A${R_IN} ${R_IN} 0 ${large} 0 ${f(i0)}`,
    `A${ri} ${ri} 0 0 1 ${f(s0i)}`,
    "Z",
  ].join(" ");
}

function midAngle(i: number, n: number): number {
  return -Math.PI / 2 + (i + 0.5) * ((2 * Math.PI) / n);
}

// ---- Glass -------------------------------------------------------------------------------------

async function makeGlass(): Promise<string> {
  const bd = inTauri ? await fetchBackdrop("wheel_backdrop") : demoBackdrop(SIZE * devicePixelRatio, SIZE * devicePixelRatio);
  if (!bd) return "";
  return buildGlass(bd, SIZE, { kind: "ring", cx: C, cy: C, rIn: R_IN, rOut: R_OUT, hubR: HUB_R }, {
    blur: 12,
    bezel: 16,
    refraction: 15,
  });
}

function dropGlass() {
  if (glassUrl) URL.revokeObjectURL(glassUrl);
  glassUrl = "";
}

/** Puts the finished glass into every shape; CSS fades it in over the plain tint. */
function applyGlass() {
  if (!glassUrl) return;
  stage.querySelectorAll("image.glass-img").forEach((img) => img.setAttribute("href", glassUrl));
  // Next frame, so the fade starts from the image's transparent state.
  afterPaint(() => stage.classList.add("glass-ready"));
}

// ---- Rendering ---------------------------------------------------------------------------------

/** Glass contents of one shape: the refracted backdrop clipped to it, plus tint and rim light. */
function glassBody(clipId: string, d: string, extraClass = ""): string {
  return `
    <clipPath id="${clipId}"><path d="${d}"/></clipPath>
    <g clip-path="url(#${clipId})" class="glass ${extraClass}">
      <image class="glass-img" x="0" y="0" width="${SIZE}" height="${SIZE}" preserveAspectRatio="none"/>
      <rect class="tint" width="${SIZE}" height="${SIZE}"/>
      <rect class="tint-hot" width="${SIZE}" height="${SIZE}"/>
      <path class="sheen" d="${d}"/>
    </g>
    <path class="rim" d="${d}"/>`;
}

function slicesMarkup(): string {
  if (!info || info.options.length === 0) {
    // Placeholder ring while waiting for the dragged files to arrive.
    const n = 8;
    return Array.from({ length: n }, (_, i) => {
      const d = slicePath(i, n);
      return `<g class="ghost" style="--i:${i}">${glassBody(`ghost-${i}`, d)}</g>`;
    }).join("");
  }
  const n = info.options.length;
  const rm = (R_IN + R_OUT) / 2 + 2;
  return info.options
    .map((o, i) => {
      const a = midAngle(i, n);
      const x = C + rm * Math.cos(a);
      const y = C + rm * Math.sin(a);
      const dx = (Math.cos(a) * HOT_LIFT).toFixed(2);
      const dy = (Math.sin(a) * HOT_LIFT).toFixed(2);
      const kx = C + (R_OUT - 13) * Math.cos(a);
      const ky = C + (R_OUT - 13) * Math.sin(a) + 3;
      return `
        <g class="slice-in" style="--i:${i}">
          <g class="slice" data-i="${i}" style="--dx:${dx}px;--dy:${dy}px">
            ${glassBody(`slice-${i}`, slicePath(i, n))}
            <text class="slice-label" x="${x.toFixed(1)}" y="${(y + 1).toFixed(1)}">${escapeHtml(o.label)}</text>
            <text class="slice-hint" x="${x.toFixed(1)}" y="${(y + 13).toFixed(1)}">${escapeHtml(hintOf(o))}</text>
            <text class="slice-key" x="${kx.toFixed(1)}" y="${ky.toFixed(1)}">${i < 9 ? i + 1 : ""}</text>
          </g>
        </g>`;
    })
    .join("");
}

function hubMarkup(): string {
  let glyph = glyphs.drop;
  let title = t("wheel.drop");
  let sub = t("wheel.release");
  if (unsupported) {
    glyph = glyphs.file;
    title = t("wheel.unsupported");
    sub = t("wheel.unsupportedSub");
  } else if (info) {
    glyph = glyphs[info.category];
    title = info.title;
    sub = info.skipped > 0 ? t("wheel.skipped", { n: info.skipped }) : "";
  }
  // The keep/delete choice; clickable (or key D) when the wheel has the mouse.
  const pill = unsupported
    ? ""
    : `<button class="keep-pill" type="button" tabindex="-1">${pillLabel()}</button>`;
  return `
    <div class="hub">
      <div class="hub-face hub-main">
        <div class="hub-glyph">${glyph}</div>
        <div class="hub-title">${escapeHtml(title)}</div>
        ${sub ? `<div class="hub-sub">${escapeHtml(sub)}</div>` : ""}
        ${pill}
      </div>
      <div class="hub-face hub-action">
        <div class="hub-to">${t("wheel.to")}</div>
        <div class="hub-target"></div>
      </div>
    </div>`;
}

function pillLabel(): string {
  return deleteOriginal
    ? `${glyphs.trash}<span>${t("wheel.delete")}</span>`
    : `${glyphs.keep}<span>${t("wheel.keep")}</span>`;
}

function updatePill() {
  const pill = stage.querySelector<HTMLElement>(".keep-pill");
  if (!pill) return;
  pill.innerHTML = pillLabel();
  pill.classList.toggle("on", deleteOriginal);
  pill.classList.remove("flip");
  void pill.offsetWidth; // restart the flip animation
  pill.classList.add("flip");
}

async function toggleDeleteOriginal() {
  deleteOriginal = !deleteOriginal;
  updatePill();
  await call("set_delete_original", { value: deleteOriginal });
}

function hubCircle(): string {
  return `M${C - HUB_R} ${C} A${HUB_R} ${HUB_R} 0 1 1 ${C + HUB_R} ${C} A${HUB_R} ${HUB_R} 0 1 1 ${C - HUB_R} ${C}Z`;
}

function render() {
  const cat = unsupported ? "unsupported" : (info?.category ?? "none");
  stage.innerHTML = `
    <div class="wheel" data-cat="${cat}">
      <svg class="ring" viewBox="0 0 ${SIZE} ${SIZE}" aria-hidden="true">
        <defs>
          <linearGradient id="hot" x1="0" y1="0" x2="${SIZE}" y2="${SIZE}" gradientUnits="userSpaceOnUse">
            <stop offset="0" class="stop-a"/>
            <stop offset="1" class="stop-b"/>
          </linearGradient>
          <linearGradient id="rim" x1="${SIZE * 0.09}" y1="${SIZE * 0.045}" x2="${SIZE * 0.91}" y2="${SIZE * 0.955}" gradientUnits="userSpaceOnUse">
            <stop offset="0" stop-color="#fff" stop-opacity="0.95"/>
            <stop offset="0.32" stop-color="#fff" stop-opacity="0.18"/>
            <stop offset="0.62" stop-color="#fff" stop-opacity="0.06"/>
            <stop offset="1" stop-color="#fff" stop-opacity="0.6"/>
          </linearGradient>
        </defs>
        <g class="hub-glass">${glassBody("hub", hubCircle())}</g>
        ${slicesMarkup()}
      </svg>
      ${hubMarkup()}
    </div>`;
  hot = OUTSIDE;
  stage.querySelector(".keep-pill")?.classList.toggle("on", deleteOriginal);
  applyGlass();
}

function setHot(i: number) {
  if (i === hot) return;
  hot = i;
  stage.querySelectorAll<SVGGElement>(".slice").forEach((el) => {
    el.classList.toggle("hot", Number(el.dataset.i) === i);
  });
  const hub = stage.querySelector(".hub");
  const option = info && i >= 0 ? info.options[i] : undefined;
  hub?.classList.toggle("acting", !!option);
  if (option) stage.querySelector(".hub-target")!.textContent = option.label;
}

/** Which slice is under a point in CSS pixels (HUB / OUTSIDE otherwise). */
function hitTest(x: number, y: number): number {
  const dx = x - C;
  const dy = y - C;
  const r = Math.hypot(dx, dy);
  if (r < R_IN - 4) return HUB;
  if (r > R_OUT + 18 || !info || info.options.length === 0) return OUTSIDE;
  const n = info.options.length;
  let a = Math.atan2(dy, dx) + Math.PI / 2;
  if (a < 0) a += 2 * Math.PI;
  return Math.floor(a / ((2 * Math.PI) / n)) % n;
}

function hitPhysical(p: { x: number; y: number }): number {
  return hitTest(p.x / devicePixelRatio, p.y / devicePixelRatio);
}

// ---- Lifecycle ---------------------------------------------------------------------------------

function reset() {
  stage.className = "stage";
  stage.innerHTML = "";
  info = null;
  unsupported = false;
  hot = OUTSIDE;
  phase = "hidden";
  pendingDragPaths = [];
  clearTimeout(releaseTimer);
  dropGlass();
}

async function open(payload: WheelOpen) {
  reset();
  const token = ++openToken;
  phase = "opening";
  mode = payload.mode;
  info = payload.info;
  deleteOriginal = !!payload.deleteOriginal;
  stage.classList.toggle("drag", mode === "drag");
  stage.classList.toggle("instant", reducedMotion);

  // Show the wheel right away; the glass is built meanwhile and fades in when ready.
  render();
  phase = "open";
  // Start the entrance once the window is actually painted.
  afterPaint(() => {
    if (token === openToken) stage.classList.add("open");
  });

  const url = await makeGlass();
  if (token !== openToken) {
    if (url) URL.revokeObjectURL(url);
    return;
  }
  glassUrl = url;
  stage.classList.toggle("no-glass", !glassUrl);
  applyGlass();
  // A DragEnter that arrived before we were listening for it.
  if (mode === "drag" && !info && pendingDragPaths.length) loadDroppedInfo(pendingDragPaths);
}

/**
 * Hides the window and clears the page, unless the wheel was reopened meanwhile (pressing the
 * shortcut again during the exit animation must not close the new wheel).
 */
async function finish(token: number) {
  if (token !== openToken) return;
  await call("hide_wheel");
  if (token === openToken) reset();
}

async function close() {
  if (phase !== "open" && phase !== "opening") return;
  const token = ++openToken;
  phase = "closing";
  stage.classList.add("closing");
  await sleep(reducedMotion ? 0 : 190);
  await finish(token);
}

async function choose(i: number, paths?: string[]) {
  if (phase !== "open" || !info || !info.options[i]) return;
  const token = ++openToken;
  phase = "closing";
  const target = info.options[i].id;
  const files = paths && paths.length ? paths : info.files;
  setHot(i);
  stage.querySelector(`.slice[data-i="${i}"]`)?.classList.add("chosen");
  stage.classList.add("choosing");
  // Queue the work right away; the animation only decides when the wheel disappears.
  const started = call("start_convert", { paths: files, target });
  await sleep(reducedMotion ? 0 : 170);
  if (token === openToken) stage.classList.add("closing");
  await sleep(reducedMotion ? 0 : 190);
  await started;
  await finish(token);
}

async function loadDroppedInfo(paths: string[]) {
  const next = await call<WheelInfo>("wheel_info", { paths });
  if (phase !== "open") return;
  info = next;
  unsupported = !next;
  render();
}

// ---- Events ------------------------------------------------------------------------------------

const demos: WheelInfo[] = [
  {
    category: "image",
    categoryLabel: "Image",
    title: "IMG_2041.HEIC",
    files: ["C:\\demo\\IMG_2041.HEIC"],
    skipped: 0,
    options: [
      { id: "jpg", label: "JPG", hint: "photo" },
      { id: "png", label: "PNG", hint: "lossless" },
      { id: "webp", label: "WEBP", hint: "web" },
      { id: "avif", label: "AVIF", hint: "smallest" },
      { id: "tiff", label: "TIFF", hint: "print" },
      { id: "bmp", label: "BMP", hint: "bitmap" },
      { id: "ico", label: "ICO", hint: "icon" },
    ],
  },
  {
    category: "video",
    categoryLabel: "Video",
    title: "3 files",
    files: ["a.mkv", "b.mkv", "c.mkv"],
    skipped: 0,
    options: [
      { id: "mp4", label: "MP4", hint: "H.264" },
      { id: "mov", label: "MOV", hint: "apple" },
      { id: "webm", label: "WEBM", hint: "VP9" },
      { id: "avi", label: "AVI", hint: "legacy" },
      { id: "gif", label: "GIF", hint: "animated" },
      { id: "mp3", label: "MP3", hint: "audio only" },
      { id: "m4a", label: "M4A", hint: "audio only" },
    ],
  },
];
let demoIndex = 0;

function openDemo() {
  open({ mode: "hotkey", info: demos[demoIndex++ % demos.length] });
}

// Language: re-render an open wheel if it changes (rare; the wheel is usually hidden).
initLang(() => {
  if (phase === "open") render();
});

if (!inTauri) {
  // Show the same fake desktop behind the page that the glass is built from.
  const bd = demoBackdrop(SIZE * devicePixelRatio, SIZE * devicePixelRatio);
  const c = document.createElement("canvas");
  c.width = bd.width;
  c.height = bd.height;
  c.getContext("2d")!.putImageData(new ImageData(new Uint8ClampedArray(bd.pixels), bd.width, bd.height), 0, 0);
  document.body.classList.add("demo");
  document.body.style.backgroundImage = `url(${c.toDataURL()})`;
  openDemo();
} else {
  listen<WheelOpen>("wheel:open", (e) => open(e.payload));

  // While a file is dragged the source owns the mouse, so the backend reports the cursor
  // (window-relative, physical pixels) and the release point.
  listen<{ x: number; y: number }>("wheel:drag-move", (e) => {
    if (phase === "open" && mode === "drag") setHot(hitPhysical(e.payload));
  });

  listen<{ x: number; y: number }>("wheel:drag-released", (e) => {
    if (mode !== "drag" || (phase !== "open" && phase !== "opening")) return;
    clearTimeout(releaseTimer);
    if (phase === "open" && info) {
      const i = hitPhysical(e.payload);
      if (i >= 0) choose(i);
      else close();
      return;
    }
    // Files unknown (drag from outside Explorer): give an OLE drop a moment to tell us.
    releaseTimer = window.setTimeout(() => {
      if ((phase === "open" || phase === "opening") && mode === "drag") close();
    }, 280);
  });

  getCurrentWebview().onDragDropEvent(async (e) => {
    const p = e.payload;
    if (p.type === "enter") pendingDragPaths = p.paths;
    if (phase !== "open") return;
    if (p.type === "enter") {
      const known = info && p.paths.length === info.files.length && p.paths.every((x) => info!.files.includes(x));
      if (!known) await loadDroppedInfo(p.paths);
      setHot(hitPhysical(p.position));
    } else if (p.type === "over") {
      setHot(hitPhysical(p.position));
    } else if (p.type === "drop") {
      clearTimeout(releaseTimer);
      if (!info) await loadDroppedInfo(p.paths);
      const i = hitPhysical(p.position);
      if (i >= 0) choose(i, p.paths);
      else close();
    } else if (p.type === "leave") {
      setHot(OUTSIDE);
    }
  });

  getCurrentWindow().onFocusChanged(({ payload: focused }) => {
    if (!focused && mode === "hotkey" && phase === "open") close();
  });
}

stage.addEventListener("pointermove", (e) => {
  if (phase === "open") setHot(hitTest(e.clientX, e.clientY));
});

stage.addEventListener("pointerleave", () => setHot(OUTSIDE));

stage.addEventListener("pointerdown", (e) => {
  if (phase !== "open" || e.button !== 0) return;
  if ((e.target as Element).closest?.(".keep-pill")) {
    toggleDeleteOriginal();
    return;
  }
  const i = hitTest(e.clientX, e.clientY);
  if (i >= 0) choose(i);
  else close();
});

window.addEventListener("keydown", (e) => {
  if (phase !== "open") return;
  const n = info?.options.length ?? 0;
  const step = (d: number) => setHot(n ? ((hot < 0 ? (d > 0 ? -1 : 0) : hot) + d + n) % n : OUTSIDE);
  switch (e.key) {
    case "Escape":
      close();
      break;
    case "d":
    case "D":
      toggleDeleteOriginal();
      break;
    case "ArrowRight":
    case "ArrowDown":
      step(1);
      break;
    case "ArrowLeft":
    case "ArrowUp":
      step(-1);
      break;
    case "Tab":
      step(e.shiftKey ? -1 : 1);
      break;
    case "Enter":
    case " ":
      if (hot >= 0) choose(hot);
      break;
    default:
      if (/^[1-9]$/.test(e.key) && Number(e.key) <= n) choose(Number(e.key) - 1);
      return;
  }
  e.preventDefault();
});

window.addEventListener("contextmenu", (e) => e.preventDefault());
