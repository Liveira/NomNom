<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { app } from '../../app/state.svelte';
  import { AUDIO_FORMATS, VIDEO_FORMATS, defaultName, formatNote } from '../../lib/formats';
  import { ipc, type Container, type Kind, type SettingsPatch } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import { downloads } from '../downloads/store.svelte';

  const dl = $derived(app.settings?.downloads);
  const kind = $derived<Kind>(dl?.default_kind ?? 'video');
  const video = $derived<Container>(dl?.default_video ?? 'original');
  const audio = $derived<Container>(dl?.default_audio ?? 'original');

  let defaultDir = $state('');
  let message = $state('');
  let updating = $state(false);
  void ipc.defaultDownloadDir().then((d) => (defaultDir = d));
  void downloads.refreshTools();

  async function save(patch: SettingsPatch) {
    message = '';
    try {
      await app.update(patch);
    } catch (e) {
      message = String(e);
    }
  }

  async function chooseFolder() {
    const picked = await open({ directory: true, multiple: false, defaultPath: dl?.output_dir ?? defaultDir });
    if (typeof picked === 'string') await save({ outputDir: picked });
  }

  async function updateYtdlp() {
    updating = true;
    message = '';
    try {
      const v = await ipc.updateYtdlp();
      message = v ? `yt-dlp is up to date (${v}).` : 'Checked for updates.';
    } catch (e) {
      message = String(e);
    } finally {
      updating = false;
      await downloads.refreshTools();
    }
  }

  const saveTo = $derived(dl?.output_dir ?? defaultDir);
</script>

<section class="group" aria-labelledby="dl-format">
  <h3 id="dl-format">NomNom Downloader · default format</h3>
  <p class="intro">
    This is what a link downloads as when you don't pick anything: with the shortcut, the quick
    popup and the browser button. You can still choose something else for any single link.
  </p>

  <div class="row">
    <div>
      <div class="label">Download as</div>
      <div class="hint">Video keeps the picture and sound. Audio saves only the sound.</div>
    </div>
    <div class="segmented" role="radiogroup" aria-label="Download as">
      <button role="radio" aria-checked={kind === 'video'} class:on={kind === 'video'} onclick={() => save({ defaultKind: 'video' })}>Video</button>
      <button role="radio" aria-checked={kind === 'audio'} class:on={kind === 'audio'} onclick={() => save({ defaultKind: 'audio' })}>Audio</button>
    </div>
  </div>

  <div class="row stack">
    <div class="line">
      <label class="label" for="fmt-video">Video format</label>
      <select id="fmt-video" value={video} onchange={(e) => save({ defaultVideo: e.currentTarget.value as Container })}>
        {#each VIDEO_FORMATS as f (f.value)}<option value={f.value}>{f.label}</option>{/each}
      </select>
    </div>
    <div class="hint">{formatNote('video', video)}</div>
  </div>

  <div class="row stack">
    <div class="line">
      <label class="label" for="fmt-audio">Audio format</label>
      <select id="fmt-audio" value={audio} onchange={(e) => save({ defaultAudio: e.currentTarget.value as Container })}>
        {#each AUDIO_FORMATS as f (f.value)}<option value={f.value}>{f.label}</option>{/each}
      </select>
    </div>
    <div class="hint">{formatNote('audio', audio)}</div>
  </div>

  <p class="summary" role="status">
    Right now, a link downloads as <strong>{defaultName(kind, video, audio)}</strong>, at the best quality available.
  </p>
</section>

<section class="group" aria-labelledby="dl-files">
  <h3 id="dl-files">NomNom Downloader · files</h3>
  <div class="row">
    <div>
      <div class="label">Save to</div>
      <div class="hint path">{saveTo}</div>
    </div>
    <div class="buttons">
      <button class="plain" onclick={chooseFolder}>Choose folder</button>
      {#if dl?.output_dir}<button class="plain" onclick={() => save({ outputDir: '' })}>Use Downloads</button>{/if}
    </div>
  </div>
  <div class="row">
    <div>
      <div class="label">Sort into folders</div>
      <div class="hint">
        Videos go in a Videos folder and audio in an Audio folder, inside the folder above, so
        nothing lands in a pile. Example: <span class="path">{saveTo}\Videos\clip.mp4</span>
      </div>
    </div>
    <Switch checked={dl?.sort_into_folders ?? true} label="Sort into folders" onchange={(v) => save({ sortIntoFolders: v })} />
  </div>
  <div class="row">
    <div class="label">Downloads at the same time</div>
    <select
      aria-label="Downloads at the same time"
      value={dl?.max_concurrent ?? 3}
      onchange={(e) => save({ maxConcurrent: Number(e.currentTarget.value) })}
    >
      {#each [1, 2, 3, 4, 5, 6, 8] as n (n)}<option value={n}>{n}</option>{/each}
    </select>
  </div>
  <div class="row">
    <div>
      <div class="label">Allow downloading the same link again</div>
      <div class="hint">When off, NomNom asks before downloading something you already have.</div>
    </div>
    <Switch checked={dl?.allow_duplicates ?? false} label="Allow downloading the same link again" onchange={(v) => save({ allowDuplicates: v })} />
  </div>
  <div class="row">
    <div class="label">Show a notification when downloads finish</div>
    <Switch checked={dl?.notify ?? true} label="Show a notification when downloads finish" onchange={(v) => save({ notifyDownloads: v })} />
  </div>
  <div class="row">
    <div>
      <div class="label">Use Firefox cookies for sites that need a sign-in</div>
      <div class="hint">NomNom reads your Firefox cookies only while downloading, and only on this computer. Off by default.</div>
    </div>
    <Switch checked={dl?.use_firefox_cookies ?? false} label="Use Firefox cookies" onchange={(v) => save({ firefoxCookies: v })} />
  </div>

  <h3 class="sub">Tools</h3>
  {#if downloads.tools}
    {#each [['yt-dlp', downloads.tools.ytdlp, 'ytdlp'], ['FFmpeg', downloads.tools.ffmpeg, 'ffmpeg']] as const as [name, tool, id] (id)}
      <div class="row">
        <div>
          <div class="label">{name}</div>
          <div class="hint">
            {#if tool.path}{tool.version ?? 'Installed'}{tool.managed ? '' : ' · found on this computer'}{:else}Not installed{/if}
          </div>
        </div>
        <div class="buttons">
          {#if !tool.path}
            <button class="plain" disabled={downloads.installing != null} onclick={() => downloads.install(id)}>Get {name}</button>
          {:else if id === 'ytdlp' && tool.managed}
            <button class="plain" disabled={updating} onclick={updateYtdlp}>{updating ? 'Checking…' : 'Update'}</button>
          {/if}
        </div>
      </div>
    {/each}
  {/if}
  {#if message}<p class="hint" role="status">{message}</p>{/if}
</section>

<style>
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 8px; }
  h3.sub { margin-top: 22px; margin-bottom: 12px; }
  .intro { margin: 0 0 14px; color: var(--text-2); }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .row + .row { margin-top: 14px; }
  .row.stack { flex-direction: column; align-items: stretch; gap: 4px; }
  .line { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .path { font-variant-numeric: tabular-nums; word-break: break-all; }
  .buttons { display: flex; gap: 8px; flex: none; }
  .plain, select {
    padding: 6px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); cursor: pointer;
  }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .plain:disabled { opacity: 0.5; cursor: default; }
  .summary { margin: 16px 0 0; padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-2); color: var(--text-2); }
  .summary strong { color: var(--text); }
  .segmented { display: inline-flex; background: var(--surface-2); border-radius: var(--radius-s); padding: 2px; flex: none; }
  .segmented button { border: 0; background: transparent; color: var(--text-2); padding: 5px 14px; border-radius: 6px; cursor: pointer; transition: background var(--dur-1), color var(--dur-1); }
  .segmented button.on { background: var(--surface-3); color: var(--text); }
</style>
