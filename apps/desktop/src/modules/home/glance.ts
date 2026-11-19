/** The drive the person cares about most: the one Windows or the system lives on, else the biggest fixed one. */
export function systemDisk<T extends { mount: string; total: number; removable: boolean }>(disks: T[]): T | null {
  const fixed = disks.filter((d) => !d.removable && d.total > 0);
  if (fixed.length === 0) return null;
  const root = fixed.find((d) => /^[cC]:/.test(d.mount) || d.mount === '/');
  if (root) return root;
  return [...fixed].sort((a, b) => b.total - a.total)[0];
}
