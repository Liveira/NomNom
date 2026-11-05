import type { ModuleDef } from '../lib/module-def';

/** Finds every `modules/<id>/index.ts`. Only these small files load eagerly; views stay lazy. */
export function discoverModules(): ModuleDef[] {
  const found = import.meta.glob<{ default: ModuleDef }>('../modules/*/index.ts', { eager: true });
  return Object.values(found).map((m) => m.default);
}
