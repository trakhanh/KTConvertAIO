// Liquid glass for the wheel and the toast.
//
// The page cannot blur what is behind a transparent window, so the backend hands us a screenshot
// of that area and a Web Worker turns it into frosted, refracting glass (see glass-core.ts). The
// result is a static image used as the background of each glass shape, so hover and entrance
// animations cost nothing extra, and the UI thread is never busy while they play.

import { invoke } from "@tauri-apps/api/core";
import { renderGlass, type Backdrop, type GlassOptions, type ShapeDesc } from "./glass-core";

export type { Backdrop, GlassOptions, ShapeDesc };

export async function fetchBackdrop(command: string): Promise<Backdrop | null> {
  try {
    const buf = await invoke<ArrayBuffer>(command);
    if (!(buf instanceof ArrayBuffer) || buf.byteLength < 8) return null;
    const view = new DataView(buf);
    const width = view.getUint32(0, true);
    const height = view.getUint32(4, true);
    if (buf.byteLength < 8 + width * height * 4) return null;
    return { width, height, pixels: new Uint8ClampedArray(buf, 8, width * height * 4) };
  } catch {
    return null;
  }
}

// ---- Worker ----------------------------------------------------------------------------------------

const WORKER_TIMEOUT_MS = 1500;
let worker: Worker | null = null;
let nextId = 0;
const pending = new Map<number, { resolve: (b: Blob) => void; reject: (e: Error) => void }>();

function getWorker(): Worker | null {
  if (worker) return worker;
  try {
    worker = new Worker(new URL("./glass.worker.ts", import.meta.url), { type: "module" });
    worker.onmessage = (e: MessageEvent<{ id: number; blob?: Blob; error?: string }>) => {
      const job = pending.get(e.data.id);
      if (!job) return;
      pending.delete(e.data.id);
      if (e.data.blob) job.resolve(e.data.blob);
      else job.reject(new Error(e.data.error ?? "glass failed"));
    };
    worker.onerror = () => {
      // Fail every waiting job; the next call falls back to the main thread.
      for (const job of pending.values()) job.reject(new Error("glass worker crashed"));
      pending.clear();
      worker = null;
    };
  } catch {
    worker = null;
  }
  return worker;
}

/** Builds the glass for `desc` and returns an object URL of the image ("" if it failed). */
export async function buildGlass(bd: Backdrop, cssWidth: number, desc: ShapeDesc, opts: GlassOptions = {}): Promise<string> {
  try {
    const w = getWorker();
    let blob: Blob;
    if (w) {
      // Copy into a buffer of our own so it can be transferred, not cloned.
      const pixels = new Uint8ClampedArray(bd.pixels);
      const id = ++nextId;
      const viaWorker = new Promise<Blob>((resolve, reject) => {
        pending.set(id, { resolve, reject });
        w.postMessage({ id, bd: { width: bd.width, height: bd.height, pixels }, cssWidth, desc, opts }, [pixels.buffer]);
      });
      // A worker that never answers must not leave the UI without glass: build it here instead.
      const timeout = new Promise<null>((resolve) => setTimeout(() => resolve(null), WORKER_TIMEOUT_MS));
      const fromWorker = await Promise.race([viaWorker, timeout]);
      if (fromWorker) {
        blob = fromWorker;
      } else {
        pending.delete(id);
        blob = await renderGlass(bd, cssWidth, desc, opts);
      }
    } else {
      blob = await renderGlass(bd, cssWidth, desc, opts);
    }
    return URL.createObjectURL(blob);
  } catch {
    return "";
  }
}

// ---- Demo backdrop (browser preview only) --------------------------------------------------------

/**
 * A fake desktop so the glass can be previewed outside Tauri: an abstract "bloom" wallpaper,
 * drawn for the whole viewport so the window's glass and the page background line up.
 */
export function demoWallpaper(w: number, h: number): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const ctx = c.getContext("2d")!;
  const base = ctx.createLinearGradient(0, 0, w, h);
  base.addColorStop(0, "#0b1030");
  base.addColorStop(0.55, "#16123d");
  base.addColorStop(1, "#2a0f3a");
  ctx.fillStyle = base;
  ctx.fillRect(0, 0, w, h);
  const s = Math.max(w, h);
  const blooms: [number, number, number, string][] = [
    [0.18, 0.2, 0.55, "rgba(56, 189, 248, 0.85)"],
    [0.82, 0.22, 0.5, "rgba(168, 85, 247, 0.9)"],
    [0.7, 0.85, 0.6, "rgba(244, 63, 94, 0.75)"],
    [0.2, 0.9, 0.45, "rgba(250, 204, 21, 0.55)"],
    [0.5, 0.5, 0.35, "rgba(99, 102, 241, 0.6)"],
  ];
  for (const [x, y, r, col] of blooms) {
    const g = ctx.createRadialGradient(x * w, y * h, 0, x * w, y * h, r * s);
    g.addColorStop(0, col);
    g.addColorStop(1, "rgba(0, 0, 0, 0)");
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
  }
  // A few soft ribbons so the refraction at the glass edges is visible.
  ctx.globalCompositeOperation = "screen";
  for (let i = 0; i < 4; i++) {
    ctx.strokeStyle = `rgba(255, 255, 255, ${0.08 + i * 0.03})`;
    ctx.lineWidth = s * (0.012 + i * 0.006);
    ctx.beginPath();
    ctx.moveTo(-0.1 * w, (0.25 + i * 0.17) * h);
    ctx.bezierCurveTo(0.3 * w, (0.05 + i * 0.2) * h, 0.6 * w, (0.7 + i * 0.05) * h, 1.1 * w, (0.35 + i * 0.12) * h);
    ctx.stroke();
  }
  ctx.globalCompositeOperation = "source-over";
  return c;
}

/** The part of the demo wallpaper behind a window at (x, y) of size w x h, in device pixels. */
export function demoBackdrop(w: number, h: number, wallpaper?: HTMLCanvasElement, x = 0, y = 0): Backdrop {
  const paper = wallpaper ?? demoWallpaper(w, h);
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const ctx = c.getContext("2d", { willReadFrequently: true })!;
  ctx.drawImage(paper, -x, -y);
  return { width: w, height: h, pixels: ctx.getImageData(0, 0, w, h).data };
}
