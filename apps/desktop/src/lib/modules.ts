import type { ModuleManifest } from './ipc';
import type { ModuleDef } from './module-def';

export { defineModule, type ModuleDef } from './module-def';

export interface NavItem {
  id: string;
  name: string;
  icon: string;
  order: number;
  view: ModuleDef['view'];
}

/** Settings is pinned to the bottom of the sidebar, so it is not part of the main list. */
export const FOOTER_MODULES = new Set(['settings']);

/** Merges frontend definitions with backend manifests. Modules unknown to the backend are hidden. */
export function buildNav(defs: ModuleDef[], manifests: ModuleManifest[]): NavItem[] {
  const byId = new Map(defs.map((d) => [d.id, d]));
  const items: NavItem[] = [];
  for (const m of manifests) {
    const def = byId.get(m.id);
    if (!def || !m.nav) continue;
    items.push({ id: m.id, name: m.name, icon: m.nav.icon, order: m.nav.order, view: def.view });
  }
  return items.sort((a, b) => a.order - b.order);
}
