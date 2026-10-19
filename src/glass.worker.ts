// Builds liquid glass off the UI thread, so opening animations never wait for it.
import { renderGlass, type Backdrop, type GlassOptions, type ShapeDesc } from "./glass-core";

type Request = { id: number; bd: Backdrop; cssWidth: number; desc: ShapeDesc; opts: GlassOptions };

self.onmessage = async (e: MessageEvent<Request>) => {
  const { id, bd, cssWidth, desc, opts } = e.data;
  try {
    const blob = await renderGlass(bd, cssWidth, desc, opts);
    self.postMessage({ id, blob });
  } catch (err) {
    self.postMessage({ id, error: String(err) });
  }
};
