import { describe, expect, it } from 'vitest';
import { CORE, DEFAULT_PICKS, TOOLS, sidebarItems, withTool } from './tools';

const nav = ['home', 'tools', 'downloads', 'activity', 'backup', 'monitor', 'settings'].map((id) => ({ id }));
const ids = (list: { id: string }[]) => list.map((n) => n.id);

describe('sidebarItems', () => {
  it('shows everything until the person chooses', () => {
    expect(ids(sidebarItems(nav, null))).toEqual(ids(nav));
  });
  it('shows the chosen tools and always the core pages, in the usual order', () => {
    expect(ids(sidebarItems(nav, ['monitor', 'downloads']))).toEqual(['home', 'tools', 'downloads', 'monitor', 'settings']);
  });
  it('an empty choice still leaves Home, Tools and Settings', () => {
    expect(ids(sidebarItems(nav, []))).toEqual(['home', 'tools', 'settings']);
  });
  it('ignores names that are not tools', () => {
    expect(ids(sidebarItems(nav, ['nope']))).toEqual(['home', 'tools', 'settings']);
  });
});

describe('withTool', () => {
  it('starts from "everything" when nothing was chosen yet', () => {
    expect(withTool(null, 'backup', false)).toEqual(TOOLS.map((t) => t.id).filter((id) => id !== 'backup'));
  });
  it('adds a tool without duplicating it, in the tool list order', () => {
    expect(withTool(['monitor'], 'downloads', true)).toEqual(['downloads', 'monitor']);
    expect(withTool(['downloads'], 'downloads', true)).toEqual(['downloads']);
  });
  it('removes a tool', () => {
    expect(withTool(['downloads', 'monitor'], 'monitor', false)).toEqual(['downloads']);
  });
});

describe('the tool list', () => {
  it('has a title, a description and an icon for each tool, and none of them is a core page', () => {
    for (const t of TOOLS) {
      expect(t.title && t.desc && t.icon).toBeTruthy();
      expect(CORE.has(t.id)).toBe(false);
    }
  });
  it('starts a new install with tools that exist', () => {
    const known = new Set(TOOLS.map((t) => t.id));
    for (const id of DEFAULT_PICKS) expect(known.has(id)).toBe(true);
  });
});
