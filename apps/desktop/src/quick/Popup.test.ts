import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  analyze: vi.fn(),
  enqueue: vi.fn(),
  getSettings: vi.fn(),
  context: vi.fn(),
  ready: vi.fn(),
  close: vi.fn(),
}));

vi.mock('../lib/ipc', () => ({
  ipc: { analyze: mocks.analyze, enqueue: mocks.enqueue, getSettings: mocks.getSettings },
  quickIpc: { context: mocks.context, ready: mocks.ready, close: mocks.close },
  onQuickContext: vi.fn(() => Promise.resolve(() => {})),
}));

import Popup from './Popup.svelte';

const analysis = {
  site: 'YouTube',
  known_site: true,
  info: {
    title: 'Me at the zoo',
    uploader: 'jawed',
    duration_secs: 19,
    heights: [1080, 720],
    size_by_height: { '1080': 5 * 1024 * 1024 },
    audio_formats: [],
    entries: null,
  },
};

const settingsWith = (d: Partial<Record<string, string>>) => ({
  downloads: { default_kind: 'video', default_video: 'original', default_audio: 'original', ...d },
});

beforeEach(() => {
  vi.clearAllMocks();
  vi.useFakeTimers({ toFake: ['setTimeout'] });
  mocks.getSettings.mockResolvedValue(settingsWith({}));
  mocks.context.mockResolvedValue({ url: 'https://youtu.be/abc', site: 'YouTube' });
  mocks.enqueue.mockResolvedValue({ outcome: 'queued', job_id: '1' });
  mocks.close.mockResolvedValue(undefined);
  mocks.ready.mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const press = (key: string) => fireEvent.keyDown(window, { key });
const flush = () => vi.advanceTimersByTimeAsync(0);

describe('quick popup', () => {
  it('asks to be shown right away, without waiting for the link lookup', async () => {
    mocks.analyze.mockReturnValue(new Promise(() => {})); // lookup never finishes
    render(Popup);
    // Two animation frames after the context arrives, as the app does before showing the window.
    await vi.waitFor(() => expect(mocks.ready).toHaveBeenCalled(), { timeout: 1000 });
    expect(mocks.analyze).toHaveBeenCalled();
  });

  it('Enter downloads the defaults immediately, before analysis finishes', async () => {
    mocks.analyze.mockReturnValue(new Promise(() => {})); // never resolves
    render(Popup);
    await flush();
    await screen.findByText('Looking up this link…');

    await press('Enter');
    await flush();

    expect(mocks.enqueue).toHaveBeenCalledTimes(1);
    expect(mocks.enqueue).toHaveBeenCalledWith(
      'https://youtu.be/abc',
      { kind: 'video', container: 'original', height: null },
      undefined,
      false,
    );
  });

  it('shows only "Best available" until analysis, then the real resolutions', async () => {
    let resolve!: (v: unknown) => void;
    mocks.analyze.mockReturnValue(new Promise((r) => (resolve = r)));
    render(Popup);
    await flush();

    const quality = screen.getByRole('combobox', { name: 'Quality' }) as HTMLButtonElement;
    // While the lookup runs, nothing but the default is offered, and it can't be opened.
    expect(quality.disabled).toBe(true);
    expect(quality.textContent).toContain('Best available');

    resolve(analysis);
    await screen.findByText('Me at the zoo');
    await fireEvent.keyDown(quality, { key: 'ArrowDown' });
    expect(screen.getAllByRole('option').map((o) => o.textContent?.trim())).toEqual([
      'Best available',
      '1080p · 5.0 MB',
      '720p',
    ]);
  });

  it('arrow keys pick an option, and Enter inside an open list selects instead of downloading', async () => {
    mocks.analyze.mockResolvedValue(analysis);
    render(Popup);
    await screen.findByText('Me at the zoo');

    const quality = screen.getByRole('combobox', { name: 'Quality' });
    await fireEvent.keyDown(quality, { key: 'ArrowDown' }); // open
    await fireEvent.keyDown(quality, { key: 'ArrowDown' }); // 1080p
    await fireEvent.keyDown(quality, { key: 'Enter' }); // select
    expect(mocks.enqueue).not.toHaveBeenCalled();

    await press('Enter'); // list is closed now: confirm
    await flush();
    expect(mocks.enqueue).toHaveBeenCalledWith(
      'https://youtu.be/abc',
      { kind: 'video', container: 'original', height: 1080 },
      'Me at the zoo',
      false,
    );
  });

  it('switching to audio queues the right container with no resolution', async () => {
    mocks.analyze.mockResolvedValue(analysis);
    render(Popup);
    await screen.findByText('Me at the zoo');

    const format = screen.getByRole('combobox', { name: 'Format' });
    await fireEvent.keyDown(format, { key: 'ArrowDown' });
    await fireEvent.keyDown(format, { key: 'ArrowDown' }); // Audio
    await fireEvent.keyDown(format, { key: 'Enter' });
    await press('Enter');
    await flush();
    expect(mocks.enqueue.mock.calls[0][1]).toEqual({ kind: 'audio', container: 'original', height: null });
  });

  it('Escape closes the popup, but only closes the list first when a list is open', async () => {
    mocks.analyze.mockResolvedValue(analysis);
    render(Popup);
    await screen.findByText('Me at the zoo');

    const format = screen.getByRole('combobox', { name: 'Format' });
    await fireEvent.keyDown(format, { key: 'ArrowDown' });
    await fireEvent.keyDown(format, { key: 'Escape' });
    expect(mocks.close).not.toHaveBeenCalled();

    await press('Escape');
    expect(mocks.close).toHaveBeenCalledTimes(1);
  });

  it('leaves nothing from the previous link behind when it closes', async () => {
    mocks.analyze.mockResolvedValue(analysis);
    render(Popup);
    await screen.findByText('Me at the zoo');
    await press('Escape');
    await flush();
    // A popup kept hidden for reuse must not flash the old title the next time it appears.
    expect(screen.queryByText('Me at the zoo')).toBeNull();
    expect(screen.getByText('Looking up this link…')).toBeTruthy();
  });

  it('closes itself shortly after queueing', async () => {
    mocks.analyze.mockResolvedValue(analysis);
    render(Popup);
    await screen.findByText('Me at the zoo');
    await press('Enter');
    await flush();
    expect(screen.getByText('Downloading')).toBeTruthy();
    await vi.advanceTimersByTimeAsync(500);
    expect(mocks.close).toHaveBeenCalled();
  });

  it('a second Enter does not queue the same download twice', async () => {
    mocks.analyze.mockResolvedValue(analysis);
    render(Popup);
    await screen.findByText('Me at the zoo');
    await press('Enter');
    await press('Enter');
    await flush();
    expect(mocks.enqueue).toHaveBeenCalledTimes(1);
  });

  it('offers to download again when the file already exists', async () => {
    mocks.analyze.mockResolvedValue(analysis);
    mocks.enqueue.mockResolvedValueOnce({ outcome: 'already_downloaded', job_id: '1', path: 'C:\\x.mp4' });
    render(Popup);
    await screen.findByText('Me at the zoo');
    await press('Enter');
    await flush();
    await screen.findByText(/already downloaded/);
    expect(mocks.close).not.toHaveBeenCalled();

    await press('Enter');
    await flush();
    expect(mocks.enqueue).toHaveBeenLastCalledWith(expect.any(String), expect.anything(), 'Me at the zoo', true);
  });

  it('does not queue anything when the link could not be read', async () => {
    mocks.analyze.mockRejectedValue('This site isn\'t supported.');
    render(Popup);
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain("isn't supported"));
    await press('Enter');
    await flush();
    expect(mocks.enqueue).not.toHaveBeenCalled();
  });

  it('follows the type and formats chosen in Settings', async () => {
    mocks.getSettings.mockResolvedValue(settingsWith({ default_kind: 'audio', default_audio: 'mp3', default_video: 'mp4' }));
    mocks.analyze.mockReturnValue(new Promise(() => {}));
    render(Popup);
    await flush();
    // Starts on Audio with the saved audio format, and a bare Enter downloads exactly that.
    expect(screen.getByRole('combobox', { name: 'Format' }).textContent).toContain('Audio · MP3');
    await press('Enter');
    await flush();
    expect(mocks.enqueue.mock.calls[0][1]).toEqual({ kind: 'audio', container: 'mp3', height: null });
  });

  it('uses the saved video format when downloading video', async () => {
    mocks.getSettings.mockResolvedValue(settingsWith({ default_video: 'mp4' }));
    mocks.analyze.mockReturnValue(new Promise(() => {}));
    render(Popup);
    await flush();
    expect(screen.getByRole('combobox', { name: 'Format' }).textContent).toContain('Video · MP4');
    await press('Enter');
    await flush();
    expect(mocks.enqueue.mock.calls[0][1]).toEqual({ kind: 'video', container: 'mp4', height: null });
  });

  it('still works if Settings cannot be read', async () => {
    mocks.getSettings.mockRejectedValue(new Error('offline'));
    mocks.analyze.mockReturnValue(new Promise(() => {}));
    render(Popup);
    await flush();
    await press('Enter');
    await flush();
    expect(mocks.enqueue.mock.calls[0][1]).toEqual({ kind: 'video', container: 'original', height: null });
  });
});
