import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { escapeHtml } from "./glyphs";
import { hintOf, initLang, setLangLocally, t, type Key } from "./i18n";

type TargetOption = { id: string; label: string; hint: string; hintVi?: string; ext: string };
type CategoryModel = { key: string; all: TargetOption[]; selected: string[]; defaults: string[] };
type SettingsModel = {
  categories: CategoryModel[];
  deleteOriginal: boolean;
  gestureEnabled: boolean;
  autostart: boolean;
  maxOnWheel: number;
  hotkey: string;
  language: string;
  lang: string;
};

const ACCENT: Record<string, [string, string]> = {
  image: ["#8b5cff", "#c08bff"],
  animated: ["#ff4f9a", "#ff9a6b"],
  video: ["#2f7bff", "#22d3ff"],
  audio: ["#10c48b", "#7cf0b4"],
};

const inTauri = "__TAURI_INTERNALS__" in window;
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

let model: SettingsModel | null = null;
let current = "image";
let savedTimer: number | undefined;

// ---- Backend (with a stand-in for previewing the page in a browser) -----------------------------

const demoModel: SettingsModel = {
  deleteOriginal: false,
  gestureEnabled: true,
  autostart: true,
  maxOnWheel: 8,
  hotkey: "Ctrl+Alt+C",
  language: "auto",
  lang: "vi",
  categories: [
    {
      key: "image",
      defaults: ["jpg", "png", "webp", "avif", "tiff", "bmp", "ico"],
      selected: ["jpg", "png", "webp", "avif", "tiff", "bmp", "ico"],
      all: [
        ["jpg", "JPG", "photo"], ["png", "PNG", "lossless"], ["webp", "WEBP", "web"], ["avif", "AVIF", "smallest"],
        ["tiff", "TIFF", "print"], ["bmp", "BMP", "bitmap"], ["ico", "ICO", "icon"], ["gif", "GIF", "still"],
        ["jxl", "JXL", "JPEG XL"], ["jp2", "JP2", "JPEG 2000"], ["tga", "TGA", "game / 3D"], ["qoi", "QOI", "fast lossless"],
        ["exr", "EXR", "HDR float"], ["hdr", "HDR", "radiance"], ["ppm", "PPM", "raw pixels"], ["pcx", "PCX", "legacy"],
      ].map(([id, label, hint]) => ({ id, label, hint, ext: id })),
    },
    { key: "animated", defaults: ["mp4", "webm"], selected: ["mp4", "webm"], all: [{ id: "mp4", label: "MP4", hint: "video", ext: "mp4" }, { id: "webm", label: "WEBM", hint: "web video", ext: "webm" }] },
    { key: "video", defaults: ["mp4"], selected: ["mp4"], all: [{ id: "mp4", label: "MP4", hint: "H.264", ext: "mp4" }, { id: "hevc", label: "HEVC", hint: "H.265 · smaller", ext: "mp4" }] },
    { key: "audio", defaults: ["mp3"], selected: ["mp3"], all: [{ id: "mp3", label: "MP3", hint: "universal", ext: "mp3" }] },
  ],
};

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args);
  if (cmd === "get_settings") return structuredClone(demoModel) as T;
  if (cmd === "set_wheel_formats") {
    const ids = (args!.ids as string[]).slice(0, 8);
    return (ids.length ? ids : demoModel.categories.find((c) => c.key === args!.category)!.defaults) as T;
  }
  return args?.value as T;
}

function flashSaved() {
  const el = $("saved");
  el.classList.add("show");
  clearTimeout(savedTimer);
  savedTimer = window.setTimeout(() => el.classList.remove("show"), 1200);
}

// ---- Rendering ----------------------------------------------------------------------------------

function cat(): CategoryModel {
  return model!.categories.find((c) => c.key === current)!;
}

function renderTabs() {
  $("tabs").innerHTML = model!.categories
    .map(
      (c) =>
        `<button role="tab" type="button" class="tab ${c.key === current ? "active" : ""}" data-key="${c.key}"
           aria-selected="${c.key === current}">${t(`tab.${c.key}` as Key)}<span>${c.selected.length}</span></button>`,
    )
    .join("");
}

/** A small, flat version of the wheel so the order is easy to picture. */
function renderPreview() {
  const c = cat();
  const [a, b] = ACCENT[c.key] ?? ACCENT.image;
  const n = c.selected.length;
  const R = 92;
  const r = 44;
  const C = 100;
  const pt = (rad: number, ang: number) => `${(C + rad * Math.cos(ang)).toFixed(2)} ${(C + rad * Math.sin(ang)).toFixed(2)}`;
  const slices = c.selected
    .map((id, i) => {
      const step = (2 * Math.PI) / n;
      const a0 = -Math.PI / 2 + i * step + 0.03;
      const a1 = a0 + step - 0.06;
      const large = a1 - a0 > Math.PI ? 1 : 0;
      const mid = (a0 + a1) / 2;
      const label = c.all.find((f) => f.id === id)?.label ?? id;
      return `
        <path class="mini-slice" style="--i:${i}" d="M${pt(R, a0)} A${R} ${R} 0 ${large} 1 ${pt(R, a1)} L${pt(r, a1)} A${r} ${r} 0 ${large} 0 ${pt(r, a0)}Z"/>
        <text x="${(C + 68 * Math.cos(mid)).toFixed(1)}" y="${(C + 68 * Math.sin(mid) + 3.5).toFixed(1)}">${escapeHtml(label)}</text>`;
    })
    .join("");
  $("mini").innerHTML = `
    <defs><linearGradient id="mini-grad" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs>
    ${slices}
    <circle cx="${C}" cy="${C}" r="${r - 6}" class="mini-hub"/>`;
  $("count").textContent = t("set.slots", { n, max: model!.maxOnWheel });
}

function renderChosen() {
  const c = cat();
  $("chosen").innerHTML = c.selected
    .map((id, i) => {
      const f = c.all.find((x) => x.id === id);
      if (!f) return "";
      return `
        <li class="chip" draggable="true" data-id="${f.id}">
          <span class="grip" aria-hidden="true">⋮⋮</span>
          <span class="num">${i + 1}</span>
          <strong>${escapeHtml(f.label)}</strong>
          <span class="hint">${escapeHtml(hintOf(f))}</span>
          <button type="button" class="remove" data-id="${f.id}" title="${t("set.remove")}" aria-label="${t("set.remove")}: ${escapeHtml(f.label)}">×</button>
        </li>`;
    })
    .join("");
}

function renderGrid() {
  const c = cat();
  const full = c.selected.length >= model!.maxOnWheel;
  $("grid").innerHTML = c.all
    .map((f) => {
      const on = c.selected.includes(f.id);
      const disabled = !on && full;
      const isDefault = c.defaults.includes(f.id);
      return `
        <button type="button" class="fmt ${on ? "on" : ""}" data-id="${f.id}" ${disabled ? "disabled" : ""}
          title="${disabled ? t("set.full") : t("set.savesAs", { ext: f.ext })}">
          <span class="check" aria-hidden="true"></span>
          <strong>${escapeHtml(f.label)}</strong>
          <span class="hint">${escapeHtml(hintOf(f))}</span>
          ${isDefault ? `<span class="tag">${t("set.default")}</span>` : ""}
        </button>`;
    })
    .join("");
}

/** Fills every element marked data-i18n with its string in the current language. */
function translateStatic() {
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    el.textContent = t(el.dataset.i18n as Key);
  });
  document.title = `KT Convert · ${t("set.title")}`;
}

function render() {
  translateStatic();
  if (!model) return;
  ($("language") as HTMLSelectElement).value = model.language;
  ($("delete-original") as HTMLInputElement).checked = model.deleteOriginal;
  ($("gesture") as HTMLInputElement).checked = model.gestureEnabled;
  ($("autostart") as HTMLInputElement).checked = model.autostart;
  $("hotkey").textContent = model.hotkey;
  document.documentElement.dataset.cat = current;
  renderTabs();
  renderPreview();
  renderChosen();
  renderGrid();
}

// ---- Changes ------------------------------------------------------------------------------------

async function saveSelection(ids: string[]) {
  const c = cat();
  c.selected = await call<string[]>("set_wheel_formats", { category: c.key, ids });
  render();
  flashSaved();
}

async function load() {
  model = await call<SettingsModel>("get_settings");
  render();
}

$("language").addEventListener("change", async (e) => {
  const value = (e.target as HTMLSelectElement).value;
  model!.language = value;
  // The backend answers with a language:changed event, which re-renders the page.
  await call<string>("set_language", { language: value });
  if (!inTauri) {
    setLangLocally(value);
    render();
  }
  flashSaved();
});

$("tabs").addEventListener("click", (e) => {
  const tab = (e.target as Element).closest<HTMLElement>(".tab");
  if (!tab) return;
  current = tab.dataset.key!;
  render();
});

$("grid").addEventListener("click", (e) => {
  const btn = (e.target as Element).closest<HTMLButtonElement>(".fmt");
  if (!btn || btn.disabled) return;
  const id = btn.dataset.id!;
  const sel = cat().selected;
  saveSelection(sel.includes(id) ? sel.filter((x) => x !== id) : [...sel, id]);
});

$("chosen").addEventListener("click", (e) => {
  const btn = (e.target as Element).closest<HTMLButtonElement>(".remove");
  if (!btn) return;
  const sel = cat().selected;
  if (sel.length <= 1) return; // keep at least one
  saveSelection(sel.filter((x) => x !== btn.dataset.id));
});

$("reset").addEventListener("click", () => saveSelection([]));

$("delete-original").addEventListener("change", async (e) => {
  model!.deleteOriginal = await call<boolean>("set_delete_original", { value: (e.target as HTMLInputElement).checked });
  flashSaved();
});

$("autostart").addEventListener("change", async (e) => {
  model!.autostart = await call<boolean>("set_autostart", { value: (e.target as HTMLInputElement).checked });
  flashSaved();
});

$("gesture").addEventListener("change", async (e) => {
  model!.gestureEnabled = await call<boolean>("set_gesture", { value: (e.target as HTMLInputElement).checked });
  flashSaved();
});

// Drag chips to reorder.
let dragId: string | null = null;
$("chosen").addEventListener("dragstart", (e) => {
  const chip = (e.target as Element).closest<HTMLElement>(".chip");
  if (!chip) return;
  dragId = chip.dataset.id!;
  chip.classList.add("dragging");
  e.dataTransfer!.effectAllowed = "move";
});
$("chosen").addEventListener("dragover", (e) => {
  if (!dragId) return;
  e.preventDefault();
  const over = (e.target as Element).closest<HTMLElement>(".chip");
  const list = $("chosen");
  const dragging = list.querySelector<HTMLElement>(".dragging");
  if (!over || !dragging || over === dragging) return;
  const box = over.getBoundingClientRect();
  const after = e.clientY > box.top + box.height / 2;
  list.insertBefore(dragging, after ? over.nextSibling : over);
});
$("chosen").addEventListener("dragend", () => {
  if (!dragId) return;
  dragId = null;
  const ids = [...$("chosen").querySelectorAll<HTMLElement>(".chip")].map((el) => el.dataset.id!);
  saveSelection(ids);
});

$("homepage").addEventListener("click", () => {
  if (inTauri) call("open_homepage");
  else window.open("https://github.com/trakhanh/KTConvertAIO", "_blank");
});

if (inTauri) {
  listen("settings:refresh", () => load());
  getVersion().then((v) => ($("version").textContent = v));
} else {
  $("version").textContent = "dev";
}
initLang(() => render());
load();
