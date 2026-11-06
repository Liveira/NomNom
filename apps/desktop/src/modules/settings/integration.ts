/** The sentences Settings shows about the right-click menu. Pure, so it can be tested. */
import type { IntegrationStatus } from '../../lib/ipc';

export function menuLine(s: IntegrationStatus): string {
  const { enabled, mode, note } = s.shell;
  if (!enabled) return 'Off. NomNom isn\'t in the right-click menu.';
  if (mode === 'modern') return 'In the Windows 11 right-click menu.';
  if (mode === 'classic') {
    const base = 'In the classic menu: right-click, then choose Show more options.';
    return note ? `${note.replace(/ Using the classic menu \(Show more options\)\.$/, '')} ${base}` : base;
  }
  return note ?? 'Turned on, but not added to the menu yet. Try Repair.';
}

/** Whether the "Repair" button is worth showing. */
export function needsRepair(s: IntegrationStatus): boolean {
  return s.shell.enabled && s.shell.mode === 'none';
}

export function autostartLine(on: boolean): string {
  return on ? 'NomNom starts with Windows, in the tray, without a window.' : 'NomNom starts only when you open it.';
}
