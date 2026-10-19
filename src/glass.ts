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
      blob = await new Promise<Blob>((resolve, reject) => {
        pending.set(id, { resolve, reject });
        w.postMessage({ id, bd: { width: bd.width, height: bd.height, pixels }, cssWidth, desc, opts }, [pixels.buffer]);
      });
    } else {
      blob = await renderGlass(bd, cssWidth, desc, opts);
    }
    return URL.createObjectURL(blob);
  } catch {
    return "";
  }
}

// ---- Demo backdrop (browser preview only) --------------------------------------------------------

/** A colourful fake desktop so the glass can be previewed outside Tauri. */
export function demoBackdrop(w: number, h: number): Backdrop {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const ctx = c.getContext("2d", { willReadFrequently: true })!;
  const g = ctx.createLinearGradient(0, 0, w, h);
  g.addColorStop(0, "#1d4ed8");
  g.addColorStop(0.5, "#7c3aed");
  g.addColorStop(1, "#db2777");
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, w, h);
  const blobs: [number, number, number, string][] = [
    [0.22, 0.25, 0.22, "#22d3ee"],
    [0.8, 0.3, 0.18, "#facc15"],
    [0.7, 0.82, 0.24, "#f97316"],
    [0.18, 0.8, 0.16, "#34d399"],
  ];
  for (const [x, y, r, col] of blobs) {
    ctx.fillStyle = col;
    ctx.beginPath();
    ctx.arc(x * w, y * h, r * w, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.fillStyle = "rgba(255,255,255,0.9)";
  ctx.font = `700 ${Math.round(w * 0.09)}px Segoe UI, sans-serif`;
  ctx.textAlign = "center";
  ctx.fillText("Desktop", w / 2, h * 0.53);
  for (let i = 0; i < 12; i++) {
    ctx.fillStyle = "rgba(255,255,255,0.85)";
    ctx.fillRect(w * 0.06, h * (0.05 + i * 0.075), w * 0.08, h * 0.012);
  }
  return { width: w, height: h, pixels: ctx.getImageData(0, 0, w, h).data };
}
