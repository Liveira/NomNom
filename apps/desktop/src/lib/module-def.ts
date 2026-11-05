import type { Component } from 'svelte';

/** Frontend half of a module: how it renders. The backend manifest decides nav order and icon. */
export interface ModuleDef {
  id: string;
  name: string;
  /** Lazy so that a module's code is only loaded when its view is opened. */
  view: () => Promise<{ default: Component }>;
}

export const defineModule = (def: ModuleDef): ModuleDef => def;
