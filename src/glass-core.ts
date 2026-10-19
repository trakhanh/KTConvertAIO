// The liquid-glass pixel work, free of DOM and Tauri so it can run inside a Web Worker.
//
// It blurs and saturates a capture of the desktop, then bends it near the shape's edges like
// light passing through the rim of a lens, and encodes the result as a JPEG.

export type Backdrop = { width: number; height: number; pixels: Uint8ClampedArray };

export type GlassOptions = {
  blur?: number; // CSS px
  saturate?: number;
  brightness?: number;
  bezel?: number; // width of the refracting rim, CSS px
  refraction?: number; // how far the rim bends the image, CSS px
};

/** Serializable description of a glass shape, in CSS pixels. */
export type ShapeDesc =
  | { kind: "ring"; cx: number; cy: number; rIn: number; rOut: number; hubR: number }
  | { kind: "rect"; left: number; top: number; width: number; height: number; radius: number };

/** Distance inside the shape and the outward edge normal; null outside. */
type ShapeHit = { depth: number; nx: number; ny: number };
type Shape = (x: number, y: number) => ShapeHit | null;

/** A ring between rIn and rOut plus a disc of radius hubR, all centred on (cx, cy). */
function ringWithHub(cx: number, cy: number, rIn: number, rOut: number, hubR: number): Shape {
  return (x, y) => {
    const dx = x - cx;
    const dy = y - cy;
    const r = Math.hypot(dx, dy) || 1e-6;
    const ux = dx / r;
    const uy = dy / r;
    if (r <= hubR) return { depth: hubR - r, nx: ux, ny: uy };
    if (r >= rIn && r <= rOut) {
      const toOuter = rOut - r;
      const toInner = r - rIn;
      return toOuter < toInner ? { depth: toOuter, nx: ux, ny: uy } : { depth: toInner, nx: -ux, ny: -uy };
    }
    return null;
  };
}

function roundedRect(left: number, top: number, width: number, height: number, radius: number): Shape {
  const cx = left + width / 2;
  const cy = top + height / 2;
  const hx = width / 2 - radius;
  const hy = height / 2 - radius;
  return (x, y) => {
    const px = x - cx;
    const py = y - cy;
    const qx = Math.abs(px) - hx;
    const qy = Math.abs(py) - hy;
    const outside = Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - radius;
    if (outside > 0) return null;
    let nx: number;
    let ny: number;
    if (qx > 0 && qy > 0) {
      const l = Math.hypot(qx, qy);
      nx = qx / l;
      ny = qy / l;
    } else if (qx > qy) {
      nx = 1;
      ny = 0;
    } else {
      nx = 0;
      ny = 1;
    }
    return { depth: -outside, nx: nx * Math.sign(px || 1), ny: ny * Math.sign(py || 1) };
  };
}

function shapeOf(d: ShapeDesc): Shape {
  return d.kind === "ring"
    ? ringWithHub(d.cx, d.cy, d.rIn, d.rOut, d.hubR)
    : roundedRect(d.left, d.top, d.width, d.height, d.radius);
}

function canvas(w: number, h: number) {
  const c = new OffscreenCanvas(w, h);
  return { c, ctx: c.getContext("2d", { willReadFrequently: true })! };
}

/** Renders the glass over a window cssWidth wide; returns a JPEG the size of the backdrop. */
export async function renderGlass(bd: Backdrop, cssWidth: number, desc: ShapeDesc, opts: GlassOptions = {}): Promise<Blob> {
  const { blur = 16, saturate = 1.8, brightness = 1.04, bezel = 18, refraction = 16 } = opts;
  const { width: w, height: h } = bd;
  const scale = w / cssWidth;
  const shape = shapeOf(desc);

  const src = canvas(w, h);
  src.ctx.putImageData(new ImageData(new Uint8ClampedArray(bd.pixels), w, h), 0, 0);

  // Extend the edges outward before blurring so the borders do not fade to black.
  const pad = Math.ceil(blur * scale * 3);
  const padded = canvas(w + pad * 2, h + pad * 2);
  padded.ctx.drawImage(src.c, 0, 0, padded.c.width, padded.c.height);
  padded.ctx.drawImage(src.c, pad, pad);

  const frosted = canvas(w, h);
  frosted.ctx.filter = `blur(${blur * scale}px) saturate(${saturate}) brightness(${brightness})`;
  frosted.ctx.drawImage(padded.c, -pad, -pad);
  frosted.ctx.filter = "none";
  const from = frosted.ctx.getImageData(0, 0, w, h).data;

  // Refract: near the rim, sample from further out along the edge normal (a convex lens edge).
  const out = frosted.ctx.createImageData(w, h);
  const dst = out.data;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = (y * w + x) * 4;
      const hit = shape((x + 0.5) / scale, (y + 0.5) / scale);
      let j = i;
      if (hit && hit.depth < bezel) {
        const t = 1 - hit.depth / bezel;
        const m = refraction * t * t * scale;
        const sx = Math.min(w - 1, Math.max(0, Math.round(x + hit.nx * m)));
        const sy = Math.min(h - 1, Math.max(0, Math.round(y + hit.ny * m)));
        j = (sy * w + sx) * 4;
      }
      dst[i] = from[j];
      dst[i + 1] = from[j + 1];
      dst[i + 2] = from[j + 2];
      dst[i + 3] = 255;
    }
  }
  frosted.ctx.putImageData(out, 0, 0);
  return frosted.c.convertToBlob({ type: "image/jpeg", quality: 0.92 });
}
