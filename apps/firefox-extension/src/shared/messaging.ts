/** Messages between the page scripts and the background script. Page scripts never talk to the
 *  native app directly; only the background script does. */
export interface DownloadPreset {
  kind: 'video' | 'audio';
  format?: string;
  height?: number;
}

export type ToBackground =
  | { kind: 'download'; url: string; preset?: DownloadPreset }
  | { kind: 'media'; source: string; media: string[] }
  | { kind: 'open'; url: string }
  | { kind: 'ping' };

export interface Result {
  ok: boolean;
  /** A short sentence for the user, ready to show. */
  message: string;
  duplicate?: boolean;
  version?: string;
}

export async function send(message: ToBackground): Promise<Result> {
  try {
    return (await browser.runtime.sendMessage(message)) as Result;
  } catch {
    return { ok: false, message: "Couldn't reach the NomNom extension. Reload the page and try again." };
  }
}
