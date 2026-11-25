// Appearance on this browser: the system's, or always light or dark. Applied
// before the first render (main.tsx), so a page never flashes the other one.
export type Theme = 'system' | 'light' | 'dark';
const KEY = 'threadbear.theme';

export function readTheme(): Theme {
  const saved = localStorage.getItem(KEY);
  return saved === 'light' || saved === 'dark' ? saved : 'system';
}

export function applyTheme(theme: Theme = readTheme()): void {
  if (theme === 'system') delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
}

export function setTheme(theme: Theme): void {
  localStorage.setItem(KEY, theme);
  applyTheme(theme);
}

// The colours on this browser: the stock ones, or a named theme (the desktop
// app's Settings > General > Appearance > Theme). Harbor has a light and a
// dark side and follows the choice above; Squall is always dark. Kept apart
// from the theme, so the two combine. styles.css has the colours.
export type Palette = 'stock' | 'harbor' | 'squall';
export const PALETTES: Palette[] = ['stock', 'harbor', 'squall'];
const PALETTE_KEY = 'threadbear.palette';

/** What a saved value means: anything unknown is the stock look. */
export function paletteFrom(saved: string | null): Palette {
  return saved === 'harbor' || saved === 'squall' ? saved : 'stock';
}

/** Squall has no light side, so light and "follow the system" don't apply to it. */
export function darkOnly(palette: Palette): boolean {
  return palette === 'squall';
}

export function readPalette(): Palette {
  return paletteFrom(localStorage.getItem(PALETTE_KEY));
}

export function applyPalette(palette: Palette = readPalette()): void {
  if (palette === 'stock') delete document.documentElement.dataset.palette;
  else document.documentElement.dataset.palette = palette;
}

export function setPalette(palette: Palette): void {
  if (palette === 'stock') localStorage.removeItem(PALETTE_KEY);
  else localStorage.setItem(PALETTE_KEY, palette);
  applyPalette(palette);
}
