/** The sentences Settings shows about the right-click menu and autostart. Pure, so it can be tested. */
import type { IntegrationStatus } from '../../lib/ipc';

export function menuLine(s: IntegrationStatus): string {
  const { enabled, mode, note } = s.shell;
  if (!enabled) return "Off. NomNom isn't in the right-click menu.";
  if (mode === 'modern') return 'In the Windows 11 right-click menu.';
  if (mode === 'linux') return 'Installed for KDE Dolphin, GNOME Files (Nautilus) and Nemo.';
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

export function autostartLine(on: boolean, platform: 'windows' | 'linux' = 'windows'): string {
  if (!on) return 'NomNom starts only when you open it.';
  return platform === 'linux'
    ? 'NomNom starts on login, in the tray, without a window.'
    : 'NomNom starts with Windows, in the tray, without a window.';
}
