// Line icons (24x24, stroke = currentColor) for the wheel hub and the toast.

const svg = (body: string) =>
  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${body}</svg>`;

export const glyphs = {
  image: svg('<rect x="3" y="4" width="18" height="16" rx="3"/><circle cx="9" cy="9.5" r="1.8"/><path d="m4 18 5.5-5.5a1.5 1.5 0 0 1 2.1 0L20 21"/><path d="m14 15 2-2a1.5 1.5 0 0 1 2.1 0L21 16"/>'),
  animated: svg('<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M10.5 9.5H8.8A1.8 1.8 0 0 0 7 11.3v1.4a1.8 1.8 0 0 0 1.8 1.8h1.7V12H9"/><path d="M13 9.5v5"/><path d="M16 14.5v-5h2.5M16 12h2"/>'),
  video: svg('<rect x="3" y="5" width="18" height="14" rx="3"/><path d="m10 9.2 4.8 2.8-4.8 2.8z" fill="currentColor"/>'),
  audio: svg('<path d="M9 18V6l11-2v12"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="17.5" cy="16" r="2.5"/>'),
  file: svg('<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>'),
  drop: svg('<path d="M12 4v11"/><path d="m7.5 10.5 4.5 4.5 4.5-4.5"/><path d="M5 20h14"/>'),
  check: svg('<path d="m5 12.5 4.5 4.5L19 7.5"/>'),
  alert: svg('<path d="M12 8v5"/><path d="M12 16.5h.01"/>'),
  info: svg('<path d="M12 11v6"/><path d="M12 7.5h.01"/>'),
  convert: svg('<path d="M4 12a8 8 0 0 1 13.7-5.6L20 8.5"/><path d="M20 4v4.5h-4.5"/><path d="M20 12a8 8 0 0 1-13.7 5.6L4 15.5"/><path d="M4 20v-4.5h4.5"/>'),
  trash: svg('<path d="M4 7h16"/><path d="M9 7V4.5h6V7"/><path d="M6 7l1 13h10l1-13"/><path d="M10 11v5M14 11v5"/>'),
  keep: svg('<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/><path d="m9 14 2 2 4-4"/>'),
  close: svg('<path d="m7 7 10 10M17 7 7 17"/>'),
  folder: svg('<path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2h7.5A2.5 2.5 0 0 1 21 9.5v8a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5z"/>'),
};

/**
 * Runs `fn` once the current state has been painted (two animation frames), so a CSS transition
 * starts from it. Falls back to a timer: a WebView that was just shown can hold back frames for a
 * moment, and the UI must never stay invisible because of that.
 */
export function afterPaint(fn: () => void, fallbackMs = 80): void {
  let done = false;
  const run = () => {
    if (!done) {
      done = true;
      fn();
    }
  };
  requestAnimationFrame(() => requestAnimationFrame(run));
  setTimeout(run, fallbackMs);
}

export function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}
