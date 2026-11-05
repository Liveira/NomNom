import { describe, expect, it } from 'vitest';
import { buildNav, defineModule } from './modules';
import type { ModuleManifest } from './ipc';

const view = async () => ({ default: (() => {}) as never });

describe('buildNav', () => {
  const defs = [
    defineModule({ id: 'a', name: 'A', view }),
    defineModule({ id: 'b', name: 'B', view }),
    defineModule({ id: 'orphan', name: 'Orphan', view }),
  ];
  const manifests: ModuleManifest[] = [
    { id: 'b', name: 'Bee', nav: { order: 20, icon: 'b' }, settings_key: null },
    { id: 'a', name: 'Ay', nav: { order: 10, icon: 'a' }, settings_key: null },
    { id: 'nofrontend', name: 'X', nav: { order: 1, icon: 'x' }, settings_key: null },
    { id: 'hidden', name: 'H', nav: null, settings_key: null },
  ];

  it('orders by backend nav order and uses backend names', () => {
    expect(buildNav(defs, manifests).map((n) => n.name)).toEqual(['Ay', 'Bee']);
  });

  it('ignores backend modules without a frontend view or nav entry', () => {
    const ids = buildNav(defs, manifests).map((n) => n.id);
    expect(ids).not.toContain('nofrontend');
    expect(ids).not.toContain('hidden');
    expect(ids).not.toContain('orphan');
  });
});
