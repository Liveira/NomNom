/** Palettes whose colours are light. Everything else is dark. Used for the browser's colour scheme. */
export const LIGHT_PALETTES = new Set(['light', 'sakura', 'matcha', 'lavender', 'frost', 'candy']);
export const isLight = (palette: string): boolean => LIGHT_PALETTES.has(palette);
